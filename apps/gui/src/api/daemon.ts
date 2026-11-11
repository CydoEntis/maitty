import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'
import { bytesOf } from './ipcBytes'
import { assertLocal, type LocalFeature } from '@/lib/localOnly'
import { LOCAL_MACHINE, addressOf, machineArgs, terminalOf, terminalSessionId } from './machines'
import { previewConversations, type Conversation, type Message, type SessionStatus } from '@/data/preview'

const StatusSchema = z.enum(['None', 'Idle', 'Working', 'Blocked', 'Done'])
const ConversationSummarySchema = z.object({
  id: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
  execution: z.enum(['Native', 'Structured']),
  provider: z.enum(['Codex', 'Claude']),
  cwd: z.string(),
  worktree: z.string().nullable(),
  state: z.enum(['Stopped', 'Running', 'Transferring', 'RecoveryRequired']),
  status: StatusSchema,
  native_term: z.number().int().nonnegative().nullable(),
  checks: z.array(z.unknown()),
  title: z.string().nullable().default(null),
  archived: z.boolean().default(false),
  root: z.string().nullable().default(null),
  branch: z.string().nullable().default(null),
})
const WorkspaceSummarySchema = z.object({
  cwd: z.string(),
  git: z.object({ branch: z.string(), dirty: z.number().int().nonnegative() }).nullable(),
})
const TerminalSessionSchema = z.object({
  id: z.number().int().nonnegative(),
  agent: z.string().nullable(),
  process: z.string().default(''),
  summary: z.string().default(''),
  status: StatusSchema,
  cwd: z.string(),
  root: z.string().nullable().default(null),
  branch: z.string().nullable().default(null),
  // The pane runs in a linked worktree rather than the main checkout (the daemon's `TermInfo.linked`).
  linked: z.boolean().default(false),
  label: z.string().default(''),
  pinned: z.boolean().default(false),
  // Whether the agent's hooks have reported in; an agent started before they were installed never does.
  hooked: z.boolean().default(false),
  archived: z.boolean().default(false),
  conversation_id: z.number().nullable().default(null),
  // Lines added and removed in the session's checkout against HEAD; zero outside git.
  diff_added: z.number().int().nonnegative().default(0),
  diff_removed: z.number().int().nonnegative().default(0),
  popup: z.boolean().default(false),
  remote: z.string().nullable().default(null),
  // When the status last changed, in unix seconds; the sidebar row's time.
  since: z.number().nonnegative().default(0),
  asking: z.string().default(''),
  // The agent's own hook said what its permission prompt asks (not read off the screen).
  ask: z.object({ tool: z.string(), detail: z.string(), answerable: z.boolean().default(false), choices: z.array(z.string()).default([]) }).nullable().default(null),
  interrupted: z.object({ agent: z.string(), resume: z.string() }).nullable().default(null),
  // How the last command ended, when it ended in failure (the daemon's `TermInfo.exit`); absent from an older daemon.
  exit: z.object({ code: z.number().int().nullable().default(null), at: z.number().default(0) }).nullable().default(null),
  timeline: z.array(z.object({
    at: z.number().int().nonnegative(),
    kind: z.enum(['Working', 'NeedsYou', 'Done', 'Idle', 'Prompt', 'CommandStarted', 'CommandFinished']),
    detail: z.string().default(''),
  })).default([]),
  usage: z.object({
    context: z.number().nullable().default(null),
    cost: z.number().nullable().default(null),
    tokens: z.number().nullable().default(null),
  }).default({ context: null, cost: null, tokens: null }),
  // The helpers the session asked to start and waits on the person for, one task each (`TermInfo.helper_ask`).
  helper_ask: z.array(z.string()).default([]),
  // The agent's subagents as its runtime reports them (`TermInfo.subagent_activity`); times are unix seconds.
  subagent_activity: z.array(z.object({
    id: z.string(),
    name: z.string(),
    state: z.enum(['Starting', 'Working', 'WaitingForInput', 'AwaitingApproval', 'Completed', 'Failed', 'Cancelled', 'ConnectionLost']),
    task: z.string().nullable().default(null),
    started_at: z.number().int().nonnegative().nullable().default(null),
    updated_at: z.number().int().nonnegative().default(0),
    latest_activity: z.string().nullable().default(null),
    result: z.string().nullable().default(null),
  })).default([]),
})
const PlanLimitSchema = z.object({ name: z.string(), used: z.number(), resets_at: z.number().default(0) })

const DaemonSnapshotSchema = z.object({
  // Plan limits by agent (`Snapshot.limits`), as each agent last reported them. A shape this app cannot read is no limits, not no sessions.
  limits: z.array(z.tuple([z.string(), z.array(PlanLimitSchema)])).default([]).catch([]),
  // Read one by one, like the terminals: a record this app can't read must not take every session offline.
  conversations: z.array(z.unknown()).default([]),
  workspaces: z.array(WorkspaceSummarySchema),
  // Read one by one: a terminal this app can't read is left out rather than hiding every session.
  terms: z.record(z.string(), z.unknown()).default({}),
})

