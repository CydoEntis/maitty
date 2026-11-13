import { machineNameOf } from '@/api/machines'
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
 * Drops each held session of this computer that its full list, asked for at `askedAt` after the session was missed,
 * does not name. Another machine's session is never in this computer's list, so this list says nothing of it.
 */
export function confirmGone(missing: ReadonlyMap<string, Missing>, listed: ReadonlySet<string>, askedAt: number): ReadonlyMap<string, Missing> {
  return keep(missing, (id, entry) => machineNameOf(id) !== null || listed.has(id) || entry.miss.at > askedAt)
}

/** A machine's last list as the host holds it: when it came, and the sessions it names. */
export type MachineList = { listedAt: number; ids: ReadonlySet<string> }

/**
 * Drops each held session of another machine that a list from that machine (`lists`, by machine name), come at
 * least `LINGER_MS` after the miss, does not name. Every list the host sends of a machine is its whole list, and
 * the window asks the machine for one while it holds a row of it, so one comes while the machine is up.
 */
export function confirmGoneOnMachines(missing: ReadonlyMap<string, Missing>, lists: ReadonlyMap<string, MachineList>): ReadonlyMap<string, Missing> {
  return keep(missing, (id, entry) => {
    const name = machineNameOf(id)
    const list = name === null ? undefined : lists.get(name)
    return list === undefined || list.ids.has(id) || list.listedAt - entry.miss.at < LINGER_MS
  })
}

function keep(missing: ReadonlyMap<string, Missing>, kept: (id: string, entry: Missing) => boolean): ReadonlyMap<string, Missing> {
  const next = new Map([...missing].filter(([id, entry]) => kept(id, entry)))
  return next.size === missing.size ? missing : next
}

/** When to read the full list next: `LINGER_MS` after the first miss, and after the last check (`checkedAt`); null with nothing held. */
export function nextCheckAt(missing: ReadonlyMap<string, Missing>, checkedAt: number): number | null {
  if (missing.size === 0) return null
  const firstMiss = Math.min(...[...missing.values()].map((entry) => entry.miss.at))
  return Math.max(firstMiss, checkedAt) + LINGER_MS
}

/** A held session as the sidebar shows it: it keeps its place, and its status is unknown until the list names it again. */
export const heldView = (missing: Missing): SessionView => ({ ...missing.session, status: 'unknown' })
