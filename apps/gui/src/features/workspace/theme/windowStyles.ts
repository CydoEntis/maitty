import { composite } from './color'
import type { StyleLook, ThemeEntry, ThemeTokens } from './themes'

/**
 * The four window styles of the desktop. Glass is the redesign's Frosted look (the redesign's own opaque Glass was dropped). A style is data: a set of token overrides for the
 * dark (Midnight) and light (Day) theme, the fonts, and the two glass dials. No component branches on
 * the style; the shape rules keyed by `[data-style]` live in `material.css`, and the component tokens
 * (`--c-*`) each style sets live there too. Values are copied from the design's `STYLES` object.
 */
export const WINDOW_STYLE_IDS = ['glass', 'mono', 'console', 'pixel'] as const
export type WindowStyle = (typeof WINDOW_STYLE_IDS)[number]
export const DEFAULT_WINDOW_STYLE: WindowStyle = 'glass'

export const isWindowStyle = (value: unknown): value is WindowStyle => WINDOW_STYLE_IDS.some((id) => id === value)

/**
 * The themes a style's palette replaces: Maitty Night and Maitty Day. Glass has no palette of its own: it wears Maitty's.
 * Any other theme keeps its own colours and takes only the style's fonts, dials and shapes.
 */
const PALETTE_THEMES: readonly string[] = ['Maitty Night', 'Maitty Day']

type Palette = Partial<ThemeTokens>

/** One side (dark or light) of a style: palette overrides and the look that goes with them. */
type Side = { palette: Palette; look: StyleLook }

export type WindowStyleEntry = {
  id: WindowStyle
  name: string
  blurb: string
  /** What the picker's mini preview paints: panel ground, accent, line (dark, then light). */
  preview: { dark: readonly [string, string, string]; light: readonly [string, string, string] }
  /** The interface font, for the picker's name. */
  font: string
  dark: Side
  light: Side
}

const SPACE_MONO = "'Space Mono',monospace"
const INTER = "'Inter Variable','Inter',system-ui,sans-serif"
const JETBRAINS_MONO = "'JetBrains Mono Variable','JetBrains Mono',monospace"
const PLEX_SANS = "'IBM Plex Sans',system-ui,sans-serif"
const PIXELIFY = "'Pixelify Sans Variable','Pixelify Sans',system-ui,sans-serif"

const NO_SHEEN = 'none'
// Light themes dim the glass dials of the flat styles by the same amounts.
const LIGHT_FLAT = { hk: '.55', sk: '.25', sh: NO_SHEEN }

