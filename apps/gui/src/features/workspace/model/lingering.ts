import type { SessionView } from './session'

/** How long a held session waits before a full list is read to confirm it is gone, and between such reads. */
export const LINGER_MS = 10_000
/** The wait added to a check's time, so the check runs past it. */
export const LINGER_SLACK_MS = 50

/** When a list first left a session out, and the revisions of the list that named it and of the one that did not. */
export type Miss = { at: number; listedIn: number; missedIn: number }

/** A session that dropped out of the list with no reason given. */
export type Missing = { session: SessionView; miss: Miss }

/**
 * The sessions to keep showing though the last list left them out. A list can miss a session for a moment (a
 * daemon that was restarting, a read that came back short) and name it again on the next; a session must stay in
 * the sidebar until it is closed, so one that vanishes with no reason is held, shown as unknown, until a full list
 * confirms it is gone (`confirmGone`). A session that left with a reason (`leftWithReason`: the daemon said why, or
 * this window closed it) is not held, and one back in the list stops being missing.
 */
export function trackMissing(
  before: ReadonlyMap<string, Missing>,
  previous: readonly SessionView[],
  current: readonly SessionView[],
  miss: Miss,
  leftWithReason: (id: string) => boolean,
): ReadonlyMap<string, Missing> {
  const listed = new Set(current.map((session) => session.id))
  const next = new Map<string, Missing>()
  for (const session of previous) {
    if (listed.has(session.id) || leftWithReason(session.id)) continue
    next.set(session.id, before.get(session.id) ?? { session, miss })
  }
  for (const [id, missing] of before) {
    if (!listed.has(id) && !leftWithReason(id)) next.set(id, missing)
  }
  return next
}

/**
 * Drops each held session that a full list, asked for at `askedAt` after the session was missed, does not name.
 * Another machine's session is never in this computer's list, so for it the check only ends the grace.
 */
export function confirmGone(missing: ReadonlyMap<string, Missing>, listed: ReadonlySet<string>, askedAt: number): ReadonlyMap<string, Missing> {
  const kept = new Map([...missing].filter(([id, entry]) => listed.has(id) || entry.miss.at > askedAt))
  return kept.size === missing.size ? missing : kept
}

/** When to read the full list next: `LINGER_MS` after the first miss, and after the last check (`checkedAt`); null with nothing held. */
export function nextCheckAt(missing: ReadonlyMap<string, Missing>, checkedAt: number): number | null {
  if (missing.size === 0) return null
  const firstMiss = Math.min(...[...missing.values()].map((entry) => entry.miss.at))
  return Math.max(firstMiss, checkedAt) + LINGER_MS
}

/** A held session as the sidebar shows it: it keeps its place, and its status is unknown until the list names it again. */
export const heldView = (missing: Missing): SessionView => ({ ...missing.session, status: 'unknown' })
