import { useCallback, useMemo, useRef, useState } from 'react'
import { cancelTurn, dequeueMessage, submitMessage } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import type { ChatLink } from '@/features/workspace/model/chatView'
import { hasPendingApproval, type ChatFeed } from './chatFeed'
import { buildAttachmentMessage } from './attachmentMessage'
import { peekComposer, updateComposer, useComposerState, type DraftAttachment } from './composerStore'
import { revokePreviews, useAttachmentIntake, type AttachmentIntake } from './useAttachmentIntake'
import { reserveOperation, STALE_OPERATION } from './sendOperation'

export const PENDING_APPROVAL_NOTE = 'Answer the request above first'
export const READ_ONLY_NOTE = 'Read-only: talk to the agent in the terminal'
/** Why a chat that types into the agent's terminal is off while the agent is busy or asking. */
export const TYPING_BUSY_NOTE = 'The agent is working. Stop it to send something else'
export const TYPING_ASKING_NOTE = 'The agent is asking something in the terminal. Answer it there first'
/** Sent to the terminal around a message, so a multi-line message is one paste and not several Enters. */
export const PASTE_START = '\x1b[200~'
export const PASTE_END = '\x1b[201~'
/** The key that interrupts the agent's turn. */
export const INTERRUPT_KEY = '\x1b'

/** A message the daemon holds until the running turn ends. `id` is its operation number. */
export type QueuedItem = { readonly id: number; readonly text: string }

type Input = {
  link: ChatLink
  feed: ChatFeed
  /** The daemon is connected, so a message can be sent. */
  online: boolean
  /** Reads the conversation again now. */
  refresh: () => void
  /**
   * Types into the terminal the agent runs in (ADR 0056). When given, a chat that only follows the
   * agent (ADR 0052) has a working composer while the agent is idle: the message is pasted into the
   * agent's own prompt, so the chat and the terminal are one session.
   */
  typeInto?: (text: string) => Promise<void>
}

export type ChatComposer = {
  draft: string
  setDraft: (text: string) => void
  /** The files attached to the draft, in the order they were added. */
  attachments: readonly DraftAttachment[]
  /** Adding and removing them. */
  attach: AttachmentIntake
  /** The chat only follows an agent running in the terminal (ADR 0052): there is nothing to attach to. */
  readOnly: boolean
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
export function useChatComposer({ link, feed, online, refresh, typeInto }: Input): ChatComposer {
  const { draft, attachments } = useComposerState(link.conversation)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // A send that failed is tried again with the same operation number, so the daemon can tell it is the same message.
  const retry = useRef<{ text: string; operation: number } | null>(null)

  const readOnly = link.readOnly === true
  // A chat that follows a terminal agent talks to it by typing, when it has a terminal to type in.
  const typeText = readOnly ? typeInto : undefined
  const typing = typeText !== undefined
  // Without that it cannot interrupt or queue, so it never offers Stop or "queue" wording.
  const running = feed.status === 'Working' && (!readOnly || typing)
  const awaitingAnswer = hasPendingApproval(feed)
  // Typed text answers a prompt the agent is showing, and the chat cannot tell the difference, so
  // it types only while the agent is known to be waiting for a message.
  const waitingForMessage = feed.status === 'Idle' || feed.status === 'Done'
  const disabledReason = typing
    ? (!online ? 'The daemon is offline' : feed.status === 'Working' ? TYPING_BUSY_NOTE : awaitingAnswer || feed.status === 'Blocked' ? TYPING_ASKING_NOTE : !waitingForMessage ? 'Waiting for the agent to be ready' : null)
    : readOnly ? READ_ONLY_NOTE : !online ? 'The daemon is offline' : awaitingAnswer ? PENDING_APPROVAL_NOTE : null
  const conversation = link.conversation
  const attach = useAttachmentIntake({ conversation, blocked: disabledReason, machine: link.machine ?? null })
  const queue = useMemo<readonly QueuedItem[]>(() => feed.queued.map((message) => ({ id: message.operation, text: message.text })), [feed.queued])

  const setDraft = useCallback((text: string) => {
    updateComposer(conversation, (state) => (state.draft === text ? state : { ...state, draft: text }))
  }, [conversation])

  const send = useCallback(async (text: string): Promise<boolean> => {
    setSending(true)
    setError(null)
    if (typeText !== undefined) {
      try {
        // One paste, then Enter: the agent takes a multi-line message as one message.
        await typeText(`${PASTE_START}${text}${PASTE_END}\r`)
        refresh()
        return true
      } catch (failure) {
        setError(errorMessage(failure, 'Could not type into the terminal'))
        return false
      } finally {
        setSending(false)
      }
    }
    const operation = retry.current?.text === text ? retry.current.operation : reserveOperation(conversation, feed)
    retry.current = { text, operation }
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
  }, [conversation, link.generation, feed, refresh, typeText])

  const submit = useCallback(() => {
    const text = draft.trim()
    // A message may be only attachments; the block naming them is then the whole text (ADR 0053).
    if ((text === '' && attachments.length === 0) || disabledReason !== null || sending) return
    const files = attachments
    updateComposer(conversation, (state) => ({ ...state, draft: '', attachments: [] }))
    void send(buildAttachmentMessage(text, files)).then((delivered) => {
      if (delivered) {
        revokePreviews(files)
        return
      }
      // A message that did not go is given back, unless something new was typed or attached meanwhile.
      const now = peekComposer(conversation)
      if (now.draft !== '' || now.attachments.length > 0) {
        revokePreviews(files)
        return
      }
      updateComposer(conversation, () => ({ draft: text, attachments: files }))
    })
  }, [draft, attachments, disabledReason, sending, conversation, send])

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
    if (typeText !== undefined) {
      typeText(INTERRUPT_KEY).catch((failure: unknown) => { setError(errorMessage(failure, 'Could not interrupt the agent')) })
      return
    }
    cancelTurn({ id: String(conversation), generation: link.generation })
      .then(refresh)
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not interrupt the turn')) })
  }, [running, feed.cancelling, conversation, link.generation, refresh, typeText])

  return { draft, setDraft, attachments, attach, readOnly, queue, removeQueued, submit, stop, running, sending, disabledReason, error }
}
