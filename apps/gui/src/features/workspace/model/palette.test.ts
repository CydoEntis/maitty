import { describe, expect, it } from 'vitest'
import { matchScore, PALETTE_LIMIT, searchPalette, type PaletteItem } from './palette'

const item = (id: string, label: string, extra: Partial<PaletteItem> = {}): PaletteItem => ({ id, label, kind: 'Action', run: () => undefined, ...extra })

describe('palette matching', () => {
  it('scores a run of the text above the same letters spread out, and nothing for a miss', () => {
    expect(matchScore('spl', 'Split right')).toBeGreaterThan(matchScore('spt', 'Split right'))
    expect(matchScore('spt', 'Split right')).toBeGreaterThan(0)
    expect(matchScore('xyz', 'Split right')).toBe(0)
  })

  it('prefers a match at the start', () => {
    expect(matchScore('set', 'Settings')).toBeGreaterThan(matchScore('set', 'Reset pane widths'))
  })

  it('lists everything for an empty query, except themes', () => {
    const items = [item('a', 'Split right'), item('t', 'Nord', { kind: 'Theme', needsQuery: true })]
    expect(searchPalette(items, '').map((entry) => entry.id)).toEqual(['a'])
    expect(searchPalette(items, 'nor').map((entry) => entry.id)).toEqual(['t'])
  })

  it('matches on the detail too, ranks by score, and caps the list', () => {
    const items = [item('a', 'Open worktree', { detail: 'maitty' }), item('b', 'maitty docs')]
    expect(searchPalette(items, 'maitty').map((entry) => entry.id)).toEqual(['b', 'a'])
    const many = Array.from({ length: 30 }, (_, index) => item(String(index), `Action ${String(index)}`))
    expect(searchPalette(many, 'action')).toHaveLength(PALETTE_LIMIT)
  })
})
