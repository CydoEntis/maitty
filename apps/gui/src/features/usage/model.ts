import type { DayUsage, SessionUsage, UsageFigures, UsagePeriod } from '@/api/usage'
import { validBudget } from '@/features/settings/preferences'
import { formatTokens } from '@/features/workspace/model/usage'

/** What a figure nobody reported looks like. A made-up zero would read as a fact (ADR 0009). */
export const DASH = '—'

const SECS_PER_MINUTE = 60
const SECS_PER_HOUR = 3600
const MS_PER_DAY = 86_400_000
const MINUTES_PER_HOUR = 60
/** Costs under half a cent would round to $0.00, which reads as free. */
const HALF_CENT = 0.005
const COST_DECIMALS = 2
export const PERCENT = 100
const MINUTE_DIGITS = 2
/** Enough of a session id to tell rows apart. */
const SESSION_ID_CHARS = 8
const ISO_DAY_LENGTH = 10
/** The ring turns warn from here and danger from the next (v18 C.7). */
const RING_WARN_AT = 0.7
const RING_DANGER_AT = 0.9

export const PERIOD_CHOICES: readonly { period: UsagePeriod; label: string; window: string }[] = [
  { period: 'day', label: 'Day', window: 'Today (UTC)' },
  { period: 'week', label: 'Week', window: 'Last 7 days' },
  { period: 'month', label: 'Month', window: 'Last 30 days' },
]

/** The honest name of a period: the daemon counts rolling UTC days that end today, not calendar weeks or months. */
export function windowLabel(period: UsagePeriod): string {
  return PERIOD_CHOICES.find((choice) => choice.period === period)?.window ?? ''
}

export const formatTokenCount = (tokens: number | null): string => (tokens === null ? DASH : formatTokens(tokens))

export function formatCost(cost: number | null): string {
  if (cost === null) return DASH
  if (cost > 0 && cost < HALF_CENT) return '<$0.01'
  return `$${cost.toFixed(COST_DECIMALS)}`
}

/** `42s`, `12m`, `1h 05m`: working time is observed by the daemon, so zero is a real figure. */
export function formatWorking(seconds: number): string {
  if (seconds < SECS_PER_MINUTE) return `${String(seconds)}s`
  const minutes = Math.floor(seconds / SECS_PER_MINUTE)
  if (seconds < SECS_PER_HOUR) return `${String(minutes)}m`
  const hours = Math.floor(minutes / MINUTES_PER_HOUR)
  return `${String(hours)}h ${String(minutes % MINUTES_PER_HOUR).padStart(MINUTE_DIGITS, '0')}m`
}

/** Input plus output, or null when the provider reported neither. */
export function totalTokens(figures: Pick<UsageFigures, 'tokensIn' | 'tokensOut'>): number | null {
  if (figures.tokensIn === null && figures.tokensOut === null) return null
  return (figures.tokensIn ?? 0) + (figures.tokensOut ?? 0)
}

const dayMs = (day: string) => Date.parse(`${day}T00:00:00Z`)

/** `Today`, `Yesterday` or `Nov 3`, counted from the report's own last day (UTC). */
export function formatLastActive(lastDay: string, today: string): string {
  const gap = Math.round((dayMs(today) - dayMs(lastDay)) / MS_PER_DAY)
  if (gap === 0) return 'Today'
  if (gap === 1) return 'Yesterday'
  return shortDay(lastDay)
}

export function shortDay(day: string): string {
  return new Date(dayMs(day)).toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' })
}

export type SortKey = 'session' | 'project' | 'machine' | 'agent' | 'tokensIn' | 'tokensOut' | 'cost' | 'turns' | 'workingSecs' | 'lastDay'
export type SortState = { key: SortKey; direction: 'ascending' | 'descending' }

export const DEFAULT_SORT: SortState = { key: 'cost', direction: 'descending' }

const TEXT_KEYS: readonly SortKey[] = ['session', 'project', 'machine', 'agent']

/** Clicking a header sorts by it (text A to Z, numbers biggest first); clicking it again flips the direction. */
export function nextSort(current: SortState, key: SortKey): SortState {
  if (current.key === key) return { key, direction: current.direction === 'ascending' ? 'descending' : 'ascending' }
  return { key, direction: TEXT_KEYS.includes(key) ? 'ascending' : 'descending' }
}

