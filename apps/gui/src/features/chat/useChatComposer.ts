import { useCallback, useMemo, useRef, useState } from 'react'
import { cancelTurn, dequeueMessage, submitMessage } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import type { ChatLink } from '@/features/workspace/model/chatView'
import { hasPendingApproval, type ChatFeed } from './chatFeed'
import { updateComposer, useComposerState } from './composerStore'

export const PENDING_APPROVAL_NOTE = 'Answer the request above first'
export const READ_ONLY_NOTE = 'Read-only: talk to the agent in the terminal'

/** The daemon refuses an operation number it has already used or passed; a retry must pick a new one. */
const STALE_OPERATION = /stale or reused/i

/** A message the daemon holds until the running turn ends. `id` is its operation number. */
export type QueuedItem = { readonly id: number; readonly text: string }

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
  /** What the daemon holds for the running turn, oldest first. */
  queue: readonly QueuedItem[]
  removeQueued: (id: number) => void
  /** Sends the draft; while the agent works the daemon holds it and sends it when the turn ends. */
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
 * an operation number the daemon uses to tell a retry from a new message. While a turn runs the
 * daemon holds the message and delivers it when the turn ends, in order and once each, whether or
 * not this window stays open; what it holds comes back with every read (`feed.queued`) and is shown
 * until it is sent. While an approval waits, nothing is sent.
 */
export function useChatComposer({ link, feed, online, refresh }: Input): ChatComposer {
  const { draft } = useComposerState(link.conversation)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const lastOperation = useRef(0)
  // A send that failed is tried again with the same operation number, so the daemon can tell it is the same message.
  const retry = useRef<{ text: string; operation: number } | null>(null)

  const readOnly = link.readOnly === true
  // A read-only chat cannot interrupt or queue, so it never offers Stop or "queue" wording.
  const running = feed.status === 'Working' && !readOnly
  const awaitingAnswer = hasPendingApproval(feed)
  const disabledReason = readOnly ? READ_ONLY_NOTE : !online ? 'The daemon is offline' : awaitingAnswer ? PENDING_APPROVAL_NOTE : null
  const conversation = link.conversation
  const queue = useMemo<readonly QueuedItem[]>(() => feed.queued.map((message) => ({ id: message.operation, text: message.text })), [feed.queued])

  const setDraft = useCallback((text: string) => {
    updateComposer(conversation, (state) => (state.draft === text ? state : { ...state, draft: text }))
  }, [conversation])

  const send = useCallback(async (text: string): Promise<boolean> => {
    const waitingOperation = feed.queued.reduce((highest, message) => Math.max(highest, message.operation), 0)
    const operation = retry.current?.text === text
      ? retry.current.operation
      : Math.max(feed.receipt?.operation ?? 0, waitingOperation, lastOperation.current) + 1
    retry.current = { text, operation }
    lastOperation.current = Math.max(lastOperation.current, operation)
    setSending(true)
    setError(null)
    try {
      await submitMessage({ id: String(conversation), generation: link.generation }, operation, text)
      retry.current = null
      refresh()
      return true
    } catch (failure) {
      const message = errorMessage(failure, 'Could not send the message')
      setError(message)
      // A number the daemon has passed (another window sent in between) is not tried again; read, then pick a new one.
      if (STALE_OPERATION.test(message)) {
        retry.current = null
        refresh()
      }
      return false
    } finally {
      setSending(false)
    }
  }, [conversation, link.generation, feed.receipt, feed.queued, refresh])

  const submit = useCallback(() => {
    const text = draft.trim()
    if (text === '' || disabledReason !== null || sending) return
    setDraft('')
    void send(text).then((delivered) => {
      // A message that did not go is given back, unless something new was typed meanwhile.
      if (!delivered) updateComposer(conversation, (state) => (state.draft === '' ? { ...state, draft: text } : state))
    })
  }, [draft, disabledReason, sending, conversation, setDraft, send])

  const removeQueued = useCallback((operation: number) => {
    dequeueMessage({ id: String(conversation), generation: link.generation }, operation)
      .then(refresh)
      .catch((failure: unknown) => {
        setError(errorMessage(failure, 'Could not remove the queued message'))
        // Most often it was sent a moment ago: show what the daemon has now.
        refresh()
      })
  }, [conversation, link.generation, refresh])

  const stop = useCallback(() => {
    if (!running || feed.cancelling) return
    cancelTurn({ id: String(conversation), generation: link.generation })
      .then(refresh)
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not interrupt the turn')) })
  }, [running, feed.cancelling, conversation, link.generation, refresh])

  return { draft, setDraft, queue, removeQueued, submit, stop, running, sending, disabledReason, error }
}
