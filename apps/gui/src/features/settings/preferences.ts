import { isSpinnerStyle, type SpinnerStyle } from '@/lib/spinner-clock'
import type { EffectStyle } from '@/lib/animation-level'
import type { AnimationMode } from '@/lib/animation-auto'
import { DEFAULT_FONT_WEIGHT, DEFAULT_FONT_WEIGHT_BOLD, isFontFamily, isFontWeight, type TerminalFontWeight } from '@/lib/terminalFont'
import { readThemePreference, type ThemePreference } from '@/data/themePreference'
import { THEMES, type ThemeMode } from '@/features/workspace/theme/themes'

export type ToastPosition = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right'
export type CursorStyle = 'block' | 'bar' | 'underline'
/** How much room the sidebar's rows, the tabs and the panes get. */
export type Density = 'compact' | 'comfortable' | 'spacious'
export const DENSITIES: readonly Density[] = ['compact', 'comfortable', 'spacious']
/** How big the sidebar's and the rail's icons and tiles are. */
export type IconSize = 'small' | 'medium' | 'large'
/** Which working rows and tabs show the working effect: every one, only the one you are in, or none. */
export type WorkingScope = 'all' | 'selected' | 'off'
/** How a project's other sessions sit under its main row in the sidebar. */
export type TrayStyle = 'names' | 'icons' | 'stacked'
/** Which side of the window the sidebar sits on. */
export type SidebarSide = 'left' | 'right'
/** The window style: Flat is solid panels, Tiled is edge-to-edge panes with square corners, Glass is floating frosted cards with sweeps. */
export type StyleChoice = 'flat' | 'tiled' | 'glass'
/** App and folder icons keep their colours, or drop to neutral gray tiles. Independent of the style. */
export type IconColor = 'color' | 'grayscale'
/** Which session leads a project: the one you are in, or the one that most needs you. */
export type LeadMode = 'focused' | 'attention'

const SCROLLBACK_SHORT = 5000
const SCROLLBACK_DEFAULT = 10000
const SCROLLBACK_LONG = 50000
export const SCROLLBACK_CHOICES = [SCROLLBACK_SHORT, SCROLLBACK_DEFAULT, SCROLLBACK_LONG] as const
export type ScrollbackLines = (typeof SCROLLBACK_CHOICES)[number]
/** Lines scrolled per wheel notch, as a multiple of xterm's default (`scrollSensitivity`). */
const SCROLL_NORMAL = 1
const SCROLL_DOUBLE = 2
const SCROLL_TRIPLE = 3
const SCROLL_FASTEST = 5
export const SCROLL_SPEED_CHOICES = [SCROLL_NORMAL, SCROLL_DOUBLE, SCROLL_TRIPLE, SCROLL_FASTEST] as const
export type ScrollSpeed = (typeof SCROLL_SPEED_CHOICES)[number]

