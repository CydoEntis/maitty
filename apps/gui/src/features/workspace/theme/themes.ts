import type { ITheme } from '@xterm/xterm'
import maittyTokens from '@tokens/maitty-tokens.json'
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
  /** The accent as text on the card: the same brass, darkened where the ground is light. */
  accentText: string
  /** What "working" is drawn in; nothing else is this colour. */
  working: string
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

/**
 * What a window style adds to a theme besides colours: the fonts, the two glass dials (`hk` is the
 * highlight strength, `sk` the shadow strength), the resting card sheen (`sh`) and the background glow
 * strengths. A field left out keeps the stylesheet's default.
 */
export type StyleLook = { ui: string; mono: string; hk?: string; sk?: string; sh?: string; glowAccent?: string; glowMag?: string }

/** `cardRing` is the solid card colour that rings a status badge so it reads as cut out of the card. `look` is set by `withWindowStyle`. */
export type ThemeEntry = { name: string; family: string; light: boolean; tokens: ThemeTokens; cardRing: string; look?: StyleLook }

/** Which theme is in use: the dark theme, the light theme, or whichever one the system's own setting asks for. */
export type ThemeMode = 'system' | 'dark' | 'light'

export const DEFAULT_THEME = 'Maitty Night'
export const DEFAULT_LIGHT_THEME = 'Maitty Day'
export const MIN_TEXT_CONTRAST = 4.5

const CSS_VARIABLES: Record<keyof ThemeTokens, string> = {
  bg: '--bg', main: '--main', raise: '--raise', hover: '--hover', sel: '--sel', line: '--line', line2: '--line2',
  well: '--well', term: '--term', fg: '--fg', fg2: '--fg2', muted: '--muted', dim: '--dim', accent: '--accent', accentText: '--accent-text', working: '--working',
  accentSoft: '--accent-soft', accentLine: '--accent-line', onAccent: '--on-accent', warn: '--warn',
  warnSoft: '--warn-soft', warnLine: '--warn-line', onWarn: '--on-warn', danger: '--danger',
  dangerSoft: '--danger-soft', dangerLine: '--danger-line', ok: '--ok', ansiBlue: '--ansi-blue',
  ansiMag: '--ansi-mag', ansiGreen: '--ansi-green', pane: '--pane', paneLine: '--pane-line',
}

const DARK_STATUS = { warn: '#F2B65A', warnSoft: 'rgba(242,182,90,.07)', warnLine: 'rgba(242,182,90,.28)', onWarn: '#151007', danger: '#F07F78', dangerSoft: 'rgba(240,127,120,.07)', dangerLine: 'rgba(240,127,120,.3)' }
// Dark enough for AA on the grounds of every generated light theme (Gruvbox Light's cream is the tightest).
const LIGHT_STATUS = { warn: '#8F5400', warnSoft: 'rgba(143,84,0,.07)', warnLine: 'rgba(143,84,0,.3)', onWarn: '#FFFFFF', danger: '#B8362C', dangerSoft: 'rgba(184,54,44,.06)', dangerLine: 'rgba(184,54,44,.3)' }





// Pure black and white with strong lines, for readability over polish. Warn and danger keep their meaning.
const HIGH_CONTRAST: ThemeTokens = {
  ansiBlue: '#8FB8FF', ansiMag: '#E3B5FF', ansiGreen: '#7CF29A', pane: '#000000', paneLine: 'rgba(255,255,255,.3)',
  bg: '#000000', main: '#000000', raise: '#0D0D0D', hover: '#0A0A0A', sel: '#1C1C1C', line: '#5A5A5A', line2: '#8A8A8A',
  well: '#000000', term: '#000000', fg: '#FFFFFF', fg2: '#EDEDED', muted: '#CFCFCF', dim: '#ABABAB', accent: '#3CF2D2', accentText: '#3CF2D2', working: '#3CF2D2',
  accentSoft: 'rgba(60,242,210,.16)', accentLine: 'rgba(60,242,210,.75)', onAccent: '#000000',
  warn: '#FFC94D', warnSoft: 'rgba(255,201,77,.12)', warnLine: 'rgba(255,201,77,.6)', onWarn: '#000000',
  danger: '#FF7A7A', dangerSoft: 'rgba(255,122,122,.12)', dangerLine: 'rgba(255,122,122,.6)', ok: '#7CF29A',
}

// Mix percentages from the design; `fg2`, `muted` and `dim` are raised when a theme would fall below
// MIN_TEXT_CONTRAST against the terminal ground or the window ground.
const MIX = {
  raiseDark: 7, raiseLight: 5, hover: 5, sel: 10, line: 9, line2: 15, bgDark: 18, bgLight: 4, wellDark: 25, wellLight: 3,
  fg2: 80, muted: 62, dim: 52, accentSoft: 13, accentLine: 50, paneLine: 10, contrastStep: 2, full: 100,
}

/**
 * Maitty's own themes, built from `maitty-tokens.json`: Night and Day (sea-glass green) and Dark and Light (a standard black and white ground with the same accents) (the TUI reads the same file): sea-glass and brass. Teal means
 * only "working" (`working`), coral only "needs you" (`warn`), brass is the accent. Text uses the file's *Text colours
 * so every pair meets AA; fills (a button's brass) use the fill colours with their *On ink.
 */
