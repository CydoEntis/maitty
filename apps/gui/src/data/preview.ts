export type SessionStatus = 'working' | 'needs-you' | 'done' | 'idle' | 'recovery' | 'unknown'

export type TurnKind = 'Working' | 'NeedsYou' | 'Done' | 'Idle' | 'Prompt' | 'CommandStarted' | 'CommandFinished'

/** The question a permission prompt asks, from the agent's own hook. */
export type PermissionAsk = { tool: string; detail: string; answerable: boolean; choices?: readonly string[]; /** A prompt that asks several questions at once, each with one answer, in the order the agent shows them. */ questions?: readonly { question: string; choices: readonly string[] }[] }

/** What the daemon reports about a terminal session beyond its status. */
export type TerminalDetails = {
  /** What a stopped agent is asking, off its screen: the exact command and its choices. */
  asking: string
  /** The permission prompt the agent's hook says it is stopped on; null when it said none. */
  ask?: PermissionAsk | null
  /** An agent that was mid-turn when the daemon restarted; offered a resume, never resumed on its own. */
  interrupted: { agent: string; resume: string } | null
  /** When the status last changed, in unix seconds (the daemon's `TermInfo.since`); absent or 0 when it did not say. */
  since?: number
  /** The latest turn events, oldest first; `at` is unix seconds. */
  timeline: Array<{ at: number; kind: TurnKind; detail: string }>
  /** The last command ended in failure, as the shell reported it (a non-zero exit code); null when nothing failed. Only the daemon sets it. */
  exit?: { code: number | null; at: number } | null
  /** What the provider reports the session has used; each is null when it did not say. */
  usage: { context: number | null; cost: number | null; tokens: number | null }
  /** The plan limits of the session's agent, as it last reported them (`5h`, `week`); absent from an older daemon. */
  limits?: Array<{ name: string; used: number; resetsAt: number }>
  /** The helpers the session asked to start and waits on the person for, one task each; absent from an older daemon. */
  helperAsk?: string[]
  /** The agent's subagents as its runtime reports them; absent from an older daemon. */
  subagents?: SubagentActivity[]
}

/** The runtime's states for a subagent (`SubagentState` in the protocol). */
export type SubagentState = 'Starting' | 'Working' | 'WaitingForInput' | 'AwaitingApproval' | 'Completed' | 'Failed' | 'Cancelled' | 'ConnectionLost'

/** One subagent of a session, as the daemon reports it. Times are unix seconds. */
export type SubagentActivity = {
  id: string
  name: string
  state: SubagentState
  /** What it was asked to do. */
  task: string | null
  startedAt: number | null
  updatedAt: number
  /** The runtime's last word on what it is doing. */
  latest: string | null
  /** Its report, once it finished. */
  result: string | null
}

export type Conversation = {
  id: string
  /** A terminal session's current folder, as its shell reports it. */
  cwd?: string
  /** The git repository's name when the folder is inside one. */
  repo?: string | null
  generation?: number
  execution?: 'Native' | 'Structured'
  title: string
  project: string
  folder: string
  branch: string
  provider: 'Codex' | 'Claude'
  status: SessionStatus
  lastKnownStatus?: Exclude<SessionStatus, 'unknown'>
  updated: string | null
  model: string
  permission: string
  context: number | null
  nativePane: number | null
  /** The number the host gave the machine this session lives on; absent for this computer. */
  machine?: number
  /** The machine's name; absent for this computer. */
  machineName?: string
  hasDiff: boolean
  /** Lines added and removed in the session's checkout against HEAD, from the daemon; absent when unknown. */
  /** The terminal runs in a linked worktree. */
  linked?: boolean
  diffAdded?: number
  diffRemoved?: number
  isGitRepository?: boolean
  // Put away in any client; the daemon keeps it running.
  archived?: boolean
  // Pinned to the top of its project; the daemon keeps it for every app.
  pinned?: boolean
  // Someone gave it a name (the daemon has one), as opposed to the automatic wording.
  named?: boolean
  // A terminal session: an agent running in a terminal the daemon owns, shared with the terminal
  // app. Anything else is a chat. `nativePane` is its terminal's number.
  kind?: 'terminal'
  /** The agent the daemon reports running in a terminal session; null for a plain shell. */
  agent?: string | null
  /** The program the daemon sees running in a terminal session (a shell's own name when idle). */
  process?: string
  /** The last prompt the agent in a terminal session was given, as one line; empty when unknown. */
  summary?: string
  details?: TerminalDetails
  /**
   * The daemon conversation that carries this terminal session's structured events, when it holds
   * one (a conversation whose native terminal is this pane). The chat view reads and writes
   * through it; without it the session has no chat.
   */
  chat?: { conversation: number; generation: number; /** Follows an agent the user started in the terminal (ADR 0003): the chat reads, it does not write. */ readOnly?: boolean }
}

