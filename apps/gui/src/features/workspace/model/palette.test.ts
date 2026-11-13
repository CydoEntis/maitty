import { describe, expect, it } from 'vitest'
import { cellsOf, firstHitPerPane, matchScore, OUTPUT_HIT_LIMIT, outputSnippet, PALETTE_LIMIT, searchPalette, withOutputHits, type PaletteItem } from './palette'

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

describe('output hits in the palette', () => {
  it('puts the output hits after every other kind, at most ten of them', () => {
    const results = searchPalette([item('a', 'Build logs'), item('s', 'build server', { kind: 'Session' })], 'build')
    const hits = Array.from({ length: 14 }, (_, index) => item(`o${String(index)}`, 'api server', { kind: 'Output' }))
    const merged = withOutputHits(results, hits)
    expect(merged.slice(0, 2).map((entry) => entry.id)).toEqual(['a', 's'])
    expect(merged.slice(2).every((entry) => entry.kind === 'Output')).toBe(true)
    expect(merged).toHaveLength(2 + OUTPUT_HIT_LIMIT)
  })

  it('cleans a line of escape codes and control characters', () => {
    expect(outputSnippet('\u001b[1;31mfailed\u001b[0m\tto\u0007 build\r', 'build')).toBe('failed to build')
  })

  it('drops 8-bit escape sequences and bidirectional controls', () => {
    expect(outputSnippet('\u009b1;31mred‮ ⁦text⁩‎؜', 'red')).toBe('red text')
  })

  it('cuts a long line to 60 cells, keeping 16 cells before the match', () => {
    const snippet = outputSnippet(`${'x'.repeat(150)} the needle ${'y'.repeat(150)}`, 'needle')
    expect(snippet).toBe(`…${'x'.repeat(11)} the needle ${'y'.repeat(35)}…`)
    expect(cellsOf(snippet)).toBe(60)
  })

  it('counts a wide character or an emoji as two cells', () => {
    expect(cellsOf('漢a👍🏽')).toBe(5)
    expect(outputSnippet('漢'.repeat(100), '漢')).toBe(`${'漢'.repeat(29)}…`)
    expect(outputSnippet(`${'漢'.repeat(20)}needle`, 'needle')).toBe(`…${'漢'.repeat(8)}needle`)
  })

  it('keeps only the first hit of each pane', () => {
    const hit = (term: number, line: number) => ({ term, line, text: '', machine: 0 })
    expect(firstHitPerPane([hit(1, 9), hit(1, 4), hit(2, 3), { ...hit(1, 2), machine: 1 }])).toEqual([hit(1, 9), hit(2, 3), { ...hit(1, 2), machine: 1 }])
  })
})
