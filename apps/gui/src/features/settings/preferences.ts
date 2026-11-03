import type { SpinnerStyle } from '@/lib/spinner-clock'
import { readThemePreference, type ThemePreference } from '@/data/themePreference'

export type ToastPosition = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right'
export type CursorStyle = 'block' | 'bar' | 'underline'
/** How much room the sidebar's rows, the tabs and the panes get. */
export type Density = 'compact' | 'comfortable' | 'spacious'
export const DENSITIES: readonly Density[] = ['compact', 'comfortable', 'spacious']
/** How big the sidebar's and the rail's icons and tiles are. */
export type IconSize = 'small' | 'medium' | 'large'
/** How a project's other sessions sit under its main row in the sidebar. */
export type TrayStyle = 'names' | 'icons' | 'stacked'
/** Which session leads a project: the one you are in, or the one that most needs you. */
export type LeadMode = 'focused' | 'attention'

const SCROLLBACK_SHORT = 5000
const SCROLLBACK_DEFAULT = 10000
const SCROLLBACK_LONG = 50000
export const SCROLLBACK_CHOICES = [SCROLLBACK_SHORT, SCROLLBACK_DEFAULT, SCROLLBACK_LONG] as const
export type ScrollbackLines = (typeof SCROLLBACK_CHOICES)[number]

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
  /** Where a new session starts. Empty: your home folder (ADR 0046). */
  startFolder: string
  /** How the sidebar groups sessions: by git repository, or by the folder they are in. */
  groupBy: 'repo' | 'folder'
  /** The desktop theme's name (Settings › Appearance). */
  theme: string
  cursorStyle: CursorStyle
  /** Lines each pane keeps; maittyd keeps the full history. */
  scrollback: ScrollbackLines
  /** Status effects on sidebar rows and tabs, each off by itself; Reduce motion turns all off. */
  effectWorking: boolean
  effectNeedsYou: boolean
  effectDone: boolean
  density: Density
  /** Text size of the sidebar's folder names and session rows, in px. */
  sidebarTextSize: number
  /** Text size of the tabs, in px. */
  tabTextSize: number
  /** The docked sidebar's width in px; dragged with the mouse. */
  sidebarWidth: number
  iconSize: IconSize
  trayStyle: TrayStyle
  leadMode: LeadMode
  /** A colour the user gave a project, by the project's key (its repo or folder path). */
  projectColors: Record<string, string>
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
  return {
    accent: 'teal',
    spinner: 'braille',
    restoreTabs: true,
    reduceMotion: false,
    animateSidebarActivity: true,
    compactSidebar: false,
    toastPosition: 'bottom-right',
    notifyNeedsYou: true,
    notifyFinished: true,
    notifyError: true,
    notificationSound: false,
    terminalFontSize: 13,
    startFolder: '',
    groupBy: 'repo',
    theme: THEME_FROM_PREFERENCE[readThemePreference()],
    cursorStyle: 'block',
    scrollback: SCROLLBACK_DEFAULT,
    effectWorking: true,
    effectNeedsYou: true,
    effectDone: true,
    density: 'comfortable',
    sidebarTextSize: UI_TEXT_SIZE_DEFAULT,
    tabTextSize: UI_TEXT_SIZE_DEFAULT,
    sidebarWidth: SIDEBAR_WIDTH_DEFAULT,
    iconSize: 'medium',
    trayStyle: 'names',
    leadMode: 'focused',
    projectColors: {},
  }
}

const bool = (value: unknown, fallback: boolean) => (typeof value === 'boolean' ? value : fallback)

export function readSettingsPreferences(): SettingsPreferences {
  const fallback = defaults()
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === null) return fallback
  try {
    const value: unknown = JSON.parse(stored)
    if (typeof value !== 'object' || value === null) return fallback
    const preferences = value as Partial<Record<keyof SettingsPreferences, unknown>>
    return {
      accent: isAccent(preferences.accent) ? preferences.accent : fallback.accent,
      spinner: isSpinner(preferences.spinner) ? preferences.spinner : fallback.spinner,
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
      startFolder: typeof preferences.startFolder === 'string' ? preferences.startFolder.trim() : fallback.startFolder,
      groupBy: preferences.groupBy === 'folder' || preferences.groupBy === 'repo' ? preferences.groupBy : fallback.groupBy,
      theme: typeof preferences.theme === 'string' && preferences.theme.trim() ? preferences.theme : fallback.theme,
      cursorStyle: isCursorStyle(preferences.cursorStyle) ? preferences.cursorStyle : fallback.cursorStyle,
      scrollback: isScrollback(preferences.scrollback) ? preferences.scrollback : fallback.scrollback,
      effectWorking: bool(preferences.effectWorking, fallback.effectWorking),
      effectNeedsYou: bool(preferences.effectNeedsYou, fallback.effectNeedsYou),
      effectDone: bool(preferences.effectDone, fallback.effectDone),
      density: isDensity(preferences.density) ? preferences.density : fallback.density,
      sidebarTextSize: clampNumber(preferences.sidebarTextSize, UI_TEXT_SIZE_MIN, UI_TEXT_SIZE_MAX, fallback.sidebarTextSize),
      tabTextSize: clampNumber(preferences.tabTextSize, UI_TEXT_SIZE_MIN, UI_TEXT_SIZE_MAX, fallback.tabTextSize),
      sidebarWidth: clampNumber(preferences.sidebarWidth, SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX, fallback.sidebarWidth),
      iconSize: preferences.iconSize === 'small' || preferences.iconSize === 'medium' || preferences.iconSize === 'large' ? preferences.iconSize : fallback.iconSize,
      trayStyle: preferences.trayStyle === 'names' || preferences.trayStyle === 'icons' || preferences.trayStyle === 'stacked' ? preferences.trayStyle : fallback.trayStyle,
      projectColors: stringRecord(preferences.projectColors),
      leadMode: preferences.leadMode === 'focused' || preferences.leadMode === 'attention' ? preferences.leadMode : fallback.leadMode,
    }
  } catch {
    console.warn('Ignoring invalid Maitty settings')
    return fallback
  }
}

export function writeSettingsPreferences(preferences: SettingsPreferences): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
}

function stringRecord(value: unknown): Record<string, string> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return {}
  return Object.fromEntries(Object.entries(value).filter((entry): entry is [string, string] => typeof entry[1] === 'string'))
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

function isDensity(value: unknown): value is Density {
  return value === 'compact' || value === 'comfortable' || value === 'spacious'
}

function isAccent(value: unknown): value is SettingsPreferences['accent'] {
  return value === 'teal' || value === 'blue' || value === 'violet'
}

function isSpinner(value: unknown): value is SpinnerStyle {
  return value === 'braille' || value === 'snake' || value === 'bars'
}

function isCursorStyle(value: unknown): value is CursorStyle {
  return value === 'block' || value === 'bar' || value === 'underline'
}

function isScrollback(value: unknown): value is ScrollbackLines {
  return SCROLLBACK_CHOICES.some((choice) => choice === value)
}