const HistoryCursorSchema = z.object({
  generation: z.number().int().nonnegative(),
  conversation: z.number().int().nonnegative(),
  revision: z.number().int().nonnegative(),
  next_position: z.number().int().nonnegative(),
  // Absent from a daemon that predates them. A cursor goes back to the daemon as it came, so these are kept.
  epoch: z.number().int().nonnegative().optional(),
  delta: z.boolean().optional(),
  snapshot: z.number().int().nonnegative().optional(),
})
const HistoryCompletenessSchema = z.union([
  z.enum(['Unknown', 'Loading', 'Complete']),
  z.object({ Incomplete: z.enum(['Disconnected', 'UnsupportedItem', 'InvalidEvent', 'MissingPages', 'RevisionExhausted']) }),
])
const HistoryEntrySchema = z.object({
  source: z.object({ conversation: z.number().int().nonnegative(), turn: z.string(), item: z.string() }),
  content: z.union([
    z.literal('TurnStatus'),
    z.object({ Message: z.object({ role: z.enum(['User', 'Assistant']), text: z.string() }) }),
    z.object({
      Tool: z.object({
        kind: z.string(),
        output: z.string(),
        label: z.string().optional(),
        input: z.unknown().optional(),
        exit_code: z.number().int().optional(),
        duration_ms: z.number().int().nonnegative().optional(),
      }),
    }),
    z.object({ Unsupported: z.object({ kind: z.string() }) }),
  ]),
  state: z.enum(['Running', 'Completed', 'Failed', 'Interrupted']),
  started_at_ms: z.number().int().nonnegative().optional(),
  completed_at_ms: z.number().int().nonnegative().optional(),
})
const HistoryPageSchema = z.object({
  entries: z.array(HistoryEntrySchema),
  next: HistoryCursorSchema.nullable(),
  completeness: HistoryCompletenessSchema,
  // Read from here to get only what changed since this page; the page starts over when `reset`.
  resume: HistoryCursorSchema.nullable().optional(),
  reset: z.boolean().optional(),
})
const ConversationReceiptSchema = z.object({
  conversation: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
  operation: z.number().int().nonnegative(),
  delivery: z.union([
    z.literal('SubmissionUncertain'),
    z.object({ Acknowledged: z.object({ turn_id: z.string() }) }),
  ]),
})
// The daemon held the message because a turn was running (see `ReadResult.queued`).
const QueuedReceiptSchema = z.object({
  conversation: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
  operation: z.number().int().nonnegative(),
})
const QueuedMessageSchema = z.object({ operation: z.number().int().nonnegative(), text: z.string() })
const FileChangePreviewSchema = z.object({ path: z.string(), kind: z.string(), diff: z.string() })
const UserInputQuestionSchema = z.object({
  id: z.string(),
  header: z.string(),
  question: z.string(),
  is_other: z.boolean(),
  is_secret: z.boolean(),
  options: z.array(z.object({ label: z.string(), description: z.string() })),
})
const ApprovalRequestSchema = z.object({
  request_id: z.union([z.number().int(), z.string()]),
  kind: z.enum(['Command', 'FileChange', 'Permission', 'UserInput']),
  // Names the tool step the request is about (Claude: the tool-use id; Codex: the item id).
  item_id: z.string().nullable().optional(),
  turn_id: z.string().nullable().optional(),
  command: z.string().nullable(),
  reason: z.string().nullable(),
  cwd: z.string().nullable(),
  changes: z.array(FileChangePreviewSchema),
  questions: z.array(UserInputQuestionSchema),
  permissions: z.unknown().nullable().optional(),
  choices: z.array(z.enum(['Accept', 'AcceptForSession', 'Decline', 'Cancel'])),
})
const ReadResultSchema = z.object({
  conversation: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
  status: StatusSchema,
  receipt: ConversationReceiptSchema.nullable(),
  error: z.string().nullable(),
  cancelling: z.boolean(),
  history: HistoryPageSchema,
  checks: z.array(z.unknown()),
  approvals: z.array(ApprovalRequestSchema),
  // Messages the daemon holds until the running turn ends; absent from a daemon that predates the queue.
  queued: z.array(QueuedMessageSchema).default([]),
  // Approvals a person already answered (most recent last); absent from a daemon that predates it.
  answered_approvals: z
    .array(
      z.object({
        request: ApprovalRequestSchema,
        decision: z.enum(['Accept', 'AcceptForSession', 'Decline', 'Cancel']),
        answered_at_ms: z.number().int().nonnegative().nullable().optional(),
      }),
    )
    .default([]),
})
const ConversationDiffSchema = z.object({
  files: z.array(z.object({ path: z.string(), additions: z.number().int().nonnegative(), deletions: z.number().int().nonnegative(), patch: z.string(), binary: z.boolean() })),
  additions: z.number().int().nonnegative(),
  deletions: z.number().int().nonnegative(),
  total_files: z.number().int().nonnegative(),
  truncated: z.boolean(),
})
const ConversationFileEntrySchema = z.object({ name: z.string(), is_directory: z.boolean() })
const CreatedConversationSchema = z.object({
  id: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
})
const DaemonRestartStatusSchema = z.object({
  daemonProtocol: z.number().int().positive(),
  guiProtocol: z.number().int().positive(),
  canRestart: z.boolean(),
  sessionCount: z.number().int().nonnegative(),
  activeTurnCount: z.number().int().nonnegative().nullable(),
})

/** A service that still talks to this app but runs an older release; `behind` is false for one that is current, newer or silent. */
const DaemonUpdateStatusSchema = z.object({
  daemonVersion: z.string().nullable(),
  behind: z.boolean(),
  canRestart: z.boolean(),
  sessionCount: z.number().int().nonnegative(),
  activeTurnCount: z.number().int().nonnegative().nullable(),
})

type ConversationSummary = z.infer<typeof ConversationSummarySchema>
type WorkspaceSummary = z.infer<typeof WorkspaceSummarySchema>
type ReadResult = z.infer<typeof ReadResultSchema>
type HistoryCompleteness = z.infer<typeof HistoryCompletenessSchema>

export type ConversationReceipt = {
  conversation: number
  generation: number
  operation: number
  delivery: 'SubmissionUncertain' | { Acknowledged: { turn_id: string } }
}

export type ConversationHistory = {
  generation: number
  receipt: ConversationReceipt | null
  messages: Message[]
  nextOperation: number
  error: string | null
  cancelling: boolean
  completeness: HistoryCompleteness
}

export type ConversationDiff = {
  files: Array<{
    path: string
    additions: number
    deletions: number
    patch: string
    binary: boolean
  }>
  additions: number
  deletions: number
  total_files: number
  truncated: boolean
}

export type ConversationFileEntry = z.infer<typeof ConversationFileEntrySchema>

export function isDesktop(): boolean {
  return isTauri()
}

export async function loadCodexExecutablePath(): Promise<string> {
  if (!isDesktop()) throw new Error('Codex executable lookup is available in the desktop app')
  return invokeParsed('codex_executable_path', z.string().min(1))
}

export async function loadClaudeExecutablePath(): Promise<string> {
  if (!isDesktop()) throw new Error('Claude executable lookup is available in the desktop app')
  return invokeParsed('claude_executable_path', z.string().min(1))
}

export async function loadDaemonRestartStatus() {
  if (!isDesktop()) throw new Error('Daemon restart status is available in the desktop app')
  return invokeParsed('daemon_restart_status', DaemonRestartStatusSchema)
}

