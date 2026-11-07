import type { ChatEvent, ItemFields, SubagentEvent, TodoItem } from './chatEvents'

/**
 * Facts the step cards show, computed from the projection's events. Pure: the clock comes in as an
 * argument. Nothing here estimates; a fact the history lacks comes back as `null`.
 */

/**
 * How long an item took, or has been running. A running item counts from the daemon's start time,
 * else from when this app first saw it (the best a live item has). A finished item needs its own
 * measured duration or both of the daemon's times; one read back from saved history has neither, so
 * its time is `null` and the card says it is unavailable.
 */
export function elapsedMs(item: ItemFields, durationMs: number | null, nowMs: number): number | null {
  if (item.status === 'running') return Math.max(0, nowMs - (item.timing.startedAtMs ?? item.observed.firstSeenAtMs))
  if (durationMs !== null) return durationMs
  const { startedAtMs, completedAtMs } = item.timing
  return startedAtMs !== null && completedAtMs !== null ? Math.max(0, completedAtMs - startedAtMs) : null
}

const MS_PER_SECOND = 1000
const SECONDS_PER_MINUTE = 60
const CLOCK_SECONDS_WIDTH = 2

/** "m:ss", the form the running pill and the working line use. */
export function clockText(ms: number): string {
  const seconds = Math.floor(Math.max(0, ms) / MS_PER_SECOND)
  return `${String(Math.floor(seconds / SECONDS_PER_MINUTE))}:${String(seconds % SECONDS_PER_MINUTE).padStart(CLOCK_SECONDS_WIDTH, '0')}`
}

/** "0.4s", "5m 30s": how long a finished step took. Sub-second times keep a decimal. */
export function tookText(ms: number): string {
  if (ms < MS_PER_SECOND) return `${(ms / MS_PER_SECOND).toFixed(1)}s`
  const seconds = Math.round(ms / MS_PER_SECOND)
  if (seconds < SECONDS_PER_MINUTE) return `${String(seconds)}s`
  return `${String(Math.floor(seconds / SECONDS_PER_MINUTE))}m ${String(seconds % SECONDS_PER_MINUTE)}s`
}

/** A file path relative to the session's folder; a path outside it, or with no folder known, is unchanged. */
export function relativePath(file: string, root: string | null): string {
  if (root === null || root === '') return file
  const slash = (value: string) => value.replace(/\\/g, '/')
  const base = slash(root).replace(/\/+$/, '')
  const path = slash(file)
  // Windows paths compare without case; a POSIX path that differs only by case is a different file.
  const windows = /^[A-Za-z]:\//.test(base)
  const head = path.slice(0, base.length)
  const same = windows ? head.toLowerCase() === base.toLowerCase() : head === base
  return same && path.charAt(base.length) === '/' ? path.slice(base.length + 1) : file
}

/** The words that name a step in a link to it ("Edited src/a.ts", "pnpm test"). */
export function describeStep(event: ChatEvent, root: string | null): string | null {
  switch (event.kind) {
    case 'edit':
      return `Edit ${event.file === null ? 'a file' : relativePath(event.file, root)}`
    case 'command':
      return event.command === '' ? 'A command' : event.command
    case 'tool':
      return event.target === null ? event.name : `${event.name} ${event.target}`
    case 'todo':
      return 'Checklist'
    case 'subagent':
      return event.title ?? 'A helper agent'
    case 'message':
    case 'approval':
    case 'turn':
      return null
  }
}

export type TodoProgress = { readonly done: number; readonly total: number }

export function todoProgress(items: readonly TodoItem[]): TodoProgress {
  return { done: items.filter((item) => item.state === 'done').length, total: items.length }
}

/** "3 agents, 2 working" or "2 agents, all done". */
export function agentsSummary(agents: readonly SubagentEvent[]): string {
  const working = agents.filter((agent) => agent.status === 'running').length
  const noun = agents.length === 1 ? 'agent' : 'agents'
  return `${String(agents.length)} ${noun}, ${working === 0 ? 'all done' : `${String(working)} working`}`
}
