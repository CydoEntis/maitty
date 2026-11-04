import { describe, expect, it } from 'vitest'
import { levelForGpu, stepDown, styleUnderCap } from './animation-level'

describe('animation levels', () => {
  it('steps down one level at a time and stops at none', () => {
    expect(stepDown('waves')).toBe('sweep')
    expect(stepDown('sweep')).toBe('none')
    expect(stepDown('none')).toBe('none')
  })

  it('starts software rendering at Sweep and WebGL at Waves', () => {
    expect(levelForGpu('software')).toBe('sweep')
    expect(levelForGpu('webgl')).toBe('waves')
  })

  it('cheapens a chosen style to what the level allows', () => {
    expect(styleUnderCap('waves', 'waves')).toBe('waves')
    expect(styleUnderCap('waves', 'sweep')).toBe('sweep')
    expect(styleUnderCap('sweep', 'sweep')).toBe('sweep')
    expect(styleUnderCap('sweep', 'waves')).toBe('sweep')
    expect(styleUnderCap('waves', 'none')).toBeNull()
  })
})
