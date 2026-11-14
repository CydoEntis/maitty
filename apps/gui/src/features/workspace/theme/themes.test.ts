import { describe, expect, it } from 'vitest'
import { contrastRatio, mixOklab, parseColor } from './color'
import themeManifest from '@tokens/themes.json'
import type { ThemeEntry } from './themes'
import { findTheme, generateTheme, MIN_TEXT_CONTRAST, paletteTheme, terminalMinimumContrast, themeStyle, THEMES, xtermTheme } from './themes'

const PALETTES = new Map(themeManifest.themes.flatMap((theme) => ('palette' in theme ? [[theme.name, theme.palette] as const] : [])))
const MAITTY = ['Maitty Night', 'Maitty Day', 'Maitty Dark', 'Maitty Light', 'Abyss', 'Ember', 'Moss', 'Dusk', 'Paper', 'Maitty Retro Dark', 'Maitty Retro Light']

describe('themes', () => {
  it("ships fifty-five themes, twenty-six light: Maitty's eleven, then twenty-two families with a dark and a light each", () => {
    expect(THEMES).toHaveLength(55)
    expect(THEMES.filter((theme) => theme.light)).toHaveLength(26)
    expect(THEMES.slice(0, MAITTY.length).map((theme) => theme.name)).toEqual(MAITTY)
    expect(findTheme('Catppuccin Mocha').family).toBe('Catppuccin · dark')
    expect(findTheme('Maitty Retro Light').family).toBe('Maitty · light')
    expect(findTheme('Paper').family).toBe('Maitty · light')
    // Every family after Maitty's own has exactly one dark and one light.
    const sides = new Map<string, boolean[]>()
    for (const theme of THEMES.slice(MAITTY.length)) sides.set(theme.family.split(' · ')[0] ?? '', [...(sides.get(theme.family.split(' · ')[0] ?? '') ?? []), theme.light])
    expect(sides.size).toBe(22)
    for (const [family, lights] of sides) expect([family, [...lights].sort()]).toEqual([family, [false, true]])
    expect(new Set(THEMES.map((theme) => theme.id)).size).toBe(55)
    expect(findTheme('Gruvbox Light').light).toBe(true)
    expect(findTheme('No such theme').name).toBe('Maitty Night')
  })

  it("draws the design's nine themes exactly as written, shading included", () => {
    expect([...PALETTES.keys()]).toEqual(MAITTY.slice(0, 9))
    for (const [name, palette] of PALETTES) {
      const theme = findTheme(name)
      expect(theme.tokens, name).toEqual(paletteTheme(palette))
      const style = themeStyle(theme)
      expect([style['--hk'], style['--sk']], name).toEqual([palette.hk, palette.sk])
    }
    const night = themeStyle(findTheme('Maitty Night'))
    expect(night['--bg']).toBe('#071318')
    expect(night['--main']).toBe('#0f2026')
    expect(night['--pane']).toBe(night['--main'])
    expect(night['--accent']).toBe('#d39a2a')
    expect(night['--accent-text']).toBe('#e0ab45')
    expect(night['--working']).toBe('#3dd6c6')
    expect(night['--warn']).toBe('#ff7a5c')
    expect(night['--m-or']).toBe('#f0964a')
    const day = themeStyle(findTheme('Maitty Day'))
    expect(day['--bg']).toBe('#e9f0ed')
    expect(day['--accent-text']).toBe('#8a5a00')
    expect(day['--warn']).toBe('#9e2e12')
    // One violet for every branch name, deeper on a light ground.
    expect(night['--branch']).toBe('#A78BFA')
    expect(day['--branch']).toBe('#6B46C1')
  })

  it('gives every theme an opaque pane that is its main surface', () => {
    for (const theme of THEMES) {
      expect(theme.tokens.pane, theme.name).toBe(theme.tokens.main)
      expect(parseColor(theme.tokens.pane).a, theme.name).toBe(1)
    }
  })

  it('never changes what amber and red mean within a kind of theme', () => {
    // The design's themes set their own amber and red; every theme built from six inputs shares one of each.
    const generated = THEMES.filter((candidate) => !PALETTES.has(candidate.name))
    for (const theme of generated) {
      expect(theme.tokens.warn).toBe(theme.light ? '#8F5400' : '#F2B65A')
      const danger = theme.light ? '#B8362C' : '#F07F78'
      if (contrastRatio(danger, theme.tokens.term) >= MIN_TEXT_CONTRAST && contrastRatio(danger, theme.tokens.bg) >= MIN_TEXT_CONTRAST) {
        expect(theme.tokens.danger, theme.name).toBe(danger)
      } else {
        // A ground the red falls short on (Tokyo Night Day's window) moves it toward white until it reads, and it stays red.
        const { r, g, b } = parseColor(theme.tokens.danger)
        expect(r, theme.name).toBeGreaterThan(g)
        expect(r, theme.name).toBeGreaterThan(b)
      }
    }
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
    const { tokens } = findTheme('Maitty Night')
    const palette = xtermTheme(tokens)
    expect(palette.cyan).toBe('#e0ab45')
    expect(palette.magenta).toBe('#C3A6F2')
    expect(palette.blue).toBe('#82AAFF')
    expect(palette.green).toBe('#7fd99a')
    expect(palette.yellow).toBe('#ff7a5c')
    expect(palette.red).toBe('#F07F78')
    expect(palette.brightBlack).toBe('#8fa49e')
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
const STATUS_TEXT_SLOTS = ['accentText', 'working', 'warn', 'danger', 'ok'] as const
const ANSI_TEXT_SLOTS = ['ansiBlue', 'ansiMag', 'ansiGreen'] as const
// Black is left out: it is the ground on purpose, so programs that paint with it vanish into the background.
const ANSI_TEXT_COLOURS = [
  'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
  'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite',
] as const

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

  it.each(MAITTY.slice(0, 9))('%s keeps accent, status and ANSI text readable on --term and --bg', (name) => {
    const { tokens } = findTheme(name)
    for (const [ground, colour] of TEXT_GROUNDS(tokens)) {
      for (const slot of [...STATUS_TEXT_SLOTS, ...ANSI_TEXT_SLOTS]) {
        expect(contrastRatio(tokens[slot], colour), `${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
      }
    }
  })

  it('keeps accent, status and ANSI text readable on --term and --bg in every light theme', () => {
    for (const theme of THEMES.filter((candidate) => candidate.light)) {
      for (const [ground, colour] of TEXT_GROUNDS(theme.tokens)) {
        for (const slot of [...STATUS_TEXT_SLOTS, ...ANSI_TEXT_SLOTS]) {
          expect(contrastRatio(theme.tokens[slot], colour), `${theme.name} ${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
        }
      }
    }
  })

  it('keeps every ANSI colour but black readable on --term and --bg in every light theme', () => {
    for (const theme of THEMES.filter((candidate) => candidate.light)) {
      const palette = xtermTheme(theme.tokens)
      for (const slot of ANSI_TEXT_COLOURS) {
        for (const [ground, colour] of TEXT_GROUNDS(theme.tokens)) {
          expect(contrastRatio(palette[slot] ?? '', colour), `${theme.name} ${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
        }
      }
    }
  })

  it('keeps accent, status, ANSI slots and every ANSI colour but black readable on --term and --bg in every dark theme', () => {
    for (const theme of THEMES.filter((candidate) => !candidate.light)) {
      const palette = xtermTheme(theme.tokens)
      for (const [ground, colour] of TEXT_GROUNDS(theme.tokens)) {
        for (const slot of [...STATUS_TEXT_SLOTS, ...ANSI_TEXT_SLOTS]) {
          expect(contrastRatio(theme.tokens[slot], colour), `${theme.name} ${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
        }
        for (const slot of ANSI_TEXT_COLOURS) {
          expect(contrastRatio(palette[slot] ?? '', colour), `${theme.name} ${slot} on ${ground}`).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
        }
      }
    }
  })

  it('darkens a pastel light hue without leaving its family (Solarized Light cyan stays teal)', () => {
    const { tokens } = findTheme('Solarized Light')
    const { r, g, b } = parseColor(tokens.accent)
    expect(g).toBeGreaterThan(r)
    expect(b).toBeGreaterThan(r)
  })

  it('asks xterm to lift the colours a program picks itself on light themes only', () => {
    expect(terminalMinimumContrast(findTheme('Maitty Day'))).toBe(MIN_TEXT_CONTRAST)
    expect(terminalMinimumContrast(findTheme('Maitty Night'))).toBe(1)
  })

  it('raises a generated theme\'s derived text until it passes (Tokyo Night Day foreground and dim)', () => {
    const { tokens } = findTheme('Tokyo Night Day')
    expect(contrastRatio(tokens.fg, tokens.bg)).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
    expect(contrastRatio(tokens.dim, tokens.term)).toBeGreaterThanOrEqual(MIN_TEXT_CONTRAST)
  })
})