/** How far the tokens file's colours are mixed to make the roles it does not name (percent). */
const MAITTY_MIX = { hover: 50, line2: 40, muted: 50, warnSoft: 7, warnLine: 30, paneLine: 8 }
type MaittySide = typeof maittyTokens.night
function maittyTheme(side: MaittySide, light: boolean): ThemeTokens {
  const { bg, line, text, accent, state } = side
  const status = light ? LIGHT_STATUS : DARK_STATUS
  return {
    bg: bg.page, main: bg.card, pane: bg.card, term: bg.terminal,
    raise: bg.surface, hover: mixOklab(bg.card, bg.selected, MAITTY_MIX.hover), sel: bg.selected,
    line: line.default, line2: mixOklab(line.default, text.dim, MAITTY_MIX.line2), well: bg.depth3,
    fg: text.strong, fg2: text.body, muted: mixOklab(text.body, text.dim, MAITTY_MIX.muted), dim: text.dim,
    accent: accent.brass, accentText: accent.brassText, onAccent: accent.brassOn,
    accentSoft: withAlpha(accent.brass, MIX.accentSoft), accentLine: withAlpha(accent.brass, MIX.accentLine),
    working: state.workingText,
    warn: state.needsText, warnSoft: withAlpha(state.needs, MAITTY_MIX.warnSoft), warnLine: withAlpha(state.needs, MAITTY_MIX.warnLine), onWarn: state.needsOn,
    danger: status.danger, dangerSoft: status.dangerSoft, dangerLine: status.dangerLine,
    ok: state.doneText,
    ansiBlue: light ? '#2F5FD0' : '#82AAFF', ansiMag: light ? '#8146C9' : '#C3A6F2', ansiGreen: state.doneText,
    paneLine: withAlpha(text.strong, MAITTY_MIX.paneLine),
  }
}
const MAITTY_NIGHT = maittyTheme(maittyTokens.night, false)
const MAITTY_DAY = maittyTheme(maittyTokens.day, true)
const MAITTY_DARK = maittyTheme(maittyTokens.dark, false)
const MAITTY_LIGHT = maittyTheme(maittyTokens.light, true)

const SHIFT_STEP = 2

/** Mixes a colour toward black (light themes) or white (dark themes) a step at a time until it reads as text on every ground. */
function shiftUntilReadable(colour: string, grounds: readonly string[], toward: string): string {
  let percent = 0
  const passes = (candidate: string) => grounds.every((ground) => contrastRatio(candidate, ground) >= MIN_TEXT_CONTRAST)
  while (percent < MIX.full && !passes(mixOklab(colour, toward, percent))) percent += SHIFT_STEP
  return mixOklab(colour, toward, percent)
}

