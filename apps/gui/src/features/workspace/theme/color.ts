/** A colour as sRGB channels in 0–1 and an alpha in 0–1. */
export type Rgba = { r: number; g: number; b: number; a: number }

const HEX_SHORT_LENGTH = 3
const HEX_LONG_LENGTH = 6
const HEX_RADIX = 16
const HEX_PAIR = 2
const CHANNEL_MAX = 255
const PERCENT = 100

// Constants from the sRGB transfer function (IEC 61966-2-1).
const SRGB = { linearLimit: 0.04045, linearScale: 12.92, offset: 0.055, scale: 1.055, gamma: 2.4, encodedLimit: 0.0031308 }

// Björn Ottosson's Oklab matrices, the space CSS `color-mix(in oklab, …)` interpolates in.
const LMS_FROM_RGB = {
  l: { r: 0.4122214708, g: 0.5363325363, b: 0.0514459929 },
  m: { r: 0.2119034982, g: 0.6806995451, b: 0.1073969566 },
  s: { r: 0.0883024619, g: 0.2817188376, b: 0.6299787005 },
}
const LAB_FROM_LMS = {
  L: { l: 0.2104542553, m: 0.793617785, s: -0.0040720468 },
  a: { l: 1.9779984951, m: -2.428592205, s: 0.4505937099 },
  b: { l: 0.0259040371, m: 0.7827717662, s: -0.808675766 },
}
const LMS_FROM_LAB = {
  l: { a: 0.3963377774, b: 0.2158037573 },
  m: { a: -0.1055613458, b: -0.0638541728 },
  s: { a: -0.0894841775, b: -1.291485548 },
}
const RGB_FROM_LMS = {
  r: { l: 4.0767416621, m: -3.3077115913, s: 0.2309699292 },
  g: { l: -1.2684380046, m: 2.6097574011, s: -0.3413193965 },
  b: { l: -0.0041960863, m: -0.7034186147, s: 1.707614701 },
}
const LUMINANCE = { r: 0.2126, g: 0.7152, b: 0.0722 }
const CONTRAST_OFFSET = 0.05
const CUBE = 3

export function parseColor(css: string): Rgba {
  const value = css.trim()
  if (value.startsWith('#')) {
    const hex = value.slice(1)
    const full = hex.length === HEX_SHORT_LENGTH ? hex.split('').map((digit) => digit + digit).join('') : hex
    if (full.length !== HEX_LONG_LENGTH) throw new Error(`Unsupported colour ${css}`)
    const channel = (index: number) => parseInt(full.slice(index * HEX_PAIR, index * HEX_PAIR + HEX_PAIR), HEX_RADIX) / CHANNEL_MAX
    return { r: channel(0), g: channel(1), b: channel(HEX_PAIR), a: 1 }
  }
  const match = /^rgba?\(([^)]+)\)$/.exec(value)
  if (!match?.[1]) throw new Error(`Unsupported colour ${css}`)
  const parts = match[1].split(/[\s,/]+/).filter(Boolean).map(Number)
  const [r = 0, g = 0, b = 0, a = 1] = parts
  return { r: r / CHANNEL_MAX, g: g / CHANNEL_MAX, b: b / CHANNEL_MAX, a }
}

const clamp = (value: number) => Math.min(1, Math.max(0, value))
const toLinear = (c: number) => (c <= SRGB.linearLimit ? c / SRGB.linearScale : ((c + SRGB.offset) / SRGB.scale) ** SRGB.gamma)
const fromLinear = (c: number) => (c <= SRGB.encodedLimit ? SRGB.linearScale * c : SRGB.scale * c ** (1 / SRGB.gamma) - SRGB.offset)

type Lab = { L: number; a: number; b: number }

function toOklab(color: Rgba): Lab {
  const r = toLinear(color.r)
  const g = toLinear(color.g)
  const b = toLinear(color.b)
  const row = (k: { r: number; g: number; b: number }) => Math.cbrt(k.r * r + k.g * g + k.b * b)
  const l = row(LMS_FROM_RGB.l)
  const m = row(LMS_FROM_RGB.m)
  const s = row(LMS_FROM_RGB.s)
  const lab = (k: { l: number; m: number; s: number }) => k.l * l + k.m * m + k.s * s
  return { L: lab(LAB_FROM_LMS.L), a: lab(LAB_FROM_LMS.a), b: lab(LAB_FROM_LMS.b) }
}

function fromOklab(lab: Lab, alpha: number): Rgba {
  const cone = (k: { a: number; b: number }) => (lab.L + k.a * lab.a + k.b * lab.b) ** CUBE
  const l = cone(LMS_FROM_LAB.l)
  const m = cone(LMS_FROM_LAB.m)
  const s = cone(LMS_FROM_LAB.s)
  const rgb = (k: { l: number; m: number; s: number }) => clamp(fromLinear(k.l * l + k.m * m + k.s * s))
  return { r: rgb(RGB_FROM_LMS.r), g: rgb(RGB_FROM_LMS.g), b: rgb(RGB_FROM_LMS.b), a: alpha }
}

/**
 * The same result as CSS `color-mix(in oklab, from, to <percent>%)` for opaque colours: `percent`
 * of `to` and the rest of `from`. Tokens are computed here rather than left to CSS because xterm's
 * theme and the contrast check need concrete colours.
 */
export function mixOklab(from: string, to: string, percent: number): string {
  const a = toOklab(parseColor(from))
  const b = toOklab(parseColor(to))
  const t = percent / PERCENT
  return formatColor(fromOklab({ L: a.L + (b.L - a.L) * t, a: a.a + (b.a - a.a) * t, b: a.b + (b.b - a.b) * t }, 1))
}

/** CSS `color-mix(in oklab, color <percent>%, transparent)`: the same colour at that opacity. */
export function withAlpha(color: string, percent: number): string {
  const parsed = parseColor(color)
  return formatColor({ ...parsed, a: percent / PERCENT })
}

export function formatColor(color: Rgba): string {
  const channel = (value: number) => Math.round(clamp(value) * CHANNEL_MAX)
  if (color.a >= 1) {
    return `#${[color.r, color.g, color.b].map((value) => channel(value).toString(HEX_RADIX).padStart(HEX_PAIR, '0')).join('').toUpperCase()}`
  }
  return `rgba(${String(channel(color.r))},${String(channel(color.g))},${String(channel(color.b))},${String(Math.round(color.a * PERCENT) / PERCENT)})`
}

/** An opaque colour as its three channels, 0 to 255, as a terminal's true-colour sequence takes them. */
export function channelsOf(color: string): readonly [number, number, number] {
  const { r, g, b } = parseColor(color)
  const channel = (value: number) => Math.round(clamp(value) * CHANNEL_MAX)
  return [channel(r), channel(g), channel(b)]
}

/** Paints a translucent colour over an opaque one. */
export function composite(top: string, under: string): string {
  const t = parseColor(top)
  const u = parseColor(under)
  const over = (a: number, b: number) => a * t.a + b * (1 - t.a)
  return formatColor({ r: over(t.r, u.r), g: over(t.g, u.g), b: over(t.b, u.b), a: 1 })
}

export function luminance(color: string): number {
  const { r, g, b } = parseColor(color)
  return LUMINANCE.r * toLinear(r) + LUMINANCE.g * toLinear(g) + LUMINANCE.b * toLinear(b)
}

/** WCAG 2 contrast ratio between two opaque colours. */
export function contrastRatio(first: string, second: string): number {
  const a = luminance(first)
  const b = luminance(second)
  return (Math.max(a, b) + CONTRAST_OFFSET) / (Math.min(a, b) + CONTRAST_OFFSET)
}
