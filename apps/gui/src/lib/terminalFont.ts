/** The terminal font settings that reach xterm: validated here, once, where stored values come in. */

/** The bundled font, used when no family is chosen and behind any family that is not installed. */
export const BUNDLED_FONT_STACK = '"Geist Mono Variable", "Geist Mono", ui-monospace, monospace'
export const BUNDLED_FONT_NAME = 'Geist Mono'

export const WEIGHT_LIGHT = 300
export const WEIGHT_REGULAR = 400
export const WEIGHT_MEDIUM = 500
export const WEIGHT_SEMIBOLD = 600
export const WEIGHT_BOLD = 700
export const WEIGHT_EXTRA_BOLD = 800
export const TERMINAL_FONT_WEIGHTS = [WEIGHT_LIGHT, WEIGHT_REGULAR, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD, WEIGHT_BOLD, WEIGHT_EXTRA_BOLD] as const
export type TerminalFontWeight = (typeof TERMINAL_FONT_WEIGHTS)[number]
export const DEFAULT_FONT_WEIGHT: TerminalFontWeight = WEIGHT_REGULAR
export const DEFAULT_FONT_WEIGHT_BOLD: TerminalFontWeight = WEIGHT_BOLD

const MAX_FAMILY_LENGTH = 64
/** A single family name. No quotes, commas, semicolons or braces, so it cannot break out of the CSS font list it is put in. */
const FAMILY_PATTERN = /^[\p{L}\p{N} ._+-]+$/u

export function isFontWeight(value: unknown): value is TerminalFontWeight {
  return TERMINAL_FONT_WEIGHTS.some((weight) => weight === value)
}

/** A family name that is safe to put in a font list; empty means the bundled font. */
export function isFontFamily(value: unknown): value is string {
  if (typeof value !== 'string') return false
  const name = value.trim()
  return name === '' || (name.length <= MAX_FAMILY_LENGTH && FAMILY_PATTERN.test(name))
}

/** The font list xterm gets: the chosen family first, the bundled stack behind it so a missing font still draws. */
export function fontStackFor(family: string): string {
  const name = family.trim()
  return name === '' || !isFontFamily(name) ? BUNDLED_FONT_STACK : `"${name}", ${BUNDLED_FONT_STACK}`
}

/** Monospace families worth offering; only the ones this computer has are shown. */
const CANDIDATE_FAMILIES: readonly string[] = [
  'JetBrains Mono', 'Fira Code', 'Fira Mono', 'Cascadia Code', 'Cascadia Mono', 'Consolas', 'Menlo', 'Monaco', 'SF Mono',
  'Courier New', 'DejaVu Sans Mono', 'Liberation Mono', 'Ubuntu Mono', 'Source Code Pro', 'Hack', 'Inconsolata',
  'IBM Plex Mono', 'Roboto Mono', 'Iosevka', 'Noto Sans Mono', 'Droid Sans Mono', 'Victor Mono', 'Berkeley Mono', 'Lucida Console',
]

const PROBE_TEXT = 'mmmmmmmmmmlli1|WQ@'
const PROBE_SIZE_PX = 72
const BASELINES = ['monospace', 'serif'] as const

/**
 * Which of the candidate monospace families are installed. A webview cannot list fonts (the Local
 * Font Access API is Chromium-only), so each name is drawn against two generic families and counts
 * as installed when its text measures differently from both fallbacks. Cheap canvas work: a few
 * dozen measurements, no I/O. Empty where there is no canvas.
 */
export function installedMonospaceFamilies(candidates: readonly string[] = CANDIDATE_FAMILIES): string[] {
  const context = typeof document === 'undefined' ? null : document.createElement('canvas').getContext('2d')
  if (!context) return []
  const widthIn = (font: string) => {
    context.font = `${String(PROBE_SIZE_PX)}px ${font}`
    return context.measureText(PROBE_TEXT).width
  }
  const baselineWidths = BASELINES.map((baseline) => widthIn(baseline))
  return candidates.filter((name) => BASELINES.some((baseline, index) => widthIn(`"${name}", ${baseline}`) !== baselineWidths[index]))
}
