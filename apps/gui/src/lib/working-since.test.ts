import { describe, expect, it } from 'vitest'
import { formatElapsed } from './working-since'

describe('formatElapsed', () => {
  it('shows minutes and zero-padded seconds', () => {
    expect(formatElapsed(0)).toBe('0:00')
    expect(formatElapsed(9_000)).toBe('0:09')
    expect(formatElapsed(102_000)).toBe('1:42')
  })

  it('never shows a negative time if the clock moved backwards', () => {
    expect(formatElapsed(-5_000)).toBe('0:00')
  })
})