export async function restartMismatchedDaemon(expectedProtocol: number): Promise<void> {
  if (!isDesktop()) throw new Error('Daemon restart is available in the desktop app')
  if (!Number.isSafeInteger(expectedProtocol) || expectedProtocol < 1) throw new Error('Daemon protocol is invalid')
  await invoke('restart_mismatched_daemon', { expectedProtocol })
}

export async function loadDaemonUpdateStatus() {
  if (!isDesktop()) throw new Error('Service update status is available in the desktop app')
  return invokeParsed('daemon_update_status', DaemonUpdateStatusSchema)
}

/** Restarts the service shown as `expectedVersion`; the host refuses when it is no longer that version. */
export async function restartStaleDaemon(expectedVersion: string): Promise<void> {
  if (!isDesktop()) throw new Error('Service restart is available in the desktop app')
  if (!expectedVersion) throw new Error('Service version is invalid')
  await invoke('restart_stale_daemon', { expectedVersion })
}

/** Shows the service's log in the system's file manager. Rejects with the host's words, such as "There is no log yet." */
export async function openDaemonLog(): Promise<void> {
  if (!isDesktop()) throw new Error('The log is available in the desktop app')
  await invoke('open_daemon_log')
}

/** Opens a web address in the default browser. Rejects for anything that is not an http(s) address. */
export async function openLink(url: string): Promise<void> {
  await invokeParsed('open_link', NullSchema, { url })
}

/** Saves a copy of the service's recent log in the Downloads folder, to send to someone; resolves to where it is. */
export async function exportDaemonLog(): Promise<string> {
  if (!isDesktop()) throw new Error('The log is available in the desktop app')
  return invokeParsed('export_daemon_log', z.string(), {})
}

const ProcessLoadSchema = z.object({
  pid: z.number().int().nonnegative(),
  name: z.string(),
  command: z.string(),
  cpu: z.number(),
  memory_bytes: z.number().nonnegative(),
})
const ProcessSampleSchema = z.object({
  cores: z.number().int().positive(),
  host: ProcessLoadSchema,
  webview: z.array(ProcessLoadSchema),
  daemons: z.array(ProcessLoadSchema),
  terminal_frames_sent: z.number().int().nonnegative(),
})

export type ProcessLoad = z.infer<typeof ProcessLoadSchema>
/** The host's, the web view's and the daemons' CPU (percent of one core) and memory. */
export type ProcessSample = z.infer<typeof ProcessSampleSchema>

export async function loadPerformanceSample(): Promise<ProcessSample> {
  if (!isDesktop()) throw new Error('Process figures are available in the desktop app')
  return invokeParsed('performance_sample', ProcessSampleSchema)
}

const MachineListSchema = z.object({ names: z.array(z.string()), error: z.string().nullable() })
export type MachineList = z.infer<typeof MachineListSchema>

/**
 * The machines in this computer's `config.local.toml`, by name, and what is wrong with the file
 * when it is bad. The host reads it once at start, so a hand edit applies on the next start.
 */
export async function loadMachines(): Promise<MachineList> {
  if (!isDesktop()) return { names: [], error: null }
  return invokeParsed('list_machines', MachineListSchema)
}

async function invokeParsed<T>(command: string, schema: z.ZodType<T>, args?: Record<string, unknown>): Promise<T> {
  const result = await invoke<unknown>(command, args)
  return schema.parse(result)
}

const NullSchema = z.null()
const ReadPageLimit = 128

export async function loadConversations(): Promise<Conversation[]> {
  if (!isDesktop()) return previewConversations
  return conversationsFromSnapshot(await invoke<unknown>('list_sessions'))
}

/**
 * Calls back with the daemon's snapshot each time it changes (the same shape `list_sessions`
 * returns), or with null when the host lost the daemon and the list has to be asked for again.
 */
export function listenSessionsChanged(onChange: (snapshot: unknown) => void): Promise<() => void> {
  return listen<unknown>('sessions-changed', (event) => { onChange(event.payload) })
}

/** A machine whose snapshot this is: its number for this run, and its name. This computer when left out. */
export type SnapshotMachine = { id: number; name: string }

/**
 * The sessions in a daemon snapshot, from `list_sessions` or the `sessions-changed` event, or
 * (with `machine`) in another machine's snapshot from `machine-sessions-changed`. Another
 * machine's terminals are all the window shows of it; its ids carry the machine's name.
 */
export function conversationsFromSnapshot(raw: unknown, machine?: SnapshotMachine): Conversation[] {
  const snapshot = DaemonSnapshotSchema.parse(raw)
  const from = machine?.id ?? LOCAL_MACHINE
  for (const key of lastGoodTerm.keys()) {
    if (isOfMachine(key, from) && !(String(termOfKey(key)) in snapshot.terms)) lastGoodTerm.delete(key)
  }
  const summaries = machine === undefined
    ? snapshot.conversations.flatMap((raw) => {
      const parsed = ConversationSummarySchema.safeParse(raw)
      return parsed.success ? [parsed.data] : []
    })
    : []
  // A terminal session has a chat when the daemon holds a conversation whose native terminal is that pane.
  // The daemon only follows an agent the user started there, so that chat is read-only (ADR 0003).
  const chatOf = new Map(summaries.flatMap((summary) => (summary.native_term === null ? [] : [[summary.native_term, { conversation: summary.id, generation: summary.generation, ...(summary.execution === 'Native' ? { readOnly: true } : {}) }] as const])))
  const limitsOf = new Map(snapshot.limits)
  const listed = Object.values(snapshot.terms)
    .flatMap((term) => {
      const read = readTerminal(term, from)
      return read ? [read] : []
    })
    .filter(isTerminalSession)
  // A conversation that belongs to a pane in this list is that session's chat, not a session of its own.
  const shownPanes = new Set(listed.map((term) => term.id))
  const terminals = listed
    .map((term) => toTerminalConversation(term, snapshot.workspaces, machine, machine === undefined ? chatOf.get(term.id) : undefined, term.agent === null ? undefined : limitsOf.get(term.agent)))
  if (machine !== undefined) return terminals
  return [
    ...summaries
      .filter((summary) => summary.native_term === null || !shownPanes.has(summary.native_term))
      .map((conversation) => toConversation(conversation, snapshot.workspaces)),
    ...terminals,
  ]
}

type TerminalSession = z.infer<typeof TerminalSessionSchema>

