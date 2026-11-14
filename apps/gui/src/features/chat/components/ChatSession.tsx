import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { toast } from '@/lib/toast'
import { allowAlways, answerUserInput, approveRequest, loadTerminalScreen } from '@/api/daemon'
import { machineNameOf, terminalOf } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import { readConversationPermissions } from '@/data/conversationPermissions'
import type { ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import { abilitiesOf } from '@/features/workspace/model/agentAbilities'
import type { SessionStatus } from '@/features/workspace/model/session'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { hasPendingApproval, runningTurnStartedAt, type ChatFeed } from '../chatFeed'
import { buildRows, type ChatRow } from '../chatRows'
import { findRow, registerChatFind } from '../chatSearch'
import { useLatest } from '@/lib/useLatest'
import { useChatFeed } from '../useChatFeed'
import { CODEX_SUBMIT_GAP_MS, typeMessage, useChatComposer, type PaneQueue } from '../useChatComposer'
import { countUserMessages } from '../workPhase'
import { startedWithYolo } from '../yoloSessions'
import { CODEX_MODE_COMMAND, CODEX_MODE_REFUSED, CODEX_START_MODES, DANGER_MODES, modeFromScreen, modeLabel } from '../chatMeta'
import type { ApprovalDecision } from '../chatEvents'
import type { PermissionAsk } from '@/data/preview'
import { ANSWER_KEY, ANSWER_KEYS, ANSWER_STEP_MS, answerKeys, choiceKey, type AskChoice } from '../permissionAsk'
import { draftWithQuote, pickedText, type PickedText } from '../selectionActions'
import { peekComposer } from '../composerStore'
import { COPY_TOAST_ID, COPY_TOAST_MS } from '@/features/terminal/copyToast'
import { SelectionBar } from './SelectionBar'
import { OpenFileContext, type OpenFile } from '../openFile'
import { keepComposerFocus } from '../keepComposerFocus'
import { followComposer } from '../composerStore'
import type { SessionUsage } from '../usageStrip'
import { releaseFirstMessage, releaseStartChoice, startChoiceOf, takeCarriedDraft, useHeldByDaemon, useHeldFirstMessage } from '../firstMessage'
import { writeChoice } from '../agentChoice'
import { isWorktree, useFolderBranches } from '@/features/workspace/model/newChatHost'
import { Composer } from './Composer'
import { ChatItem, type ChatItemContext } from './ChatItems'
import { KnownRowsContext } from '../knownRows'
import { ChatSkeleton } from './ChatSkeleton'
import { askedPlan, PLAN_TOOL } from '../planApproval'
import { ApprovalTray, PromptTray } from './QuestionTray'
import type { BranchMenuSpec } from './BranchMenu'
import { ProblemBanner } from './ProblemBanner'
import { summaryOf } from '../summary'
import type { ChatProblem } from '../problem'
import { SubagentBar, SubagentChat } from './SubagentChat'
import { VirtualList } from './VirtualList'
import { copyText } from '@/lib/clipboard'
import { ConfirmCloseDialog } from '@/features/workspace/components/ConfirmCloseDialog'
import type { RewindPlace } from '../editFromHere'
import type { FileSearchPlace } from '../fileMention'
import { useEditFromHere } from '../useEditFromHere'

/** When a turn ends the status changes at once, but the agent's last words may not be read yet: read now and again soon, instead of waiting for the idle poll. */
const END_OF_TURN_READ_MS = 600
/** How long the working line stays after a turn ends with no reply read yet, so the pane is never blank between the two. */
const SETTLE_MS = 650
/** Shift+Tab, which cycles the agent's permission mode, and how long after it the new mode is read back. */
const SHIFT_TAB = '\x1b[Z'
const MODE_READ_MS = 400
/**
 * How long after typing a Codex command its Enter is pressed. Codex takes keys that arrive together for a paste, and an Enter
 * that comes with them for a line break: the command then sits in its box unsent (seen on Codex 0.166 on Windows).
 */
/** The most presses of Shift+Tab a full turn of Claude's modes takes. */
const MODE_CYCLE_MAX = 5

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
  /** The pane runs in a linked worktree, as the daemon says. Without it the folder's own worktree list is asked. */
  worktree?: boolean
  /** The session's folder, so file names read relative to it. */
  root?: string | null
  /** Opens a file's diff, or a turn's, in the Changes panel; absent where there is none. */
  onOpenDiff?: ChatItemContext['openDiff']
  /** Opens a file a reply names, at its line when it gave one. */
  onOpenFile?: OpenFile | null
  /** Types into the terminal the agent runs in, for a chat that follows it (ADR 0003). */
  typeInto?: ((text: string) => Promise<void>) | undefined
  /** What the pane's daemon holds for its working agent: a message or a pick made while it works joins it. */
  paneQueue?: PaneQueue | undefined
  /** The permission prompt the agent's hook says its terminal is stopped on, for a chat that follows it. */
  ask?: PermissionAsk | null
  /** The pane's own status, live in the sidebar: the chat shows working at once and reads then, instead of at its next poll. */
  paneStatus?: SessionStatus
  /** The session's id: a message typed while its agent was starting is held under it, and goes out from here. */
  sessionId?: string
  /** The project folder's name and the branch, for the tray under the message box. */
  folder?: string
  branch?: string | null
  /** Shows the session's files beside the pane (the tray's folder). */
  onOpenFolder?: (() => void) | null
  /** A subagent whose own chat shows instead of the session's, and the way back. */
  subagent?: SubagentView | null
  onBackToMain?: () => void
  /** What the session's agent reported using (context, plan limits), for the strip under the message box. */
  usage?: SessionUsage
  /** Shows the session's terminal in place of this chat, for a command whose menu only the terminal draws. */
  onShowTerminal?: () => void
  /** What is wrong with the chat, if anything: shown as a banner over the message box when nothing else is asked of the person. */
  problem?: ChatProblem | null
  /** Where the tray's branch button can move this chat; without it the branch is only said. */
  branchMenu?: BranchMenuSpec | undefined
  /** The effort the agent's own files say it runs at; null when they do not say. */
  effort?: string | null | undefined
  /** Where a sent message's Edit from here restores the files from; absent where it is not offered. */
  rewind?: RewindPlace | undefined
  /** Where an `@` in the message box looks for files; absent where it is not offered. */
  fileSearch?: FileSearchPlace | undefined
  /** Leaves out runs of reads and searches that succeeded (Settings › Chat). */
  hideSuccessfulTools?: boolean
}

