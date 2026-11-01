import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'
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
})
const WorkspaceSummarySchema = z.object({
  cwd: z.string(),
  git: z.object({ branch: z.string(), dirty: z.number().int().nonnegative() }).nullable(),
})
const DaemonSnapshotSchema = z.object({
  conversations: z.array(ConversationSummarySchema),
  workspaces: z.array(WorkspaceSummarySchema),
})

const HistoryCursorSchema = z.object({
  generation: z.number().int().nonnegative(),
  conversation: z.number().int().nonnegative(),
  revision: z.number().int().nonnegative(),
  next_position: z.number().int().nonnegative(),
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
    z.object({ Tool: z.object({ kind: z.string(), output: z.string(), label: z.string().optional() }) }),
    z.object({ Unsupported: z.object({ kind: z.string() }) }),
  ]),
  state: z.enum(['Running', 'Completed', 'Failed', 'Interrupted']),
})
const HistoryPageSchema = z.object({
  entries: z.array(HistoryEntrySchema),
  next: HistoryCursorSchema.nullable(),
  completeness: HistoryCompletenessSchema,
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
const MaximumByteValue = 0xff
const ByteArraySchema = z.array(z.number().int().min(0).max(MaximumByteValue))

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

async function invokeParsed<T>(command: string, schema: z.ZodType<T>, args?: Record<string, unknown>): Promise<T> {
  const result = await invoke<unknown>(command, args)
  return schema.parse(result)
}

const NullSchema = z.null()
const ReadPageLimit = 128

export async function loadConversations(): Promise<Conversation[]> {
  if (!isDesktop()) return previewConversations

  const snapshot = await invokeParsed('list_sessions', DaemonSnapshotSchema)
  return snapshot.conversations.map((conversation) => toConversation(conversation, snapshot.workspaces))
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
  if (!isDesktop()) return { generation: 0, receipt: null, messages: [], nextOperation: 1, error: null, cancelling: false, completeness: 'Complete' }

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

export async function loadConversationDiff(conversation: Conversation): Promise<ConversationDiff> {
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
  if (!isDesktop() || conversation.generation === undefined) {
    throw new Error('Project files are available in the desktop app')
  }
  return invokeParsed('conversation_file', z.string(), {
    id: Number(conversation.id),
    generation: conversation.generation,
    path,
  })
}

export async function suggestProjectFolders(typed: string): Promise<string[]> {
  if (!isDesktop()) return []
  return invokeParsed('suggest_project_folders', z.array(z.string()), { typed })
}

export async function loadTerminalScreen(conversation: Pick<Conversation, 'id'> & { generation: number }): Promise<string> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  return invokeParsed('terminal_screen', z.string(), {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

export async function loadTerminalReplay(conversation: Pick<Conversation, 'id'> & { generation: number }): Promise<Uint8Array> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  const bytes = await invokeParsed('terminal_replay', ByteArraySchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
  return Uint8Array.from(bytes)
}

export async function sendTerminalInput(conversation: Pick<Conversation, 'id'> & { generation: number }, input: string | Uint8Array): Promise<string> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  return invokeParsed('send_terminal_input', z.string(), {
    id: Number(conversation.id),
    generation: conversation.generation,
    data: Array.from(typeof input === 'string' ? new TextEncoder().encode(input) : input),
  })
}

export async function resizeTerminal(conversation: Pick<Conversation, 'id'> & { generation: number }, cols: number, rows: number): Promise<void> {
  if (!isDesktop()) throw new Error('The native terminal is only available in the desktop app')
  await invokeParsed('resize_terminal', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    cols,
    rows,
  })
}

export async function submitMessage(conversation: Conversation, operation: number, text: string): Promise<ConversationReceipt> {
  return invokeParsed('send_conversation', ConversationReceiptSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    operation,
    text,
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

export async function approveRequest(conversation: Conversation, requestId: number | string, decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel'): Promise<void> {
  await invokeParsed('approve_conversation', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    requestId,
    decision,
  })
}

export async function answerUserInput(conversation: Conversation, requestId: number | string, answers: Record<string, string[]>): Promise<void> {
  await invokeParsed('answer_conversation_input', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
    requestId,
    answers,
  })
}

export async function cancelTurn(conversation: Conversation): Promise<void> {
  await invokeParsed('cancel_conversation', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

export async function handoffToNative(conversation: Conversation): Promise<void> {
  if (!isDesktop()) throw new Error('Native handoff is only available in the desktop app')
  if (conversation.generation === undefined) throw new Error('Conversation generation is unavailable')
  await invokeParsed('handoff_conversation_to_native', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

export async function handoffToChat(conversation: Conversation): Promise<void> {
  if (!isDesktop()) throw new Error('Chat handoff is only available in the desktop app')
  if (conversation.generation === undefined) throw new Error('Conversation generation is unavailable')
  await invokeParsed('handoff_conversation_to_chat', NullSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
}

export async function createConversation(
  folder: string,
  provider: 'Codex' | 'Claude',
): Promise<{ id: number; generation: number }> {
  return invokeParsed('create_conversation', CreatedConversationSchema, {
    cwd: folder,
    provider,
  })
}

export async function resumeConversation(conversation: Conversation): Promise<{ id: number; generation: number }> {
  if (!isDesktop()) throw new Error('Conversation recovery is only available in the desktop app')
  if (conversation.generation === undefined) throw new Error('Conversation generation is unavailable')
  return invokeParsed('resume_codex_conversation', CreatedConversationSchema, {
    id: Number(conversation.id),
    generation: conversation.generation,
  })
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

  return {
    id: String(summary.id),
    generation: summary.generation,
    execution: summary.execution,
    title: `${summary.provider} conversation ${String(summary.id)}`,
    project,
    folder: cwd,
    branch: workspace?.git?.branch ?? 'working tree',
    provider: summary.provider,
    status,
    updated: null,
    model: 'Provider default',
    permission: 'Provider permissions',
    context: null,
    nativePane: summary.native_term,
    hasDiff: (workspace?.git?.dirty ?? 0) > 0,
    ...(workspace ? { isGitRepository: workspace.git !== null } : {}),
  }
}
