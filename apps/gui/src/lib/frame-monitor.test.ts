import { describe, expect, it, vi } from 'vitest'
import { createFrameJudge, GAP_MS, SAMPLE_MS, startFrameMonitor, type FrameEnvironment, type FrameVerdict } from './frame-monitor'

const SMOOTH_MS = 16
const CHOPPY_MS = 60

/** Feeds frames `every` ms apart for `duration` ms and returns the last verdict. */
function run(every: number, duration: number): FrameVerdict {
  const judge = createFrameJudge()
  let verdict: FrameVerdict = { state: 'pending' }
  for (let at = 0; at <= duration && verdict.state === 'pending'; at += every) verdict = judge.push(at)
  return verdict
}

describe('frame judge', () => {
  it('stays pending until the sample time has passed', () => {
    expect(run(SMOOTH_MS, SAMPLE_MS - SMOOTH_MS * 2).state).toBe('pending')
  })

  it('calls steady 60 Hz frames fine', () => {
    const verdict = run(SMOOTH_MS, SAMPLE_MS + SMOOTH_MS)
    expect(verdict.state).toBe('ok')
  })

  it('calls frames at 15 per second slow, with the evidence', () => {
    const verdict = run(CHOPPY_MS, SAMPLE_MS + CHOPPY_MS)
    expect(verdict).toMatchObject({ state: 'slow', evidence: { slowFrames: expect.any(Number) as number, worstFrameMs: CHOPPY_MS } })
  })

  it('forgives a few hitches among smooth frames', () => {
    const judge = createFrameJudge()
    let verdict: FrameVerdict = { state: 'pending' }
    let at = 0
    let count = 0
    while (verdict.state === 'pending') {
      count += 1
      // One frame in twenty is a 90 ms hitch.
      at += count % 20 === 0 ? 90 : SMOOTH_MS
      verdict = judge.push(at)
    }
    expect(verdict.state).toBe('ok')
  })

  it('starts over after a suspended window instead of counting the gap as a slow frame', () => {
    const judge = createFrameJudge()
    for (let at = 0; at < 1000; at += SMOOTH_MS) judge.push(at)
    expect(judge.push(1000 + GAP_MS + 500).state).toBe('pending')
    let verdict: FrameVerdict = { state: 'pending' }
    for (let at = 3000; verdict.state === 'pending'; at += SMOOTH_MS) verdict = judge.push(at)
    expect(verdict.state).toBe('ok')
  })
})

/** A fake animation-frame clock: `tick` runs what was requested, `ms` later. */
function fakeFrames(active = true) {
  let now = 0
  let next = 1
  const pending = new Map<number, (timestamp: number) => void>()
  let isActive = active
  const environment: FrameEnvironment = {
    requestFrame: (callback) => { pending.set(next, callback); return next++ },
    cancelFrame: (handle) => { pending.delete(handle) },
    isActive: () => isActive,
  }
  return {
    environment,
    pendingCount: () => pending.size,
    setActive: (value: boolean) => { isActive = value },
    tick: (ms: number) => {
      now += ms
      const callbacks = [...pending.values()]
      pending.clear()
      for (const callback of callbacks) callback(now)
    },
  }
}

describe('frame monitor', () => {
  it('reports once, then stops asking for frames', () => {
    const frames = fakeFrames()
    const onVerdict = vi.fn()
    startFrameMonitor(onVerdict, frames.environment)

    for (let elapsed = 0; elapsed <= SAMPLE_MS + SMOOTH_MS * 2; elapsed += SMOOTH_MS) frames.tick(SMOOTH_MS)

    expect(onVerdict).toHaveBeenCalledTimes(1)
    expect(onVerdict).toHaveBeenCalledWith('ok', expect.objectContaining({ slowFrames: 0 }))
    expect(frames.pendingCount()).toBe(0)
  })

  it('reports slow frames as slow', () => {
    const frames = fakeFrames()
    const onVerdict = vi.fn()
    startFrameMonitor(onVerdict, frames.environment)

    for (let elapsed = 0; elapsed <= SAMPLE_MS + CHOPPY_MS * 2; elapsed += CHOPPY_MS) frames.tick(CHOPPY_MS)

    expect(onVerdict).toHaveBeenCalledWith('slow', expect.objectContaining({ worstFrameMs: CHOPPY_MS }))
  })

  it('does not count frames while the window is in the background', () => {
    const frames = fakeFrames(false)
    const onVerdict = vi.fn()
    startFrameMonitor(onVerdict, frames.environment)

    for (let elapsed = 0; elapsed <= SAMPLE_MS * 2; elapsed += CHOPPY_MS) frames.tick(CHOPPY_MS)

    expect(onVerdict).not.toHaveBeenCalled()
    expect(frames.pendingCount()).toBe(1)
  })

  it('stops when told to', () => {
    const frames = fakeFrames()
    const onVerdict = vi.fn()
    const stop = startFrameMonitor(onVerdict, frames.environment)

    stop()
    frames.tick(SAMPLE_MS * 2)

    expect(onVerdict).not.toHaveBeenCalled()
    expect(frames.pendingCount()).toBe(0)
  })
})
