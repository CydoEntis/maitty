import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { CLOCK_TICK_MS, useNow, useRecent } from './clock'

describe('shared clock', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('moves forward a second at a time while something shows it', () => {
    const { result } = renderHook(() => useNow())
    const first = result.current

    act(() => { vi.advanceTimersByTime(CLOCK_TICK_MS) })

    expect(result.current).toBeGreaterThanOrEqual(first + CLOCK_TICK_MS)
  })

  it('stands still while the window is in the background', () => {
    const { result } = renderHook(() => useNow())
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    act(() => { window.dispatchEvent(new Event('blur')) })
    const held = result.current

    act(() => { vi.advanceTimersByTime(CLOCK_TICK_MS * 5) })

    expect(result.current).toBe(held)
  })

  it('stops the clock once nothing shows it', () => {
    const { unmount } = renderHook(() => useNow())
    unmount()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('says a moment is recent until its window passes, then turns false by itself', () => {
    const at = Date.now()
    const { result } = renderHook(() => useRecent(at, 1600))
    expect(result.current).toBe(true)

    act(() => { vi.advanceTimersByTime(1700) })

    expect(result.current).toBe(false)
  })

  it('never says an old moment is recent, not even on the first render', () => {
    const { result } = renderHook(() => useRecent(Date.now() - 5000, 1600))
    expect(result.current).toBe(false)
  })
})