export type Message = {
  id: string
  role: 'user' | 'assistant' | 'activity' | 'approval'
  content: string
  time: string
  detail?: string
  activityState?: 'Running' | 'Completed' | 'Failed' | 'Interrupted'
  resolved?: boolean
  providerRequestId?: number | string
  approvalReason?: string
  approvalKind?: 'Command' | 'FileChange' | 'Permission' | 'UserInput'
  approvalScope?: string
  approvalChanges?: Array<{ path: string; kind: string; diff: string }>
  approvalPermissions?: Record<string, unknown>
  userQuestions?: UserInputQuestion[]
  approvalChoices?: Array<'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel'>
}

export type UserInputQuestion = {
  id: string
  header: string
  question: string
  is_other: boolean
  is_secret: boolean
  options: Array<{ label: string; description: string }>
}

export const previewConversations: Conversation[] = [
  {
    id: 'codex-43',
    title: 'Tighten the branch view navigation',
    project: 'maitty',
    folder: 'C:/dev/maitty',
    branch: 'gui/branch-view-navigation',
    provider: 'Codex',
    status: 'needs-you',
    updated: 'now',
    model: 'GPT-5 Codex',
    permission: 'Workspace write · approvals on request',
    context: 38,
    nativePane: 43,
    hasDiff: true,
  },
  {
    id: 'codex-41',
    title: 'Add a conversation reconnect test',
    project: 'maitty',
    folder: 'C:/dev/maitty',
    branch: 'feat/reconnect-proof',
    provider: 'Codex',
    status: 'working',
    updated: '2m',
    model: 'GPT-5 Codex',
    permission: 'Read only · approvals on request',
    context: 61,
    nativePane: 41,
    hasDiff: true,
  },
  {
    id: 'claude-18',
    title: 'Review the daemon restore flow',
    project: 'maitty',
    folder: 'C:/dev/maitty',
    branch: 'main',
    provider: 'Claude',
    status: 'done',
    updated: '14m',
    model: 'Claude Sonnet',
    permission: 'Workspace write · approvals on request',
    context: 47,
    nativePane: 18,
    hasDiff: false,
  },
  {
    id: 'codex-12',
    title: 'Check Windows named pipe errors',
    project: 'maitty',
    folder: 'C:/dev/maitty',
    branch: 'fix/named-pipe-errors',
    provider: 'Codex',
    status: 'idle',
    updated: '1h',
    model: 'GPT-5 Codex',
    permission: 'Read only · approvals off',
    context: 23,
    nativePane: 12,
    hasDiff: true,
  },
  {
    id: 'claude-07',
    title: 'Polish empty state wording',
    project: 'website',
    folder: 'C:/dev/website',
    branch: 'copy/empty-state',
    provider: 'Claude',
    status: 'idle',
    updated: 'yesterday',
    model: 'Claude Sonnet',
    permission: 'Read only · approvals off',
    context: 12,
    nativePane: 7,
    hasDiff: false,
  },
]

export const previewMessages: Record<string, Message[]> = {
  'codex-43': [
    {
      id: 'm1',
      role: 'user',
      content: 'Can you make the branch view easier to scan and keep the session status visible while it is open?',
      time: '10:42 AM',
    },
    {
      id: 'm2',
      role: 'assistant',
      content: 'I’ll look at the current branch sheet and its tests, then tighten the hierarchy without changing what the controls do.',
      time: '10:42 AM',
    },
    {
      id: 'm3',
      role: 'activity',
      content: 'Read branch_sheet.rs and frame_tests.rs',
      detail: '2 files · 4.2 seconds',
      activityState: 'Completed',
      time: '10:43 AM',
    },
    {
      id: 'm4',
      role: 'assistant',
      content: 'The sheet already keeps its selection across refreshes. I found one command that changes the checkout, so I’m asking before I run it.',
      time: '10:43 AM',
    },
    {
      id: 'm5',
      role: 'approval',
      content: 'Run the branch view tests',
      detail: 'cargo test branch_view_tests',
      time: '10:43 AM',
    },
  ],
  'codex-41': [
    { id: 'c1', role: 'user', content: 'Add a reconnect test that proves an in-flight conversation stays attached after the GUI reconnects.', time: '10:41 AM' },
    { id: 'c2', role: 'assistant', content: 'I’m tracing the attach handshake and the conversation generation checks first.', time: '10:42 AM' },
    { id: 'c3', role: 'activity', content: 'Inspecting daemon conversation restore', detail: '2 files · running', activityState: 'Running', time: '10:44 AM' },
  ],
  'claude-18': [
    { id: 'a1', role: 'user', content: 'Review the restore flow for sessions that were active when Maitty closed.', time: '9:18 AM' },
    { id: 'a2', role: 'assistant', content: 'The restore path keeps active sessions marked as recovery-required and avoids launching a duplicate provider process.', time: '9:21 AM' },
    { id: 'a3', role: 'activity', content: 'Review complete', detail: 'No findings · 3 files checked', activityState: 'Completed', time: '9:22 AM' },
  ],
}

export const previewDiff = [
  { file: 'src/client/screen/branch_sheet.rs', added: 18, removed: 7, lines: ['+ let status = view.status_label();', '+ render_status(buf, status, theme);', '  render_branch_name(buf, view.branch);'] },
  { file: 'src/client/screen/frame_tests.rs', added: 12, removed: 0, lines: ['+ assert!(frame.contains("working"));', '+ assert!(frame.contains("fix/gui"));'] },
]
