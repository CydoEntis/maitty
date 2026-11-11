import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { readConversationPage } from '@/api/daemon'
import { listenConversationChanged } from '@/api/conversationNews'
import type { ApprovalDecision, ChatProvider } from './chatEvents'
import { afterFailure, createFeed, hasPendingApproval, syncFeed, type ChatFeed, type ReadPage } from './chatFeed'
import { recordApprovalAnswer, selectChatEvents } from './projection'

/** How often an open chat asks the daemon for news: quickly while the agent works or waits on a person, slowly otherwise. */
export const ACTIVE_POLL_MS = 300
export const IDLE_POLL_MS = 3000
/**
 * Once the daemon has said this conversation changed, it is the daemon that wakes the chat, and a
 * working chat asks on its own only this often, in case a change went unsaid.
 */
export const TOLD_ACTIVE_POLL_MS = 1000
/** Reads the daemon's news asks for are at least this far apart: a reply streams in many small changes. */
export const NEWS_READ_GAP_MS = 100

/** The feeds the window has read, so reopening a chat shows its history at once and reads only what changed. */
const CACHE_LIMIT = 8
const cache = new Map<number, ChatFeed>()

function remember(feed: ChatFeed): void {
  cache.delete(feed.conversation)
  cache.set(feed.conversation, feed)
  const oldest = cache.keys().next()
  if (cache.size > CACHE_LIMIT && !oldest.done) cache.delete(oldest.value)
}

/** The feed this window last read for the conversation, for code outside the chat that must number a message after it. */
export function peekChatFeed(conversation: number): ChatFeed | undefined {
  return cache.get(conversation)
}

/** For tests: the cache belongs to the window's lifetime. */
export function forgetChatFeeds(): void {
  cache.clear()
}

function isActive(feed: ChatFeed): boolean {
  return feed.status === 'Working' || feed.status === 'Blocked' || hasPendingApproval(feed) || feed.queued.length > 0
}

/** The feed this window already read for the conversation, unless the conversation has started over since. */
function startFeed(provider: ChatProvider, conversation: number, generation: number): ChatFeed {
  const held = cache.get(conversation)
  return held && (held.generation === null || held.generation === generation) ? held : createFeed(provider, conversation)
}

type Options = {
  conversation: number
  /** The generation the session list reports; a different one means the conversation started over. */
  generation: number
  provider: ChatProvider
}

/**
 * Follows one conversation's history for as long as the chat is open. The first read pages the
 * whole history; after that each poll asks only for the last page and applies what is new
 * (see `syncFeed`). A failed read keeps what is shown, marks the chat offline and tries again; the
 * next read starts from the beginning, so nothing is shown twice and nothing is missed. The daemon
 * also says when the conversation changed (`listenConversationChanged`), and the chat reads then.
 */
export function useChatFeed({ conversation, generation, provider }: Options) {
  const [feed, setFeed] = useState<ChatFeed>(() => startFeed(provider, conversation, generation))
  const [forGeneration, setForGeneration] = useState(generation)
  const [offline, setOffline] = useState(false)
  // Why the last read failed, so the chat can say it; never the conversation's own text.
  const [readError, setReadError] = useState<string | null>(null)
  // A conversation that started over (a new generation) shows nothing of the old one while the new one is read.
  if (forGeneration !== generation) {
    setForGeneration(generation)
    setFeed(createFeed(provider, conversation))
  }
  // How many reads brought what the person was not there to watch arrive: the first since the chat opened (before it
  // the feed is the one this window remembered) and each one after the window was hidden. The chat shows what such a
  // read holds whole, and types in only what comes while it is being looked at.
  const [unwatchedReads, setUnwatchedReads] = useState(0)
  const latest = useRef(feed)
  const wake = useRef<() => void>(() => undefined)

  useEffect(() => {
    const life = { stopped: false }
    // A function, so a check after an await is not taken for the one before it.
    const gone = () => life.stopped
    let timer = 0
    let running = false
    // A read was asked for while one ran: at once (the person did something) or after the gap (the daemon's news).
    let again: 'now' | 'soon' | null = null
    let told = false
    let lastReadAt = 0
    let unwatched = true
    latest.current = startFeed(provider, conversation, generation)
    const read: ReadPage = (cursor) => readConversationPage(conversation, cursor)

    const tick = async () => {
      if (gone()) return
      // A refresh that arrives during a read waits for it, then reads once more.
      if (running) {
        again = 'now'
        return
      }
      if (document.hidden) {
        unwatched = true
        timer = window.setTimeout(() => { void tick() }, IDLE_POLL_MS)
        return
      }
      running = true
      try {
        const next = await syncFeed(latest.current, read, Date.now)
        if (gone()) return
        latest.current = next
        remember(next)
        setFeed(next)
        if (unwatched) {
          unwatched = false
          setUnwatchedReads((count) => count + 1)
        }
        setOffline(false)
        setReadError(null)
      } catch (error) {
        if (gone()) return
        // Not the text of the conversation: only that the read failed.
        const reason = error instanceof Error ? error.message : String(error)
        console.warn('Could not read the conversation', reason)
        latest.current = afterFailure(latest.current)
        setOffline(true)
        setReadError(reason)
      } finally {
        running = false
        lastReadAt = Date.now()
      }
      const poll = !isActive(latest.current) ? IDLE_POLL_MS : told ? TOLD_ACTIVE_POLL_MS : ACTIVE_POLL_MS
      const delay = again === 'now' ? 0 : again === 'soon' ? NEWS_READ_GAP_MS : poll
      again = null
      timer = window.setTimeout(() => { void tick() }, delay)
    }
    const onNews = (changed: number) => {
      if (gone() || changed !== conversation) return
      told = true
      if (running) {
        again ??= 'soon'
        return
      }
      window.clearTimeout(timer)
      timer = window.setTimeout(() => { void tick() }, Math.max(0, NEWS_READ_GAP_MS - (Date.now() - lastReadAt)))
    }
    const listening = listenConversationChanged(onNews)
    wake.current = () => {
      window.clearTimeout(timer)
      void tick()
    }
    void tick()
    return () => {
      life.stopped = true
      window.clearTimeout(timer)
      void listening.then((stop) => { stop() })
    }
  }, [conversation, generation, provider])

  /** Reads again now, for after an answer or a message was sent. */
  const refresh = useCallback(() => { wake.current() }, [])

  /** Shows an approval as answered with the person's choice as soon as the daemon has taken it. */
  const recordAnswer = useCallback((requestId: number | string, decision: ApprovalDecision) => {
    const next = { ...latest.current, projection: recordApprovalAnswer(latest.current.projection, requestId, decision) }
    latest.current = next
    remember(next)
    setFeed(next)
  }, [])

  const events = useMemo(() => selectChatEvents(feed.projection), [feed.projection])
  return { feed, events, offline, readError, refresh, recordAnswer, unwatchedReads }
}
