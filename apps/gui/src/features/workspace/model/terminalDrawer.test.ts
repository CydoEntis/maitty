import { describe, expect, it } from 'vitest'
import { clampDrawerHeight, DRAWER_MAX_PX, DRAWER_MIN_PX } from './terminalDrawer'

describe('the terminals drawer', () => {
  it('is never shorter than its least height nor taller than its most', () => {
    expect(clampDrawerHeight(10, 2000)).toBe(DRAWER_MIN_PX)
    expect(clampDrawerHeight(5000, 2000)).toBe(DRAWER_MAX_PX)
  })

  it('leaves the chat the larger part of a short pane', () => {
    expect(clampDrawerHeight(500, 500)).toBe(300)
  })

  it('keeps its least height in a pane too short to share', () => {
    expect(clampDrawerHeight(300, 100)).toBe(DRAWER_MIN_PX)
  })
})
