import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { cancelTurn, dequeueMessage, submitMessage } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { useLatest } from '@/lib/useLatest'
import type { ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import { abilitiesOf } from '@/features/workspace/model/agentAbilities'
import { hasPendingApproval, type ChatFeed } from './chatFeed'
import { buildAttachmentMessage, parseAttachmentMessage } from './attachmentMessage'
import { announceOutsideSend, onOutsideSend } from './outsideSends'
import { selectChatEvents } from './projection'
import { countUserMessages } from './workPhase'
import { peekComposer, updateComposer, useComposerSlice, type ComposerState, type DraftAttachment } from './composerStore'

import { newId, revokePreviews, useAttachmentIntake, type AttachmentIntake } from './useAttachmentIntake'
import { wireChoice } from './agentChoice'
import { reserveOperation, STALE_OPERATION } from './sendOperation'
import { isBangLine, isSlashLine } from './slashCommands'
import { withRestoreNote } from './editFromHere'

export const PENDING_APPROVAL_NOTE = 'Answer the request above first'
export const READ_ONLY_NOTE = 'Read-only: talk to the agent in the terminal'
/** Why a chat that types into the agent's terminal is off while the agent is busy or asking. */
export const TYPING_BUSY_NOTE = 'The agent is working. Stop it to send something else'
export const TYPING_ASKING_NOTE = 'The agent is asking something in the terminal. Answer it there first'
/** Sent to the terminal around a message, so a multi-line message is one paste and not several Enters. */
export const PASTE_START = '\x1b[200~'
export const PASTE_END = '\x1b[201~'

/**
 * A message as it is typed into an agent's terminal: one paste and Enter, so a multi-line message is one message. The
 * agent's own command is typed instead, as its picker's commands are: it reads a typed slash as a command.
 */
export function asTyped(text: string): string {
  return isSlashLine(text) ? `${text}\r` : `${PASTE_START}${text}${PASTE_END}\r`
}

/** How long a pasted message is given before its Enter is typed. */
export const PASTE_SUBMIT_GAP_MS = 150

/** Codex takes a typed command only when Enter comes a moment after it: sent together, the Enter is lost. */
export const CODEX_SUBMIT_GAP_MS = 400

/** Types a message into an agent's terminal with `type`, the way that agent takes it (`asTyped`; Codex's commands with a pause before Enter). */
/** How long the agent is given to switch to its shell mode after the `!` before the command is typed. */
export const BANG_MODE_GAP_MS = 150

export async function typeMessage(type: (text: string) => Promise<void>, agent: string | undefined, text: string): Promise<void> {
  if (abilitiesOf(agent).slashCommands?.enterAfterPause === true && isSlashLine(text)) {
    await type(text)
    await new Promise<void>((resolve) => { window.setTimeout(resolve, CODEX_SUBMIT_GAP_MS) })
    await type('\r')
    return
  }
  // `!command` is typed, never pasted: the agent switches to its shell mode on the typed `!`, runs the command in the
  // terminal it owns and keeps the output in its context. A paste starting with `!` is only text.
  if (abilitiesOf(agent).bangToTerminal && isBangLine(text)) {
    await type('!')
    await new Promise<void>((resolve) => { window.setTimeout(resolve, BANG_MODE_GAP_MS) })
    await type(`${text.slice(1).trim()}\r`)
    return
  }
  // The Enter goes in on its own, after the paste has been taken: sent in the same write, an agent that is busy
  // reads it as part of the paste and the message sits typed in its box, unsent.
  if (!isSlashLine(text)) {
    await type(`${PASTE_START}${text}${PASTE_END}`)
    await new Promise<void>((resolve) => { window.setTimeout(resolve, PASTE_SUBMIT_GAP_MS) })
    await type('\r')
    return
  }
  await type(asTyped(text))
}
/** The key that interrupts the agent's turn. */
export const INTERRUPT_KEY = '\x1b'

/** How long a message just sent stays on screen before the history shows it; past this it is dropped rather than left as a claim. */
const ECHO_MAX_MS = 20_000
/** How long a message just sent claims the agent is thinking; after this the status itself has to say it. */
const ECHO_THINKING_MS = 4_000
/** A second read soon after a send, because a quiet chat is read only every few seconds. */
const FOLLOW_UP_REFRESH_MS = 700

/** The follow-ups a pane's daemon holds for its working agent, and the ways to add one and to take one back. */
export type PaneQueue = {
  readonly queued: readonly string[]
  /** Leaves a message with the daemon; `first` puts it ahead of the ones already waiting. */
  add: (text: string, first: boolean) => Promise<void>
  /** Takes back the one at `index`, while it still reads `text`. */
  drop: (index: number, text: string) => Promise<void>
}

const NO_QUEUED: readonly string[] = []
/** A pane's queued messages are numbered below zero by their place, so they never share a number with a conversation's operations. */
const paneQueuedId = (index: number): number => -(index + 1)
const paneQueuedIndex = (id: number): number => -id - 1


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
  /** What the pane's daemon holds for its working agent: a message sent while the agent works joins it. */
  paneQueue?: PaneQueue
  /** The chat shows the prompt the agent is stopped on with buttons that answer it, so the composer points there. */
  answersPrompt?: boolean
  /** The pane itself says its agent is working, which is known before the conversation's next read says so. */
  paneWorking?: boolean
}

