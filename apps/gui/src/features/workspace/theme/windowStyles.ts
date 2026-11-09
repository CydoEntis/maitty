import type { StyleLook, ThemeEntry } from './themes'

/**
 * The four window styles of the desktop: Glass, Mono Type, Console and Retro Pixel. A style is shape, not colour: its corners, borders,
 * shadows and type (the shape rules keyed by `[data-style]` live in `material.css`), plus the fonts and the two glass dials
 * kept here. The colours are always the theme in use, so every style can wear every theme.
 */
export const WINDOW_STYLE_IDS = ['glass', 'mono', 'console', 'pixel'] as const
export type WindowStyle = (typeof WINDOW_STYLE_IDS)[number]
export const DEFAULT_WINDOW_STYLE: WindowStyle = 'glass'

export const isWindowStyle = (value: unknown): value is WindowStyle => WINDOW_STYLE_IDS.some((id) => id === value)

/** One side (dark or light) of a style: the fonts, glass dials and glow that go with it. Colours are the theme's, never the style's. */
type Side = { look: StyleLook }

export type WindowStyleEntry = {
  id: WindowStyle
  name: string
  blurb: string
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
    id: 'glass', name: 'Glass', blurb: 'Soft frosted panels with rounded corners.',
    font: INTER,
    dark: {
      look: { ui: INTER, mono: JETBRAINS_MONO, hk: '.7', sk: '1', glowAccent: '18%', glowMag: '12%' },
    },
    light: {
      look: { ui: INTER, mono: JETBRAINS_MONO, hk: '1', sk: '.35', sh: NO_SHEEN, glowAccent: '22%', glowMag: '15%' },
    },
  },
  {
    id: 'mono', name: 'Mono Type', blurb: 'Every glyph the same width, square corners.',
    font: SPACE_MONO,
    dark: {
      look: { ui: SPACE_MONO, mono: SPACE_MONO, hk: '.4', sk: '.5', glowAccent: '0%', glowMag: '0%', sh: NO_SHEEN },
    },
    light: {
      look: { ui: SPACE_MONO, mono: SPACE_MONO, ...LIGHT_FLAT },
    },
  },
  {
    id: 'console', name: 'Console', blurb: 'IDE panels with hairline borders. Plex Sans with JetBrains Mono.',
    font: PLEX_SANS,
    dark: {
      look: { ui: PLEX_SANS, mono: JETBRAINS_MONO, hk: '.45', sk: '.7', glowAccent: '6%', glowMag: '0%', sh: NO_SHEEN },
    },
    light: {
      look: { ui: PLEX_SANS, mono: JETBRAINS_MONO, ...LIGHT_FLAT },
    },
  },
  {
    id: 'pixel', name: 'Retro Pixel', blurb: '8-bit console. Hard corners, chunky borders, scanlines.',
    font: PIXELIFY,
    dark: {
      look: { ui: PIXELIFY, mono: SPACE_MONO, hk: '.5', sk: '1', glowAccent: '0%', glowMag: '0%', sh: NO_SHEEN },
    },
    light: {
      look: { ui: PIXELIFY, mono: SPACE_MONO, ...LIGHT_FLAT, sk: '.4' },
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
 * The theme as a window style draws it: the theme's own colours, with the style's fonts, glass dials and glow laid over
 * them. A style is shape (corners, borders, shadows, type); the colours are always the theme in use.
 */
export function withWindowStyle(theme: ThemeEntry, id: WindowStyle): ThemeEntry {
  const side = theme.light ? findWindowStyle(id).light : findWindowStyle(id).dark
  return { ...theme, look: side.look }
}
