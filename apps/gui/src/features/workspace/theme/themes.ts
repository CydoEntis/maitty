import type { ITheme } from '@xterm/xterm'
import { composite, contrastRatio, mixOklab, withAlpha } from './color'

/** Every colour role the desktop app reads, set as CSS custom properties on the app root. */
export type ThemeTokens = {
  bg: string
  main: string
  raise: string
  hover: string
  sel: string
  line: string
  line2: string
  well: string
  term: string
  fg: string
  fg2: string
  muted: string
  dim: string
  accent: string
  accentSoft: string
  accentLine: string
  onAccent: string
  warn: string
  warnSoft: string
  warnLine: string
  onWarn: string
  danger: string
  dangerSoft: string
  dangerLine: string
  ok: string
  ansiBlue: string
  ansiMag: string
  ansiGreen: string
  pane: string
  paneLine: string
}

/** The six colours (and a light flag) a theme is made from; a TUI theme imports as these. */
export type ThemeInputs = { bg: string; fg: string; accent: string; mag: string; blue: string; green: string; light: boolean }

/** `cardRing` is the solid card colour that rings a status badge so it reads as cut out of the card. */
export type ThemeEntry = { name: string; family: string; light: boolean; tokens: ThemeTokens; cardRing: string }

export const DEFAULT_THEME = 'Midnight'
export const MIN_TEXT_CONTRAST = 4.5

const CSS_VARIABLES: Record<keyof ThemeTokens, string> = {
  bg: '--bg', main: '--main', raise: '--raise', hover: '--hover', sel: '--sel', line: '--line', line2: '--line2',
  well: '--well', term: '--term', fg: '--fg', fg2: '--fg2', muted: '--muted', dim: '--dim', accent: '--accent',
  accentSoft: '--accent-soft', accentLine: '--accent-line', onAccent: '--on-accent', warn: '--warn',
  warnSoft: '--warn-soft', warnLine: '--warn-line', onWarn: '--on-warn', danger: '--danger',
  dangerSoft: '--danger-soft', dangerLine: '--danger-line', ok: '--ok', ansiBlue: '--ansi-blue',
  ansiMag: '--ansi-mag', ansiGreen: '--ansi-green', pane: '--pane', paneLine: '--pane-line',
}

const DARK_STATUS = { warn: '#F2B65A', warnSoft: 'rgba(242,182,90,.07)', warnLine: 'rgba(242,182,90,.28)', onWarn: '#151007', danger: '#F07F78', dangerSoft: 'rgba(240,127,120,.07)', dangerLine: 'rgba(240,127,120,.3)' }
const LIGHT_STATUS = { warn: '#9A5B00', warnSoft: 'rgba(154,91,0,.07)', warnLine: 'rgba(154,91,0,.3)', onWarn: '#FFFFFF', danger: '#C0392F', dangerSoft: 'rgba(192,57,47,.06)', dangerLine: 'rgba(192,57,47,.3)' }

const MIDNIGHT: ThemeTokens = {
  ansiBlue: '#82AAFF', ansiMag: '#C3A6F2', ansiGreen: '#9FD39A', pane: 'rgba(14,18,24,.86)', paneLine: 'rgba(255,255,255,.07)',
  bg: '#090C10', main: '#0D1116', raise: '#141A21', hover: '#12171E', sel: '#1A212A', line: '#1B222B', line2: '#26303B',
  well: '#080A0D', term: '#06080B', fg: '#E8ECEF', fg2: '#B3BCC5', muted: '#8C96A1', dim: '#75808B', accent: '#5FD4BF',
  accentSoft: 'rgba(95,212,191,.11)', accentLine: 'rgba(95,212,191,.5)', onAccent: '#06110F', ...DARK_STATUS, ok: '#8DB89B',
}

const ABYSS: ThemeTokens = {
  ...MIDNIGHT, pane: 'rgba(9,20,32,.86)', bg: '#07111B', main: '#0B1724', raise: '#112131', hover: '#0F1C29', sel: '#162839',
  line: '#14243A', line2: '#1E3248', well: '#06101A', term: '#050D15',
}

const PAPER: ThemeTokens = {
  ansiBlue: '#2F5FD0', ansiMag: '#8146C9', ansiGreen: '#2F7A3B', pane: 'rgba(255,255,255,.92)', paneLine: 'rgba(0,0,0,.08)',
  bg: '#EEF0EE', main: '#FFFFFF', raise: '#F5F6F4', hover: '#F0F2F0', sel: '#E7EAE8', line: '#E3E6E4', line2: '#D2D7D4',
  well: '#F2F4F2', term: '#FAFBFA', fg: '#12171C', fg2: '#3B444C', muted: '#58626C', dim: '#646E78', accent: '#0E8F7A',
  accentSoft: 'rgba(14,143,122,.1)', accentLine: 'rgba(14,143,122,.5)', onAccent: '#FFFFFF', ...LIGHT_STATUS, ok: '#2F7A4B',
}