// A session that fails to read for one refresh must not vanish from the sidebar and come back
// 1.5 seconds later, so it keeps its last good state, and the reason is logged once.
// Each machine numbers its panes from 1, so these are kept by machine and pane together.
const lastGoodTerm = new Map<string, TerminalSession>()
const unreadable = new Set<string>()

function paneKey(machine: number, term: number): string {
  return `${String(machine)}:${String(term)}`
}

function isOfMachine(key: string, machine: number): boolean {
  return key.startsWith(`${String(machine)}:`)
}

function termOfKey(key: string): number {
  return Number(key.slice(key.indexOf(':') + 1))
}

function readTerminal(raw: unknown, machine: number): TerminalSession | null {
  const parsed = TerminalSessionSchema.safeParse(raw)
  const id = typeof raw === 'object' && raw !== null && 'id' in raw && typeof raw.id === 'number' ? raw.id : null
  if (parsed.success) {
    const key = paneKey(machine, parsed.data.id)
    lastGoodTerm.set(key, parsed.data)
    unreadable.delete(key)
    return parsed.data
  }
  if (id === null) return null
  const key = paneKey(machine, id)
  if (!unreadable.has(key)) {
    unreadable.add(key)
    console.warn(`Session ${machine === LOCAL_MACHINE ? '' : `of machine ${String(machine)} `}t${String(id)} could not be read; showing its last good state`, parsed.error.issues)
  }
  return lastGoodTerm.get(key) ?? null
}

// Terminal sessions are ids like "t12", the pane's number kept apart from a chat's number, or
// "build/t12" for a pane of another machine (see `machines.ts`).
export { terminalOf, terminalSessionId }

// A terminal this app can show as a session: a shell (with or without an agent running in it) in
// a pane of its own, not a chat's terminal, a popup, or another machine's. The daemon
// already leaves out panes another app owns (ADR 0004).
function isTerminalSession(term: TerminalSession): boolean {
  return term.conversation_id === null && !term.popup && term.remote === null
}

function toolActivityLabel(tool: { kind: string; label?: string | undefined }): string {
  const label = tool.label?.trim()
  if (label) return label
  return tool.kind === 'commandExecution' ? 'Run command' : tool.kind
}

export async function loadMessages(
  conversationId: string,
  provider: Conversation['provider'],
): Promise<ConversationHistory> {
  if (!isDesktop() || terminalOf(conversationId) !== null) return { generation: 0, receipt: null, messages: [], nextOperation: 1, error: null, cancelling: false, completeness: 'Complete' }

  const id = Number(conversationId)
  if (!Number.isSafeInteger(id) || id < 0) throw new Error('Conversation id is invalid')

  let page = await invokeParsed('read_conversation', ReadResultSchema, { id, cursor: null, limit: ReadPageLimit })
  if (page.conversation !== id) throw new Error('Daemon returned a different conversation')
  const generation = page.generation
  const entries = [...page.history.entries]
  let completeness = page.history.completeness
  const visitedCursors = new Set<string>()
  while (page.history.next) {
    const cursor = page.history.next
    if (cursor.conversation !== id || cursor.generation !== generation) {
      throw new Error('Conversation history cursor belongs to a different conversation')
    }
    const cursorKey = JSON.stringify(cursor)
    if (visitedCursors.has(cursorKey)) throw new Error('Conversation history cursor repeated')
    visitedCursors.add(cursorKey)

    const nextPage = await invokeParsed('read_conversation', ReadResultSchema, {
      id,
      cursor,
      limit: ReadPageLimit,
    })
    if (nextPage.conversation !== id || nextPage.generation !== generation) {
      throw new Error('Conversation changed while its history was being read')
    }
    entries.push(...nextPage.history.entries)
    if (nextPage.history.completeness !== 'Complete') completeness = nextPage.history.completeness
    page = nextPage
  }

  const history: ReadResult = { ...page, history: { ...page.history, entries } }
  const messages = history.history.entries.flatMap((entry): Message[] => {
    if (entry.content === 'TurnStatus') return []
    if ('Message' in entry.content) {
      return [{
        id: `${entry.source.turn}:${entry.source.item}`,
        role: entry.content.Message.role === 'User' ? 'user' : 'assistant',
        content: entry.content.Message.text,
        time: 'Recent',
      }]
    }
    if ('Tool' in entry.content) {
      return [{
        id: `${entry.source.turn}:${entry.source.item}`,
        role: 'activity',
        content: toolActivityLabel(entry.content.Tool),
        detail: entry.content.Tool.output,
        activityState: entry.state,
        time: 'Recent',
      }]
    }
    return [{
      id: `${entry.source.turn}:${entry.source.item}`,
      role: 'activity',
      content: 'Unsupported provider activity',
      detail: entry.content.Unsupported.kind,
      activityState: entry.state,
      time: 'Recent',
    }]
  })
  messages.push(...history.approvals.map((approval): Message => ({
    id: `approval:${typeof approval.request_id}:${String(approval.request_id)}`,
    role: 'approval',
    content: approval.kind === 'FileChange' ? `${provider} wants to change project files` : approval.kind === 'Permission' ? `${provider} is requesting additional permissions` : approval.kind === 'UserInput' ? `${provider} needs your input` : `${provider} wants to run a command`,
    detail: approval.command ?? '',
    time: 'Needs your decision',
    providerRequestId: approval.request_id,
    approvalChoices: approval.choices,
    ...(approval.reason ? { approvalReason: approval.reason } : {}),
    approvalKind: approval.kind === 'UserInput' ? 'UserInput' : approval.kind,
    ...(approval.cwd ? { approvalScope: approval.cwd } : {}),
    ...(approval.changes.length > 0 ? { approvalChanges: approval.changes } : {}),
    ...(approval.permissions && typeof approval.permissions === 'object' && !Array.isArray(approval.permissions) ? { approvalPermissions: approval.permissions as Record<string, unknown> } : {}),
    ...(approval.questions.length > 0 ? { userQuestions: approval.questions } : {}),
  })))
  if (history.error) {
    messages.push({
      id: 'conversation-error',
      role: 'activity',
      content: 'Conversation needs attention',
      detail: history.error,
      time: 'Now',
    })
  }
  return {
    generation,
    receipt: history.receipt,
    messages,
    nextOperation: (history.receipt?.operation ?? 0) + 1,
    error: history.error,
    cancelling: history.cancelling,
    completeness,
  }
}