export const WINDOW_STYLES: readonly WindowStyleEntry[] = [
  {
    id: 'glass', name: 'Glass', blurb: 'Soft sea-glass panels with brass. Maitty’s own look.',
    preview: { dark: ['#0A2329', '#E3A72F', '#17383C'], light: ['#F8FBFA', '#E3A72F', '#B9CCC5'] },
    font: INTER,
    dark: {
      look: { ui: INTER, mono: JETBRAINS_MONO, hk: '.7', sk: '1', glowAccent: '16%', glowMag: '11%' },
      palette: {},
    },
    light: {
      look: { ui: INTER, mono: JETBRAINS_MONO, hk: '1', sk: '.35', sh: NO_SHEEN, glowAccent: '14%', glowMag: '10%' },
      palette: {},
    },
  },
  {
    id: 'mono', name: 'Mono Type', blurb: 'Every glyph the same width. Black, white, greys.',
    preview: { dark: ['#161616', '#F0F0F0', '#2E2E2E'], light: ['#FFFFFF', '#111111', '#CCCCCC'] },
    font: SPACE_MONO,
    dark: {
      look: { ui: SPACE_MONO, mono: SPACE_MONO, hk: '.4', sk: '.5', glowAccent: '0%', glowMag: '0%', sh: NO_SHEEN },
      palette: {
        bg: '#0F0F0F', main: '#151515', raise: '#1C1C1C', hover: '#1A1A1A', sel: '#222222', line: '#262626', line2: '#333333', well: '#0C0C0C', term: '#121212',
        pane: '#151515', paneLine: '#2A2A2A', fg: '#F0F0F0', fg2: '#C4C4C4', muted: '#949494', dim: '#7C7C7C', accent: '#F0F0F0',
        accentSoft: 'rgba(240,240,240,.08)', accentLine: 'rgba(240,240,240,.45)', onAccent: '#111111', ok: '#BDBDBD',
      },
    },
    light: {
      look: { ui: SPACE_MONO, mono: SPACE_MONO, ...LIGHT_FLAT },
      palette: {
        bg: '#F0F0F0', main: '#FFFFFF', raise: '#F6F6F6', hover: '#F1F1F1', sel: '#E8E8E8', line: '#DEDEDE', line2: '#CCCCCC', well: '#ECECEC', term: '#FFFFFF',
        pane: '#FFFFFF', paneLine: '#D4D4D4', fg: '#111111', fg2: '#333333', muted: '#555555', dim: '#666666', accent: '#111111',
        accentSoft: 'rgba(17,17,17,.06)', accentLine: 'rgba(17,17,17,.4)', onAccent: '#FFFFFF', ok: '#444444',
      },
    },
  },
  {
    id: 'console', name: 'Console', blurb: 'IDE panels on true black. Plex Sans with JetBrains Mono.',
    preview: { dark: ['#121212', '#5FD4BF', '#2A2A2A'], light: ['#FAFAFA', '#0A6F86', '#CACACA'] },
    font: PLEX_SANS,
    dark: {
      look: { ui: PLEX_SANS, mono: JETBRAINS_MONO, hk: '.45', sk: '.7', glowAccent: '6%', glowMag: '0%', sh: NO_SHEEN },
      palette: {
        bg: '#080808', main: '#121212', raise: '#1A1A1A', hover: '#171717', sel: '#1F1F1F', line: '#232323', line2: '#2E2E2E', well: '#0B0B0B', term: '#0D0D0D',
        pane: '#121212', paneLine: '#2A2A2A', fg: '#E6E6E6', fg2: '#B8B8B8', muted: '#8A8A8A', dim: '#767676',
      },
    },
    light: {
      look: { ui: PLEX_SANS, mono: JETBRAINS_MONO, ...LIGHT_FLAT },
      palette: {
        bg: '#EBEBEB', main: '#FAFAFA', raise: '#F2F2F2', hover: '#EFEFEF', sel: '#E5E5E5', line: '#DADADA', line2: '#CACACA', well: '#E8E8E8', term: '#FFFFFF',
        pane: '#FAFAFA', paneLine: '#D2D2D2', fg: '#141414', fg2: '#383838', muted: '#585858', dim: '#666666',
      },
    },
  },
  {
    id: 'pixel', name: 'Retro Pixel', blurb: '8-bit console. Hard corners, chunky borders, scanlines.',
    preview: { dark: ['#222538', '#A7F070', '#566C86'], light: ['#FFFFFF', '#2E8B1E', '#8F97AE'] },
    font: PIXELIFY,
    dark: {
      look: { ui: PIXELIFY, mono: SPACE_MONO, hk: '.5', sk: '1', glowAccent: '0%', glowMag: '0%', sh: NO_SHEEN },
      palette: {
        bg: '#1A1C2C', main: '#222538', raise: '#2B2F47', hover: '#292D44', sel: '#333C57', line: '#3A4462', line2: '#566C86', well: '#161827', term: '#1C1E30',
        pane: '#222538', paneLine: '#566C86', fg: '#E0F8CF', fg2: '#C4DDB4', muted: '#94A98A', dim: '#7F92A8', accent: '#A7F070',
        accentSoft: 'rgba(167,240,112,.12)', accentLine: '#A7F070', onAccent: '#1A1C2C', warn: '#FFEC27', onWarn: '#1A1C2C', danger: '#FF004D', ok: '#A7F070',
        ansiBlue: '#29ADFF', ansiMag: '#EF7D57', ansiGreen: '#A7F070',
      },
    },
    light: {
      look: { ui: PIXELIFY, mono: SPACE_MONO, ...LIGHT_FLAT, sk: '.4' },
      palette: {
        bg: '#E4E6EE', main: '#FFFFFF', raise: '#F2F3F7', hover: '#F0F1F6', sel: '#E6E8F0', line: '#C9CDD9', line2: '#8F97AE', well: '#EEF0F5', term: '#FFFFFF',
        pane: '#FFFFFF', paneLine: '#1A1C2C', fg: '#1A1C2C', fg2: '#333C57', muted: '#4D5670', dim: '#566C86', accent: '#2E8B1E',
        accentSoft: 'rgba(46,139,30,.1)', accentLine: '#2E8B1E', onAccent: '#FFFFFF', warn: '#B05A00', onWarn: '#FFFFFF', danger: '#D0003C', ok: '#2E8B1E',
        ansiBlue: '#1868C8', ansiMag: '#C24A24', ansiGreen: '#2E8B1E',
      },
    },
  },
]

/**
 * Which of the two material finishes a style is built on (`material.css`): Glass is
 * translucent fills over a blur; Mono, Console and Pixel are solid fills with their own shape rules.
 */
export const finishOf = (style: WindowStyle): 'glass' | 'flat' => (style === 'glass' ? 'glass' : 'flat')

export function findWindowStyle(id: WindowStyle): WindowStyleEntry {
  const found = WINDOW_STYLES.find((style) => style.id === id) ?? WINDOW_STYLES[0]
  if (!found) throw new Error('No window styles are defined')
  return found
}

/**
 * The theme as a window style draws it. The style's palette replaces the colours of Midnight and Day
 * (the themes the design defines, Day also standing in for Paper); every other theme keeps its own colours and takes the style's
 * fonts, dials and shapes only. The status colours (amber, red, green) stay the theme's unless the
 * style names its own, so "needs you" keeps its meaning.
 */
export function withWindowStyle(theme: ThemeEntry, id: WindowStyle): ThemeEntry {
  const style = findWindowStyle(id)
  const side = theme.light ? style.light : style.dark
  const owns = PALETTE_THEMES.includes(theme.name)
  // A palette that sets its own accent sets what the accent is read as (text, and "working") too, so brass does not show through a
  // style that is green or white.
  const own = side.palette.accent
  const tokens: ThemeTokens = owns
    ? { ...theme.tokens, ...(own === undefined ? {} : { accentText: own, working: own }), ...side.palette }
    : theme.tokens
  return { ...theme, tokens, cardRing: composite(tokens.pane, tokens.bg), look: side.look }
}
