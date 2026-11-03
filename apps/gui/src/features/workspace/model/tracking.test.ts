import { describe, expect, it } from 'vitest'
import { tracksEqual, type SessionTrack } from './tracking'

const track: SessionTrack = { status: 'working', repoRoot: '/repo', changedAt: 10, workingSince: 10, unread: false, movedAt: null }

describe('tracks equality', () => {
  it('reads two maps with the same entries as the same, so an unchanged list does not re-render', () => {
    const a = new Map([['s1', track], ['s2', { ...track, status: 'idle' as const }]])
    const b = new Map([['s2', { ...track, status: 'idle' as const }], ['s1', { ...track }]])
    expect(tracksEqual(a, b)).toBe(true)
  })

  it('tells a changed field, a new session and a missing one apart from the same', () => {
    const a = new Map([['s1', track]])
    expect(tracksEqual(a, new Map([['s1', { ...track, unread: true }]]))).toBe(false)
    expect(tracksEqual(a, new Map([['s1', track], ['s2', track]]))).toBe(false)
    expect(tracksEqual(a, new Map())).toBe(false)
  })
})
