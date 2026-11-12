import type { SessionView } from './session'

/** How long a session the list stopped naming stays in the sidebar before it is taken to be gone. */
export const LINGER_MS = 10_000
/** The wait before a held session is checked again, so it is past its grace when the check runs. */
export const LINGER_SLACK_MS = 50

/** A session that dropped out of the list, and when it was first missed. */
export type Missing = { session: SessionView; since: number }

/**
 * The sessions to keep showing though the last list left them out. A list can miss a session for a moment (a
 * daemon that was restarting, a read that came back short) and name it again on the next; a session must stay in
 * the sidebar until it is closed, so one that vanishes is held, shown as unknown, for `LINGER_MS`. A session the
 * window itself closed (`closed`) is not held, and one back in the list stops being missing.
 */
export function trackMissing(
  before: ReadonlyMap<string, Missing>,
  previous: readonly SessionView[],
  current: readonly SessionView[],
  now: number,
  closed: (id: string) => boolean,
): ReadonlyMap<string, Missing> {
  const listed = new Set(current.map((session) => session.id))
  const next = new Map<string, Missing>()
  for (const session of previous) {
    if (listed.has(session.id) || closed(session.id)) continue
    next.set(session.id, before.get(session.id) ?? { session, since: now })
  }
  for (const [id, missing] of before) {
    if (!listed.has(id) && !closed(id) && now - missing.since < LINGER_MS) next.set(id, missing)
  }
  return next
}

/** Drops what has been missing past `LINGER_MS`. */
export function pruneMissing(missing: ReadonlyMap<string, Missing>, now: number): ReadonlyMap<string, Missing> {
  const kept = new Map([...missing].filter(([, entry]) => now - entry.since < LINGER_MS))
  return kept.size === missing.size ? missing : kept
}

/** A held session as the sidebar shows it: it keeps its place, and its status is unknown until the list names it again. */
export const heldView = (missing: Missing): SessionView => ({ ...missing.session, status: 'unknown' })
