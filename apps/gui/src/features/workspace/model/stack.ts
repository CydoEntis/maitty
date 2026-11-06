import type { TrayStyle } from '@/features/settings/preferences'
import type { SessionStatus, SessionView } from './session'

/** What needs you first: waiting for you, then failed, finished, still running, and last the quiet ones. */
const PRIORITY: Record<SessionStatus, number> = { needs: 0, error: 1, done: 2, working: 3, idle: 4, unknown: 5 }

/** When a session last changed, in ms; 0 when this window has not seen it change. */
export type Activity = (sessionId: string) => number

/** Most urgent first; sessions in the same state go most recently active first. */
export function byPriority(activity: Activity) {
  return (a: SessionView, b: SessionView): number => PRIORITY[a.status] - PRIORITY[b.status] || activity(b.id) - activity(a.id)
}

/**
 * The session a project shows as its full card: the one you are in, else the one last picked in this
 * project, else the first by creation order. `sessions` is already in creation order and stays so:
 * status, pin and focus never move a session.
 */
export function chooseCard(sessions: readonly SessionView[], focusedId: string | null, remembered: string | undefined): SessionView | undefined {
  return sessions.find((session) => session.id === focusedId) ?? sessions.find((session) => session.id === remembered) ?? sessions[0]
}

export type StackPlan = {
  /** The sessions created before the card: compact tree rows above it. All of them show. */
  before: SessionView[]
  /** The sessions created after the card that get their own item. */
  shown: SessionView[]
  /** How many more the +N item stands for; 0 when everything fits. */
  hidden: number
  /** Whether any of those hidden ones needs you, so +N can say so. */
  hiddenNeeds: boolean
}

/** How many items after the card each style fits before the rest move behind +N. */
const AFTER_FIT: Record<TrayStyle, number> = { tree: 4, names: 2, icons: 6, stacked: 3 }
/** How many it keeps when it does overflow: +N takes a slot in a grid and a whole line in a list. */
const AFTER_KEPT: Record<TrayStyle, number> = { tree: 2, names: 2, icons: 5, stacked: 2 }

/** What a project shows around its card. The cap applies only to the rows after the card. */
export function planStack(sessions: readonly SessionView[], card: SessionView, style: TrayStyle): StackPlan {
  const at = sessions.findIndex((session) => session.id === card.id)
  const before = sessions.slice(0, at)
  const after = sessions.slice(at + 1)
  if (after.length <= AFTER_FIT[style]) return { before, shown: after, hidden: 0, hiddenNeeds: false }
  const rest = after.slice(AFTER_KEPT[style])
  return { before, shown: after.slice(0, AFTER_KEPT[style]), hidden: rest.length, hiddenNeeds: rest.some((session) => session.status === 'needs') }
}
