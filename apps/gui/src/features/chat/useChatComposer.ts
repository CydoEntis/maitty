import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { cancelTurn, dequeueMessage, submitMessage } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import type { ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import { hasPendingApproval, type ChatFeed } from './chatFeed'
import { buildAttachmentMessage } from './attachmentMessage'
import { selectChatEvents } from './projection'
import { countUserMessages } from './workPhase'
import { peekComposer, updateComposer, useComposerState, type DraftAttachment } from './composerStore'

import { revokePreviews, useAttachmentIntake, type AttachmentIntake } from './useAttachmentIntake'
import { wireChoice } from './agentChoice'
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

/** How long a message just sent stays on screen before the history shows it; past this it is dropped rather than left as a claim. */
const ECHO_MAX_MS = 20_000
/** How long a message just sent claims the agent is thinking; after this the status itself has to say it. */
const ECHO_THINKING_MS = 4_000
/** A second read soon after a send, because a quiet chat is read only every few seconds. */
const FOLLOW_UP_REFRESH_MS = 700

/** Held messages count down from -1, so they never share a number with the daemon's own operations. */
let heldCounter = 0
const nextHeldId = (): number => { heldCounter -= 1; return heldCounter }
/** After a held message is typed in, the next waits this long for the agent to start on it, so two never go at once. */
const HELD_GAP_MS = 4_000

/** What was just sent into the terminal, and how many times those words were already in the history. */
export type SentEcho = { readonly text: string; readonly baseline: number; readonly quiet: boolean }

/** A message the daemon holds until the running turn ends. `id` is its operation number. */
export type QueuedItem = { readonly id: number; readonly text: string }

type Input = {
  link: ChatLink
  /** The chat's agent: the model and reasoning picked for it go with each message. */
  agent?: ChatAgent
  feed: ChatFeed
  /** The daemon is connected, so a message can be sent. */
  online: boolean
  /** Reads the conversation again now. */
  refresh: () => void
  /**
   * Types into the terminal the agent runs in (ADR 0003). When given, a chat that only follows the
   * agent (ADR 0003) has a working composer while the agent is idle: the message is pasted into the
   * agent's own prompt, so the chat and the terminal are one session.
   */
  typeInto?: (text: string) => Promise<void>
  /** The chat shows the prompt the agent is stopped on with buttons that answer it, so the composer points there. */
  answersPrompt?: boolean
  /** The pane itself says its agent is working, which is known before the conversation's next read says so. */
  paneWorking?: boolean
}

export type ChatComposer = {
  draft: string
  setDraft: (text: string) => void
  /** The files attached to the draft, in the order they were added. */
  attachments: readonly DraftAttachment[]
  /** Adding and removing them. */
  attach: AttachmentIntake
  /** The chat only follows an agent running in the terminal (ADR 0003): there is nothing to attach to. */
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
  /** The agent can take a message this moment: the box is on and nothing is running (a held message waits for this). */
  ready: boolean
  /** The last send that failed, or `null`. Cleared by the next send. */
  error: string | null
  /** Sends this text now, as `submit` would send the draft; true when it went. For a message held while the agent started. */
  sendText: (text: string) => Promise<boolean>
  /** The message just sent, until the history shows it; null otherwise. */
  echo: SentEcho | null
}

/**
 * The composer's behaviour. A message goes to the daemon through the conversation's own send, with
 * an operation number the daemon uses to tell a retry from a new message. While a turn runs the
 * daemon holds the message and delivers it when the turn ends, in order and once each, whether or
 * not this window stays open; what it holds comes back with every read (`feed.queued`) and is shown
 * until it is sent. While an approval waits, nothing is sent.
 */
export function useChatComposer({ link, agent, feed, online, refresh, typeInto, answersPrompt = false, paneWorking = false }: Input): ChatComposer {
  const { draft, attachments, held } = useComposerState(link.conversation)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [echo, setEcho] = useState<SentEcho | null>(null)
  // A send that failed is tried again with the same operation number, so the daemon can tell it is the same message.
  const retry = useRef<{ text: string; operation: number } | null>(null)

  const readOnly = link.readOnly === true
  // A chat that follows a terminal agent talks to it by typing, when it has a terminal to type in.
  const typeText = readOnly ? typeInto : undefined
  const typing = typeText !== undefined
  // Without that it cannot interrupt or queue, so it never offers Stop or "queue" wording.
  const busy = feed.status === 'Working' || paneWorking
  const running = busy && (!readOnly || typing)
  const awaitingAnswer = hasPendingApproval(feed)
  // Typed text answers a prompt the agent is showing, and the chat cannot tell the difference, so
  // it types only while the agent is known to be waiting for a message.
  const waitingForMessage = feed.status === 'Idle' || feed.status === 'Done'
  // A terminal agent that is working still takes a message: it is held here and typed in once the agent is ready, so
  // nothing reaches a prompt the agent might be showing meanwhile.
  const disabledReason = typing
    ? (!online ? 'The daemon is offline' : awaitingAnswer || feed.status === 'Blocked' ? (answersPrompt ? PENDING_APPROVAL_NOTE : TYPING_ASKING_NOTE) : !busy && !waitingForMessage ? 'Waiting for the agent to be ready' : null)
    : readOnly ? READ_ONLY_NOTE : !online ? 'The daemon is offline' : awaitingAnswer ? PENDING_APPROVAL_NOTE : null
  const conversation = link.conversation
  const attach = useAttachmentIntake({ conversation, blocked: disabledReason, machine: link.machine ?? null })
  const queue = useMemo<readonly QueuedItem[]>(() => [...feed.queued.map((message) => ({ id: message.operation, text: message.text })), ...held], [feed.queued, held])

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
        // Drawn at once: the history only has it when the agent next writes its file.
        setEcho({ text, baseline: countUserMessages(selectChatEvents(feed.projection), text), quiet: false })
        // The "thinking" line it brings is only a stand-in until the agent's own status says so: it stops after a few seconds, the message stays.
        window.setTimeout(() => { setEcho((current) => (current?.text === text ? { ...current, quiet: true } : current)) }, ECHO_THINKING_MS)
        window.setTimeout(() => { setEcho((current) => (current?.text === text ? null : current)) }, ECHO_MAX_MS)
        refresh()
        window.setTimeout(refresh, FOLLOW_UP_REFRESH_MS)
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
    // Drawn at once, as the agent's own history only has it once the turn has begun. A message that
    // is queued behind a running turn shows in the queue instead, so it is not drawn twice.
    const drawn = !running
    if (drawn) {
      setEcho({ text, baseline: countUserMessages(selectChatEvents(feed.projection), text), quiet: true })
      window.setTimeout(() => { setEcho((current) => (current?.text === text ? null : current)) }, ECHO_MAX_MS)
    }
    try {
      const picked = agent === undefined ? null : wireChoice(conversation, agent)
      const target = { id: String(conversation), generation: link.generation }
      await (picked === null ? submitMessage(target, operation, text) : submitMessage(target, operation, text, picked))
      retry.current = null
      refresh()
      return true
    } catch (failure) {
      const message = errorMessage(failure, 'Could not send the message')
      setError(message)
      if (drawn) setEcho((current) => (current?.text === text ? null : current))
      // A number the daemon has passed (another window sent in between) is not tried again; read, then pick a new one.
      if (STALE_OPERATION.test(message)) {
        retry.current = null
        refresh()
      }
      return false
    } finally {
      setSending(false)
    }
  }, [conversation, link.generation, agent, feed, refresh, typeText, running])

  const submit = useCallback(() => {
    const text = draft.trim()
    // A message may be only attachments; the block naming them is then the whole text.
    if ((text === '' && attachments.length === 0) || disabledReason !== null || sending) return
    const files = attachments
    if (typing && busy) {
      // Held until the agent is ready for a message; shown in the queue meanwhile.
      updateComposer(conversation, (state) => ({ ...state, draft: '', attachments: [], held: [...state.held, { id: nextHeldId(), text: buildAttachmentMessage(text, files) }] }))
      return
    }
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
      updateComposer(conversation, (state) => ({ ...state, draft: text, attachments: files }))
    })
  }, [draft, attachments, disabledReason, sending, conversation, send, typing, busy])

  const removeQueued = useCallback((operation: number) => {
    if (operation < 0) {
      updateComposer(conversation, (state) => ({ ...state, held: state.held.filter((message) => message.id !== operation) }))
      return
    }
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

  // The agent is ready for a message: type in the oldest held one, then give the agent time to start on it.
  const heldSentAt = useRef(0)
  const first = held[0]
  useEffect(() => {
    if (!typing || first === undefined || !online || !waitingForMessage || awaitingAnswer || sending) return
    const wait = Math.max(0, heldSentAt.current + HELD_GAP_MS - Date.now())
    const timer = window.setTimeout(() => {
      heldSentAt.current = Date.now()
      updateComposer(conversation, (state) => ({ ...state, held: state.held.filter((message) => message.id !== first.id) }))
      void send(first.text).then((delivered) => {
        // One that did not go is put back at the front, to be tried again.
        if (!delivered) updateComposer(conversation, (state) => ({ ...state, held: [first, ...state.held] }))
      })
    }, wait)
    return () => { window.clearTimeout(timer) }
  }, [typing, first, online, waitingForMessage, awaitingAnswer, sending, conversation, send])

  return { draft, setDraft, attachments, attach, readOnly, queue, removeQueued, submit, stop, running, sending, disabledReason, ready: disabledReason === null && !(typing && busy), error, sendText: send, echo }
}
