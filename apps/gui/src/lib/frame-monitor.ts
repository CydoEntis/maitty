import { isWindowActive } from './window-activity'

/** How long one measurement watches the frames. */
export const SAMPLE_MS = 3000
/** A frame longer than this is below 25 frames a second: choppy next to a looping animation. */
export const SLOW_FRAME_MS = 40
/** A quarter of the frames being slow means the animation itself is not affordable. */
export const SLOW_SHARE = 0.25
/** A gap this long is a suspended window, not a slow frame; the measurement starts over. */
export const GAP_MS = 1000

export type FrameEvidence = { frames: number; slowFrames: number; worstFrameMs: number }

export type FrameVerdict = { state: 'pending' } | { state: 'ok' | 'slow'; evidence: FrameEvidence }

const PENDING: FrameVerdict = { state: 'pending' }

/**
 * Judges a run of animation-frame timestamps. Pure: it is fed times and never reads a clock, so a
 * test drives it with a fake one. It decides once `SAMPLE_MS` of frames have gone by.
 */
export function createFrameJudge() {
  let start: number | null = null
  let last = 0
  let frames = 0
  let slowFrames = 0
  let worstFrameMs = 0

  const restart = (at: number | null) => {
    start = at
    last = at ?? 0
    frames = 0
    slowFrames = 0
    worstFrameMs = 0
  }

  return {
    /** Forget what was seen, for when frames stop meaning anything (the window went to the background). */
    reset: () => { restart(null) },
    push(timestamp: number): FrameVerdict {
      if (start === null || timestamp - last > GAP_MS) {
        restart(timestamp)
        return PENDING
      }
      const delta = timestamp - last
      last = timestamp
      frames += 1
      worstFrameMs = Math.max(worstFrameMs, delta)
      if (delta > SLOW_FRAME_MS) slowFrames += 1
      if (timestamp - start < SAMPLE_MS) return PENDING
      const evidence = { frames, slowFrames, worstFrameMs: Math.round(worstFrameMs) }
      return { state: slowFrames / frames >= SLOW_SHARE ? 'slow' : 'ok', evidence }
    },
  }
}

/** The browser calls the monitor needs, so a test can stand in for them. */
export type FrameEnvironment = {
  requestFrame: (callback: (timestamp: number) => void) => number
  cancelFrame: (handle: number) => void
  isActive: () => boolean
}

const BROWSER: FrameEnvironment = {
  requestFrame: (callback) => requestAnimationFrame(callback),
  cancelFrame: (handle) => { cancelAnimationFrame(handle) },
  isActive: isWindowActive,
}

/**
 * Watches animation frames until it has a verdict, reports it once and stops itself: it costs one
 * callback a frame for `SAMPLE_MS` and nothing after. Returns a function that stops it early.
 */
export function startFrameMonitor(onVerdict: (verdict: 'ok' | 'slow', evidence: FrameEvidence) => void, environment: FrameEnvironment = BROWSER): () => void {
  const judge = createFrameJudge()
  let handle = 0
  let stopped = false
  const onFrame = (timestamp: number) => {
    if (stopped) return
    if (!environment.isActive()) {
      judge.reset()
    } else {
      const verdict = judge.push(timestamp)
      if (verdict.state !== 'pending') {
        stopped = true
        onVerdict(verdict.state, verdict.evidence)
        return
      }
    }
    handle = environment.requestFrame(onFrame)
  }
  handle = environment.requestFrame(onFrame)
  return () => {
    stopped = true
    environment.cancelFrame(handle)
  }
}