export type ChatComposer = {
  /** The conversation whose stored draft this is; the box reads the text itself with `useComposerDraft`, so typing does not redraw the chat. */
  conversationKey?: number
  /** A draft the view holds itself, for a chat that has no conversation yet; it wins over the stored one. */
  draft?: string
  setDraft: (text: string) => void
  /** The files attached to the draft, in the order they were added. */
  attachments: readonly DraftAttachment[]
  /** Adding and removing them. */
  attach: AttachmentIntake
  /** The chat only follows an agent running in the terminal (ADR 0003): there is nothing to attach to. */
  readOnly: boolean
  /** Whether files can be attached; when absent, they can unless the chat is read-only. */
  attachable?: boolean
  /** What the daemon holds for the running turn, oldest first. */
  queue: readonly QueuedItem[]
  removeQueued: (id: number) => void
  /** Takes a queued message back into the message box, ahead of anything typed there, so it can be changed and sent again. */
  editQueued: (id: number) => void
  /**
   * Types a queued message into the agent now, in the turn that is running, as typing in its terminal does: the agent
   * reads it at its next step instead of when the turn ends. Only for an agent in a terminal.
   */
  sendQueuedNow?: ((id: number) => void) | undefined
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
 * A message put back in the box, ahead of anything written there: its files come back as attachments, as they were
 * when it was written, not as the lines that name them.
 */
export function messageIntoDraft(text: string): (state: ComposerState) => ComposerState {
  const parsed = parseAttachmentMessage(text)
  const words = parsed?.text ?? text
  const files = (parsed?.files ?? []).map((file) => ({ id: newId(), path: file.path, name: file.name, size: null, kind: file.kind, preview: null }))
  return (state) => ({
    ...state,
    draft: state.draft.trim() === '' ? words : words === '' ? state.draft : `${words}\n\n${state.draft}`,
    attachments: [...files, ...state.attachments],
  })
}

/**
 * The composer's behaviour. A message goes to the daemon through the conversation's own send, with
 * an operation number the daemon uses to tell a retry from a new message. While a turn runs the
 * daemon holds the message and delivers it when the turn ends, in order and once each, whether or
 * not this window stays open; what it holds comes back with every read (`feed.queued`) and is shown
 * until it is sent. While an approval waits, nothing is sent.
 */
export function useChatComposer({ link, agent, feed, online, refresh, typeInto, paneQueue, answersPrompt = false, paneWorking = false }: Input): ChatComposer {
  const attachments = useComposerSlice(link.conversation, (state) => state.attachments)
  const paneQueued = paneQueue?.queued ?? NO_QUEUED
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
  // A terminal agent that is working, or has not said what it is doing yet, still takes a message: its daemon holds it and
  // types it in once the turn is over, so nothing reaches a prompt the agent might be showing meanwhile and nobody waits to type.
  const holding = typing && paneQueue !== undefined && (busy || !waitingForMessage)
  const disabledReason = typing
    ? (!online ? 'The daemon is offline' : awaitingAnswer || feed.status === 'Blocked' ? (answersPrompt ? PENDING_APPROVAL_NOTE : TYPING_ASKING_NOTE) : null)
    : readOnly ? READ_ONLY_NOTE : !online ? 'The daemon is offline' : awaitingAnswer ? PENDING_APPROVAL_NOTE : null
  const conversation = link.conversation
  const attach = useAttachmentIntake({ conversation, blocked: disabledReason, machine: link.machine ?? null })
  const queue = useMemo<readonly QueuedItem[]>(() => [...feed.queued.map((message) => ({ id: message.operation, text: message.text })), ...paneQueued.map((text, index) => ({ id: paneQueuedId(index), text }))], [feed.queued, paneQueued])

  const setDraft = useCallback((text: string) => {
    updateComposer(conversation, (state) => (state.draft === text ? state : { ...state, draft: text }))
  }, [conversation])

  // A comment sent from the diff is drawn in the chat at once, like a message typed here.
  const eventsNow = useLatest(() => selectChatEvents(feed.projection))
  useEffect(() => onOutsideSend(link.conversation, (text) => {
    setEcho({ text, baseline: countUserMessages(eventsNow(), text), quiet: false })
    window.setTimeout(() => { setEcho((current) => (current?.text === text ? { ...current, quiet: true } : current)) }, ECHO_THINKING_MS)
    window.setTimeout(() => { setEcho((current) => (current?.text === text ? null : current)) }, ECHO_MAX_MS)
    refresh()
  }), [link.conversation, eventsNow, refresh])

  const send = useCallback(async (text: string): Promise<boolean> => {
    setSending(true)
    setError(null)
    if (typeText !== undefined) {
      try {
        await typeMessage(typeText, agent, text)
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
    const { draft, attachments, note: boxNote } = peekComposer(conversation)
    const text = draft.trim()
    // A message may be only attachments; the block naming them is then the whole text.
    if ((text === '' && attachments.length === 0) || disabledReason !== null || sending) return
    const files = attachments
    // The agent's own command is not a message to it, so the note waits for the next one that is.
    const note = isSlashLine(text) || isBangLine(text) ? null : boxNote
    updateComposer(conversation, (state) => ({ ...state, draft: '', attachments: [] }))
    if (note === null && boxNote !== null) updateComposer(conversation, (state) => ({ ...state, note: boxNote }))
    const settle = (delivered: boolean) => {
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
      updateComposer(conversation, (state) => ({ ...state, draft: text, attachments: files, note: note ?? state.note }))
    }
    if (holding) {
      // Left with the pane's daemon until the agent is ready for a message; shown in the queue meanwhile.
      setError(null)
      paneQueue.add(withRestoreNote(note, buildAttachmentMessage(text, files)), false).then(
        () => { settle(true) },
        (failure: unknown) => {
          setError(errorMessage(failure, 'Could not queue the message'))
          settle(false)
        },
      )
      return
    }
    void send(withRestoreNote(note, buildAttachmentMessage(text, files))).then(settle)
  }, [disabledReason, sending, conversation, send, holding, paneQueue])

  // The ones this box took back, so their leaving the queue is not read as the daemon having sent them.
  const taken = useRef(new Set<string>())
  const takeBack = useCallback((operation: number): { text: string; gone: Promise<void> } | null => {
    const index = paneQueuedIndex(operation)
    const text = paneQueued[index]
    if (text === undefined || paneQueue === undefined) return null
    taken.current.add(text)
    return { text, gone: paneQueue.drop(index, text) }
  }, [paneQueued, paneQueue])

  // The daemon typed the oldest queued message into the agent: it is drawn in the chat at once, like one sent from here.
  const queuedBefore = useRef(paneQueued)
  useEffect(() => {
    const before = queuedBefore.current
    queuedBefore.current = paneQueued
    const first = before[0]
    if (first === undefined || before.length !== paneQueued.length + 1 || before.slice(1).some((text, index) => text !== paneQueued[index])) return
    if (taken.current.delete(first)) return
    announceOutsideSend(link.conversation, first)
  }, [paneQueued, link.conversation])

  const sendQueuedNow = useCallback((operation: number) => {
    // Typed text answers a prompt the agent is showing, so nothing is pushed in while it asks something.
    if (operation >= 0 || disabledReason !== null || sending) return
    const taking = takeBack(operation)
    if (taking === null) return
    // Typed in only once the daemon has let go of it, or it would go in twice.
    taking.gone.then(
      () => send(taking.text).then((delivered) => {
        // The daemon has let go of it, so a message that did not reach the agent goes back in the box, not nowhere.
        if (delivered) return
        const words = taking.text
        updateComposer(conversation, (state) => ({ ...state, draft: state.draft.trim() === '' ? words : `${words}\n\n${state.draft}` }))
      }),
      (failure: unknown) => {
        taken.current.delete(taking.text)
        setError(errorMessage(failure, 'Could not send the queued message now'))
      },
    )
  }, [disabledReason, sending, send, takeBack, conversation])

  const removeQueued = useCallback((operation: number) => {
    if (operation < 0) {
      const taking = takeBack(operation)
      if (taking === null) return
      taking.gone.catch((failure: unknown) => {
        // Most often it went to the agent a moment ago.
        taken.current.delete(taking.text)
        setError(errorMessage(failure, 'Could not remove the queued message'))
      })
      return
    }
    dequeueMessage({ id: String(conversation), generation: link.generation }, operation)
      .then(refresh)
      .catch((failure: unknown) => {
        setError(errorMessage(failure, 'Could not remove the queued message'))
        // Most often it was sent a moment ago: show what the daemon has now.
        refresh()
      })
  }, [conversation, link.generation, refresh, takeBack])

  const editQueued = useCallback((operation: number) => {
    const message = queue.find((item) => item.id === operation)
    if (message === undefined) return
    const intoDraft = messageIntoDraft(message.text)
    if (operation < 0) {
      const taking = takeBack(operation)
      if (taking === null) return
      // The words go back in the box only once the daemon has let go of them, or a message sent a moment ago would be sent twice.
      taking.gone.then(
        () => { updateComposer(conversation, intoDraft) },
        (failure: unknown) => {
          taken.current.delete(taking.text)
          setError(errorMessage(failure, 'Could not take the queued message back'))
        },
      )
      return
    }
    // The words go back in the box only once the daemon has let go of them, or a message sent a moment ago would be sent twice.
    dequeueMessage({ id: String(conversation), generation: link.generation }, operation)
      .then(() => {
        updateComposer(conversation, intoDraft)
        refresh()
      })
      .catch((failure: unknown) => {
        setError(errorMessage(failure, 'Could not take the queued message back'))
        refresh()
      })
  }, [queue, conversation, link.generation, refresh, takeBack])

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

  return { conversationKey: link.conversation, setDraft, attachments, attach, readOnly, attachable: !readOnly || typing, queue, removeQueued, editQueued, sendQueuedNow: typing ? sendQueuedNow : undefined, submit, stop, running, sending, disabledReason, ready: disabledReason === null && !holding, error, sendText: send, echo }
}