// Mix percentages from the design; `muted` and `dim` are raised when a theme would fall below
// MIN_TEXT_CONTRAST against its pane.
const MIX = {
  raiseDark: 7, raiseLight: 5, hover: 5, sel: 10, line: 9, line2: 15, bgDark: 18, bgLight: 4, wellDark: 25, wellLight: 3,
  fg2: 80, muted: 62, dim: 52, accentSoft: 13, accentLine: 50, pane: 90, paneLine: 10, contrastStep: 2, full: 100,
}

/** Builds every token from a theme's six inputs, as the TUI themes import. */
export function generateTheme(inputs: ThemeInputs): ThemeTokens {
  const { bg, fg, accent, mag, blue, green, light } = inputs
  const towardFg = (percent: number) => mixOklab(bg, fg, percent)
  const windowBg = light ? towardFg(MIX.bgLight) : mixOklab(bg, '#000000', MIX.bgDark)
  const pane = withAlpha(bg, MIX.pane)
  const paneOnWindow = composite(pane, windowBg)
  const readable = (start: number) => {
    let percent = start
    while (percent < MIX.full && contrastRatio(mixOklab(bg, fg, percent), paneOnWindow) < MIN_TEXT_CONTRAST) percent += MIX.contrastStep
    return mixOklab(bg, fg, Math.min(percent, MIX.full))
  }
  return {
    bg: windowBg,
    main: bg,
    raise: towardFg(light ? MIX.raiseLight : MIX.raiseDark),
    hover: towardFg(MIX.hover),
    sel: towardFg(MIX.sel),
    line: towardFg(MIX.line),
    line2: towardFg(MIX.line2),
    well: light ? towardFg(MIX.wellLight) : mixOklab(bg, '#000000', MIX.wellDark),
    term: bg,
    fg,
    fg2: mixOklab(bg, fg, MIX.fg2),
    muted: readable(MIX.muted),
    dim: readable(MIX.dim),
    accent,
    accentSoft: withAlpha(accent, MIX.accentSoft),
    accentLine: withAlpha(accent, MIX.accentLine),
    onAccent: light ? '#FFFFFF' : bg,
    ...(light ? LIGHT_STATUS : DARK_STATUS),
    ok: green,
    ansiBlue: blue,
    ansiMag: mag,
    ansiGreen: green,
    pane,
    paneLine: withAlpha(fg, MIX.paneLine),
  }
}

// [name, bg, fg, accent, mag, blue, green, light, family]
type GeneratedSource = readonly [string, string, string, string, string, string, string, boolean, string]

