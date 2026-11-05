import { useEffect } from 'react'
import { recordMeasurement, type MeasuredPhase } from '@/lib/animation-auto'
import { startFrameMonitor } from '@/lib/frame-monitor'

/** Let start-up work (xterm, the first layout) finish before the frames are judged. */
const START_DELAY_MS = 2000
/** Let the wave layers mount and rasterise before the frames are judged. */
const WAVES_DELAY_MS = 500

/** Runs one measurement after `delayMs`, while `active`, and stops it if the effect ends first. */
function useMeasurement(active: boolean, phase: MeasuredPhase, delayMs: number) {
  useEffect(() => {
    if (!active) return
    let stop: (() => void) | undefined
    const timer = window.setTimeout(() => {
      stop = startFrameMonitor((verdict, evidence) => { recordMeasurement(verdict, phase, evidence) })
    }, delayMs)
    return () => {
      window.clearTimeout(timer)
      stop?.()
    }
  }, [active, phase, delayMs])
}

/**
 * Auto mode's frame checks: once shortly after start, and again each time Waves begin to run. Each
 * one samples a few seconds of animation frames and stops itself; a slow verdict steps the level
 * down in `animation-auto`. Nothing runs under a fixed choice, Reduce motion, or when there is
 * already nothing left to give up.
 */
export function useAdaptiveAnimations(measuring: boolean, wavesRunning: boolean) {
  useMeasurement(measuring, 'start', START_DELAY_MS)
  useMeasurement(measuring && wavesRunning, 'waves', WAVES_DELAY_MS)
}
