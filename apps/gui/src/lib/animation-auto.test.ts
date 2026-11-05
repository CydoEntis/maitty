import { describe, expect, it } from 'vitest'
import { afterMeasurement, describeAnimations, initialAuto, levelFor, type AutoState } from './animation-auto'
import type { FrameEvidence } from './frame-monitor'

const SLOW: FrameEvidence = { frames: 100, slowFrames: 40, worstFrameMs: 120 }
const FINE: FrameEvidence = { frames: 180, slowFrames: 2, worstFrameMs: 45 }

describe('Auto animation level', () => {
  it('starts at Waves with WebGL 2 and at Sweep without it', () => {
    expect(initialAuto('webgl').level).toBe('waves')
    expect(initialAuto('software').level).toBe('sweep')
  })

  it('steps down one level for each slow measurement, and stops at none', () => {
    let state = initialAuto('webgl')
    state = afterMeasurement(state, 'slow', 'start', SLOW)
    expect(state.level).toBe('sweep')
    state = afterMeasurement(state, 'slow', 'waves', SLOW)
    expect(state.level).toBe('none')
    expect(afterMeasurement(state, 'slow', 'waves', SLOW)).toBe(state)
  })

  it('never steps back up, however good the frames are', () => {
    const stepped = afterMeasurement(initialAuto('webgl'), 'slow', 'waves', SLOW)
    expect(afterMeasurement(stepped, 'ok', 'start', FINE)).toBe(stepped)
    expect(afterMeasurement(stepped, 'ok', 'waves', FINE).level).toBe('sweep')
  })

  it('leaves the state alone when the frames are fine', () => {
    const state = initialAuto('webgl')
    expect(afterMeasurement(state, 'ok', 'waves', FINE)).toBe(state)
  })

  it('lets a fixed choice ignore what Auto measured', () => {
    const stepped: AutoState = afterMeasurement(initialAuto('webgl'), 'slow', 'waves', SLOW)
    expect(levelFor('auto', stepped)).toBe('sweep')
    expect(levelFor('full', stepped)).toBe('waves')
    expect(levelFor('light', stepped)).toBe('sweep')
    expect(levelFor('off', stepped)).toBe('none')
  })
})

describe('animation description', () => {
  it('says why Auto chose Sweep on software rendering', () => {
    expect(describeAnimations('auto', initialAuto('software'), false)).toBe('Auto: Software rendering detected, using Sweep')
  })

  it('says Waves are in use with WebGL 2', () => {
    expect(describeAnimations('auto', initialAuto('webgl'), false)).toBe('Auto: WebGL 2 detected, using Waves')
  })

  it('reports the measurement behind a step down', () => {
    const stepped = afterMeasurement(initialAuto('webgl'), 'slow', 'waves', SLOW)
    expect(describeAnimations('auto', stepped, false)).toBe('Auto: frames ran slow with Waves running (40% over 40 ms, worst 120 ms), using Sweep')
    expect(describeAnimations('auto', afterMeasurement(initialAuto('software'), 'slow', 'start', SLOW), false)).toContain('after start')
  })

  it('names the fixed choices, and lets Reduce motion win over all of them', () => {
    const auto = initialAuto('webgl')
    expect(describeAnimations('full', auto, false)).toMatch(/^Full/)
    expect(describeAnimations('light', auto, false)).toMatch(/^Light/)
    expect(describeAnimations('off', auto, false)).toMatch(/^Off/)
    expect(describeAnimations('full', auto, true)).toMatch(/^Reduce motion/)
  })
})
