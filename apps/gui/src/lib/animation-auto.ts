import { useSyncExternalStore } from 'react'
import { LEVEL_NAME, levelForGpu, stepDown, type AnimationLevel } from './animation-level'
import type { FrameEvidence } from './frame-monitor'
import { gpuKind, type GpuKind } from './gpu'

/** Settings › Appearance › Animations: Auto measures, the others fix the level. */
export type AnimationMode = 'auto' | 'full' | 'light' | 'off'

/** When a measurement ran: shortly after start, or once Waves began to run. */
export type MeasuredPhase = 'start' | 'waves'

export type AutoReason =
  | { kind: 'gpu'; gpu: GpuKind }
  | { kind: 'slow'; phase: MeasuredPhase; evidence: FrameEvidence }

/** What Auto chose, and why. It only ever moves down within a run: nothing here steps back up. */
export type AutoState = { level: AnimationLevel; reason: AutoReason }

export function initialAuto(gpu: GpuKind): AutoState {
  return { level: levelForGpu(gpu), reason: { kind: 'gpu', gpu } }
}

/** One measurement's outcome: slow frames cost a level, good frames change nothing. */
export function afterMeasurement(state: AutoState, verdict: 'ok' | 'slow', phase: MeasuredPhase, evidence: FrameEvidence): AutoState {
  if (verdict === 'ok' || state.level === 'none') return state
  return { level: stepDown(state.level), reason: { kind: 'slow', phase, evidence } }
}

/** The richest level the choice allows. Auto uses what it measured; the rest are fixed. */
export function levelFor(mode: AnimationMode, auto: AutoState): AnimationLevel {
  if (mode === 'full') return 'waves'
  if (mode === 'light') return 'sweep'
  if (mode === 'off') return 'none'
  return auto.level
}

const PERCENT = 100

/** One sentence for Settings › Performance: what is allowed and why. Reduce motion is named first because it wins. */
export function describeAnimations(mode: AnimationMode, auto: AutoState, reduceMotion: boolean): string {
  if (reduceMotion) return 'Reduce motion is on, so there are no status effects'
  if (mode === 'full') return 'Full: Waves are always allowed, whatever the frame rate'
  if (mode === 'light') return 'Light: Sweep only'
  if (mode === 'off') return 'Off: no status effects'
  const using = auto.level === 'none' ? 'no status effects' : `using ${LEVEL_NAME[auto.level]}`
  if (auto.reason.kind === 'gpu') {
    return auto.reason.gpu === 'webgl' ? `Auto: WebGL 2 detected, ${using}` : `Auto: Software rendering detected, ${using}`
  }
  const { evidence, phase } = auto.reason
  const share = Math.round((evidence.slowFrames / Math.max(evidence.frames, 1)) * PERCENT)
  const when = phase === 'waves' ? 'with Waves running' : 'after start'
  return `Auto: frames ran slow ${when} (${String(share)}% over 40 ms, worst ${String(evidence.worstFrameMs)} ms), ${using}`
}

let current: AutoState | null = null
const listeners = new Set<() => void>()

/** Asked lazily: the first answer needs the web view's WebGL check. */
export function autoState(): AutoState {
  current ??= initialAuto(gpuKind())
  return current
}

export function recordMeasurement(verdict: 'ok' | 'slow', phase: MeasuredPhase, evidence: FrameEvidence): void {
  const next = afterMeasurement(autoState(), verdict, phase, evidence)
  if (next === current) return
  current = next
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

export function useAutoState(): AutoState {
  return useSyncExternalStore(subscribe, autoState)
}

/** For tests: forget a step-down. The app never does this. */
export function resetAutoState(): void {
  current = null
}
