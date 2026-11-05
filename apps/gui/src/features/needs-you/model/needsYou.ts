import type { SessionView } from '@/features/workspace/model/session'

/**
 * Creation order. The daemon's pane number only grows and each machine numbers its own, so the
 * machine breaks ties; the sidebar sorts by the same number, so a jump walks the sessions in the order the sidebar lists them.
 */
export function byCreation(a: Pick<SessionView, 'pane' | 'machine'>, b: Pick<SessionView, 'pane' | 'machine'>): number {
  return a.pane - b.pane || a.machine - b.machine
}

export type JumpTarget = { session: SessionView; /** 1-based place among the sessions that need you. */ position: number; total: number }

/**
 * The next session that needs you after `currentId`, wrapping. "After" is by creation order, so it
 * is right even when the current session does not need you itself. Null when nothing needs you: the
 * jump then only says so; it never falls back to a finished session.
 */
export function nextNeedsYou(sessions: readonly SessionView[], currentId: string | null): JumpTarget | null {
  const asking = sessions.filter((session) => session.status === 'needs').sort(byCreation)
  if (asking.length === 0) return null
  const current = sessions.find((session) => session.id === currentId)
  const next = (current && asking.find((session) => byCreation(session, current) > 0)) ?? asking[0]
  if (!next) return null
  return { session: next, position: asking.indexOf(next) + 1, total: asking.length }
}

/** The toast that says where a jump went, when it opened a session rather than moved a ring. */
export const jumpToast = (target: JumpTarget): string => `Needs you · ${target.session.name ?? target.session.title} · ${String(target.position)} of ${String(target.total)}`

export const NOTHING_NEEDS_YOU = 'Nothing needs you'
