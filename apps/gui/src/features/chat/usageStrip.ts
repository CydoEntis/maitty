/** One of the agent's plan limits, as the daemon last heard it: the window (`5h`, `week`), the share used and when it starts over. */
export type PlanLimit = { readonly name: string; readonly used: number; readonly resetsAt: number }

/** What a session's agent reported using: how full its context is, and its plan's limits. Each is absent when the agent did not say. */
export type SessionUsage = { readonly context: number | null; readonly limits: readonly PlanLimit[] }

export type UsageItem = {
  readonly key: string
  readonly label: string
  /** 0 to 100, whole. */
  readonly percent: number
  /** How long until the limit starts over (`2d 4h`); null for the context, which does not. */
  readonly resetsIn: string | null
  readonly tone: 'plain' | 'warn' | 'danger'
}

const FULL = 100
/** From here a meter is amber: worth a look before it is a problem. */
const WARN_FROM = 75
/** From here it is red: the next long turn may not fit. */
const DANGER_FROM = 90
const MINUTE = 60
const HOUR = 3600
const DAY = 86_400

/** The names the daemon gives the limit windows, as the strip says them; any other is shown as it came. */
const LIMIT_LABEL: Readonly<Record<string, string>> = { '5h': '5-hour limit', week: 'Weekly limit' }

const toneOf = (percent: number): UsageItem['tone'] => (percent >= DANGER_FROM ? 'danger' : percent >= WARN_FROM ? 'warn' : 'plain')
const whole = (value: number): number => Math.min(FULL, Math.max(0, Math.round(value)))

/** `2d 4h`, `3h 10m`, `12m`: the two largest parts of a wait, or `now` when it is over. */
export function waitWords(seconds: number): string {
  if (seconds < MINUTE) return 'now'
  const days = Math.floor(seconds / DAY)
  const hours = Math.floor((seconds % DAY) / HOUR)
  const minutes = Math.floor((seconds % HOUR) / MINUTE)
  if (days > 0) return hours > 0 ? `${String(days)}d ${String(hours)}h` : `${String(days)}d`
  if (hours > 0) return minutes > 0 ? `${String(hours)}h ${String(minutes)}m` : `${String(hours)}h`
  return `${String(minutes)}m`
}

/**
 * The meters of the strip under the message box: the context first, then each limit in the order the daemon gave
 * them. A figure the agent did not report has no meter, never a zero (ADR 0009). `now` is unix seconds.
 */
export function usageItems(usage: SessionUsage, now: number): UsageItem[] {
  const context: UsageItem[] = usage.context === null ? [] : [{ key: 'context', label: 'Context', percent: whole(usage.context), resetsIn: null, tone: toneOf(whole(usage.context)) }]
  const limits = usage.limits.map((limit): UsageItem => ({
    key: `limit-${limit.name}`,
    label: LIMIT_LABEL[limit.name] ?? limit.name,
    percent: whole(limit.used),
    resetsIn: limit.resetsAt > 0 ? waitWords(limit.resetsAt - now) : null,
    tone: toneOf(whole(limit.used)),
  }))
  return [...context, ...limits]
}
