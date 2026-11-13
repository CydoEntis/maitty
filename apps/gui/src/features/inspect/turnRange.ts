import type { RepoSnapshot } from './api'
import { MS_PER_SECOND } from './format'

/** When a turn ran, in unix milliseconds. */
export type TurnSpan = { readonly startedAtMs: number; readonly endedAtMs: number }

export type SnapshotRange = { readonly from: string; readonly to: string }
/** Which side of a turn has no snapshot to compare: none by its start, or none newer than that one by its end. */
export type MissingSnapshot = { readonly missing: 'before' | 'after' }

/**
 * How long after a turn's start or end its snapshot may be stamped. The daemon takes it off its loop once it hears of
 * the start or end, so its time trails the turn's by the git work; a later turn's snapshots come after more than that.
 */
export const SNAPSHOT_SKEW_MS = 3000
/** A turn's start may claim this share of its length, so a short turn's own end snapshot is not read as its start. */
const START_SHARE = 0.5

/**
 * The snapshots a turn's diff runs between: the last one taken by its start and the last one taken by its end, either
 * of which may carry another turn's label. Only a turn that edited files is asked about, so no newer snapshot by its
 * end means the end snapshot is missing (not taken, or not yet), not that nothing changed.
 */
export function turnRange(snapshots: readonly RepoSnapshot[], turn: TurnSpan): SnapshotRange | MissingSnapshot {
  // The daemon lists newest first, exact to the millisecond; the stable sort keeps that order within a second.
  // A snapshot's time keeps whole seconds; read as milliseconds it is the earliest it could have been taken.
  const oldestFirst = [...snapshots].reverse().sort((a, b) => a.at - b.at)
  const lastBy = (ms: number) => oldestFirst.findLastIndex((snapshot) => snapshot.at * MS_PER_SECOND <= ms)
  const startSlack = Math.min(SNAPSHOT_SKEW_MS, (turn.endedAtMs - turn.startedAtMs) * START_SHARE)
  const before = oldestFirst[lastBy(turn.startedAtMs + startSlack)]
  const after = oldestFirst[lastBy(turn.endedAtMs + SNAPSHOT_SKEW_MS)]
  if (before === undefined) return { missing: 'before' }
  if (after === undefined || after === before) return { missing: 'after' }
  return { from: before.commit, to: after.commit }
}
