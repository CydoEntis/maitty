import { describe, expect, it } from 'vitest'
import { buildLayout, isAtBottom, visibleRange } from './windowing'

const keys = (count: number) => Array.from({ length: count }, (_, index) => `row-${String(index)}`)

describe('the layout of a list with rows of different heights', () => {
  it('stacks measured rows and counts the rest at the estimate', () => {
    const layout = buildLayout(['a', 'b', 'c'], new Map([['b', 100]]), 40)

    expect(layout.offsets).toEqual([0, 40, 140])
    expect(layout.total).toBe(180)
  })
})

describe('the rows to draw', () => {
  const layout = buildLayout(keys(2000), new Map(), 50)

  it('draws a screenful plus the overscan, however long the list is', () => {
    const range = visibleRange(layout, 25_000, 600, 5)

    expect(range.end - range.start).toBeLessThanOrEqual(12 + 2 * 5 + 1)
    expect(range.start).toBeLessThanOrEqual(500)
    expect(range.end).toBeGreaterThan(512)
  })

  it('starts at the top and stops at the end', () => {
    expect(visibleRange(layout, 0, 600, 3).start).toBe(0)
    expect(visibleRange(layout, layout.total, 600, 3).end).toBe(2000)
  })

  it('draws nothing for an empty list', () => {
    expect(visibleRange(buildLayout([], new Map(), 50), 0, 600, 3)).toEqual({ start: 0, end: 0 })
  })

  it('follows rows whose heights were measured', () => {
    const measured = buildLayout(keys(100), new Map(keys(100).map((key) => [key, 200])), 50)

    const range = visibleRange(measured, 4_000, 600, 0)

    expect(range.start).toBe(20)
    expect(range.end).toBe(24)
  })
})

describe('following the newest row', () => {
  it('counts a list as at the bottom within a few pixels of it', () => {
    expect(isAtBottom(1_000, 600, 1_600)).toBe(true)
    expect(isAtBottom(980, 600, 1_600)).toBe(true)
    expect(isAtBottom(500, 600, 1_600)).toBe(false)
  })
})
