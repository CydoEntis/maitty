import type { LeadMode, TrayStyle } from '@/features/settings/preferences'
import type { SessionStatus, SessionView } from './session'

/** What needs you first: waiting for you, then failed, finished, still running, and last the quiet ones. */
const PRIORITY: Record<SessionStatus, number> = { needs: 0, error: 1, done: 2, working: 3, idle: 4, unknown: 5 }

/** When a session last changed, in ms; 0 when this window has not seen it change. */
export type Activity = (sessionId: string) => number

/** Most urgent first; sessions in the same state go most recently active first. */
export function byPriority(activity: Activity) {
  return (a: SessionView, b: SessionView): number => PRIORITY[a.status] - PRIORITY[b.status] || activity(b.id) - activity(a.id)
}

/** Pinned sessions first, then most urgent first. */
function pinnedFirst(activity: Activity) {
  const urgent = byPriority(activity)
  return (a: SessionView, b: SessionView): number => Number(b.pinned === true) - Number(a.pinned === true) || urgent(a, b)
}

/**
 * The session a project shows as its main row. With "Focused" that is the session you are in, else
 * a pinned one, else the one last picked in this project, else the most urgent. With "Needs
 * attention" it is always the most urgent.
 */
export function chooseLead(sessions: readonly SessionView[], mode: LeadMode, focusedId: string | null, remembered: string | undefined, activity: Activity): SessionView | undefined {
  const urgent = [...sessions].sort(byPriority(activity))[0]
  if (mode === 'attention') return urgent
  const pinned = sessions.filter((session) => session.pinned).sort(byPriority(activity))[0]
  return sessions.find((session) => session.id === focusedId) ?? pinned ?? sessions.find((session) => session.id === remembered) ?? urgent
}

export type TrayPlan = {
  /** The sessions that get their own item: pinned first, then most urgent first. */
  shown: SessionView[]
  /** How many more the +N item stands for; 0 when everything fits. */
  hidden: number
  /** Whether any of those hidden ones needs you, so +N can say so. */
  hiddenNeeds: boolean
}

/** How many items each style fits before the rest move behind +N, which takes one of the slots. */
const TRAY_FIT: Record<TrayStyle, number> = { names: 2, icons: 6, stacked: 3 }

/** What the tray under a project's main row shows for the sessions that are not the lead. */
export function planTray(others: readonly SessionView[], style: TrayStyle, activity: Activity): TrayPlan {
  const sorted = [...others].sort(pinnedFirst(activity))
  const fit = TRAY_FIT[style]
  if (sorted.length <= fit) return { shown: sorted, hidden: 0, hiddenNeeds: false }
  // "+N" sits in the last column of a grid (with names, icons) or on its own line (stacked).
  const keep = style === 'names' ? fit : fit - 1
  const rest = sorted.slice(keep)
  return { shown: sorted.slice(0, keep), hidden: rest.length, hiddenNeeds: rest.some((session) => session.status === 'needs') }
}
