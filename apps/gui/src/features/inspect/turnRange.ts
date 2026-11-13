import type { RepoSnapshot } from './api'
import { MS_PER_SECOND } from './format'

/** When a turn ran, in unix milliseconds. */
export type TurnSpan = { readonly startedAtMs: number; readonly endedAtMs: number }

export type SnapshotRange = { readonly from: string; readonly to: string }

/**
 * How long after a turn's start or end its snapshot may be stamped. The daemon takes it off its loop once it hears of
 * the start or end, so its time trails the turn's by the git work; a later turn's snapshots come after more than that.
 */
export const SNAPSHOT_SKEW_MS = 3000
/** A turn's start may claim this share of its length, so a short turn's own end snapshot is not read as its start. */
const START_SHARE = 0.5

/**
 * The snapshots a turn's diff runs between: the last one taken by its start and the last one taken by its end. The
 * daemon keeps no new snapshot when the files equal the newest one, so a turn that changed nothing gets the same one
 * twice, and either side may carry another turn's label. Null when no snapshot was taken by the turn's start.
 */
export function turnRange(snapshots: readonly RepoSnapshot[], turn: TurnSpan): SnapshotRange | null {
  // The daemon lists newest first, exact to the millisecond; the stable sort keeps that order within a second.
  // A snapshot's time keeps whole seconds; read as milliseconds it is the earliest it could have been taken.
  const oldestFirst = [...snapshots].reverse().sort((a, b) => a.at - b.at)
  const lastBy = (ms: number) => oldestFirst.findLast((snapshot) => snapshot.at * MS_PER_SECOND <= ms)
  const startSlack = Math.min(SNAPSHOT_SKEW_MS, (turn.endedAtMs - turn.startedAtMs) * START_SHARE)
  const before = lastBy(turn.startedAtMs + startSlack)
  const after = lastBy(turn.endedAtMs + SNAPSHOT_SKEW_MS)
  if (before === undefined || after === undefined) return null
  return { from: before.commit, to: after.commit }
}
