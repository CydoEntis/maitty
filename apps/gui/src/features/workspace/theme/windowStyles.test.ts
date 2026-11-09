import { describe, expect, it } from 'vitest'
import { findTheme, themeStyle } from './themes'
import { finishOf, WINDOW_STYLES, WINDOW_STYLE_IDS, withWindowStyle } from './windowStyles'

const MIDNIGHT = findTheme('Maitty Night')
const DAY = findTheme('Maitty Day')

describe('window styles', () => {
  it('defines the four styles, each with a dark and a light side', () => {
    expect(WINDOW_STYLES.map((style) => style.id)).toEqual([...WINDOW_STYLE_IDS])
    expect(WINDOW_STYLE_IDS).toEqual(['glass', 'mono', 'console', 'pixel'])
  })

  it('swaps only tokens: every style fills the same set of CSS variables, in both themes', () => {
    const names = (theme: typeof MIDNIGHT, id: (typeof WINDOW_STYLE_IDS)[number]) => Object.keys(themeStyle(withWindowStyle(theme, id))).filter((key) => key !== '--sh' && key !== '--glow-accent' && key !== '--glow-mag')
    for (const theme of [MIDNIGHT, DAY]) {
      const glass = names(theme, 'glass').sort()
      for (const id of WINDOW_STYLE_IDS) expect(names(theme, id).sort()).toEqual(glass)
    }
  })

  it('is shape only: whatever the style, the theme keeps its own colours', () => {
    for (const theme of [MIDNIGHT, DAY, findTheme('Maitty Dark'), findTheme('Nord')]) {
      for (const id of WINDOW_STYLE_IDS) expect(withWindowStyle(theme, id).tokens, `${theme.name} in ${id}`).toEqual(theme.tokens)
    }
    expect(withWindowStyle(MIDNIGHT, 'pixel').tokens.accent).toBe('#e3a72f')
    expect(withWindowStyle(DAY, 'console').tokens.bg).toBe('#e9f0ed')
  })

  it('sets the fonts and the glass dials of a style', () => {
    const mono = themeStyle(withWindowStyle(MIDNIGHT, 'mono'))
    expect(mono['--ui']).toContain('Space Mono')
    expect(mono['--hk']).toBe('.4')
    expect(mono['--sk']).toBe('.5')
    expect(themeStyle(withWindowStyle(DAY, 'mono'))['--hk']).toBe('.55')
    expect(themeStyle(withWindowStyle(MIDNIGHT, 'console'))['--mono']).toContain('JetBrains Mono')
    expect(themeStyle(withWindowStyle(MIDNIGHT, 'glass'))['--hk']).toBe('.7')
  })

  it("keeps another theme's own colours and gives it only the style's fonts, dials and shapes", () => {
    const dracula = findTheme('Dracula')
    const styled = withWindowStyle(dracula, 'pixel')
    expect(styled.tokens).toEqual(dracula.tokens)
    expect(themeStyle(styled)['--ui']).toContain('Pixelify')
  })

  it('builds Glass on the glass finish and the rest on the flat one', () => {
    expect(WINDOW_STYLE_IDS.map(finishOf)).toEqual(['glass', 'flat', 'flat', 'flat'])
  })
})
