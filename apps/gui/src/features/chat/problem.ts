import type { SessionUsage } from './usageStrip'

/** From here a chat's context is nearly full: the agent will soon forget its earliest messages. */
export const CONTEXT_FULL_FROM = 95
const PERCENT_MAX = 100

/** What is wrong with a chat, if anything, as the banner over its message box says it (layout 2g). */
export type ChatProblem =
  | { kind: 'crashed'; agent: string; code: number | null; onViewLog: () => void; onRestart: (() => void) | null }
  | { kind: 'offline'; machine: string; since: number | null; onRetry: () => void }
  | { kind: 'context'; agent: string; percent: number; onCompact: () => void }

/** The share of the context used when it is nearly full, else null. */
export function contextFull(usage: SessionUsage | undefined): number | null {
  const used = usage?.context
  return used === null || used === undefined || Math.round(used) < CONTEXT_FULL_FROM ? null : Math.min(PERCENT_MAX, Math.round(used))
}

const SECOND = 1000
const MINUTE = 60
const HOUR = 3600

/** "40s", "3m", "2h": how long ago a time in ms was. */
export function agoWords(since: number | null, now: number): string | null {
  if (since === null) return null
  const seconds = Math.max(0, Math.round((now - since) / SECOND))
  if (seconds < MINUTE) return `${String(seconds)}s`
  if (seconds < HOUR) return `${String(Math.floor(seconds / MINUTE))}m`
  return `${String(Math.floor(seconds / HOUR))}h`
}

/** The signal-killed codes a process exits with, in words. */
const KILLED: Readonly<Record<number, string>> = { 130: 'interrupted', 137: 'out of memory or killed', 139: 'crashed', 143: 'terminated' }

export function exitWords(code: number | null): string {
  if (code === null) return 'The process exited.'
  const why = KILLED[code]
  return `The process exited (code ${String(code)}${why === undefined ? '' : `, ${why}`}).`
}