// The chat view's read of a conversation: entries and approvals stay raw (`unknown`) because the chat
// projection validates each one itself, so a single entry this app cannot read never hides the rest.
const ChatPageSchema = z.object({
  conversation: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
  status: StatusSchema,
  receipt: ConversationReceiptSchema.nullable(),
  error: z.string().nullable(),
  cancelling: z.boolean(),
  history: z.object({
    entries: z.array(z.unknown()),
    next: HistoryCursorSchema.nullable(),
    completeness: HistoryCompletenessSchema,
    resume: HistoryCursorSchema.nullable().optional(),
    reset: z.boolean().optional(),
  }),
  approvals: z.array(z.unknown()),
  // Approvals a person already answered; the chat projection validates each one. Absent from an older daemon.
  answered_approvals: z.array(z.unknown()).default([]),
  queued: z.array(QueuedMessageSchema).default([]),
  // How the agent is set up, from its own file (Claude: permission mode and model); absent from an older daemon.
  meta: z.object({ mode: z.string().nullable().default(null), model: z.string().nullable().default(null) }).nullable().default(null),
})
export type HistoryCursor = z.infer<typeof HistoryCursorSchema>
export type ChatPage = z.infer<typeof ChatPageSchema>

/**
 * One page of a conversation's history from `cursor` (null: the start), with the approvals waiting
 * and the conversation's status. A cursor goes stale when the history changes; the caller then
 * reads again from the start.
 */
export async function readConversationPage(id: number, cursor: HistoryCursor | null): Promise<ChatPage> {
  if (!isDesktop()) throw new Error('Chat needs the desktop app')
  if (!Number.isSafeInteger(id) || id < 0) throw new Error('Conversation id is invalid')
  const page = await invokeParsed('read_conversation', ChatPageSchema, { id, cursor, limit: ReadPageLimit })
  if (page.conversation !== id) throw new Error('Daemon returned a different conversation')
  return page
}

export async function loadConversationDiff(conversation: Conversation): Promise<ConversationDiff> {
  if (terminalOf(conversation.id) !== null) throw new Error('Changes are shown for chat sessions')
  if (!isDesktop() || conversation.generation === undefined) {
    throw new Error('Conversation diff is unavailable')
  }
  return invokeParsed('conversation_diff', ConversationDiffSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

export async function loadConversationFiles(
  conversation: Conversation,
  path: string | null,
): Promise<ConversationFileEntry[]> {
  if (terminalOf(conversation.id) !== null) throw new Error('Files are shown for chat sessions')
  if (!isDesktop() || conversation.generation === undefined) {
    throw new Error('Project files are available in the desktop app')
  }
  return invokeParsed('conversation_files', z.array(ConversationFileEntrySchema), {
    id: Number(conversation.id),
    generation: conversation.generation,
    path,
  })
}

export async function loadConversationFile(
  conversation: Conversation,
  path: string,
): Promise<string> {
  if (terminalOf(conversation.id) !== null) throw new Error('Files are shown for chat sessions')
  if (!isDesktop() || conversation.generation === undefined) {
    throw new Error('Project files are available in the desktop app')
  }
  return invokeParsed('conversation_file', z.string(), {
    id: Number(conversation.id),
    generation: conversation.generation,
    path,
  })
}

/**
 * The same three project questions for a session's chat link (the daemon conversation a pane is
 * linked to); `loadConversation*` above take a whole `Conversation`, which a chat link is not.
 */
export type ConversationTarget = { conversation: number; generation: number }

function requireProjectAccess(target: ConversationTarget): void {
  if (!isDesktop()) throw new Error('Project files are available in the desktop app')
  if (!Number.isSafeInteger(target.conversation) || target.conversation < 0) throw new Error('Conversation id is invalid')
}

export async function loadTargetDiff(target: ConversationTarget): Promise<ConversationDiff> {
  requireProjectAccess(target)
  return invokeParsed('conversation_diff', ConversationDiffSchema, { id: target.conversation, generation: target.generation })
}

export async function loadTargetFiles(target: ConversationTarget, path: string | null): Promise<ConversationFileEntry[]> {
  requireProjectAccess(target)
  return invokeParsed('conversation_files', z.array(ConversationFileEntrySchema), { id: target.conversation, generation: target.generation, path })
}

export async function loadTargetFile(target: ConversationTarget, path: string): Promise<string> {
  requireProjectAccess(target)
  return invokeParsed('conversation_file', z.string(), { id: target.conversation, generation: target.generation, path })
}

/** The folder a new session starts in when none is chosen: the home folder. */
export async function defaultSessionFolder(): Promise<string> {
  if (!isDesktop()) return ''
  return invokeParsed('default_session_folder', z.string())
}

export async function suggestProjectFolders(typed: string): Promise<string[]> {
  if (!isDesktop()) return []
  return invokeParsed('suggest_project_folders', z.array(z.string()), { typed })
}

/** Questions the host answers by opening its own connection to this computer's daemon cannot be asked of another machine's pane. */
function refuseOtherMachine(pane: { machine: number; machineName: string | null }, feature: LocalFeature): void {
  assertLocal(pane, feature)
}

export async function loadTerminalScreen(conversation: Pick<Conversation, 'id'> & { generation: number }): Promise<string> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  const pane = addressOf(conversation.id)
  if (pane !== null) {
    refuseOtherMachine(pane, 'screen-text')
    return invokeParsed('pane_terminal_screen', z.string(), { term: pane.term })
  }
  return invokeParsed('terminal_screen', z.string(), {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

export async function loadTerminalReplay(conversation: Pick<Conversation, 'id'> & { generation: number }): Promise<Uint8Array> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  const pane = addressOf(conversation.id)
  if (pane !== null) refuseOtherMachine(pane, 'replay')
  // The host answers with raw bytes: as JSON a replay would be a number per byte.
  const reply = pane !== null
    ? await invoke<unknown>('pane_terminal_replay', { term: pane.term })
    : await invoke<unknown>('terminal_replay', {
      id: Number(conversation.id),
      generation: conversation.generation,
    })
  return bytesOf(reply)
}

export async function sendTerminalInput(conversation: Pick<Conversation, 'id'> & { generation: number }, input: string | Uint8Array): Promise<void> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  const data = Array.from(typeof input === 'string' ? new TextEncoder().encode(input) : input)
  const pane = addressOf(conversation.id)
  if (pane !== null) {
    await invokeParsed('terminal_input', NullSchema, { term: pane.term, data, ...machineArgs(pane) })
    return
  }
  await invokeParsed('send_terminal_input', z.string(), {
    id: Number(conversation.id),
    generation: conversation.generation,
    data,
  })
}

export async function resizeTerminal(conversation: Pick<Conversation, 'id'> & { generation: number }, cols: number, rows: number): Promise<void> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  const pane = addressOf(conversation.id)
  if (pane !== null) {
    await invokeParsed('pane_terminal_resize', NullSchema, { term: pane.term, cols, rows, ...machineArgs(pane) })
    return
  }
  await invokeParsed('resize_terminal', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    cols,
    rows,
  })
}

