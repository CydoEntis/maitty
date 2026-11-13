import type { CommandEvent, EditEvent, ItemStatus, ToolEvent } from './chatEvents'

/** A step that is folded into a run of work: anything but the to-do list, which stays in view. */
export type WorkStep = EditEvent | CommandEvent | ToolEvent

const READ_TOOL = /^(read|view|cat|open)/i
const SEARCH_TOOL = /(grep|glob|search|find|^ls$|list)/i

type Count = { readonly n: number; readonly one: string; readonly many: string }

function said({ n, one, many }: Count): string {
  return `${String(n)} ${n === 1 ? one : many}`
}

/**
 * What a run of steps did, in a line: "Read 2 files, edited 3, ran 1 command". Files are counted once however
 * often they were touched. Only what the steps say is counted; a tool of no known kind is "used N tools".
 */
export function workSummary(steps: readonly WorkStep[]): string {
  const read = new Set<string>()
  const edited = new Set<string>()
  let searches = 0
  let commands = 0
  let others = 0
  for (const step of steps) {
    if (step.kind === 'edit') edited.add(step.file ?? step.id)
    else if (step.kind === 'command') commands += 1
    else if (READ_TOOL.test(step.name)) read.add(step.target ?? step.id)
    else if (SEARCH_TOOL.test(step.name)) searches += 1
    else others += 1
  }
  const parts: string[] = []
  if (read.size > 0) parts.push(`read ${said({ n: read.size, one: 'file', many: 'files' })}`)
  if (searches > 0) parts.push(`searched ${said({ n: searches, one: 'time', many: 'times' })}`)
  // After "read 2 files" the noun is understood: "edited 3".
  if (edited.size > 0) parts.push(read.size > 0 ? `edited ${String(edited.size)}` : `edited ${said({ n: edited.size, one: 'file', many: 'files' })}`)
  if (commands > 0) parts.push(`ran ${said({ n: commands, one: 'command', many: 'commands' })}`)
  if (others > 0) parts.push(`used ${said({ n: others, one: 'tool', many: 'tools' })}`)
  const line = parts.join(', ')
  return line.charAt(0).toUpperCase() + line.slice(1)
}

/** The run as one status: running while any step is, failed when one failed, else how the last one ended. */
export function workStatus(steps: readonly WorkStep[]): ItemStatus {
  if (steps.some((step) => step.status === 'running')) return 'running'
  if (steps.some((step) => step.status === 'failed')) return 'failed'
  return steps.at(-1)?.status ?? 'completed'
}

/**
 * A run that can be hidden by Settings › Chat › Hide successful tool calls: every step a command or tool that finished
 * well. An edit always shows, and a step still running, failed, stopped, or a command with a nonzero exit code is
 * not counted a success.
 */
export function isQuietRun(steps: readonly WorkStep[]): boolean {
  return steps.every((step) => step.kind !== 'edit' && step.status === 'completed' && (step.kind !== 'command' || step.exitCode === null || step.exitCode === 0))
}

/** How long the run took by the daemon's own times, first start to last finish; null when it gave none. */
export function workDurationMs(steps: readonly WorkStep[]): number | null {
  const started = steps[0]?.timing.startedAtMs ?? null
  const ended = steps.at(-1)?.timing.completedAtMs ?? null
  return started === null || ended === null || ended < started ? null : ended - started
}
