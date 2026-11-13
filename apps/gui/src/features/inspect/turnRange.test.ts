import { describe, expect, it } from 'vitest'
import type { RepoSnapshot } from './api'
import { SNAPSHOT_SKEW_MS, snapshotBefore, turnRange } from './turnRange'

const SECOND = 1000
const snap = (commit: string, atSeconds: number, why = 'before claude\'s turn'): RepoSnapshot => ({ commit, at: atSeconds, why })
const turn = (startSeconds: number, endSeconds: number) => ({ startedAtMs: startSeconds * SECOND, endedAtMs: endSeconds * SECOND })

// Newest first, as the daemon lists them.
const twoTurns = [snap('after2', 161, 'after claude\'s turn'), snap('before2', 150), snap('after1', 131, 'after claude\'s turn'), snap('before1', 100)]

describe('picking the snapshots around a turn', () => {
  it('takes the last snapshot by the turn\'s start and the last one by its end', () => {
    expect(turnRange(twoTurns, turn(100, 130))).toEqual({ from: 'before1', to: 'after1' })
    expect(turnRange(twoTurns, turn(150, 160))).toEqual({ from: 'before2', to: 'after2' })
  })

  it('counts a snapshot taken a moment after the event that prompted it, up to the skew', () => {
    const late = [snap('after', 123), snap('before', 102)]
    expect(turnRange(late, { startedAtMs: 100 * SECOND, endedAtMs: 123 * SECOND - SNAPSHOT_SKEW_MS })).toEqual({ from: 'before', to: 'after' })
    expect(turnRange(late, { startedAtMs: 100 * SECOND, endedAtMs: 123 * SECOND - SNAPSHOT_SKEW_MS - 1 })).toEqual({ missing: 'after' })
  })

  it('reads a snapshot\'s whole seconds as the earliest it could have been taken', () => {
    expect(turnRange([snap('after', 132), snap('before', 100)], { startedAtMs: 100_900, endedAtMs: 129_100 })).toEqual({ from: 'before', to: 'after' })
  })

  it('starts from an older snapshot when the tree had not changed since, whatever its label', () => {
    const kept = [snap('after2', 161, 'after claude\'s turn'), snap('after1', 131, 'after claude\'s turn'), snap('before1', 100)]
    expect(turnRange(kept, turn(150, 160))).toEqual({ from: 'after1', to: 'after2' })
  })

  it('says the end snapshot is missing when none is newer than the start one by the end of the turn', () => {
    expect(turnRange([snap('after1', 131), snap('before1', 100)], turn(150, 160))).toEqual({ missing: 'after' })
    expect(turnRange([snap('later', 200), snap('before', 100)], turn(100, 130))).toEqual({ missing: 'after' })
  })

  it('never takes a short turn\'s own end snapshot as its start', () => {
    expect(turnRange([snap('after', 102), snap('before', 100)], turn(100, 102))).toEqual({ from: 'before', to: 'after' })
  })

  it('says the start snapshot is missing when none was taken by the start of the turn', () => {
    expect(turnRange([snap('later', 200)], turn(100, 130))).toEqual({ missing: 'before' })
    expect(turnRange([], turn(100, 130))).toEqual({ missing: 'before' })
  })
})

describe('the snapshot from before a turn', () => {
  it('is found for a turn that changed nothing, whose end has no snapshot of its own', () => {
    const unchanged = [snap('before2', 150), snap('after1', 131, 'after claude\'s turn')]
    expect(turnRange(unchanged, turn(150, 160))).toEqual({ missing: 'after' })
    expect(snapshotBefore(unchanged, turn(150, 160))?.commit).toBe('before2')
  })

  it('is missing when every kept snapshot is newer than the turn', () => {
    expect(snapshotBefore(twoTurns, turn(50, 60))).toBeUndefined()
  })
})