export type QueuedReceipt = z.infer<typeof QueuedReceiptSchema>

/** The model and reasoning effort picked in the composer, sent with a message. Only what was picked is present. */
export type MessageChoice = { readonly model?: string; readonly effort?: string }

/**
 * Sends a message. A conversation that is mid-turn does not refuse it: the daemon holds it and sends it
 * when the turn ends, in order, even if this window closes, and answers with a `QueuedReceipt`.
 */
export async function submitMessage(conversation: Pick<Conversation, 'id' | 'generation'>, operation: number, text: string, choice?: MessageChoice): Promise<ConversationReceipt | QueuedReceipt> {
  return invokeParsed('send_conversation', z.union([ConversationReceiptSchema, QueuedReceiptSchema]), {
    id: Number(conversation.id),
    generation: conversation.generation,
    operation,
    text,
    ...(choice?.model === undefined ? {} : { model: choice.model }),
    ...(choice?.effort === undefined ? {} : { effort: choice.effort }),
  })
}

export function classifySubmissionFailure(
  history: Pick<ConversationHistory, 'generation' | 'receipt'>,
  expectedGeneration: number,
  operation: number,
): 'not-delivered' | 'unknown' | 'delivered' {
  if (history.generation !== expectedGeneration) return 'unknown'

  const receipt = history.receipt
  if (!receipt) return 'not-delivered'
  if (receipt.generation !== expectedGeneration || receipt.operation > operation) return 'unknown'
  if (receipt.operation < operation) return 'not-delivered'
  return receipt.delivery === 'SubmissionUncertain' ? 'unknown' : 'delivered'
}

/** Takes a message back out of the daemon's queue before it is sent; it fails when it was already sent. */
export async function dequeueMessage(conversation: Pick<Conversation, 'id' | 'generation'>, operation: number): Promise<void> {
  await invokeParsed('dequeue_conversation', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    operation,
  })
}

export async function approveRequest(conversation: Pick<Conversation, 'id' | 'generation'>, requestId: number | string, decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel'): Promise<void> {
  await invokeParsed('approve_conversation', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    requestId,
    decision,
  })
}

export async function answerUserInput(conversation: Pick<Conversation, 'id' | 'generation'>, requestId: number | string, answers: Record<string, string[]>): Promise<void> {
  await invokeParsed('answer_conversation_input', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    requestId,
    answers,
  })
}

