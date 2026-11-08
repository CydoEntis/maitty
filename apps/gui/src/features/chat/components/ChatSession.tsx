import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { toast } from '@/lib/toast'
import { answerUserInput, approveRequest } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { readConversationPermissions } from '@/data/conversationPermissions'
import type { ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import type { SessionStatus } from '@/features/workspace/model/session'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { hasPendingApproval, runningTurnStartedAt, type ChatFeed } from '../chatFeed'
import { buildRows, type ChatRow } from '../chatRows'
import { useChatFeed } from '../useChatFeed'
import { useChatComposer } from '../useChatComposer'
import { countUserMessages } from '../workPhase'
import { BYPASS_MODE, modeLabel } from '../chatMeta'
import type { ApprovalDecision } from '../chatEvents'
import type { PermissionAsk } from '@/data/preview'
import { ANSWER_KEY, ANSWER_KEYS, choiceKey, type AskChoice } from '../permissionAsk'
import { releaseFirstMessage, takeCarriedDraft, useHeldFirstMessage } from '../firstMessage'
import { Composer } from './Composer'
import { ChatItem } from './ChatItems'
import { KnownRowsContext } from '../knownRows'
import { ChatSkeleton } from './ChatSkeleton'
import { ApprovalTray, PromptTray } from './QuestionTray'
import { SubagentBar, SubagentChat } from './SubagentChat'
import { VirtualList } from './VirtualList'

/** When a turn ends the status changes at once, but the agent's last words may not be read yet: read now and again soon, instead of waiting for the idle poll. */
const END_OF_TURN_READ_MS = 600
/** How long the working line stays after a turn ends with no reply read yet, so the pane is never blank between the two. */
const SETTLE_MS = 650

/** The id of a held first message in the composer's list of waiting messages; real ones carry operation numbers. */
const HELD_MESSAGE_ID = -1

/** A row not yet measured is drawn as if it were this tall, which a message or a card usually is. */
const ESTIMATED_ROW_PX = 72

const INCOMPLETE_WORD: Record<string, string> = {
  Disconnected: 'the connection to the provider was lost while it ran',
  UnsupportedItem: 'it holds an item this app cannot show',
  InvalidEvent: 'the provider sent an event this app could not read',
  MissingPages: 'some pages could not be read',
  RevisionExhausted: 'it changed more often than the daemon can follow',
}

function incompleteReason(completeness: ChatFeed['completeness']): string | null {
  if (completeness === null || typeof completeness === 'string') return null
  return INCOMPLETE_WORD[completeness.Incomplete] ?? 'part of it could not be read'
}

/** What a screen reader hears when the conversation changes state: one short line per change, never the text. */
function announcement(feed: ChatFeed, agent: ChatAgent, working: boolean): string {
  if (hasPendingApproval(feed) || feed.status === 'Blocked') return `${agent} is waiting for your answer`
  if (working) return `${agent} is working`
  if (feed.status === 'Done') return `${agent} finished`
  return ''
}

const rowKey = (row: ChatRow) => row.key

/** A pending request is asked in the tray over the message box, so it is not also a row of the list. */
const isPendingApproval = (row: ChatRow) => row.kind === 'approval' && row.event.resolution.status === 'pending'

/** The keys that answer the question tray, as typing the same letters in the terminal would. */
const KEY_DECISION: Readonly<Record<string, ApprovalDecision>> = Object.fromEntries(Object.entries(ANSWER_KEY).map(([decision, key]) => [key, decision as ApprovalDecision]))

/** Whether a key press belongs to a field, where a letter is text and not an answer. */
const typedInField = (target: EventTarget) => target instanceof Element && target.closest('input, textarea, [contenteditable="true"]') !== null

const ERROR_WORDS_MAX = 140
const truncateError = (reason: string) => (reason.length > ERROR_WORDS_MAX ? `${reason.slice(0, ERROR_WORDS_MAX)}…` : reason)

type Props = {
  link: ChatLink
  agent: ChatAgent
  /** The daemon is connected: a message or an answer can be sent. */
  online: boolean
  /** False for a chat session: the agent has no terminal to type in, so no text sends the reader there. */
  hasTerminal?: boolean
  /** The session's folder, so file names read relative to it. */
  root?: string | null
  /** Opens a file's diff in the Changes panel; absent where there is none. */
  onOpenDiff?: ((file: string) => void) | null
  /** Types into the terminal the agent runs in, for a chat that follows it (ADR 0003). */
  typeInto?: ((text: string) => Promise<void>) | undefined
  /** The permission prompt the agent's hook says its terminal is stopped on, for a chat that follows it. */
  ask?: PermissionAsk | null
  /** The pane's own status, live in the sidebar: the chat shows working at once and reads then, instead of at its next poll. */
  paneStatus?: SessionStatus
  /** The session's id: a message typed while its agent was starting is held under it, and goes out from here. */
  sessionId?: string
  /** The project folder's name and the branch, for the tray under the message box. */
  folder?: string
  branch?: string | null
  /** How many subagents the session has, for the tray. */
  subagentCount?: number
  /** Shows the session's files beside the pane (the tray's folder). */
  onOpenFolder?: (() => void) | null
  /** A subagent whose own chat shows instead of the session's, and the way back. */
  subagent?: SubagentView | null
  onBackToMain?: () => void
}

/** The chat of one session: its history as a virtualised list, the daemon's status, and the composer under it. */
export function ChatSession({ link, agent, online, hasTerminal = true, root = null, onOpenDiff = null, typeInto, ask = null, sessionId, paneStatus, folder = '', branch = null, subagentCount = 0, onOpenFolder = null, subagent = null, onBackToMain }: Props) {
  const { feed, events, offline, readError, refresh, recordAnswer } = useChatFeed({ conversation: link.conversation, generation: link.generation, provider: agent })
  const paneWorking = paneStatus === 'working'
  const working = feed.status === 'Working' || paneWorking
  const workingSince = working ? runningTurnStartedAt(feed) : null
  const incomplete = incompleteReason(feed.completeness)
  const wasWorking = useRef(false)
  // The turn has ended but its reply is not in the history yet: the working line holds until it is, or SETTLE_MS.
  const [settling, setSettling] = useState(false)
  const repliedRef = useRef(false)
  useEffect(() => {
    const changed = wasWorking.current !== working
    const ended = wasWorking.current && !working
    wasWorking.current = working
    if (!changed) return undefined
    if (ended && !repliedRef.current) {
      setSettling(true)
      const settle = window.setTimeout(() => { setSettling(false) }, SETTLE_MS)
      const read = window.setTimeout(refresh, END_OF_TURN_READ_MS)
      refresh()
      return () => { window.clearTimeout(settle); window.clearTimeout(read) }
    }
    // Read as the turn starts (its first words) and as it ends (its last), not at the next poll.
    refresh()
    if (!ended) return undefined
    const timer = window.setTimeout(refresh, END_OF_TURN_READ_MS)
    return () => { window.clearTimeout(timer) }
  }, [working, refresh])
  // The transcript has no entry for a terminal agent's prompt until it is answered, so the chat shows it from the hook's report.
  const stoppedOnPrompt = link.readOnly === true && feed.status === 'Blocked'
  const answersPrompt = stoppedOnPrompt && typeInto !== undefined && (ask?.answerable === true || (ask?.choices?.length ?? 0) > 0)
  const answerPrompt = typeInto === undefined ? undefined : (choice: AskChoice) => typeInto(ANSWER_KEYS[choice]).then(refresh)
  const pickChoice = typeInto === undefined ? undefined : (index: number) => typeInto(choiceKey(index)).then(refresh)
  const composer = useChatComposer({ link, agent, feed, online, refresh, answersPrompt, paneWorking, ...(typeInto ? { typeInto } : {}) })
  const { sendText, setDraft: setComposerDraft, ready, sending, echo } = composer
  // A message just sent shows at once, with the agent taken to be at work on it, until the history has it.
  const echoText = echo !== null && countUserMessages(events, echo.text) <= echo.baseline ? echo.text : null
  const echoThinking = echo !== null && !echo.quiet
  // What the history says without the working line: whether the agent has answered what was last asked.
  const bareRows = useMemo(() => buildRows(events, false, root, echoText, false), [events, root, echoText])
  const lastKind = bareRows.at(-1)?.kind
  const replied = lastKind !== undefined && lastKind !== 'user' && lastKind !== 'echo'
  useEffect(() => { repliedRef.current = replied })
  const holdWorking = settling && !replied
  const allRows = useMemo(() => buildRows(events, working || holdWorking, root, echoText, echoThinking), [events, working, holdWorking, root, echoText, echoThinking])
  const rows = useMemo(() => allRows.filter((row) => !isPendingApproval(row)), [allRows])
  // The rows the history had when it loaded: what arrives after them is live, and is typed in rather than shown whole.
  const [known, setKnown] = useState<ReadonlySet<string> | null>(null)
  if (feed.loaded && known === null) setKnown(new Set(rows.map(rowKey)))
  const pending = useMemo(() => allRows.find((row): row is Extract<ChatRow, { kind: 'approval' }> => isPendingApproval(row))?.event ?? null, [allRows])
  // A draft typed while the agent started comes over once, and a message sent then goes out as soon as the agent takes input.
  const held = useHeldFirstMessage(sessionId)
  useEffect(() => {
    if (sessionId === undefined) return
    const carried = takeCarriedDraft(sessionId)
    if (carried !== null) setComposerDraft(carried)
  }, [sessionId, setComposerDraft])
  useEffect(() => {
    if (sessionId === undefined || held === null || !ready || sending) return
    releaseFirstMessage(sessionId)
    void sendText(held).then((delivered) => { if (!delivered) setComposerDraft(held) })
  }, [sessionId, held, ready, sending, sendText, setComposerDraft])
  // The held message shows as waiting until it goes, like a message queued behind a running turn.
  const shownComposer = useMemo(() => (held === null ? composer : { ...composer, queue: [...composer.queue, { id: HELD_MESSAGE_ID, text: held }], removeQueued: (id: number) => { if (id === HELD_MESSAGE_ID) { if (sessionId !== undefined) releaseFirstMessage(sessionId) } else composer.removeQueued(id) } }), [composer, held, sessionId])
  // The daemon does not report how a session was started, so the window remembers the sessions it started with full access.
  const fullAccess = useMemo(() => readConversationPermissions()[String(link.conversation)] === 'full-access', [link.conversation])

  const [answering, setAnswering] = useState(false)
  const answer = useCallback((requestId: number | string, decision: ApprovalDecision) => {
    setAnswering(true)
    approveRequest({ id: String(link.conversation), generation: link.generation }, requestId, decision)
      .then(() => { recordAnswer(requestId, decision); refresh() })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
      .finally(() => { setAnswering(false) })
  }, [link.conversation, link.generation, recordAnswer, refresh])
  // A user-input answer is relayed once and recorded nowhere: it can be a secret.
  const answerInput = useCallback((requestId: number | string, answers: Record<string, string[]>) => {
    setAnswering(true)
    answerUserInput({ id: String(link.conversation), generation: link.generation }, requestId, answers)
      .then(refresh)
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
      .finally(() => { setAnswering(false) })
  }, [link.conversation, link.generation, refresh])
  // A chat that follows a terminal pane is answered in that terminal (ADR 0003), so it offers no buttons.
  const approval = useMemo(() => (online && link.readOnly !== true ? { answer, answerInput, busy: answering } : null), [online, link.readOnly, answer, answerInput, answering])

  // An approval's link to its tool step scrolls the list there; the nonce makes a second click on the same link scroll again.
  const [scrollTarget, setScrollTarget] = useState<{ key: string; nonce: number } | null>(null)
  const goToStep = useCallback((key: string) => { setScrollTarget((current) => ({ key, nonce: (current?.nonce ?? 0) + 1 })) }, [])

  const renderRow = useCallback(
    (row: ChatRow) => <ChatItem row={row} agent={agent} workingSince={workingSince} approval={approval} root={root} openDiff={onOpenDiff} goToStep={goToStep} />,
    [agent, workingSince, approval, root, onOpenDiff, goToStep],
  )

  // y, a and n answer the tray from anywhere in the chat but a field, as the same letters do in the terminal.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.ctrlKey || event.metaKey || event.altKey || typedInField(event.target)) return
    const decision = KEY_DECISION[event.key.toLowerCase()]
    if (decision === undefined) return
    if (pending !== null && approval !== null && !approval.busy && pending.approvalKind !== 'UserInput' && pending.choices.includes(decision)) {
      event.preventDefault()
      answer(pending.requestId, decision)
    } else if (stoppedOnPrompt && answerPrompt !== undefined && ask?.answerable === true && decision !== 'AcceptForSession') {
      event.preventDefault()
      void answerPrompt(decision === 'Accept' ? 'approve' : 'deny')
    }
  }

  const question = pending !== null
    ? <ApprovalTray event={pending} actions={approval} />
    : stoppedOnPrompt ? <PromptTray agent={agent} ask={ask} online={online} answer={answerPrompt} pick={pickChoice} /> : null
  const notices = [
    offline ? `Reconnecting to the daemon. What is shown may be out of date.${readError !== null ? ` (${truncateError(readError)})` : ''}` : null,
    !offline && incomplete !== null ? `History is incomplete: ${incomplete}.` : null,
    feed.error,
  ].filter((notice): notice is string => notice !== null)

  return (
    <div className="mx-chat" onKeyDown={onKeyDown}>
      {(notices.length > 0 || fullAccess) && (
        <div className="mx-chat-notices" role={offline ? 'status' : undefined}>
          {notices.map((notice) => <span key={notice}>{notice}</span>)}
          {fullAccess && <span className="mx-chat-tag is-danger" title="No sandbox and no approval prompts. End the session and start a new one to change this.">Full access</span>}
        </div>
      )}
      {/* Politely announces a change of state only; the log itself is not a live region, so streaming text never floods a screen reader. */}
      <div role="status" aria-live="polite" className="sr-only">{announcement(feed, agent, working)}</div>
      {subagent !== null ? (
        <>
          <SubagentBar subagent={subagent} onBack={() => { onBackToMain?.() }} />
          <SubagentChat subagent={subagent} agent={agent} />
        </>
      ) : feed.loaded && rows.length === 0 ? (
        <p className="mx-chat-empty">{hasTerminal ? 'No messages yet. Send one below, or type in the terminal.' : 'No messages yet. Send one below.'}</p>
      ) : feed.loaded ? (
        <KnownRowsContext.Provider value={known}>
          <VirtualList items={rows} getKey={rowKey} estimate={ESTIMATED_ROW_PX} renderItem={renderRow} label={`Conversation with ${agent}`} scrollTarget={scrollTarget} />
        </KnownRowsContext.Provider>
      ) : (
        <ChatSkeleton />
      )}
      <Composer
        agent={agent}
        composer={shownComposer}
        conversation={link.conversation}
        question={question}
        facts={{ mode: modeLabel(feed.meta?.mode ?? null), modeDanger: feed.meta?.mode === BYPASS_MODE, subagents: subagentCount, folder, branch, model: feed.meta?.model ?? null, onOpenFolder }}
      />
    </div>
  )
}
