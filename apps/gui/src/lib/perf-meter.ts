import { useSyncExternalStore } from 'react'

/**
 * What the web view is doing, once a second, for Settings › Performance and the overlay.
 *
 * The counters cost an addition each and always run. The sampler (animation frames, the DOM,
 * running animations) runs only while something shows a sample, so the monitor costs nothing
 * when it is closed. Process CPU comes from the host instead (`loadPerformanceSample`).
 */
export type PerfSample = {
  /** False while the window is hidden: animation frames stop, so the frame figures mean nothing. */
  visible: boolean
  framesPerSecond: number
  /** Frames in the last second that took longer than `SLOW_FRAME_MS`. */
  slowFrames: number
  worstFrameMs: number
  outputBytesPerSecond: number
  outputFramesPerSecond: number
  flushesPerSecond: number
  /** How long a flushed write waited until xterm had parsed it, on average. */
  writeLatencyMs: number
  /** CSS and web animations running right now. Each one repaints every frame. */
  animations: number
  domNodes: number
  renderers: { webgl: number; dom: number }
}

export type TerminalRenderer = 'webgl' | 'dom'

const MS_PER_SECOND = 1000
export const SAMPLE_EVERY_MS = MS_PER_SECOND
/** A frame over this long is felt as a hitch (the RAIL long-task threshold). */
export const SLOW_FRAME_MS = 50

type Counters = { outputBytes: number; outputFrames: number; flushes: number; writeLatencyMs: number; writesTimed: number; frames: number; slowFrames: number; worstFrameMs: number }

const zeroCounters = (): Counters => ({ outputBytes: 0, outputFrames: 0, flushes: 0, writeLatencyMs: 0, writesTimed: 0, frames: 0, slowFrames: 0, worstFrameMs: 0 })

let counters = zeroCounters()
const renderers = new Map<object, TerminalRenderer>()
const listeners = new Set<() => void>()
let sample: PerfSample | null = null
let ticker = 0
let animationFrame = 0
let lastFrameAt: number | null = null

/** One output frame arrived from the host. */
export function countOutput(bytes: number): void {
  counters.outputBytes += bytes
  counters.outputFrames += 1
}

/** A view flushed a write into xterm; call the result once xterm has parsed it. */
export function beginFlush(): () => void {
  counters.flushes += 1
  const startedAt = performance.now()
  return () => {
    counters.writeLatencyMs += performance.now() - startedAt
    counters.writesTimed += 1
  }
}

/** Which renderer a view draws with; `key` identifies the view until `forgetRenderer`. */
export function reportRenderer(key: object, renderer: TerminalRenderer): void {
  renderers.set(key, renderer)
}

export function forgetRenderer(key: object): void {
  renderers.delete(key)
}

function countRenderers(): PerfSample['renderers'] {
  let webgl = 0
  let dom = 0
  for (const renderer of renderers.values()) {
    if (renderer === 'webgl') webgl += 1
    else dom += 1
  }
  return { webgl, dom }
}

function onAnimationFrame(now: number): void {
  if (lastFrameAt !== null) {
    const frameMs = now - lastFrameAt
    counters.frames += 1
    if (frameMs > SLOW_FRAME_MS) counters.slowFrames += 1
    if (frameMs > counters.worstFrameMs) counters.worstFrameMs = frameMs
  }
  lastFrameAt = now
  animationFrame = requestAnimationFrame(onAnimationFrame)
}

function takeSample(): void {
  const seconds = SAMPLE_EVERY_MS / MS_PER_SECOND
  const taken = counters
  counters = zeroCounters()
  sample = {
    visible: !document.hidden,
    framesPerSecond: Math.round(taken.frames / seconds),
    slowFrames: taken.slowFrames,
    worstFrameMs: Math.round(taken.worstFrameMs),
    outputBytesPerSecond: Math.round(taken.outputBytes / seconds),
    outputFramesPerSecond: Math.round(taken.outputFrames / seconds),
    flushesPerSecond: Math.round(taken.flushes / seconds),
    writeLatencyMs: taken.writesTimed === 0 ? 0 : Math.round(taken.writeLatencyMs / taken.writesTimed),
    animations: typeof document.getAnimations === 'function' ? document.getAnimations().length : 0,
    domNodes: document.getElementsByTagName('*').length,
    renderers: countRenderers(),
  }
  for (const listener of listeners) listener()
}

function start(): void {
  counters = zeroCounters()
  lastFrameAt = null
  animationFrame = requestAnimationFrame(onAnimationFrame)
  ticker = window.setInterval(takeSample, SAMPLE_EVERY_MS)
}

function stop(): void {
  cancelAnimationFrame(animationFrame)
  window.clearInterval(ticker)
  animationFrame = 0
  ticker = 0
  sample = null
}

function subscribe(listener: () => void): () => void {
  if (listeners.size === 0) start()
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
    if (listeners.size === 0) stop()
  }
}

function current(): PerfSample | null {
  return sample
}

/** The latest sample, a new one each second while something is mounted; null until the first. */
export function usePerfSample(): PerfSample | null {
  return useSyncExternalStore(subscribe, current, () => null)
}