/** The `aria-sort` of a header. */
export function ariaSortOf(sort: SortState, key: SortKey): 'ascending' | 'descending' | 'none' {
  return sort.key === key ? sort.direction : 'none'
}

/** This computer is the only machine the report covers for now. */
export const THIS_COMPUTER = 'This computer'

/** What the row's Session cell says: the open pane's name, else the agent and the start of its id. */
export function sessionLabel(row: SessionUsage, openName: string | null): string {
  return openName ?? `${row.agent} ${row.session.slice(0, SESSION_ID_CHARS)}`
}

function valueOf(row: SessionUsage, key: SortKey, nameOf: (row: SessionUsage) => string): string | number | null {
  switch (key) {
    case 'session': return nameOf(row).toLowerCase()
    case 'project': return row.project.toLowerCase()
    case 'machine': return THIS_COMPUTER.toLowerCase()
    case 'agent': return row.agent.toLowerCase()
    case 'lastDay': return dayMs(row.lastDay)
    case 'tokensIn':
    case 'tokensOut':
    case 'cost':
    case 'turns':
    case 'workingSecs':
      return row[key]
  }
}

/** Sorted copy of the rows. A figure nobody reported sorts last in either direction, so dashes never lead. */
export function sortSessions(rows: readonly SessionUsage[], sort: SortState, nameOf: (row: SessionUsage) => string): SessionUsage[] {
  const sign = sort.direction === 'ascending' ? 1 : -1
  return [...rows].sort((a, b) => {
    const left = valueOf(a, sort.key, nameOf)
    const right = valueOf(b, sort.key, nameOf)
    if (left === null || right === null) {
      if (left === right) return a.session.localeCompare(b.session)
      return left === null ? 1 : -1
    }
    if (left === right) return a.session.localeCompare(b.session)
    return (left < right ? -1 : 1) * sign
  })
}

export type RingTone = 'accent' | 'warn' | 'danger'
export type Ring = { fraction: number; percent: number; tone: RingTone }

/**
 * The budget ring, only when a monthly budget is set, the cost is known and the window is the month.
 * A week of spend against a month's budget would mislead, and no cost reported means no figure to compare.
 */
export function budgetRing(period: UsagePeriod, cost: number | null, budget: number | null): Ring | null {
  if (period !== 'month' || cost === null || budget === null || budget <= 0) return null
  const used = cost / budget
  const tone: RingTone = used >= RING_DANGER_AT ? 'danger' : used >= RING_WARN_AT ? 'warn' : 'accent'
  return { fraction: Math.min(1, used), percent: Math.round(used * PERCENT), tone }
}

export type BudgetInput = { ok: true; value: number | null } | { ok: false }

/** What was typed in the budget box: empty clears the budget; anything but a positive amount is refused. */
export function parseBudget(text: string): BudgetInput {
  const cleaned = text.trim().replace(/^\$/, '').trim()
  if (cleaned === '') return { ok: true, value: null }
  if (!/^\d+(\.\d{1,2})?$/.test(cleaned)) return { ok: false }
  const value = validBudget(Number(cleaned))
  return value === null ? { ok: false } : { ok: true, value }
}

export type DayBar = { day: string; tokens: number | null; cost: number | null; fraction: number }

/** One slot per day of the window, so bars sit where their day is. A day nobody reported on has no tokens and no bar. */
export function dayBars(from: string, to: string, days: readonly DayUsage[]): DayBar[] {
  const byDay = new Map(days.map((entry) => [entry.day, entry]))
  const slots: { day: string; tokens: number | null; cost: number | null }[] = []
  for (let at = dayMs(from); at <= dayMs(to); at += MS_PER_DAY) {
    const day = new Date(at).toISOString().slice(0, ISO_DAY_LENGTH)
    const entry = byDay.get(day)
    slots.push({ day, tokens: entry ? totalTokens(entry) : null, cost: entry?.cost ?? null })
  }
  const tallest = Math.max(0, ...slots.map((slot) => slot.tokens ?? 0))
  return slots.map((slot) => ({ ...slot, fraction: tallest === 0 || slot.tokens === null ? 0 : slot.tokens / tallest }))
}
