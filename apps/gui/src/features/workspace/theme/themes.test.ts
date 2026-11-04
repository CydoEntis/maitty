import { describe, expect, it } from 'vitest'
import { contrastRatio, mixOklab, parseColor } from './color'
import { findTheme, generateTheme, MIN_TEXT_CONTRAST, paneSurface, themeStyle, THEMES, xtermTheme } from './themes'

describe('themes', () => {
  it('ships Midnight, Abyss and Paper, the 23 generated themes from the design and High Contrast', () => {
    expect(THEMES).toHaveLength(27)
    expect(THEMES.slice(0, 3).map((theme) => theme.name)).toEqual(['Midnight', 'Abyss', 'Paper'])
    expect(findTheme('Nord').family).toBe('Arctic · dark')
    expect(findTheme('High Contrast')).toMatchObject({ family: 'Accessibility · dark', light: false })
    expect(findTheme('Gruvbox Light').light).toBe(true)
    expect(findTheme('No such theme').name).toBe('Midnight')
  })

  it('keeps the Midnight values exactly as designed', () => {
    const style = themeStyle(findTheme('Midnight'))
    expect(style['--bg']).toBe('#090C10')
    expect(style['--pane']).toBe('rgba(14,18,24,.86)')
    expect(style['--accent']).toBe('#5FD4BF')
    expect(style['--accent-soft']).toBe('rgba(95,212,191,.11)')
    expect(style['--muted']).toBe('#8C96A1')
    expect(style['--dim']).toBe('#75808B')
    expect(style['--card-ring']).toBe('#0E1218')
  })

  it.each(THEMES.map((theme) => [theme.name, theme] as const))('%s keeps muted and dim text readable on its pane', (_name, theme) => {
    const pane = paneSurface(theme.tokens)
    expect(contrastRatio(theme.tokens.muted, pane)).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
    expect(contrastRatio(theme.tokens.dim, pane)).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
  })

  it('never changes what amber and red mean', () => {
    for (const theme of THEMES.filter((candidate) => candidate.name !== 'High Contrast')) {
      expect(theme.tokens.warn).toBe(theme.light ? '#9A5B00' : '#F2B65A')
      expect(theme.tokens.danger).toBe(theme.light ? '#C0392F' : '#F07F78')
    }
  })

  it('makes High Contrast pure black and white with strong lines, and still amber and red', () => {
    const { tokens } = findTheme('High Contrast')
    expect([tokens.bg, tokens.fg, tokens.line, tokens.line2]).toEqual(['#000000', '#FFFFFF', '#5A5A5A', '#8A8A8A'])
    expect([tokens.accent, tokens.warn, tokens.danger, tokens.ok]).toEqual(['#3CF2D2', '#FFC94D', '#FF7A7A', '#7CF29A'])
  })

  it('derives tokens from six inputs the way color-mix in oklab does', () => {
    const tokens = generateTheme({ bg: '#282A36', fg: '#F8F8F2', accent: '#8BE9FD', mag: '#FF79C6', blue: '#BD93F9', green: '#50FA7B', light: false })
    expect(tokens.main).toBe('#282A36')
    expect(tokens.term).toBe('#282A36')
    expect(tokens.sel).toBe(mixOklab('#282A36', '#F8F8F2', 10))
    expect(tokens.pane).toBe('rgba(40,42,54,0.9)')
    expect(tokens.accentSoft).toBe('rgba(139,233,253,0.13)')
    expect(tokens.onAccent).toBe('#282A36')
    expect(tokens.ok).toBe('#50FA7B')
  })

  it('mixes halfway between black and white to a mid grey in oklab', () => {
    const grey = parseColor(mixOklab('#000000', '#FFFFFF', 50))
    expect(grey.r).toBeCloseTo(grey.g, 3)
    expect(grey.r).toBeGreaterThan(0.3)
    expect(grey.r).toBeLessThan(0.5)
  })

  it('maps the ANSI palette for xterm from the theme tokens', () => {
    const { tokens } = findTheme('Midnight')
    const palette = xtermTheme(tokens)
    expect(palette.cyan).toBe('#5FD4BF')
    expect(palette.magenta).toBe('#C3A6F2')
    expect(palette.blue).toBe('#82AAFF')
    expect(palette.green).toBe('#9FD39A')
    expect(palette.yellow).toBe('#F2B65A')
    expect(palette.red).toBe('#F07F78')
    expect(palette.brightBlack).toBe('#75808B')
  })
})

describe('terminal palette', () => {
  it('selects with an opaque tint of the accent and keeps the text readable', () => {
    for (const theme of THEMES) {
      const palette = xtermTheme(theme.tokens)
      const selection = parseColor(palette.selectionBackground ?? '')
      expect(selection.a).toBe(1)
      expect(contrastRatio(palette.selectionForeground ?? '', palette.selectionBackground ?? ''), theme.name).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
    }
  })
})
