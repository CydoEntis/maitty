import type { SessionStatus, SessionView } from './session'

/**
 * What this window has seen happen to a session since it opened. The daemon reports only the
 * current status, so times start when the window first sees a change; until then they are null
 * and nothing is shown rather than a guess.
 */
export type SessionTrack = {
  status: SessionStatus
  repoRoot: string | null
  changedAt: number | null
  workingSince: number | null
  unread: boolean
  movedAt: number | null
}

export type SessionMove = { id: string; from: string | null; to: string | null }

export const MOVED_HIGHLIGHT_MS = 1600

const ATTENTION: ReadonlySet<SessionStatus> = new Set(['done', 'needs', 'error'])

export function trackSessions(
  previous: ReadonlyMap<string, SessionTrack>,
  sessions: readonly SessionView[],
  visibleIds: ReadonlySet<string>,
  now: number,
): { tracks: Map<string, SessionTrack>; moves: SessionMove[] } {
  const tracks = new Map<string, SessionTrack>()
  const moves: SessionMove[] = []
  for (const session of sessions) {
    const before = previous.get(session.id)
    // A disconnected daemon reports nothing new: keep what was last seen.
    if (session.status === 'unknown') {
      if (before) tracks.set(session.id, before)
      continue
    }
    if (!before) {
      tracks.set(session.id, { status: session.status, repoRoot: session.repoRoot, changedAt: null, workingSince: null, unread: false, movedAt: null })
      continue
    }
    const statusChanged = before.status !== session.status
    const moved = before.repoRoot !== session.repoRoot
    if (moved) moves.push({ id: session.id, from: before.repoRoot, to: session.repoRoot })
    const visible = visibleIds.has(session.id)
    tracks.set(session.id, {
      status: session.status,
      repoRoot: session.repoRoot,
      changedAt: statusChanged ? now : before.changedAt,
      workingSince: session.status !== 'working' ? null : statusChanged ? now : before.workingSince,
      unread: visible ? false : statusChanged ? ATTENTION.has(session.status) || before.unread : before.unread,
      movedAt: moved ? now : before.movedAt,
    })
  }
  return { tracks, moves }
}

/** Marks sessions read once they are on screen. */
export function markSeen(tracks: ReadonlyMap<string, SessionTrack>, visibleIds: ReadonlySet<string>): Map<string, SessionTrack> {
  const next = new Map(tracks)
  for (const id of visibleIds) {
    const track = next.get(id)
    if (track?.unread) next.set(id, { ...track, unread: false })
  }
  return next
}

const SECOND_MS = 1000
const MINUTE_S = 60
const MINUTE_MS = MINUTE_S * SECOND_MS
const HOURS_PER_DAY = 24
const HOUR_MS = MINUTE_S * MINUTE_MS
const DAY_MS = HOURS_PER_DAY * HOUR_MS
const SECONDS_DIGITS = 2

/** `m:ss`, as a working row and the pane's status pill show elapsed time. */
export function formatElapsed(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / SECOND_MS))
  return `${String(Math.floor(seconds / MINUTE_S))}:${String(seconds % MINUTE_S).padStart(SECONDS_DIGITS, '0')}`
}

/** `now`, `4m`, `3h`, `2d`. */
export function formatAgo(ms: number): string {
  if (ms < MINUTE_MS) return 'now'
  if (ms < HOUR_MS) return `${String(Math.floor(ms / MINUTE_MS))}m`
  if (ms < DAY_MS) return `${String(Math.floor(ms / HOUR_MS))}h`
  return `${String(Math.floor(ms / DAY_MS))}d`
}