export type SettingsPreferences = {
  accent: 'teal' | 'blue' | 'violet'
  spinner: SpinnerStyle
  restoreTabs: boolean
  reduceMotion: boolean
  animateSidebarActivity: boolean
  /** The desktop sidebar is collapsed to its rail. */
  compactSidebar: boolean
  toastPosition: ToastPosition
  notifyNeedsYou: boolean
  notifyFinished: boolean
  /** A session's program exited with an error (a native notification, while the window is in the background). */
  notifyError: boolean
  notificationSound: boolean
  terminalFontSize: number
  /** One family name for the terminal text; empty uses the bundled Geist Mono. */
  terminalFontFamily: string
  terminalFontWeight: TerminalFontWeight
  terminalFontWeightBold: TerminalFontWeight
  /** Where a new session starts. Empty: your home folder (ADR 0046). */
  startFolder: string
  /** How the sidebar groups sessions: by git repository, or by the folder they are in. */
  groupBy: 'repo' | 'folder'
  /** Which theme is in use: your dark one, your light one, or whichever the system asks for. */
  themeMode: ThemeMode
  /** The theme names used in dark and in light (Settings › Appearance). */
  darkTheme: string
  lightTheme: string
  cursorStyle: CursorStyle
  /** Lines each pane keeps; maittyd keeps the full history. */
  scrollback: ScrollbackLines
  scrollSpeed: ScrollSpeed
  /** Mirror the terminal's text into the page for screen readers. It makes every pane much heavier to draw. */
  terminalScreenReader: boolean
  /** Status effects on sidebar rows and tabs, each off by itself; Reduce motion turns all off. */
  effectWorking: boolean
  effectNeedsYou: boolean
  effectDone: boolean
  /** Waves or Sweep for each status; the animation level can only make it cheaper. */
  effectStyleWorking: EffectStyle
  effectStyleNeedsYou: EffectStyle
  effectStyleDone: EffectStyle
  workingScope: WorkingScope
  /** How much the status effects may move: Auto measures, the others fix the level. Reduce motion wins over all of them. */
  animations: AnimationMode
  density: Density
  /** Text size of the sidebar's folder names and session rows, in px. */
  sidebarTextSize: number
  /** Text size of the tabs, in px. */
  tabTextSize: number
  /** The docked sidebar's width in px; dragged with the mouse. */
  sidebarWidth: number
  sidebarSide: SidebarSide
  iconSize: IconSize
  trayStyle: TrayStyle
  leadMode: LeadMode
  style: StyleChoice
  iconColor: IconColor
  /** The bottom line of the window: daemon, branch, counts and the focused session. */
  statusBar: boolean
  /** A small readout of frames and process load over the window, for finding what lags. */
  perfOverlay: boolean
  /** The card shows behind a pane's text (Glass with WebGL); programs' own backgrounds show as boxes. */
  terminalSeeThrough: boolean
  /** Glass paints two faint tints of the theme over the window ground. Off is a flat ground; the software path is always flat. */
  backgroundGlow: boolean
  /** The terminal cursor blinks while its pane is focused. xterm drives it, so it is off by default. */
  cursorBlink: boolean
  /** Fetch a new release in the background. Installing still waits for a restart. */
  autoDownloadUpdates: boolean
  /** Restart into a downloaded update when no session is working or waiting on you. */
  /** Install a downloaded update while the app is idle. Kept under its old key so a stored choice carries over. */
  restartWhenIdle: boolean
  /** Monthly spend in US dollars the Usage page's ring measures against; null is no budget (the default). Kept here because the daemon serves none. */
  monthlyBudget: number | null
}

/** Large enough for any real budget; a typo of extra digits is refused rather than stored. */
export const MONTHLY_BUDGET_MAX = 1_000_000

/** A budget from storage or the page: a positive amount in range, else none. */
export function validBudget(value: unknown): number | null {
  if (typeof value !== 'number' || !Number.isFinite(value)) return null
  return value > 0 && value <= MONTHLY_BUDGET_MAX ? value : null
}

export const TERMINAL_FONT_SIZE_MIN = 12
export const TERMINAL_FONT_SIZE_MAX = 16
export const SIDEBAR_WIDTH_MIN = 200
export const SIDEBAR_WIDTH_MAX = 420
export const SIDEBAR_WIDTH_DEFAULT = 264
/** The sidebar and tab text sizes the controls offer, in px; any size in range that was saved is kept. */
export const UI_TEXT_SIZE_SMALL = 12
export const UI_TEXT_SIZE_DEFAULT = 13.5
export const UI_TEXT_SIZE_LARGE = 15
const UI_TEXT_SIZE_MIN = 11
const UI_TEXT_SIZE_MAX = 17

const STORAGE_KEY = 'maitty.gui.settings.v1'

// The theme was stored on its own before the desktop theme browser; carry that choice over.
const THEME_FROM_PREFERENCE: Record<ThemePreference, string> = { night: 'Midnight', abyss: 'Abyss', day: 'Paper' }

