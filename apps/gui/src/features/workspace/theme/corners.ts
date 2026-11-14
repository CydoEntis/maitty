import type { StyleLook, ThemeEntry } from './themes'

/**
 * The desktop has one look (the design's flat panels, Inter with JetBrains Mono) and one shape choice: square corners
 * (the default) or rounded ones. Corners are tokens only (`--r-*` in `tokens.css`, keyed by `data-corners` on the root);
 * nothing else about the window changes with them. The ids are what is saved.
 */
export const CORNERS = ['square', 'rounded'] as const
export type Corners = (typeof CORNERS)[number]
export const DEFAULT_CORNERS: Corners = 'square'
export const CORNER_NAMES: Record<Corners, string> = { square: 'Square', rounded: 'Rounded' }

export const isCorners = (value: unknown): value is Corners => CORNERS.some((id) => id === value)

/**
 * The corners an old saved window style becomes: the styles with rounded corners (Glass, Flat, and the names before
 * them) keep them; Squared and Retro Pixel stay square. Null for anything else.
 */
export function cornersOfWindowStyle(value: unknown): Corners | null {
  if (value === 'glass' || value === 'console' || value === 'flat' || value === 'tiled' || value === 'minimal' || value === 'expressive' || value === 'frosted') return 'rounded'
  if (value === 'mono' || value === 'pixel') return 'square'
  return null
}

const INTER = "'Inter Variable','Inter',system-ui,sans-serif"
const JETBRAINS_MONO = "'JetBrains Mono Variable','JetBrains Mono',monospace"

/**
 * The one look's fonts and shading strengths, for a theme that does not set its own (`ThemeEntry.shading`, the design's
 * palette themes do). No sheen and no background glow: the design's panels are flat.
 */
const LOOK: { dark: StyleLook; light: StyleLook } = {
  dark: { ui: INTER, mono: JETBRAINS_MONO, hk: '.7', sk: '.6', sh: 'none', glowAccent: '0%', glowMag: '0%' },
  light: { ui: INTER, mono: JETBRAINS_MONO, hk: '1', sk: '.35', sh: 'none', glowAccent: '0%', glowMag: '0%' },
}

/** The theme as the window draws it: its own colours, with the look's fonts and shading laid over them. */
export function withLook(theme: ThemeEntry): ThemeEntry {
  return { ...theme, look: theme.light ? LOOK.light : LOOK.dark }
}