/** Builds every token from a theme's six inputs, as the TUI themes import. */
export function generateTheme(inputs: ThemeInputs): ThemeTokens {
  const { bg, fg, accent, mag, blue, green, light } = inputs
  const towardFg = (percent: number) => mixOklab(bg, fg, percent)
  const windowBg = light ? towardFg(MIX.bgLight) : mixOklab(bg, '#000000', MIX.bgDark)
  // Text sits on the card and terminal ground (`bg`) and, in the sidebar's gaps and the status bar, on the window ground.
  const readable = (start: number) => {
    let percent = start
    const passes = (candidate: string) => contrastRatio(candidate, bg) >= MIN_TEXT_CONTRAST && contrastRatio(candidate, windowBg) >= MIN_TEXT_CONTRAST
    while (percent < MIX.full && !passes(mixOklab(bg, fg, percent))) percent += MIX.contrastStep
    return mixOklab(bg, fg, Math.min(percent, MIX.full))
  }
  // A light theme's published hues (Solarized cyan, Latte green) are pastel against its own ground, so
  // they are darkened in place (same hue, less lightness) until they read as text on both grounds.
  // A dark theme's mid-tone hues (Solarized blue, Zenburn red) fall short the other way, so they are
  // lightened in place instead.
  const hue = (colour: string) => shiftUntilReadable(colour, [bg, windowBg], light ? '#000000' : '#FFFFFF')
  const baseStatus = light ? LIGHT_STATUS : DARK_STATUS
  // Only the text colours move; the soft and line tints keep the published hue.
  const status = { ...baseStatus, warn: hue(baseStatus.warn), danger: hue(baseStatus.danger) }
  const [accentText, magText, blueText, greenText] = [hue(accent), hue(mag), hue(blue), hue(green)]
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
    fg2: readable(MIX.fg2),
    muted: readable(MIX.muted),
    dim: readable(MIX.dim),
    accent: accentText,
    accentText,
    working: accentText,
    accentSoft: withAlpha(accentText, MIX.accentSoft),
    accentLine: withAlpha(accentText, MIX.accentLine),
    onAccent: light ? '#FFFFFF' : bg,
    ...status,
    ok: greenText,
    ansiBlue: blueText,
    ansiMag: magText,
    ansiGreen: greenText,
    pane: bg,
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

const entry = (name: string, family: string, light: boolean, tokens: ThemeTokens): ThemeEntry => ({
  name,
  family,
  light,
  tokens,
  cardRing: composite(tokens.pane, tokens.bg),
})

export const THEMES: readonly ThemeEntry[] = [
  entry('Maitty Night', 'Maitty · default dark', false, MAITTY_NIGHT),
  entry('Maitty Day', 'Maitty · default light', true, MAITTY_DAY),
  entry('Maitty Dark', 'Maitty · black', false, MAITTY_DARK),
  entry('Maitty Light', 'Maitty · white', true, MAITTY_LIGHT),
  ...GENERATED.map(([name, bg, fg, accent, mag, blue, green, light, family]) =>
    entry(name, `${family} · ${light ? 'light' : 'dark'}`, light, generateTheme({ bg, fg, accent, mag, blue, green, light })),
  ),
  entry('High Contrast', 'Accessibility · dark', false, HIGH_CONTRAST),
]

/** The theme in use for a mode. `systemDark` only matters in System mode. */
export function activeThemeName(choice: { themeMode: ThemeMode; darkTheme: string; lightTheme: string }, systemDark: boolean): string {
  const dark = choice.themeMode === 'dark' || (choice.themeMode === 'system' && systemDark)
  return dark ? choice.darkTheme : choice.lightTheme
}

export function findTheme(name: string): ThemeEntry {
  const found = THEMES.find((theme) => theme.name === name) ?? THEMES[0]
  if (!found) throw new Error('No themes are defined')
  return found
}

/**
 * The glass finish's light and shade: the highlight colour (`--hl`, as `r,g,b`) and its strength, and
 * the shadow colour (`--sd`) and its strength. `--hs` is the colour of the sheen and the top edge: white in
 * both, because a light theme's glass is frosted by white light while its hairlines are drawn in ink. Dark themes catch white light and cast black shadows;
 * light themes are lit and shaded by their own ink, more faintly.
 */
const DARK_GLASS = { '--hl': '255,255,255', '--hs': '255,255,255', '--hk': '1', '--sd': '0,0,0', '--sk': '1' }
const LIGHT_GLASS = { '--hl': '20,32,46', '--hs': '255,255,255', '--hk': '.85', '--sd': '20,32,46', '--sk': '.3' }

/** The theme as CSS custom properties, for the app root's style. */
export function themeStyle(theme: ThemeEntry): Record<string, string> {
  const style: Record<string, string> = { '--card-ring': theme.cardRing, colorScheme: theme.light ? 'light' : 'dark', ...(theme.light ? LIGHT_GLASS : DARK_GLASS) }
  for (const [key, variable] of Object.entries(CSS_VARIABLES)) style[variable] = theme.tokens[key as keyof ThemeTokens]
  const { look } = theme
  if (look === undefined) return style
  style['--ui'] = look.ui
  style['--mono'] = look.mono
  if (look.hk !== undefined) style['--hk'] = look.hk
  if (look.sk !== undefined) style['--sk'] = look.sk
  if (look.sh !== undefined) style['--sh'] = look.sh
  if (look.glowAccent !== undefined) style['--glow-accent'] = look.glowAccent
  if (look.glowMag !== undefined) style['--glow-mag'] = look.glowMag
  return style
}

/**
 * The selection is the accent itself, opaque (xterm's WebGL renderer draws it without alpha), with the accent's own ink for
 * the text. A faint tint of the accent over the card was nearly invisible on a dark ground. The accent already passes as
 * text on the card, so its ink passes on it.
 */
function selectionColors(tokens: ThemeTokens): { background: string; foreground: string } {
  return { background: tokens.accent, foreground: tokens.onAccent }
}

/**
 * What xterm should enforce for colours a program picks itself. The 16 ANSI slots are audited, but
 * a program also paints with the 256-colour cube and truecolour (Claude Code's pale blue 153 is
 * unreadable on white), which no palette can fix; xterm nudges those until they reach the ratio.
 * Dark themes keep every colour as the program asked.
 */
export function terminalMinimumContrast(theme: ThemeEntry): number {
  return theme.light ? MIN_TEXT_CONTRAST : 1
}

/**
 * xterm's palette from the same tokens, so a pane's ANSI colours follow the theme. The background is
 * the workspace card's own ground, so a terminal sits on the card with no box of its own; the pane makes
 * it see-through when its renderer can afford that.
 */
export function xtermTheme(tokens: ThemeTokens): ITheme {
  const selection = selectionColors(tokens)
  return {
    background: tokens.main,
    foreground: tokens.fg2,
    cursor: tokens.accentText,
    cursorAccent: tokens.main,
    selectionBackground: selection.background,
    selectionForeground: selection.foreground,
    // The ground, not the well: programs paint with ANSI black expecting it to vanish into the
    // background, as Claude Code does under its prompt and status line.
    black: tokens.main,
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
    cyan: tokens.accentText,
    brightCyan: tokens.accentText,
    white: tokens.fg2,
    brightWhite: tokens.fg,
  }
}
