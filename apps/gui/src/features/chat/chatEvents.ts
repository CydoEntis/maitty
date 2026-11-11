/**
 * The chat events the view renders (design handoff v18 §C.1). They are a projection over the
 * daemon's conversation history; nothing here comes from terminal output.
 *
 * Where the daemon's history does not carry a fact the design wants, the field says so
 * (`status: 'unavailable'` or `null`) instead of holding a guess.
 */

export type ChatProvider = 'claude' | 'codex' | 'gemini'

/** The daemon's item state, lower-cased. */
export type ItemStatus = 'running' | 'completed' | 'failed' | 'interrupted'

/**
 * When this app first saw an item and when it first saw it finished. These are local observation
 * times, the fallback for a running item the daemon gave no start time: an item loaded already
 * finished has `firstSeenAtMs === settledAtMs`, and its elapsed time is not known from these.
 */
export type Observed = {
  readonly firstSeenAtMs: number
  readonly settledAtMs: number | null
}

export type DiffLine = { readonly kind: 'context' | 'add' | 'remove'; readonly text: string }
export type DiffHunk = { readonly header: string; readonly lines: readonly DiffLine[] }

export type FileDiff =
  | {
      readonly status: 'available'
      readonly hunks: readonly DiffHunk[]
      readonly added: number
      readonly removed: number
      /** The daemon cut the diff at its size limit, so the counts cover only what was kept. */
      readonly truncated: boolean
    }
  | { readonly status: 'unavailable' }
  /** The file is a secret file: the daemon kept its path and kind but not its text. */
  | { readonly status: 'omitted' }

export type FileChangeKind = 'add' | 'update' | 'delete' | 'unknown'

/** The daemon's own times for an item (Unix ms). `null` for an item read back from a provider's saved history. */
export type ItemTiming = {
  readonly startedAtMs: number | null
  readonly completedAtMs: number | null
}

/** What every event built from one history entry shares. */
export type ItemFields = {
  readonly id: string
  readonly turnId: string
  readonly status: ItemStatus
  readonly observed: Observed
  readonly timing: ItemTiming
}

export type MessageEvent = ItemFields & {
  readonly kind: 'message'
  readonly role: 'user' | 'agent'
  readonly text: string
}

/** One card per file; a Codex patch that touches three files is three events. */
export type EditEvent = ItemFields & {
  readonly kind: 'edit'
  /** The provider's tool: `Edit`, `MultiEdit`, `Write` or `fileChange`. */
  readonly tool: string
  readonly file: string | null
  readonly change: FileChangeKind
  readonly diff: FileDiff
}

export type CommandEvent = ItemFields & {
  readonly kind: 'command'
  readonly command: string
  readonly output: string
  /** The process exit code; `null` when the provider does not report one (Claude never does). */
  readonly exitCode: number | null
  /** The provider's own measured duration; `null` when it reports none. */
  readonly durationMs: number | null
}

export type TodoItemState = 'pending' | 'current' | 'done'
export type TodoItem = { readonly text: string; readonly state: TodoItemState }

export type TodoEvent = ItemFields & {
  readonly kind: 'todo'
  /** `null` when the history did not carry the list. */
  readonly items: readonly TodoItem[] | null
}

export type SubagentEvent = ItemFields & {
  readonly kind: 'subagent'
  readonly title: string | null
  readonly agentType: string | null
  /** What the helper was asked to do, when the history kept the call's input. */
  readonly prompt: string | null
  /** The helper's final report (the tool result); empty while it runs. */
  readonly report: string
}

/** Any tool the design has no card for, and anything unrecognised. Never an error. */
export type ToolEvent = ItemFields & {
  readonly kind: 'tool'
  readonly name: string
  /** What the tool acted on (a path, a pattern, a URL), when its input or label names one. */
  readonly target: string | null
  /** The tool's input as indented JSON, bounded; `null` when the history kept none. */
  readonly input: string | null
  readonly detail: string
}

export type UserInputQuestion = {
  readonly id: string
  readonly header: string
  readonly question: string
  /** The person may write an answer of their own beside the options. */
  readonly isOther: boolean
  /** The answer is a secret: it is typed hidden and never shown again. */
  readonly isSecret: boolean
  readonly options: readonly { readonly label: string; readonly description: string }[]
}

export type ApprovalDecision = 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel'
export type ApprovalKind = 'Command' | 'FileChange' | 'Permission' | 'UserInput'

export type ApprovalChange = {
  readonly path: string
  readonly kind: string
  readonly diff: FileDiff
}

export type ApprovalResolution =
  | { readonly status: 'pending' }
  /** `answer` is `unknown` when the daemon dropped the request before this app recorded a choice. */
  | { readonly status: 'answered'; readonly answer: ApprovalDecision | 'unknown'; readonly answeredAtMs?: number | null }

export type ApprovalEvent = {
  readonly kind: 'approval'
  readonly id: string
  /** The provider's request id, exactly as the daemon sent it (a number for Codex, text for Claude). */
  readonly requestId: number | string
  readonly turnId: string | null
  /** The history item the request is about, when the provider names one (Codex does). */
  readonly itemId: string | null
  /** The id of the tool step this request is about (`turn:item`), when the provider names both. */
  readonly stepId: string | null
  readonly approvalKind: ApprovalKind
  readonly command: string | null
  readonly reason: string | null
  readonly changes: readonly ApprovalChange[]
  /** The questions of a user-input request; empty for every other kind. */
  readonly questions: readonly UserInputQuestion[]
  readonly choices: readonly ApprovalDecision[]
  readonly resolution: ApprovalResolution
}

/** Closes a turn: the "Finished in 7s" divider. Only emitted once the turn has ended. */
export type TurnEvent = {
  readonly kind: 'turn'
  readonly id: string
  readonly turnId: string
  readonly outcome: Exclude<ItemStatus, 'running'>
  readonly startedAtMs: number
  readonly endedAtMs: number
  /**
   * The times are the daemon's. When false they are when this app saw the turn start and end, so
   * the duration and the clock time are not shown as facts.
   */
  readonly timed: boolean
}

export type ChatEvent =
  | MessageEvent
  | EditEvent
  | CommandEvent
  | TodoEvent
  | SubagentEvent
  | ToolEvent
  | ApprovalEvent
  | TurnEvent
