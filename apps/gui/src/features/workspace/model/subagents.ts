import type { SubagentActivity, SubagentState } from '@/data/preview'

/** A subagent whose own chat a pane shows. */
export type OpenSubagent = { sessionId: string; subagentId: string }

/** A subagent's state as the sidebar marks it: a corner shape for each, never colour alone. */
export type SubagentStatus = 'working' | 'needs' | 'done' | 'stopped'

/** One subagent of a session as the window shows it. Times are unix seconds, as the daemon reports them. */
export type SubagentView = {
  id: string
  /** What the sidebar calls it: the first line of its task, else its kind. */
  name: string
  /** The runtime's kind for it (`Explore`, `code-reviewer`). */
  kind: string
  status: SubagentStatus
  task: string | null
  latest: string | null
  result: string | null
  startedAt: number | null
  updatedAt: number
}

const STATUS_OF: Record<SubagentState, SubagentStatus> = {
  Starting: 'working',
  Working: 'working',
  WaitingForInput: 'needs',
  AwaitingApproval: 'needs',
  Completed: 'done',
  Failed: 'stopped',
  Cancelled: 'stopped',
  ConnectionLost: 'stopped',
}

/** A task longer than this is cut where it is shown as a name. */
const NAME_CHARS = 48

function nameOf(activity: SubagentActivity): string {
  const line = activity.task?.split(/\r?\n/).find((part) => part.trim() !== '')?.trim() ?? ''
  if (line === '') return activity.name
  return line.length <= NAME_CHARS ? line : `${line.slice(0, NAME_CHARS - 1).trimEnd()}…`
}

export function toSubagentView(activity: SubagentActivity): SubagentView {
  return {
    id: activity.id,
    name: nameOf(activity),
    kind: activity.name,
    status: STATUS_OF[activity.state],
    task: activity.task,
    latest: activity.latest,
    result: activity.result,
    startedAt: activity.startedAt,
    updatedAt: activity.updatedAt,
  }
}

/** How long a helper that stopped without finishing stays listed beside a working agent, after its last report. */
export const STOPPED_LISTED_SECONDS = 120

/**
 * The subagents a session's row lists. One that finished has reported back to the agent, so it goes. One that
 * stopped without finishing stays while the agent still works, since it may be why the turn goes wrong, but only
 * for `STOPPED_LISTED_SECONDS` after it last reported: a long turn would otherwise keep a dead helper in view for
 * as long as it runs. It goes when the turn is over too: nothing is left to do about it, and the daemon marks every
 * helper still running at a turn's end as stopped, so they piled up under the session. One that works or waits
 * for you always stays.
 */
export function listedSubagents(subagents: readonly SubagentView[], sessionWorking: boolean, nowSeconds: number = Date.now() / MS_PER_SECOND): SubagentView[] {
  return subagents.filter((subagent) => {
    if (subagent.status === 'done') return false
    if (subagent.status !== 'stopped') return true
    return sessionWorking && nowSeconds - subagent.updatedAt <= STOPPED_LISTED_SECONDS
  })
}

/** The subagents still running, in the order the runtime reported them. One that finished has cleared itself. */
export function runningSubagents(subagents: readonly SubagentView[]): SubagentView[] {
  return subagents.filter((subagent) => subagent.status === 'working' || subagent.status === 'needs')
}

const MS_PER_SECOND = 1000
const SECONDS_PER_MINUTE = 60
const SECONDS_PER_HOUR = 3600
/** Minutes after the hours are always two digits: `1h 05m`. */
const MINUTE_DIGITS = 2

/** `4m 23s`, `48s`, `1h 05m`: how a running time reads beside a name. */
export function durationWords(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds))
  if (whole < SECONDS_PER_MINUTE) return `${String(whole)}s`
  if (whole < SECONDS_PER_HOUR) return `${String(Math.floor(whole / SECONDS_PER_MINUTE))}m ${String(whole % SECONDS_PER_MINUTE)}s`
  const minutes = Math.floor((whole % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE)
  return `${String(Math.floor(whole / SECONDS_PER_HOUR))}h ${String(minutes).padStart(MINUTE_DIGITS, '0')}m`
}

/**
 * What the subagent bar says after its name: how long it has run while it works, how long it took
 * once it is done, or its state in words. A time the runtime never reported is left out, not guessed.
 */
export function subagentWord(subagent: SubagentView, nowSeconds: number): string {
  if (subagent.status === 'needs') return 'needs you'
  if (subagent.status === 'stopped') return 'stopped'
  if (subagent.startedAt === null) return subagent.status === 'done' ? 'done' : 'working'
  const end = subagent.status === 'done' ? subagent.updatedAt : nowSeconds
  const took = durationWords(end - subagent.startedAt)
  return subagent.status === 'done' ? `done in ${took}` : took
}
