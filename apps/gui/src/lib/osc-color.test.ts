import { describe, expect, it } from 'vitest'
import { oscColor } from './osc-color'

describe('osc colour', () => {
  it('doubles each channel to 16 bits, as xterm does', () => {
    expect(oscColor('#060d16')).toBe('rgb:0606/0d0d/1616')
    expect(oscColor('#FFFFFF')).toBe('rgb:ffff/ffff/ffff')
  })

  it('declines colours it cannot read, so xterm answers instead', () => {
    expect(oscColor('rgba(0,0,0,0)')).toBeNull()
    expect(oscColor('#fff')).toBeNull()
    expect(oscColor('')).toBeNull()
  })
})