function defaults(): SettingsPreferences {
  const legacy = THEME_FROM_PREFERENCE[readThemePreference()]
  return {
    accent: 'teal',
    spinner: 'braille',
    restoreTabs: true,
    reduceMotion: false,
    animateSidebarActivity: true,
    compactSidebar: false,
    toastPosition: 'top-right',
    notifyNeedsYou: true,
    notifyFinished: true,
    notifyError: true,
    notificationSound: false,
    terminalFontSize: 13,
    terminalFontFamily: '',
    terminalFontWeight: DEFAULT_FONT_WEIGHT,
    terminalFontWeightBold: DEFAULT_FONT_WEIGHT_BOLD,
    startFolder: '',
    groupBy: 'repo',
    // The theme chosen before there were two slots keeps its look; otherwise follow the system.
    themeMode: legacy === 'Paper' ? 'light' : legacy === 'Abyss' ? 'dark' : 'system',
    darkTheme: legacy === 'Abyss' ? 'Abyss' : 'Midnight',
    lightTheme: 'Paper',
    cursorStyle: 'block',
    scrollback: SCROLLBACK_DEFAULT,
    scrollSpeed: SCROLL_NORMAL,
    terminalScreenReader: false,
    style: 'glass',
    // Glass runs the status sweeps; Flat and Tiled start with them off. A choice made in Settings wins either way.
    effectWorking: true,
    effectNeedsYou: true,
    effectDone: true,
    effectStyleWorking: 'waves',
    effectStyleNeedsYou: 'sweep',
    effectStyleDone: 'sweep',
    workingScope: 'all',
    animations: 'auto',
    density: 'comfortable',
    sidebarTextSize: UI_TEXT_SIZE_DEFAULT,
    tabTextSize: UI_TEXT_SIZE_DEFAULT,
    sidebarWidth: SIDEBAR_WIDTH_DEFAULT,
    sidebarSide: 'left',
    iconSize: 'medium',
    trayStyle: 'stacked',
    leadMode: 'focused',
    iconColor: 'color',
    statusBar: true,
    perfOverlay: false,
    terminalSeeThrough: false,
    backgroundGlow: false,
    cursorBlink: false,
    autoDownloadUpdates: true,
    restartWhenIdle: true,
    monthlyBudget: null,
  }
}

const bool = (value: unknown, fallback: boolean) => (typeof value === 'boolean' ? value : fallback)

/** Whether any settings were ever saved: a window with none has never been used. */
export function hasSavedSettings(): boolean {
  return readStored() !== null
}

/** Browser storage can be blocked; that reads as nothing saved, and the app still starts. */
function readStored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

