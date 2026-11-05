import { useCallback, useEffect, useRef, useState } from 'react'
import { cancelTurn, submitMessage } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import type { ChatLink } from '@/features/workspace/model/chatView'
import { hasPendingApproval, type ChatFeed } from './chatFeed'
import { newQueuedMessage, updateComposer, useComposerState, type QueuedMessage } from './composerStore'

/**
 * After a message is sent the daemon takes a moment to report the turn as working. Until this long
 * has passed the next queued message waits, so it is not sent into the turn that is just starting.
 */
export const DELIVERY_GRACE_MS = 2500

export const PENDING_APPROVAL_NOTE = 'Answer the request above first'
export const READ_ONLY_NOTE = 'Read-only: talk to the agent in the terminal'

type Input = {
  link: ChatLink
  feed: ChatFeed
  /** The daemon is connected, so a message can be sent. */
  online: boolean
  /** Reads the conversation again now. */
  refresh: () => void
}

export type ChatComposer = {
  draft: string
  setDraft: (text: string) => void
  queue: readonly QueuedMessage[]
  removeQueued: (id: number) => void
  /** Sends the draft, or queues it while the agent works. */
  submit: () => void
  /** Interrupts the running turn. */
  stop: () => void
  running: boolean
  sending: boolean
  /** Why the composer is off, or `null` when it takes input. */
  disabledReason: string | null
  /** The last send that failed, or `null`. Cleared by the next send. */
  error: string | null
}

/**
 * The composer's behaviour. A message goes to the daemon through the conversation's own send, with
 * an operation number the daemon uses to tell a retry from a new message. While a turn runs, the
 * message is queued in the window and delivered once the daemon reports the turn finished (the
 * daemon has no queue of its own, so delivery is at the end of the turn, not at a step boundary,
 * and only while this chat is open). While an approval waits, nothing is sent.
 */
export function useChatComposer({ link, feed, online, refresh }: Input): ChatComposer {
  const { draft, queue } = useComposerState(link.conversation)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [graceTick, setGraceTick] = useState(0)
  const lastSentAt = useRef(Number.NEGATIVE_INFINITY)
  const lastOperation = useRef(0)
  // A send that failed is tried again with the same operation number, so the daemon can tell it is the same message.
  const retry = useRef<{ text: string; operation: number } | null>(null)

  const readOnly = link.readOnly === true
  // A read-only chat cannot interrupt or queue, so it never offers Stop or "queue" wording.
  const running = feed.status === 'Working' && !readOnly
  const awaitingAnswer = hasPendingApproval(feed)
  const disabledReason = readOnly ? READ_ONLY_NOTE : !online ? 'The daemon is offline' : awaitingAnswer ? PENDING_APPROVAL_NOTE : null
  const conversation = link.conversation

  const setDraft = useCallback((text: string) => {
    updateComposer(conversation, (state) => (state.draft === text ? state : { ...state, draft: text }))
  }, [conversation])

  const send = useCallback(async (text: string): Promise<boolean> => {
    const operation = retry.current?.text === text
      ? retry.current.operation
      : Math.max(feed.receipt?.operation ?? 0, lastOperation.current) + 1
    retry.current = { text, operation }
    lastOperation.current = Math.max(lastOperation.current, operation)
    setSending(true)
    setError(null)
    try {
      await submitMessage({ id: String(conversation), generation: link.generation }, operation, text)
      retry.current = null
      lastSentAt.current = Date.now()
      refresh()
      return true
    } catch (failure) {
      setError(errorMessage(failure, 'Could not send the message'))
      return false
    } finally {
      setSending(false)
    }
  }, [conversation, link.generation, feed.receipt, refresh])

  const submit = useCallback(() => {
    const text = draft.trim()
    if (text === '' || disabledReason !== null || sending) return
    if (running || queue.length > 0) {
      updateComposer(conversation, (state) => ({ draft: '', queue: [...state.queue, newQueuedMessage(text)] }))
      return
    }
    setDraft('')
    void send(text).then((delivered) => {
      // A message that did not go is given back, unless something new was typed meanwhile.
      if (!delivered) updateComposer(conversation, (state) => (state.draft === '' ? { ...state, draft: text } : state))
    })
  }, [draft, disabledReason, sending, running, queue.length, conversation, setDraft, send])

  const removeQueued = useCallback((id: number) => {
    updateComposer(conversation, (state) => ({ ...state, queue: state.queue.filter((message) => message.id !== id) }))
  }, [conversation])

  const stop = useCallback(() => {
    if (!running || feed.cancelling) return
    cancelTurn({ id: String(conversation), generation: link.generation })
      .then(refresh)
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not interrupt the turn')) })
  }, [running, feed.cancelling, conversation, link.generation, refresh])

  // Delivers the oldest queued message once the turn is over and nothing waits on the person.
  useEffect(() => {
    const [next] = queue
    // After a failed delivery it waits for the person, who sees the error, instead of trying again in a loop.
    if (next === undefined || running || awaitingAnswer || sending || !online || feed.status === null || error !== null) return
    const wait = lastSentAt.current + DELIVERY_GRACE_MS - Date.now()
    if (wait > 0) {
      const timer = window.setTimeout(() => { setGraceTick((tick) => tick + 1) }, wait)
      return () => { window.clearTimeout(timer) }
    }
    updateComposer(conversation, (state) => ({ ...state, queue: state.queue.filter((message) => message.id !== next.id) }))
    void send(next.text).then((delivered) => {
      // A queued message that did not go returns to the front of the queue.
      if (!delivered) updateComposer(conversation, (state) => ({ ...state, queue: [next, ...state.queue] }))
    })
  }, [queue, running, awaitingAnswer, sending, online, feed.status, error, conversation, send, graceTick])

  return { draft, setDraft, queue, removeQueued, submit, stop, running, sending, disabledReason, error }
}
