import { describe, expect, it } from 'vitest'
import { dragWidths } from './columns'

describe('dragging the gap between columns', () => {
  it('moves width from one neighbour to the other and leaves the rest alone', () => {
    expect(dragWidths([400, 400, 200], 0, 100)).toEqual([500, 300, 200])
  })

  it('never takes a column below the minimum', () => {
    expect(dragWidths([400, 400], 0, -900)).toEqual([180, 620])
    expect(dragWidths([400, 400], 0, 900)).toEqual([620, 180])
  })

  it('snaps to an even split when close to it', () => {
    expect(dragWidths([400, 400], 0, 10)).toEqual([400, 400])
    expect(dragWidths([400, 400], 0, 40)).toEqual([440, 360])
  })

  it('ignores a handle with nothing beyond it', () => {
    expect(dragWidths([400], 0, 50)).toEqual([400])
  })
})