export function readSettingsPreferences(): SettingsPreferences {
  const fresh = defaults()
  const stored = readStored()
  if (stored === null) return fresh
  try {
    const value: unknown = JSON.parse(stored)
    if (typeof value !== 'object' || value === null) return fresh
    const preferences = value as Partial<Record<keyof SettingsPreferences, unknown>>
    const fallback = fresh
    return {
      accent: isAccent(preferences.accent) ? preferences.accent : fallback.accent,
      spinner: isSpinnerStyle(preferences.spinner) ? preferences.spinner : fallback.spinner,
      restoreTabs: bool(preferences.restoreTabs, fallback.restoreTabs),
      reduceMotion: bool(preferences.reduceMotion, fallback.reduceMotion),
      animateSidebarActivity: bool(preferences.animateSidebarActivity, fallback.animateSidebarActivity),
      compactSidebar: bool(preferences.compactSidebar, fallback.compactSidebar),
      toastPosition: isToastPosition(preferences.toastPosition) ? preferences.toastPosition : fallback.toastPosition,
      notifyNeedsYou: bool(preferences.notifyNeedsYou, fallback.notifyNeedsYou),
      notifyFinished: bool(preferences.notifyFinished, fallback.notifyFinished),
      notifyError: bool(preferences.notifyError, fallback.notifyError),
      notificationSound: bool(preferences.notificationSound, fallback.notificationSound),
      terminalFontSize: terminalFontSize(preferences.terminalFontSize, fallback.terminalFontSize),
      terminalFontFamily: isFontFamily(preferences.terminalFontFamily) ? preferences.terminalFontFamily.trim() : fallback.terminalFontFamily,
      terminalFontWeight: isFontWeight(preferences.terminalFontWeight) ? preferences.terminalFontWeight : fallback.terminalFontWeight,
      terminalFontWeightBold: isFontWeight(preferences.terminalFontWeightBold) ? preferences.terminalFontWeightBold : fallback.terminalFontWeightBold,
      startFolder: typeof preferences.startFolder === 'string' ? preferences.startFolder.trim() : fallback.startFolder,
      groupBy: preferences.groupBy === 'folder' || preferences.groupBy === 'repo' ? preferences.groupBy : fallback.groupBy,
      ...themeChoice(preferences, fallback),
      cursorStyle: isCursorStyle(preferences.cursorStyle) ? preferences.cursorStyle : fallback.cursorStyle,
      scrollback: isScrollback(preferences.scrollback) ? preferences.scrollback : fallback.scrollback,
      scrollSpeed: isScrollSpeed(preferences.scrollSpeed) ? preferences.scrollSpeed : fallback.scrollSpeed,
      terminalScreenReader: bool(preferences.terminalScreenReader, fallback.terminalScreenReader),
      effectWorking: bool(preferences.effectWorking, fallback.effectWorking),
      effectNeedsYou: bool(preferences.effectNeedsYou, fallback.effectNeedsYou),
      effectDone: bool(preferences.effectDone, fallback.effectDone),
      effectStyleWorking: effectStyle(preferences.effectStyleWorking, fallback.effectStyleWorking),
      effectStyleNeedsYou: effectStyle(preferences.effectStyleNeedsYou, fallback.effectStyleNeedsYou),
      effectStyleDone: effectStyle(preferences.effectStyleDone, fallback.effectStyleDone),
      workingScope: preferences.workingScope === 'all' || preferences.workingScope === 'selected' || preferences.workingScope === 'off' ? preferences.workingScope : fallback.workingScope,
      animations: isAnimationMode(preferences.animations) ? preferences.animations : fallback.animations,
      density: isDensity(preferences.density) ? preferences.density : fallback.density,
      sidebarTextSize: clampNumber(preferences.sidebarTextSize, UI_TEXT_SIZE_MIN, UI_TEXT_SIZE_MAX, fallback.sidebarTextSize),
      tabTextSize: clampNumber(preferences.tabTextSize, UI_TEXT_SIZE_MIN, UI_TEXT_SIZE_MAX, fallback.tabTextSize),
      sidebarWidth: clampNumber(preferences.sidebarWidth, SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX, fallback.sidebarWidth),
      sidebarSide: preferences.sidebarSide === 'left' || preferences.sidebarSide === 'right' ? preferences.sidebarSide : fallback.sidebarSide,
      iconSize: preferences.iconSize === 'small' || preferences.iconSize === 'medium' || preferences.iconSize === 'large' ? preferences.iconSize : fallback.iconSize,
      trayStyle: preferences.trayStyle === 'names' || preferences.trayStyle === 'icons' || preferences.trayStyle === 'stacked' ? preferences.trayStyle : fallback.trayStyle,
      style: styleOf(preferences.style) ?? fallback.style,
      statusBar: bool(preferences.statusBar, fallback.statusBar),
      perfOverlay: bool(preferences.perfOverlay, fallback.perfOverlay),
      terminalSeeThrough: bool(preferences.terminalSeeThrough, fallback.terminalSeeThrough),
      backgroundGlow: bool(preferences.backgroundGlow, fallback.backgroundGlow),
      cursorBlink: bool(preferences.cursorBlink, fallback.cursorBlink),
      autoDownloadUpdates: bool(preferences.autoDownloadUpdates, fallback.autoDownloadUpdates),
      restartWhenIdle: bool(preferences.restartWhenIdle, fallback.restartWhenIdle),
      monthlyBudget: validBudget(preferences.monthlyBudget),
      iconColor: preferences.iconColor === 'color' || preferences.iconColor === 'grayscale' ? preferences.iconColor : fallback.iconColor,
      leadMode: preferences.leadMode === 'focused' || preferences.leadMode === 'attention' ? preferences.leadMode : fallback.leadMode,
    }
  } catch {
    console.warn('Ignoring invalid Maitty settings')
    return fresh
  }
}

