import { useCallback, useEffect, useMemo, useState } from 'react'
import { toast } from 'sonner'
import { answerUserInput, approveRequest } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { readConversationPermissions } from '@/data/conversationPermissions'
import type { ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import { hasPendingApproval, runningTurnStartedAt, type ChatFeed } from '../chatFeed'
import { buildRows, type ChatRow } from '../chatRows'
import { useChatFeed } from '../useChatFeed'
import { useChatComposer } from '../useChatComposer'
import type { ApprovalDecision } from '../chatEvents'
import type { PermissionAsk } from '@/data/preview'
import { ANSWER_KEYS, type AskChoice } from '../permissionAsk'
import { releaseFirstMessage, takeCarriedDraft, useHeldFirstMessage } from '../firstMessage'
import { Composer } from './Composer'
import { ChatItem } from './ChatItems'
import { PermissionAskCard } from './PermissionAskCard'
import { VirtualList } from './VirtualList'

/** The id of a held first message in the composer's list of waiting messages; real ones carry operation numbers. */
const HELD_MESSAGE_ID = -1

/** A row not yet measured is drawn as if it were this tall, which a message or a card usually is. */
const ESTIMATED_ROW_PX = 72

const STATUS_WORD: Record<NonNullable<ChatFeed['status']>, string> = {
  None: 'Not started',
  Idle: 'Idle',
  Working: 'Working',
  Blocked: 'Needs you',
  Done: 'Done',
}

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
function announcement(feed: ChatFeed, agent: ChatAgent): string {
  if (hasPendingApproval(feed) || feed.status === 'Blocked') return `${agent} is waiting for your answer`
  if (feed.status === 'Working') return `${agent} is working`
  if (feed.status === 'Done') return `${agent} finished`
  return ''
}

const rowKey = (row: ChatRow) => row.key

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
  /** The session's id: a message typed while its agent was starting is held under it, and goes out from here. */
  sessionId?: string
}

/** The chat of one session: its history as a virtualised list, the daemon's status, and the composer under it. */
export function ChatSession({ link, agent, online, hasTerminal = true, root = null, onOpenDiff = null, typeInto, ask = null, sessionId }: Props) {
  const { feed, events, offline, readError, refresh, recordAnswer } = useChatFeed({ conversation: link.conversation, generation: link.generation, provider: agent })
  const working = feed.status === 'Working'
  const rows = useMemo(() => buildRows(events, working, root), [events, working, root])
  const workingSince = working ? runningTurnStartedAt(feed) : null
  const incomplete = incompleteReason(feed.completeness)
  // The transcript has no entry for a terminal agent's prompt until it is answered, so the chat shows it from the hook's report.
  const stoppedOnPrompt = link.readOnly === true && feed.status === 'Blocked'
  const answersPrompt = stoppedOnPrompt && typeInto !== undefined && ask?.answerable === true
  const answerPrompt = typeInto === undefined ? undefined : (choice: AskChoice) => typeInto(ANSWER_KEYS[choice]).then(refresh)
  const composer = useChatComposer({ link, feed, online, refresh, answersPrompt, ...(typeInto ? { typeInto } : {}) })
  const { sendText, setDraft: setComposerDraft, disabledReason, sending } = composer
  // A draft typed while the agent started comes over once, and a message sent then goes out as soon as the agent takes input.
  const held = useHeldFirstMessage(sessionId)
  useEffect(() => {
    if (sessionId === undefined) return
    const carried = takeCarriedDraft(sessionId)
    if (carried !== null) setComposerDraft(carried)
  }, [sessionId, setComposerDraft])
  useEffect(() => {
    if (sessionId === undefined || held === null || disabledReason !== null || sending) return
    releaseFirstMessage(sessionId)
    void sendText(held).then((delivered) => { if (!delivered) setComposerDraft(held) })
  }, [sessionId, held, disabledReason, sending, sendText, setComposerDraft])
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

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-8 shrink-0 items-center gap-2 border-b border-line2 px-4 text-[12px] text-fg2">
        <span className="font-semibold text-fg">{feed.status === null ? 'Reading…' : STATUS_WORD[feed.status]}</span>
        {offline && <span role="status" className="text-[var(--warn)]">Reconnecting to the daemon. What is shown may be out of date.{readError !== null && ` (${truncateError(readError)})`}</span>}
        {!offline && incomplete !== null && <span className="text-dim">History is incomplete: {incomplete}.</span>}
        {feed.error !== null && <span className="text-[var(--danger)]">{feed.error}</span>}
        {fullAccess && <span className="rounded border border-[var(--danger-line)] px-1.5 text-[11px] font-semibold text-[var(--danger)]" title="No sandbox and no approval prompts. End the session and start a new one to change this.">Full access</span>}
        {!hasTerminal && <span className="ml-auto text-dim">Chat session &middot; no terminal</span>}
      </div>
      {/* Politely announces a change of state only; the log itself is not a live region, so streaming text never floods a screen reader. */}
      <div role="status" aria-live="polite" className="sr-only">{announcement(feed, agent)}</div>
      {feed.loaded && rows.length === 0 ? (
        <p className="m-0 flex flex-1 items-center justify-center px-6 text-center text-[13px] text-dim">{hasTerminal ? 'No messages yet. Send one below, or type in the terminal.' : 'No messages yet. Send one below.'}</p>
      ) : feed.loaded ? (
        <VirtualList items={rows} getKey={rowKey} estimate={ESTIMATED_ROW_PX} renderItem={renderRow} label={`Conversation with ${agent}`} scrollTarget={scrollTarget} />
      ) : (
        <p className="m-0 flex flex-1 items-center justify-center px-6 text-center text-[13px] text-dim">Reading the conversation…</p>
      )}
      {stoppedOnPrompt && <PermissionAskCard agent={agent} ask={ask} online={online} answer={answerPrompt} />}
      <Composer agent={agent} composer={shownComposer} />
    </div>
  )
}
