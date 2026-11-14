import { describe, expect, it } from 'vitest'
import { findTheme, themeStyle } from './themes'
import { CORNERS, cornersOfWindowStyle, DEFAULT_CORNERS, isCorners, withLook } from './corners'

describe('corners', () => {
  it('are square or rounded, square by default', () => {
    expect(CORNERS).toEqual(['square', 'rounded'])
    expect(DEFAULT_CORNERS).toBe('square')
    expect(isCorners('rounded')).toBe(true)
    expect(isCorners('glass')).toBe(false)
  })

  it('come from an old window style: the rounded styles stay rounded, the square ones square', () => {
    expect(['glass', 'console', 'flat', 'tiled', 'minimal', 'expressive', 'frosted'].map(cornersOfWindowStyle)).toEqual(Array(7).fill('rounded'))
    expect(['mono', 'pixel'].map(cornersOfWindowStyle)).toEqual(['square', 'square'])
    expect(cornersOfWindowStyle('neon')).toBeNull()
  })
})

describe('the one look', () => {
  it('keeps the theme its own colours and gives it Inter with JetBrains Mono', () => {
    for (const theme of [findTheme('Maitty Night'), findTheme('Maitty Day'), findTheme('Catppuccin Mocha')]) {
      const looked = withLook(theme)
      expect(looked.tokens).toEqual(theme.tokens)
      expect(themeStyle(looked)['--ui']).toContain('Inter')
      expect(themeStyle(looked)['--mono']).toContain('JetBrains Mono')
    }
  })

  it("lets a design theme's own shading win over the look's", () => {
    expect(themeStyle(withLook(findTheme('Paper')))['--sk']).toBe('.3')
    expect(themeStyle(withLook(findTheme('Catppuccin Mocha')))['--sk']).toBe('.6')
  })

  it('draws flat: no sheen and no background glow', () => {
    const style = themeStyle(withLook(findTheme('Maitty Night')))
    expect([style['--sh'], style['--glow-accent'], style['--glow-mag']]).toEqual(['none', '0%', '0%'])
  })
})
