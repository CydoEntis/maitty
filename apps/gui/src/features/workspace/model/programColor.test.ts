import { describe, expect, it } from 'vitest'
import { programColor } from './programColor'

describe('program colour', () => {
  it('gives the same program the same colour every time', () => {
    expect(programColor('aider')).toEqual(programColor('aider'))
  })

  it('spreads different programs over several colours', () => {
    const names = ['aider', 'amp', 'goose', 'cline', 'crush', 'droid', 'kiro', 'qwen', 'zed', 'cursor', 'warp', 'plandex']
    expect(new Set(names.map((name) => programColor(name).from)).size).toBeGreaterThan(3)
  })

  it('takes every colour from the theme, so a theme switch recolours the icons', () => {
    for (let index = 0; index < 40; index += 1) expect(programColor(`tool-${String(index)}`).from).not.toMatch(/#[0-9a-fA-F]{3,8}/)
  })

  it('always returns a light and a dark end', () => {
    const color = programColor('')
    expect(color.from).toMatch(/^(var|color-mix)\(/)
    expect(color.to).toMatch(/^color-mix\(/)
  })

  it('never uses amber, which only ever means needs you', () => {
    for (let index = 0; index < 60; index += 1) expect(programColor(`tool-${String(index)}`).from).not.toMatch(/--warn/)
  })
})
