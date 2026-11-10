import type { SessionStatus, SessionView } from './session'

/** What wants your attention is listed with what needs you first, then what finished, then what exited. */
const PRIORITY: Record<SessionStatus, number> = { needs: 0, done: 1, error: 2, working: 3, idle: 4, unknown: 5 }

/** When a session last changed, in ms; 0 when this window has not seen it change. */
export type Activity = (sessionId: string) => number

/** Most urgent first; sessions in the same state go most recently active first. */
export function byPriority(activity: Activity) {
  return (a: SessionView, b: SessionView): number => PRIORITY[a.status] - PRIORITY[b.status] || activity(b.id) - activity(a.id)
}