export async function cancelTurn(conversation: Pick<Conversation, 'id' | 'generation'>): Promise<void> {
  await invokeParsed('cancel_conversation', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

/** Put a session away from the project lists and tabs, or bring it back. It keeps running: this is a view setting, never a kill. */
export async function archiveSessionOnDaemon(conversation: Pick<Conversation, 'id'>, archived: boolean): Promise<void> {
  if (!isDesktop()) return
  const pane = addressOf(conversation.id)
  if (pane !== null) await invokeParsed('archive_terminal_session', NullSchema, { term: pane.term, archived, ...machineArgs(pane) })
}

/** Restart an agent in place, keeping its name, so it loads hooks it missed. */
export async function reloadAgentOnDaemon(conversation: Pick<Conversation, 'id'>): Promise<void> {
  if (!isDesktop()) return
  const pane = addressOf(conversation.id)
  if (pane !== null) await invokeParsed('reload_terminal_session', NullSchema, { term: pane.term, ...machineArgs(pane) })
}

/**
 * Ends a terminal session with a six-second undo window: the daemon hides it at once and closes it
 * for real when the window passes. `undoCloseSession` brings it back while the window is open.
 */
export async function closeSessionWithUndo(conversation: Pick<Conversation, 'id'>): Promise<void> {
  if (!isDesktop()) return
  const pane = addressOf(conversation.id)
  if (pane !== null) await invokeParsed('close_session_with_undo', NullSchema, { term: pane.term, ...machineArgs(pane) })
}

export async function undoCloseSession(conversation: Pick<Conversation, 'id'>): Promise<void> {
  if (!isDesktop()) return
  const pane = addressOf(conversation.id)
  if (pane !== null) await invokeParsed('undo_close_terminal_session', NullSchema, { term: pane.term, ...machineArgs(pane) })
}

/** The person's answer to a session that asked to start helpers: this session, every session of its project, or none. */
export async function answerHelpers(conversation: Pick<Conversation, 'id'>, choice: 'allow' | 'always' | 'no'): Promise<void> {
  if (!isDesktop()) return
  const pane = addressOf(conversation.id)
  if (pane !== null) await invokeParsed('answer_helpers', NullSchema, { term: pane.term, choice, ...machineArgs(pane) })
}

/** The user is looking at this pane: a finished agent's "done" becomes idle, as when the terminal app focuses it. */
export async function markSeenOnDaemon(conversation: Pick<Conversation, 'id'>): Promise<void> {
  if (!isDesktop()) return
  const pane = addressOf(conversation.id)
  if (pane !== null) await invokeParsed('mark_terminal_seen', NullSchema, { term: pane.term, ...machineArgs(pane) })
}

/** Hands a message to a terminal session's daemon, which types it in once the agent starting there takes input. */
export async function sendWhenAgentReady(sessionId: string, text: string): Promise<void> {
  const pane = addressOf(sessionId)
  if (pane === null) throw new Error('Only a terminal session holds a message for its agent')
  await invokeParsed('send_when_agent_ready', NullSchema, { term: pane.term, text, ...machineArgs(pane) })
}

export async function renameConversationOnDaemon(conversation: Pick<Conversation, 'id'>, title: string): Promise<void> {
  const pane = addressOf(conversation.id)
  if (pane !== null) {
    await invokeParsed('rename_terminal_session', NullSchema, { term: pane.term, name: title, ...machineArgs(pane) })
    return
  }
  await invokeParsed('rename_conversation', NullSchema, { id: Number(conversation.id), title })
}

export async function endConversation(conversation: Conversation): Promise<void> {
  if (!isDesktop()) throw new Error('Ending a conversation is only available in the desktop app')
  if (conversation.generation === undefined) throw new Error('Conversation generation is unavailable')
  await invokeParsed('end_conversation', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

// Closing a chat only puts it away, but closing a terminal session ends its program, as in the
// terminal app. A terminal session that has ended is gone, so there is nothing to bring back.
export async function archiveConversationOnDaemon(conversation: Pick<Conversation, 'id'>, archived: boolean): Promise<void> {
  const pane = addressOf(conversation.id)
  if (pane !== null) {
    if (archived) await invokeParsed('close_terminal_session', NullSchema, { term: pane.term, ...machineArgs(pane) })
    return
  }
  await invokeParsed('archive_conversation', NullSchema, { id: Number(conversation.id), archived })
}

/**
 * Closing a chat session in the window ends it, as closing a terminal session ends its program: the
 * agent is stopped first (its record stays, so the history can be read) and then the session is put
 * away (ADR 0003). If the agent cannot be stopped nothing is put away, so a session the user believes
 * ended is never left running out of sight.
 */
export async function closeChatSession(conversation: Conversation): Promise<void> {
  await endConversation(conversation)
  await archiveConversationOnDaemon(conversation, true)
}

export type SessionPermissions = 'read-only' | 'workspace-write' | 'full-access'

export async function createConversation(
  folder: string,
  provider: 'Codex' | 'Claude',
  permissions: SessionPermissions,
): Promise<{ id: number; generation: number }> {
  return invokeParsed('create_conversation', CreatedConversationSchema, {
    cwd: folder,
    provider,
    permissions,
  })
}

/**
 * Starts a chat session: a Claude or Codex conversation the daemon owns, in `folder` or, with a
 * `branch`, in a new checkout of the project at `folder` (ADR 0003). It always asks before a tool
 * use the provider does not allow by itself, unless `fullAccess` asks for the provider's own
 * skip-permissions mode (no sandbox, no prompts; ADR 0005 and 0054).
 */
export async function startChatSession(folder: string, agent: 'claude' | 'codex', branch: string | null = null, fullAccess = false): Promise<{ id: number; generation: number }> {
  return invokeParsed('start_chat_session', CreatedConversationSchema, {
    cwd: folder,
    provider: agent === 'claude' ? 'Claude' : 'Codex',
    branch,
    fullAccess,
  })
}

/** Connects an agent's status hooks (`maitty integrate <agent>`), so its chat sees when it works, needs you or is done. */
export async function connectAgent(agent: 'claude' | 'codex' | 'gemini'): Promise<void> {
  await invokeParsed('connect_agent', NullSchema, { agent })
}

const AgentModelsSchema = z.array(z.object({ id: z.string(), label: z.string(), blurb: z.string(), isDefault: z.boolean().optional(), efforts: z.array(z.string()).nullish(), more: z.boolean().optional() }))

/** The models an agent offers on this computer; empty when it has no list to read. */
export async function loadAgentModels(agent: 'claude' | 'codex'): Promise<z.infer<typeof AgentModelsSchema>> {
  return invokeParsed('agent_models', AgentModelsSchema, { agent })
}

const AgentCommandsSchema = z.array(z.object({ name: z.string(), description: z.string(), hint: z.string(), aliases: z.array(z.string()) }))

/** The slash commands an agent takes in a folder (its own, and the person's and the project's); empty for an agent with no list to ask for. */
export async function loadAgentCommands(agent: string, folder: string): Promise<z.infer<typeof AgentCommandsSchema>> {
  return invokeParsed('agent_commands', AgentCommandsSchema, { agent, folder })
}

const AgentAuthSchema = z.object({ installed: z.boolean(), signedIn: z.boolean().nullable(), account: z.string(), plan: z.string() })
export type AgentAuth = z.infer<typeof AgentAuthSchema>

/** Whether an agent is installed and its account signed in, as the agent's own command says. */
export async function loadAgentAuth(agent: 'claude' | 'codex'): Promise<AgentAuth> {
  return invokeParsed('agent_auth_status', AgentAuthSchema, { agent })
}

/** Signs an agent's account out, with the agent's own command. */
export async function signOutAgent(agent: 'claude' | 'codex'): Promise<void> {
  await invokeParsed('agent_sign_out', NullSchema, { agent })
}

const InstalledAgentsSchema = z.array(z.string())

/** The agent programs found on this computer's PATH (`claude`, `codex`, `gemini`, `opencode`, `cursor-agent`). */
export async function agentsInstalled(): Promise<string[]> {
  return invokeParsed('agents_installed', InstalledAgentsSchema, {})
}

const FolderBranchesSchema = z.object({
  current: z.string().nullable(),
  branches: z.array(z.object({ name: z.string(), current: z.boolean(), when: z.number().nullable() })),
  worktrees: z.array(z.object({ path: z.string(), branch: z.string().nullable(), main: z.boolean(), when: z.number().nullable() })),
}).nullable()
export type FolderBranches = NonNullable<z.infer<typeof FolderBranchesSchema>>

/** A folder's branches (newest commit first) and worktrees; null when the folder is not in a git repository. */
export async function folderBranches(folder: string): Promise<FolderBranches | null> {
  return invokeParsed('git_branches', FolderBranchesSchema, { folder })
}

/** Switches the folder's checkout to a branch, made first when `create`. */
export async function switchBranch(folder: string, branch: string, create: boolean): Promise<void> {
  await invokeParsed('git_switch_branch', NullSchema, { folder, branch, create })
}

/** Adds a worktree of the folder's repository for a branch (made first when `create`); resolves to its folder. */
export async function addWorktree(folder: string, branch: string, create: boolean): Promise<string> {
  return invokeParsed('git_add_worktree', z.string(), { folder, branch, create })
}

/** Opens an agent's official setup page in the browser. */
export async function openAgentDocs(agent: string): Promise<void> {
  await invokeParsed('open_agent_docs', NullSchema, { agent })
}

/** Starts a plain shell in a folder; the user types what to run. */
export async function startShellSession(folder: string): Promise<void> {
  await invokeParsed('start_shell_session', NullSchema, { cwd: folder })
}

/** Starts an agent as the first command of a new terminal session; its flags are separate words. */
export async function startAgentSession(folder: string, agent: 'claude' | 'codex' | 'gemini', flags: readonly string[]): Promise<void> {
  await invokeParsed('start_agent_session', NullSchema, { cwd: folder, agent, flags })
}

/**
 * Starts an agent ahead of its first message, hidden. A session started next with the same folder, agent and flags is that
 * agent, already running. False when the daemon running is from before this and was not asked.
 */
export async function prewarmAgentSession(folder: string, agent: 'claude' | 'codex' | 'gemini', flags: readonly string[]): Promise<boolean> {
  return invokeParsed('prewarm_agent_session', z.boolean(), { cwd: folder, agent, flags })
}

/** Stops the agent `prewarmAgentSession` started, if no session took it. */
export async function dropPrewarmedAgent(): Promise<void> {
  await invokeParsed('drop_prewarmed_agent', NullSchema, {})
}

export async function startTerminalSession(folder: string, provider: 'Codex' | 'Claude'): Promise<void> {
  await invokeParsed('start_terminal_session', NullSchema, { cwd: folder, provider })
}

export async function resumeConversation(conversation: Conversation): Promise<{ id: number; generation: number }> {
  if (!isDesktop()) throw new Error('Conversation recovery is only available in the desktop app')
  if (conversation.generation === undefined) throw new Error('Conversation generation is unavailable')
  return invokeParsed('resume_codex_conversation', CreatedConversationSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

function toTerminalConversation(term: TerminalSession, workspaces: WorkspaceSummary[], machine?: SnapshotMachine, chat?: Conversation['chat'], limits: readonly z.infer<typeof PlanLimitSchema>[] = []): Conversation {
  const folder = term.root ?? term.cwd
  const repo = term.root ? (term.root.split(/[\\/]/).filter(Boolean).at(-1) ?? null) : null
  const workspace = workspaces.find((candidate) => candidate.cwd === folder || candidate.cwd === term.cwd)
  const provider: Conversation['provider'] = term.agent === 'claude' ? 'Claude' : 'Codex'
  const label = term.label.trim()
  const status: SessionStatus = term.status === 'Blocked'
    ? 'needs-you'
    : term.status === 'Working'
      ? 'working'
      : term.status === 'Done'
        ? 'done'
        : 'idle'
  return {
    id: terminalSessionId(term.id, machine?.name ?? null),
    generation: 0,
    execution: 'Native',
    kind: 'terminal',
    title: label || (term.agent === null ? `Shell ${String(term.id)}` : `${provider} terminal ${String(term.id)}`),
    project: folder.split(/[\\/]/).filter(Boolean).at(-1) ?? 'Project',
    folder,
    cwd: term.cwd,
    repo,
    branch: term.branch ?? workspace?.git?.branch ?? 'working tree',
    provider,
    status,
    updated: null,
    model: 'Provider default',
    permission: 'Terminal session',
    context: null,
    nativePane: term.id,
    ...(machine ? { machine: machine.id, machineName: machine.name } : {}),
    linked: term.linked,
    hasDiff: term.diff_added + term.diff_removed > 0,
    diffAdded: term.diff_added,
    diffRemoved: term.diff_removed,
    named: label !== '',
    pinned: term.pinned,
    archived: term.archived,
    agent: term.agent,
    process: term.process,
    summary: term.summary,
    details: {
      asking: term.asking,
      ask: term.ask,
      interrupted: term.interrupted,
      exit: term.exit,
      since: term.since,
      timeline: term.timeline,
      usage: term.usage,
      limits: limits.map((limit) => ({ name: limit.name, used: limit.used, resetsAt: limit.resets_at })),
      helperAsk: term.helper_ask,
      subagents: term.subagent_activity.map((child) => ({ id: child.id, name: child.name, state: child.state, task: child.task, startedAt: child.started_at, updatedAt: child.updated_at, latest: child.latest_activity, result: child.result })),
    },
    ...(chat ? { chat } : {}),
  }
}

function toConversation(summary: ConversationSummary, workspaces: WorkspaceSummary[]): Conversation {
  const cwd = summary.worktree ?? summary.cwd
  const workspace = workspaces.find((candidate) => candidate.cwd === cwd || candidate.cwd === summary.cwd)
  const status: SessionStatus = summary.state === 'RecoveryRequired'
    ? 'recovery'
    : summary.status === 'Blocked'
      ? 'needs-you'
      : summary.status === 'Working'
        ? 'working'
        : summary.status === 'Done'
          ? 'done'
          : 'idle'
  const project = cwd.split(/[\\/]/).filter(Boolean).at(-1) ?? 'Project'
  // A chat session is a conversation the daemon started itself: it has no terminal, and the window writes to it
  // (ADR 0003). A followed pane's conversation is read-only and belongs to that pane, so it is not one.
  const chatSession = summary.execution === 'Structured' && summary.native_term === null
  const root = summary.root

  return {
    id: String(summary.id),
    generation: summary.generation,
    execution: summary.execution,
    title: summary.title ?? `${summary.provider} conversation ${String(summary.id)}`,
    archived: summary.archived,
    named: summary.title !== null,
    project,
    folder: chatSession && root !== null ? root : cwd,
    branch: summary.branch ?? workspace?.git?.branch ?? 'working tree',
    provider: summary.provider,
    status,
    updated: null,
    model: 'Provider default',
    permission: 'Provider permissions',
    context: null,
    nativePane: summary.native_term,
    hasDiff: (workspace?.git?.dirty ?? 0) > 0,
    ...(workspace ? { isGitRepository: workspace.git !== null } : {}),
    ...(chatSession
      ? {
          cwd,
          repo: root === null ? null : (root.split(/[\\/]/).filter(Boolean).at(-1) ?? null),
          agent: summary.provider.toLowerCase(),
          process: summary.provider.toLowerCase(),
          chat: { conversation: summary.id, generation: summary.generation },
        }
      : {}),
  }
}
