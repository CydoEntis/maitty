import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'

/**
 * What agents reported using, from the daemon's usage store (ADR 0009). Everything here is what
 * Claude Code or Codex said; nothing is estimated. A figure nobody reported is `null`, which the
 * page shows as a dash. Only this computer's daemon is asked.
 */

export const USAGE_PERIODS = ['day', 'week', 'month'] as const
export type UsagePeriod = (typeof USAGE_PERIODS)[number]

const Count = z.number().int().nonnegative()

/** The figures of a session, a day or a whole period. */
const FiguresSchema = z.object({
  /** Input tokens; null when the provider reported none. */
  tokensIn: Count.nullable(),
  tokensOut: Count.nullable(),
  /** US dollars; null for a subscription session or an agent that reports no cost. */
  cost: z.number().nonnegative().nullable(),
  turns: Count,
  /** Seconds the agent spent working. */
  workingSecs: Count,
})
export type UsageFigures = z.infer<typeof FiguresSchema>

const DaySchema = z.string().regex(/^\d{4}-\d{2}-\d{2}$/)

const SessionUsageSchema = FiguresSchema.extend({
  /** The agent's own session id. */
  session: z.string(),
  /** The pane showing the session now, when it is still open. */
  term: Count.nullable(),
  agent: z.string(),
  /** The folder the session worked in, by name. */
  project: z.string(),
  firstDay: DaySchema,
  lastDay: DaySchema,
})
export type SessionUsage = z.infer<typeof SessionUsageSchema>

const DayUsageSchema = FiguresSchema.extend({ day: DaySchema })
export type DayUsage = z.infer<typeof DayUsageSchema>

const UsageReportSchema = z.object({
  period: z.enum(['Day', 'Week', 'Month']),
  /** First and last day of the window, UTC, both included. */
  from: DaySchema,
  to: DaySchema,
  /** How many days the daemon keeps. */
  retentionDays: Count,
  totals: FiguresSchema,
  sessionCount: Count,
  /** Sessions with no reported cost: the page shows one note for them, not one per row. */
  sessionsWithoutCost: Count,
  /** Most cost first, then most working time. */
  sessions: z.array(SessionUsageSchema),
  /** Oldest first, only days with usage. */
  days: z.array(DayUsageSchema),
})
export type UsageReport = z.infer<typeof UsageReportSchema>

/** What the daemon on this computer has recorded for the last day, week or month. */
export async function loadUsageReport(period: UsagePeriod): Promise<UsageReport> {
  if (!isTauri()) throw new Error('Usage is only available in the desktop app')
  return UsageReportSchema.parse(await invoke<unknown>('usage_report', { period }))
}
