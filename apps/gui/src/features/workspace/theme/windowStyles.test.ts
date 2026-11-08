import { describe, expect, it } from 'vitest'
import { findTheme, themeStyle } from './themes'
import { finishOf, WINDOW_STYLES, WINDOW_STYLE_IDS, withWindowStyle } from './windowStyles'

const MIDNIGHT = findTheme('Midnight')
const DAY = findTheme('Day')

describe('window styles', () => {
  it('defines the five styles, each with a dark and a light side', () => {
    expect(WINDOW_STYLES.map((style) => style.id)).toEqual([...WINDOW_STYLE_IDS])
    expect(WINDOW_STYLE_IDS).toEqual(['glass', 'frosted', 'mono', 'console', 'pixel'])
  })

  it('leaves Midnight and Day exactly as they are in Glass, apart from the fonts', () => {
    expect(withWindowStyle(MIDNIGHT, 'glass').tokens).toEqual(MIDNIGHT.tokens)
    expect(withWindowStyle(DAY, 'glass').tokens).toEqual(DAY.tokens)
  })

  it('swaps only tokens: every style fills the same set of CSS variables, in both themes', () => {
    const names = (theme: typeof MIDNIGHT, id: (typeof WINDOW_STYLE_IDS)[number]) => Object.keys(themeStyle(withWindowStyle(theme, id))).filter((key) => key !== '--sh' && key !== '--glow-accent' && key !== '--glow-mag')
    for (const theme of [MIDNIGHT, DAY]) {
      const glass = names(theme, 'glass').sort()
      for (const id of WINDOW_STYLE_IDS) expect(names(theme, id).sort()).toEqual(glass)
    }
  })

  it("takes the design's values: Frosted is rose, Mono Type greyscale, Retro Pixel green on navy", () => {
    expect(withWindowStyle(MIDNIGHT, 'frosted').tokens.accent).toBe('#E89AB4')
    expect(withWindowStyle(DAY, 'frosted').tokens.accent).toBe('#C2386B')
    expect(withWindowStyle(MIDNIGHT, 'mono').tokens.accent).toBe('#F0F0F0')
    expect(withWindowStyle(MIDNIGHT, 'pixel').tokens).toMatchObject({ bg: '#1A1C2C', accent: '#A7F070', warn: '#FFEC27' })
    expect(withWindowStyle(DAY, 'pixel').tokens).toMatchObject({ bg: '#E4E6EE', accent: '#2E8B1E', danger: '#D0003C' })
  })

  it('sets the fonts and the glass dials of a style', () => {
    const mono = themeStyle(withWindowStyle(MIDNIGHT, 'mono'))
    expect(mono['--ui']).toContain('Space Mono')
    expect(mono['--hk']).toBe('.4')
    expect(mono['--sk']).toBe('.5')
    expect(themeStyle(withWindowStyle(DAY, 'mono'))['--hk']).toBe('.55')
    expect(themeStyle(withWindowStyle(MIDNIGHT, 'console'))['--mono']).toContain('JetBrains Mono')
    expect(themeStyle(withWindowStyle(MIDNIGHT, 'glass'))['--hk']).toBe('1')
  })

  it("rings a status badge with the style's own card colour", () => {
    expect(withWindowStyle(MIDNIGHT, 'pixel').cardRing).toBe('#222538')
  })

  it("keeps another theme's own colours and gives it only the style's fonts, dials and shapes", () => {
    const dracula = findTheme('Dracula')
    const styled = withWindowStyle(dracula, 'pixel')
    expect(styled.tokens).toEqual(dracula.tokens)
    expect(themeStyle(styled)['--ui']).toContain('Pixelify')
  })

  it('builds Glass and Frosted on the glass finish and the rest on the flat one', () => {
    expect(WINDOW_STYLE_IDS.map(finishOf)).toEqual(['glass', 'glass', 'flat', 'flat', 'flat'])
  })
})
