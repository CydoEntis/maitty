/** The times a session can be snoozed until, from its menu. The daemon owns the snooze (`Command::SnoozePane`); this only picks the time. */

const MS_PER_SECOND = 1000
const SECONDS_PER_HOUR = 3600
const HOURS_PER_DAY = 24
const DAYS_PER_WEEK = 7
const HOUR_MS = SECONDS_PER_HOUR * MS_PER_SECOND
/** The daemon refuses a snooze longer than a week, so a reset further off is not offered. */
const MOST_SNOOZE_MS = DAYS_PER_WEEK * HOURS_PER_DAY * HOUR_MS
const MORNING_HOUR = 9

export type SnoozeChoice = {
  /** The menu entry's words. */
  label: string
  /** When the session comes back, in unix seconds, as the daemon takes it. */
  until: number
}

/**
 * Tomorrow at 9:00 on this computer's clock, in milliseconds. Built from calendar parts, not by adding 24 hours, so a day of
 * 23 or 25 hours (a clock change) still lands on 9:00. `now` is passed in so the time is testable.
 */
export function tomorrowNine(now: number): number {
  const today = new Date(now)
  return new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1, MORNING_HOUR, 0, 0, 0).getTime()
}

/**
 * What the session menu offers at `now` (milliseconds): an hour, tomorrow at 9:00, and, when the daemon says when a plan
 * limit resets (`resumeAt`, milliseconds) and that is ahead and within a snooze's reach, until then.
 */
export function snoozeChoices(now: number, resumeAt: number | null): SnoozeChoice[] {
  const choices = [
    { label: 'Snooze 1 hour', at: now + HOUR_MS },
    { label: 'Snooze until tomorrow 9:00', at: tomorrowNine(now) },
  ]
  if (resumeAt !== null && resumeAt > now && resumeAt - now <= MOST_SNOOZE_MS) choices.push({ label: 'Snooze until the limit resets', at: resumeAt })
  return choices.map(({ label, at }) => ({ label, until: Math.ceil(at / MS_PER_SECOND) }))
}
