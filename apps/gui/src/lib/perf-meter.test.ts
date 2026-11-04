import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { beginFlush, countOutput, forgetRenderer, reportRenderer, SAMPLE_EVERY_MS, SLOW_FRAME_MS, usePerfSample } from './perf-meter'

describe('perf meter', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('has no sample until a second has passed', () => {
    const { result, unmount } = renderHook(() => usePerfSample())
    expect(result.current).toBeNull()
    act(() => { vi.advanceTimersByTime(SAMPLE_EVERY_MS) })
    expect(result.current).not.toBeNull()
    unmount()
  })

  it('turns the counters into per-second rates and clears them', () => {
    const { result, unmount } = renderHook(() => usePerfSample())
    countOutput(1500)
    countOutput(500)
    beginFlush()()
    act(() => { vi.advanceTimersByTime(SAMPLE_EVERY_MS) })
    expect(result.current).toMatchObject({ outputBytesPerSecond: 2000, outputFramesPerSecond: 2, flushesPerSecond: 1 })
    act(() => { vi.advanceTimersByTime(SAMPLE_EVERY_MS) })
    expect(result.current).toMatchObject({ outputBytesPerSecond: 0, outputFramesPerSecond: 0, flushesPerSecond: 0 })
    unmount()
  })

  it('counts the frames it was given and the slow ones', () => {
    const frames: FrameRequestCallback[] = []
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback))
    vi.stubGlobal('cancelAnimationFrame', () => undefined)
    const { result, unmount } = renderHook(() => usePerfSample())
    const paint = (at: number) => { frames.shift()?.(at) }
    act(() => {
      // The first frame only sets the clock; two follow, one of them slow.
      paint(0)
      paint(16)
      paint(16 + SLOW_FRAME_MS * 2)
      vi.advanceTimersByTime(SAMPLE_EVERY_MS)
    })
    expect(result.current).toMatchObject({ framesPerSecond: 2, slowFrames: 1, worstFrameMs: SLOW_FRAME_MS * 2 })
    unmount()
    vi.unstubAllGlobals()
  })

  it('counts the renderers of the open views', () => {
    const first = {}
    const second = {}
    reportRenderer(first, 'webgl')
    reportRenderer(second, 'dom')
    const { result, unmount } = renderHook(() => usePerfSample())
    act(() => { vi.advanceTimersByTime(SAMPLE_EVERY_MS) })
    expect(result.current?.renderers).toEqual({ webgl: 1, dom: 1 })
    forgetRenderer(first)
    forgetRenderer(second)
    act(() => { vi.advanceTimersByTime(SAMPLE_EVERY_MS) })
    expect(result.current?.renderers).toEqual({ webgl: 0, dom: 0 })
    unmount()
  })

  it('stops sampling once nothing shows it', () => {
    const { unmount } = renderHook(() => usePerfSample())
    unmount()
    expect(vi.getTimerCount()).toBe(0)
  })
})