/** The chat of one session: its history as a virtualised list, the daemon's status, and the composer under it. */
export function ChatSession({ link, agent, online, hasTerminal = true, worktree: paneWorktree, root = null, onOpenDiff = null, onOpenFile = null, typeInto, paneQueue, ask = null, sessionId, paneStatus, folder = '', branch = null, onOpenFolder = null, subagent = null, onBackToMain, onShowTerminal, usage, problem = null, branchMenu, effort = null, rewind, hideSuccessfulTools = false, fileSearch }: Props) {
  const { feed, events, offline, readError, refresh, recordAnswer, unwatchedReads } = useChatFeed({ conversation: link.conversation, generation: link.generation, provider: agent })
  const paneWorking = paneStatus === 'working'
  const [branches] = useFolderBranches(root ?? '')
  // `root` is the repository's main folder, which is never a worktree, so the pane's own answer comes first.
  const worktree = paneWorktree ?? isWorktree(branches, root)
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
  const answersPrompt = stoppedOnPrompt && typeInto !== undefined && (ask?.answerable === true || (ask?.choices?.length ?? 0) > 0 || (ask?.questions?.length ?? 0) > 1)
  const answerPrompt = typeInto === undefined ? undefined : (choice: AskChoice) => typeInto(ANSWER_KEYS[choice]).then(refresh)
  // A menu the chat has no buttons for is answered with the keys a person would press, in the agent's own terminal.
  const sendKeys = typeInto === undefined ? undefined : (keys: string) => typeInto(keys).then(refresh)
  const readScreen = useMemo(() => (sessionId === undefined ? undefined : () => loadTerminalScreen({ id: sessionId, generation: 0 })), [sessionId])
  const pickChoice = typeInto === undefined ? undefined : (index: number) => typeInto(choiceKey(index)).then(refresh)
  // Several questions are answered a key at a time, as a person would: the prompt moves on after each.
  const pickChoices = typeInto === undefined ? undefined : async (picked: readonly number[]) => {
    for (const [at, key] of answerKeys(picked).entries()) {
      if (at > 0) await new Promise<void>((resolve) => { window.setTimeout(resolve, ANSWER_STEP_MS) })
      await typeInto(key)
    }
    refresh()
  }
  // A chat that follows the agent's own terminal changes its mode, model and reasoning the way its user would: by typing the agent's commands
  // (Shift+Tab, /model, /effort). Offered for Claude, and not while it works, when the keys would land in its turn.
  // What it shows meanwhile is read back from its screen (mode) or taken from what was picked (model), until the agent's own file says it.
  const [liveMode, setLiveMode] = useState<{ base: string | null; value: string } | null>(null)
  const [liveModel, setLiveModel] = useState<{ base: string | null; value: string } | null>(null)
  const metaMode = feed.meta?.mode ?? null
  const metaModel = feed.meta?.model ?? null
  // What the chat was started with, until the agent's own file says it: the chips the starting chat drew stay as they were.
  const [started] = useState(() => startChoiceOf(sessionId))
  const shownMode = (liveMode !== null && liveMode.base === metaMode ? liveMode.value : metaMode) ?? started?.mode ?? null
  const shownModel = liveModel !== null && liveModel.base === metaModel ? liveModel.value : metaModel
  const abilities = abilitiesOf(agent)
  const canChangeAgent = typeInto !== undefined && abilities.changeAgent && link.readOnly === true
  const canPickCodexMode = typeInto !== undefined && abilities.permissionModes?.switchBy === 'command' && link.readOnly === true
  // The chips stay drawn while their keys would land in a turn or nowhere; they are only greyed out, so the box does not rearrange.
  const pickerLocked = !online ? 'The daemon is offline' : stoppedOnPrompt ? `${agent} is asking something. Answer it first.` : null
  const chipsLocked = !online ? 'The daemon is offline' : working ? `${agent} is working. Change this when it finishes.` : null
  const agentControls = useMemo(() => {
    if (canPickCodexMode) {
      return {
        modes: CODEX_START_MODES,
        modeAgent: 'Codex',
        onPickMode: (target: string) => {
          const command = CODEX_MODE_COMMAND[target]
          if (sessionId === undefined || command === undefined) return
          if (chipsLocked !== null) {
            toast.error(chipsLocked)
            return
          }
          // Codex has no file that says its mode, so the pick stands unless its screen says the command was refused.
          const before = shownMode
          setLiveMode({ base: metaMode, value: target })
          const step = async () => {
            await typeInto(command)
            await new Promise<void>((resolve) => { window.setTimeout(resolve, CODEX_SUBMIT_GAP_MS) })
            await typeInto('\r')
            await new Promise<void>((resolve) => { window.setTimeout(resolve, MODE_READ_MS) })
            if (CODEX_MODE_REFUSED.test(await loadTerminalScreen({ id: sessionId, generation: 0 }))) {
              setLiveMode(before === null ? null : { base: metaMode, value: before })
              toast.error(`Codex did not switch to ${modeLabel(target) ?? target}. Its terminal says why.`)
            }
            refresh()
          }
          void step().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not change the permission mode')) })
        },
      }
    }
    if (!canChangeAgent) return {}
    return {
      // The model and the effort can be picked while the agent works: the pick is typed in at once, as it would be in
      // the terminal, and the agent takes it as soon as it can. Only a question it is asking holds the picker, since
      // typed text would answer the question.
      agentChips: { locked: chipsLocked, pickerLocked },
      onCommand: (text: string) => {
        if (pickerLocked !== null) return
        const model = /^\/model (\S+)/.exec(text)?.[1]
        if (model !== undefined) setLiveModel({ base: metaModel, value: model })
        void typeInto(text).then(refresh)
      },
      onPickMode: (target: string) => {
        if (sessionId === undefined || chipsLocked !== null) return
        // Shift+Tab only cycles, so press it until the screen shows the picked mode; a mode that never comes up is not allowed in this session.
        // The chip shows the pick at once and the presses happen behind it; the screen has the last word when they end.
        setLiveMode({ base: metaMode, value: target })
        const step = async () => {
          for (let press = 0; press <= MODE_CYCLE_MAX; press += 1) {
            const mode = modeFromScreen(await loadTerminalScreen({ id: sessionId, generation: 0 }))
            if (mode === target) {
              refresh()
              return
            }
            if (press === MODE_CYCLE_MAX) break
            await typeInto(SHIFT_TAB)
            await new Promise<void>((resolve) => { window.setTimeout(resolve, MODE_READ_MS) })
          }
          const now = modeFromScreen(await loadTerminalScreen({ id: sessionId, generation: 0 }))
          setLiveMode(now === null ? null : { base: metaMode, value: now })
          toast.error(`${modeLabel(target) ?? target} is not available in this session. Claude has to be started allowing it.`)
          refresh()
        }
        void step().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not change the permission mode')) })
      },
    }
  }, [canChangeAgent, canPickCodexMode, chipsLocked, typeInto, refresh, sessionId, metaMode, metaModel, shownMode, pickerLocked])
  const composer = useChatComposer({ link, agent, feed, online, refresh, answersPrompt, paneWorking, ...(typeInto ? { typeInto } : {}), ...(paneQueue ? { paneQueue } : {}) })
  // A command that opens a menu of its own is typed into the agent's terminal, which is shown: the chat cannot draw the menu.
  const runInTerminal = typeInto === undefined || onShowTerminal === undefined || link.readOnly !== true ? undefined : (command: string) => {
    onShowTerminal()
    void typeMessage(typeInto, agent, command).catch((error: unknown) => { toast.error(errorMessage(error, `Could not run ${command}`)) })
  }
  const { sendText, setDraft: setComposerDraft, ready, sending, echo } = composer
  // A message just sent shows at once, with the agent taken to be at work on it, until the history has it.
  // A first message typed while the agent started is held, and shows exactly as the starting chat showed it (the bubble, then the
  // working line) until it is sent and the history has it, so the chat never says "No messages yet" in between.
  const held = useHeldFirstMessage(sessionId)
  const heldByDaemon = useHeldByDaemon(sessionId)
  const sentText = echo !== null && countUserMessages(events, echo.text) <= echo.baseline ? echo.text : null
  const echoText = sentText ?? held
  const echoThinking = sentText === null ? held !== null : echo !== null && !echo.quiet
  // What the history says without the working line or hidden runs: whether the agent has answered what was last asked,
  // and the summary a new chat starts from, which reads the last reply as it was said.
  const bareRows = useMemo(() => buildRows(events, false, root, echoText, false), [events, root, echoText])
  const lastKind = bareRows.at(-1)?.kind
  const replied = lastKind !== undefined && lastKind !== 'user' && lastKind !== 'echo'
  useEffect(() => { repliedRef.current = replied })
  const holdWorking = settling && !replied
  const allRows = useMemo(() => buildRows(events, working || holdWorking, root, echoText, echoThinking, hideSuccessfulTools), [events, working, holdWorking, root, echoText, echoThinking, hideSuccessfulTools])
  const rows = useMemo(() => allRows.filter((row) => !isPendingApproval(row)), [allRows])
  // Rows are known, and so shown whole, once a read has brought them without the person watching: the first read after
  // the chat opened (until then it shows what this window remembered, which may be behind) and one after the window was
  // hidden. Only a reply that comes while the chat is being looked at is typed in.
  // Text selected in the chat gets a small bar where the mouse let go: Copy, and Quote into the message box. It goes
  // when the selection does (a click elsewhere), on Esc, and when the chat scrolls away from under it.
  const [picked, setPicked] = useState<PickedText | null>(null)
  const chat = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (picked === null) return
    const drop = () => { setPicked(null) }
    const dropIfUnselected = () => { if (window.getSelection()?.isCollapsed !== false) drop() }
    const dropOnEscape = (event: globalThis.KeyboardEvent) => { if (event.key === 'Escape') drop() }
    document.addEventListener('selectionchange', dropIfUnselected)
    document.addEventListener('keydown', dropOnEscape)
    document.addEventListener('scroll', drop, true)
    return () => {
      document.removeEventListener('selectionchange', dropIfUnselected)
      document.removeEventListener('keydown', dropOnEscape)
      document.removeEventListener('scroll', drop, true)
    }
  }, [picked])
  const copyPicked = () => {
    if (picked === null) return
    setPicked(null)
    copyText(picked.text).then(
      () => { toast.success('Copied', { id: COPY_TOAST_ID, duration: COPY_TOAST_MS }) },
      (failure: unknown) => {
        // Why it was refused is the web view's to say; without it a refusal cannot be told from a bug here.
        console.warn('The web view refused to copy the selection', failure)
        toast.error('Could not copy the selection')
      },
    )
  }
  const quotePicked = (box: HTMLElement) => {
    if (picked === null) return
    setComposerDraft(draftWithQuote(peekComposer(link.conversation).draft, picked.text))
    setPicked(null)
    window.getSelection()?.removeAllRanges()
    // The reply is typed under the quote: the box takes the keys, with the caret at its end.
    const field = box.querySelector<HTMLTextAreaElement>('textarea.mx-box-text:not(:disabled)')
    window.requestAnimationFrame(() => {
      field?.focus({ preventScroll: true })
      field?.setSelectionRange(field.value.length, field.value.length)
    })
  }
  const [known, setKnown] = useState<Set<string> | null>(null)
  const [knownAsOf, setKnownAsOf] = useState(0)
  if (feed.loaded && knownAsOf !== unwatchedReads) {
    setKnownAsOf(unwatchedReads)
    setKnown(new Set(rows.map(rowKey)))
  }
  const pending = useMemo(() => allRows.find((row): row is Extract<ChatRow, { kind: 'approval' }> => isPendingApproval(row))?.event ?? null, [allRows])
  // The model and effort picked on the new-chat page are this conversation's choice from its first frame, so the picker shows them.
  useLayoutEffect(() => {
    if (sessionId === undefined) return
    if (started !== null) writeChoice(link.conversation, { model: started.model, reasoning: started.reasoning })
    releaseStartChoice(sessionId)
  }, [sessionId, started, link.conversation])
  // The session's unsent text comes along when its chat was given a new conversation number.
  useEffect(() => {
    if (sessionId !== undefined) followComposer(sessionId, link.conversation)
  }, [sessionId, link.conversation])
  // A draft typed while the agent started comes over once, and a message sent then goes out as soon as the agent takes input.
  useEffect(() => {
    if (sessionId === undefined) return
    const carried = takeCarriedDraft(sessionId)
    if (carried !== null) setComposerDraft(carried)
  }, [sessionId, setComposerDraft])
  useEffect(() => {
    if (sessionId === undefined || held === null || heldByDaemon || !ready || sending) return
    releaseFirstMessage(sessionId)
    void sendText(held).then((delivered) => { if (!delivered) setComposerDraft(held) })
  }, [sessionId, held, heldByDaemon, ready, sending, sendText, setComposerDraft])
  // The daemon types in a message it holds: once the history has what the person said, the stand-in for it goes.
  const personSpoke = useMemo(() => events.some((event) => event.kind === 'message' && event.role === 'user'), [events])
  useEffect(() => {
    if (sessionId !== undefined && held !== null && heldByDaemon && personSpoke) releaseFirstMessage(sessionId)
  }, [sessionId, held, heldByDaemon, personSpoke])
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
  // "Always in this project": this one is allowed now, and the daemon allows the same command in this project from then on.
  const remember = useCallback((requestId: number | string) => {
    setAnswering(true)
    allowAlways({ id: String(link.conversation), generation: link.generation }, requestId)
      .then(() => { recordAnswer(requestId, 'Accept'); refresh() })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not remember your answer')) })
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
  const approval = useMemo(() => (online && link.readOnly !== true ? { answer, answerInput, remember, busy: answering } : null), [online, link.readOnly, answer, answerInput, remember, answering])

  // An approval's link to its tool step scrolls the list there; the nonce makes a second click on the same link scroll again.
  const [scrollTarget, setScrollTarget] = useState<{ key: string; nonce: number } | null>(null)
  const goToStep = useCallback((key: string) => { setScrollTarget((current) => ({ key, nonce: (current?.nonce ?? 0) + 1 })) }, [])

  // Find (the window's search bar) looks through this chat while it shows: the row that holds the text is scrolled to and marked.
  const [found, setFound] = useState<string | null>(null)
  const rowsNow = useLatest(() => rows)
  const foundNow = useLatest(() => found)
  useEffect(() => {
    if (sessionId === undefined) return undefined
    return registerChatFind(sessionId, {
      find: (query, direction) => {
        const key = findRow(rowsNow(), query, foundNow(), direction)
        setFound(key)
        if (key !== null) goToStep(key)
        return key !== null
      },
      clear: () => { setFound(null) },
    })
  }, [sessionId, rowsNow, foundNow, goToStep])

  const waitingStep = pending?.stepId ?? null
  const plan = useMemo(() => (ask?.tool === PLAN_TOOL ? askedPlan(events) : null), [ask?.tool, events])
  // The daemon refuses a restore while the agent works too: its files would be rolled back under it.
  const rewindBusy = !online ? 'The daemon is offline' : working ? `${agent} is working. Edit from here when it finishes.` : null
  const editFromHere = useEditFromHere({ place: rewind ?? null, conversation: link.conversation, events, agent, busy: rewindBusy })
  const editAction = editFromHere.action
  const renderRow = useCallback(
    (row: ChatRow) => {
      const item = <ChatItem row={row} agent={agent} workingSince={workingSince} approval={approval} root={root} openDiff={onOpenDiff} goToStep={goToStep} waitingStep={waitingStep} editFromHere={editAction} />
      return row.key === found ? <div className="mx-chat-found" data-found="true">{item}</div> : item
    },
    [agent, workingSince, approval, root, onOpenDiff, goToStep, found, waitingStep, editAction],
  )

  // y, a and n answer the tray from anywhere in the chat but a field, as the same letters do in the terminal.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.ctrlKey || event.metaKey || event.altKey || typedInField(event.target)) return
    // A question's number picks its choice, as it does in the agent's own menu.
    const choice = Number(event.key) - 1
    const choices = ask?.choices ?? []
    if (stoppedOnPrompt && online && pickChoice !== undefined && (ask?.questions?.length ?? 0) <= 1 && Number.isInteger(choice) && choice >= 0 && choice < choices.length) {
      event.preventDefault()
      void pickChoice(choice)
      return
    }
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

  // The pane hands a new function on every render; the rows are given one that stays the same, so they are not drawn again for it.
  const openFileRef = useRef(onOpenFile)
  useEffect(() => { openFileRef.current = onOpenFile }, [onOpenFile])
  const canOpenFile = onOpenFile !== null
  const openFile = useMemo<OpenFile | null>(() => (canOpenFile ? (path, line) => { openFileRef.current?.(path, line) } : null), [canOpenFile])

  const question = pending !== null
    ? <ApprovalTray event={pending} actions={approval} agent={agent} />
    : stoppedOnPrompt ? <PromptTray agent={agent} ask={ask} online={online} answer={answerPrompt} pick={pickChoice} pickAll={pickChoices} sendKeys={sendKeys} readScreen={readScreen} plan={plan} /> : problem !== null ? <ProblemBanner problem={problem} summarize={() => summaryOf(bareRows)} /> : null
  const notices = [
    offline ? `Reconnecting to the daemon. What is shown may be out of date.${readError !== null ? ` (${truncateError(readError)})` : ''}` : null,
    !offline && incomplete !== null ? `History is incomplete: ${incomplete}.` : null,
    feed.error,
  ].filter((notice): notice is string => notice !== null)

  return (
    <div ref={chat} className="mx-chat" onKeyDown={onKeyDown} onClick={keepComposerFocus} onMouseUp={(event) => { const next = pickedText(event); if (next !== undefined) setPicked(next) }}>
      {editFromHere.ask !== null && <ConfirmCloseDialog title="Restore the files and edit this message?" body={editFromHere.ask.body} confirmLabel={editFromHere.ask.confirmLabel} onConfirm={editFromHere.confirm} onCancel={editFromHere.cancel} firstFocus="cancel" />}
      {picked !== null && <SelectionBar picked={picked} onCopy={copyPicked} onQuote={() => { if (chat.current !== null) quotePicked(chat.current) }} />}
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
          <SubagentChat subagent={subagent} agent={agent} root={root} term={sessionId !== undefined && machineNameOf(sessionId) === null ? terminalOf(sessionId) : null} />
        </>
      ) : feed.loaded && rows.length === 0 ? (
        <p className="mx-chat-empty">{hasTerminal ? 'No messages yet. Send one below, or type in the terminal.' : 'No messages yet. Send one below.'}</p>
      ) : feed.loaded || rows.length > 0 ? (
        // A message held from the starting chat is a row before the history is read, so it stays where it was and no skeleton replaces it.
        <KnownRowsContext.Provider value={known}>
          <OpenFileContext.Provider value={openFile}>
          <VirtualList items={rows} getKey={rowKey} estimate={ESTIMATED_ROW_PX} renderItem={renderRow} label={`Conversation with ${agent}`} scrollTarget={scrollTarget} />
          </OpenFileContext.Provider>
        </KnownRowsContext.Provider>
      ) : (
        <ChatSkeleton />
      )}
      <Composer
        agent={agent}
        composer={composer}
        conversation={link.conversation}
        // Claude says which commands it takes, in its folder; Codex's are a list the window keeps. Gemini has none yet.
        {...(abilities.slashCommands !== null && root !== null ? { commandsFrom: { agent, folder: root, inTerminal: runInTerminal } } : {})}
        question={question}
        waiting={held !== null}
        files={fileSearch}
        facts={{ ...(usage === undefined ? {} : { usage }), ...(branchMenu === undefined ? {} : { branchMenu }), mode: modeLabel(shownMode), modeDanger: shownMode !== null && DANGER_MODES.includes(shownMode), folder, branch, worktree, model: shownModel, effort, onOpenFolder, modeKey: shownMode, bypassAllowed: startedWithYolo(sessionId), ...agentControls }}
      />
    </div>
  )
}