const GENERATED: readonly GeneratedSource[] = [
  ['Dracula', '#282A36', '#F8F8F2', '#8BE9FD', '#FF79C6', '#BD93F9', '#50FA7B', false, 'Vivid'],
  ['Nord', '#2E3440', '#ECEFF4', '#88C0D0', '#B48EAD', '#81A1C1', '#A3BE8C', false, 'Arctic'],
  ['Gruvbox Dark', '#282828', '#EBDBB2', '#8EC07C', '#D3869B', '#83A598', '#B8BB26', false, 'Retro'],
  ['Gruvbox Light', '#FBF1C7', '#3C3836', '#427B58', '#8F3F71', '#076678', '#79740E', true, 'Retro'],
  ['Catppuccin Mocha', '#1E1E2E', '#CDD6F4', '#94E2D5', '#CBA6F7', '#89B4FA', '#A6E3A1', false, 'Pastel'],
  ['Catppuccin Latte', '#EFF1F5', '#4C4F69', '#179299', '#8839EF', '#1E66F5', '#40A02B', true, 'Pastel'],
  ['Tokyo Night', '#1A1B26', '#C0CAF5', '#7DCFFF', '#BB9AF7', '#7AA2F7', '#9ECE6A', false, 'Neon'],
  ['One Dark', '#282C34', '#ABB2BF', '#56B6C2', '#C678DD', '#61AFEF', '#98C379', false, 'Classic'],
  ['Solarized Dark', '#002B36', '#93A1A1', '#2AA198', '#D33682', '#268BD2', '#859900', false, 'Classic'],
  ['Solarized Light', '#FDF6E3', '#586E75', '#2AA198', '#D33682', '#268BD2', '#859900', true, 'Classic'],
  ['Rosé Pine', '#191724', '#E0DEF4', '#9CCFD8', '#C4A7E7', '#31748F', '#9CCFD8', false, 'Muted'],
  ['Rosé Pine Dawn', '#FAF4ED', '#575279', '#56949F', '#907AA9', '#286983', '#56949F', true, 'Muted'],
  ['Kanagawa', '#1F1F28', '#DCD7BA', '#7AA89F', '#957FB8', '#7E9CD8', '#98BB6C', false, 'Ink'],
  ['Everforest', '#2D353B', '#D3C6AA', '#83C092', '#D699B6', '#7FBBB3', '#A7C080', false, 'Forest'],
  ['Monokai', '#272822', '#F8F8F2', '#66D9EF', '#AE81FF', '#66D9EF', '#A6E22E', false, 'Vivid'],
  ['GitHub Dark', '#0D1117', '#C9D1D9', '#39C5CF', '#BC8CFF', '#58A6FF', '#3FB950', false, 'Clean'],
  ['GitHub Light', '#FFFFFF', '#24292F', '#1B7C83', '#8250DF', '#0969DA', '#1A7F37', true, 'Clean'],
  ['Ayu Mirage', '#1F2430', '#CBCCC6', '#95E6CB', '#D4BFFF', '#73D0FF', '#BAE67E', false, 'Soft'],
  ['Material Ocean', '#0F111A', '#A6ACCD', '#89DDFF', '#C792EA', '#82AAFF', '#C3E88D', false, 'Deep'],
  ['Night Owl', '#011627', '#D6DEEB', '#7FDBCA', '#C792EA', '#82AAFF', '#ADDB67', false, 'Deep'],
  ['Horizon', '#1C1E26', '#E0E0E0', '#25B0BC', '#B877DB', '#26BBD9', '#29D398', false, 'Warm'],
  ['Oxocarbon', '#161616', '#F2F4F8', '#3DDBD9', '#BE95FF', '#78A9FF', '#42BE65', false, 'Mono'],
  ['Zenburn', '#3F3F3F', '#DCDCCC', '#93E0E3', '#DC8CC3', '#8CD0D3', '#7F9F7F', false, 'Low contrast'],
]

const entry = (name: string, family: string, light: boolean, tokens: ThemeTokens, cardRing?: string): ThemeEntry => ({
  name,
  family,
  light,
  tokens,
  cardRing: cardRing ?? composite(tokens.pane, tokens.bg),
})

export const THEMES: readonly ThemeEntry[] = [
  entry('Midnight', 'Maitty · default', false, MIDNIGHT, '#0E1218'),
  entry('Abyss', 'Maitty · deeper blue', false, ABYSS),
  entry('Paper', 'Maitty · light', true, PAPER),
  ...GENERATED.map(([name, bg, fg, accent, mag, blue, green, light, family]) =>
    entry(name, `${family} · ${light ? 'light' : 'dark'}`, light, generateTheme({ bg, fg, accent, mag, blue, green, light })),
  ),
]

export function findTheme(name: string): ThemeEntry {
  const found = THEMES.find((theme) => theme.name === name) ?? THEMES[0]
  if (!found) throw new Error('No themes are defined')
  return found
}

/** The theme as CSS custom properties, for the app root's style. */
export function themeStyle(theme: ThemeEntry): Record<string, string> {
  const style: Record<string, string> = { '--card-ring': theme.cardRing, colorScheme: theme.light ? 'light' : 'dark' }
  for (const [key, variable] of Object.entries(CSS_VARIABLES)) style[variable] = theme.tokens[key as keyof ThemeTokens]
  return style
}

const SELECTION_ALPHA = 30

/** xterm's palette from the same tokens, so a pane's ANSI colours follow the theme. */
export function xtermTheme(tokens: ThemeTokens): ITheme {
  return {
    background: 'rgba(0,0,0,0)',
    foreground: tokens.fg2,
    cursor: tokens.accent,
    cursorAccent: tokens.term,
    selectionBackground: withAlpha(tokens.accent, SELECTION_ALPHA),
    black: tokens.well,
    brightBlack: tokens.dim,
    red: tokens.danger,
    brightRed: tokens.danger,
    green: tokens.ansiGreen,
    brightGreen: tokens.ok,
    yellow: tokens.warn,
    brightYellow: tokens.warn,
    blue: tokens.ansiBlue,
    brightBlue: tokens.ansiBlue,
    magenta: tokens.ansiMag,
    brightMagenta: tokens.ansiMag,
    cyan: tokens.accent,
    brightCyan: tokens.accent,
    white: tokens.fg2,
    brightWhite: tokens.fg,
  }
}

/** The pane colour as it is seen: painted over the window ground. */
export function paneSurface(tokens: ThemeTokens): string {
  return composite(tokens.pane, tokens.bg)
}
