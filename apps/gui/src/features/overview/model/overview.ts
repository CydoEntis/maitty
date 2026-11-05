import type { SessionStatus, SessionView } from '@/features/workspace/model/session'

/** The Overview's sections, in the order the screen shows them (v18 C.3). */
export const OVERVIEW_SECTIONS = ['needs', 'working', 'done', 'idle', 'unknown'] as const
export type OverviewSection = (typeof OVERVIEW_SECTIONS)[number]

export const SECTION_LABEL: Record<OverviewSection, string> = {
  needs: 'Needs you',
  working: 'Working',
  done: 'Done',
  idle: 'Idle',
  unknown: 'Unknown',
}

/** Exited sessions sit in Done and keep their own shape on the card, so the section never hides a failure. */
const SECTION_OF: Record<SessionStatus, OverviewSection> = {
  needs: 'needs',
  working: 'working',
  done: 'done',
  error: 'done',
  idle: 'idle',
  unknown: 'unknown',
}

export const sectionOf = (status: SessionStatus): OverviewSection => SECTION_OF[status]

/**
 * Creation order. The daemon's pane number only grows and each machine numbers its own, so the
 * machine breaks ties; the sidebar sorts by the same number, which keeps the two screens in step.
 */
export function byCreation(a: Pick<SessionView, 'pane' | 'machine'>, b: Pick<SessionView, 'pane' | 'machine'>): number {
  return a.pane - b.pane || a.machine - b.machine
}

export type OverviewGroup = { section: OverviewSection; sessions: SessionView[] }

/**
 * Every session in its section, each section in creation order. The sections always come back, empty
 * or not, so the screen decides which empty ones to hide. A status change moves a card between
 * sections but never reorders the cards that stay, because the order never reads the status.
 */
export function overviewSections(sessions: readonly SessionView[]): OverviewGroup[] {
  const ordered = [...sessions].sort(byCreation)
  return OVERVIEW_SECTIONS.map((section) => ({ section, sessions: ordered.filter((session) => sectionOf(session.status) === section) }))
}

export const needsYouCount = (sessions: readonly SessionView[]): number => sessions.filter((session) => session.status === 'needs').length

/** How many machines the sessions are on; this computer counts as one. */
export const machineCount = (sessions: readonly SessionView[]): number => new Set(sessions.map((session) => session.machine)).size

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