export function writeSettingsPreferences(preferences: SettingsPreferences): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
}

function isToastPosition(value: unknown): value is ToastPosition {
  return value === 'top-left' || value === 'top-center' || value === 'top-right' || value === 'bottom-left' || value === 'bottom-center' || value === 'bottom-right'
}

// Sizes saved before the 12–16 range are kept as the nearest size in it.
function terminalFontSize(value: unknown, fallback: number): number {
  if (typeof value !== 'number' || !Number.isInteger(value)) return fallback
  return Math.min(TERMINAL_FONT_SIZE_MAX, Math.max(TERMINAL_FONT_SIZE_MIN, value))
}

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, value))
}

function effectStyle(value: unknown, fallback: EffectStyle): EffectStyle {
  return value === 'waves' || value === 'sweep' ? value : fallback
}

function isAnimationMode(value: unknown): value is AnimationMode {
  return value === 'auto' || value === 'full' || value === 'light' || value === 'off'
}

function isDensity(value: unknown): value is Density {
  return value === 'compact' || value === 'comfortable' || value === 'spacious'
}

function isAccent(value: unknown): value is SettingsPreferences['accent'] {
  return value === 'teal' || value === 'blue' || value === 'violet'
}

/** "minimal" and "expressive" were the names before the window styles; they read as Flat and Glass. */
function styleOf(value: unknown): StyleChoice | null {
  if (value === 'flat' || value === 'tiled' || value === 'glass') return value
  if (value === 'minimal') return 'flat'
  if (value === 'expressive') return 'glass'
  return null
}

const isThemeMode = (value: unknown): value is ThemeMode => value === 'system' || value === 'dark' || value === 'light'

/** A saved theme name, if it is a known theme of the wanted kind. */
const themeOfKind = (value: unknown, light: boolean): string | null =>
  typeof value === 'string' && THEMES.some((theme) => theme.name === value && theme.light === light) ? value : null

/** The mode and both slots. Settings saved with one theme keep it: it fills its own slot and the mode pins to it. */
function themeChoice(saved: Partial<Record<string, unknown>>, fallback: SettingsPreferences): Pick<SettingsPreferences, 'themeMode' | 'darkTheme' | 'lightTheme'> {
  const old = typeof saved.theme === 'string' ? THEMES.find((theme) => theme.name === saved.theme) : undefined
  const migrating = old !== undefined && !('themeMode' in saved)
  return {
    themeMode: isThemeMode(saved.themeMode) ? saved.themeMode : migrating ? (old.light ? 'light' : 'dark') : fallback.themeMode,
    darkTheme: themeOfKind(saved.darkTheme, false) ?? (migrating && !old.light ? old.name : fallback.darkTheme),
    lightTheme: themeOfKind(saved.lightTheme, true) ?? (migrating && old.light ? old.name : fallback.lightTheme),
  }
}

function isCursorStyle(value: unknown): value is CursorStyle {
  return value === 'block' || value === 'bar' || value === 'underline'
}

function isScrollSpeed(value: unknown): value is ScrollSpeed {
  return SCROLL_SPEED_CHOICES.some((choice) => choice === value)
}

function isScrollback(value: unknown): value is ScrollbackLines {
  return SCROLLBACK_CHOICES.some((choice) => choice === value)
}
