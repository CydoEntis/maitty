import { describe, expect, it } from 'vitest'
import { contrastRatio, mixOklab, parseColor } from './color'
import type { ThemeEntry } from './themes'
import { findTheme, generateTheme, MIN_TEXT_CONTRAST, themeStyle, THEMES, xtermTheme } from './themes'

describe('themes', () => {
  it('ships Midnight, Abyss, Paper and Day, the 23 generated themes from the design and High Contrast', () => {
    expect(THEMES).toHaveLength(28)
    expect(THEMES.slice(0, 4).map((theme) => theme.name)).toEqual(['Midnight', 'Abyss', 'Paper', 'Day'])
    expect(findTheme('Nord').family).toBe('Arctic · dark')
    expect(findTheme('High Contrast')).toMatchObject({ family: 'Accessibility · dark', light: false })
    expect(findTheme('Gruvbox Light').light).toBe(true)
    expect(findTheme('No such theme').name).toBe('Midnight')
  })

  it('keeps the Midnight values exactly as designed', () => {
    const style = themeStyle(findTheme('Midnight'))
    expect(style['--bg']).toBe('#090C10')
    expect(style['--pane']).toBe(style['--main'])
    expect(style['--accent']).toBe('#5FD4BF')
    expect(style['--accent-soft']).toBe('rgba(95,212,191,.11)')
    expect(style['--muted']).toBe('#8C96A1')
    expect(style['--dim']).toBe('#75808B')
    expect(style['--card-ring']).toBe('#0D1116')
  })

  it('gives every theme an opaque pane that is its main surface', () => {
    for (const theme of THEMES) {
      expect(theme.tokens.pane, theme.name).toBe(theme.tokens.main)
      expect(parseColor(theme.tokens.pane).a, theme.name).toBe(1)
    }
  })

  it('never changes what amber and red mean within a kind of theme', () => {
    const generated = THEMES.filter((candidate) => !['High Contrast', 'Paper', 'Day'].includes(candidate.name))
    for (const theme of generated) {
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
    expect(tokens.pane).toBe('#282A36')
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

/** What the design's contrast table (GLASS_AND_TILED.md §3) measures: text on the terminal ground and on the window ground. */
const TEXT_GROUNDS = (tokens: ThemeEntry['tokens']) => [['--term', tokens.term], ['--bg', tokens.bg]] as const
const TERMINAL_TEXT_SLOTS = ['fg', 'fg2', 'muted', 'dim'] as const
const STATUS_TEXT_SLOTS = ['accent', 'warn', 'danger', 'ok'] as const
const ANSI_TEXT_SLOTS = ['ansiBlue', 'ansiMag', 'ansiGreen'] as const

describe('contrast audit', () => {
  it.each(THEMES.map((theme) => [theme.name, theme] as const))('%s keeps text, terminal text and dim text readable on --term and --bg', (_name, theme) => {
    for (const [ground, colour] of TEXT_GROUNDS(theme.tokens)) {
      for (const slot of TERMINAL_TEXT_SLOTS) {
        expect(contrastRatio(theme.tokens[slot], colour), `${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
      }
      const palette = xtermTheme(theme.tokens)
      expect(contrastRatio(palette.foreground ?? '', colour), `terminal foreground on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
      expect(contrastRatio(palette.brightBlack ?? '', colour), `terminal dim text on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
    }
  })

  it.each(['Midnight', 'Paper', 'Day'])('%s keeps accent, status and ANSI text readable on --term and --bg', (name) => {
    const { tokens } = findTheme(name)
    for (const [ground, colour] of TEXT_GROUNDS(tokens)) {
      for (const slot of [...STATUS_TEXT_SLOTS, ...ANSI_TEXT_SLOTS]) {
        expect(contrastRatio(tokens[slot], colour), `${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
      }
    }
  })

  it('raises a generated theme\'s derived text until it passes (Monokai dim)', () => {
    const { tokens } = findTheme('Monokai')
    expect(contrastRatio(tokens.dim, tokens.term)).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
  })
})
