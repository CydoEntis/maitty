import { isSpinnerStyle, type SpinnerStyle } from '@/lib/spinner-clock'
import { DEFAULT_WINDOW_STYLE, isWindowStyle, type WindowStyle } from '@/features/workspace/theme/windowStyles'
import { DEFAULT_FONT_WEIGHT, DEFAULT_FONT_WEIGHT_BOLD, isFontFamily, isFontWeight, type TerminalFontWeight } from '@/lib/terminalFont'
import { readThemePreference, type ThemePreference } from '@/data/themePreference'
import { DEFAULT_LIGHT_THEME, DEFAULT_THEME, THEMES, type ThemeMode } from '@/features/workspace/theme/themes'
import { DEFAULT_CHAT_VIEWS, type ChatAgent, type ChatViewDefaults, type PaneView } from '@/features/workspace/model/chatView'

export type ToastPosition = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right'
export type CursorStyle = 'block' | 'bar' | 'underline'
/** How much room the sidebar's rows, the tabs and the panes get. */
export type Density = 'compact' | 'comfortable' | 'spacious'
export const DENSITIES: readonly Density[] = ['compact', 'comfortable', 'spacious']
/** How big the sidebar's and the rail's icons and tiles are. */
export type IconSize = 'small' | 'medium' | 'large'
/** How wide the chat's column of messages may be: sized to the pane, a fixed narrow or wide column, or the whole pane. */
export type ChatWidth = 'auto' | 'narrow' | 'wide' | 'full'
export const CHAT_WIDTHS: readonly ChatWidth[] = ['auto', 'narrow', 'wide', 'full']
/** The sizes the chat's text can be drawn at, as a percentage of its design size. */
const CHAT_TEXT_SMALL = 90
export const CHAT_TEXT_SIZE_DEFAULT = 100
const CHAT_TEXT_LARGE = 110
const CHAT_TEXT_LARGER = 125
const CHAT_TEXT_LARGEST = 140
export const CHAT_TEXT_SIZES: readonly number[] = [CHAT_TEXT_SMALL, CHAT_TEXT_SIZE_DEFAULT, CHAT_TEXT_LARGE, CHAT_TEXT_LARGER, CHAT_TEXT_LARGEST]
/** Which side of the window the sidebar sits on. */
export type SidebarSide = 'left' | 'right'
/** How the sidebar orders sessions: as they were created (rows never move), or with what needs you first. */
export type SidebarOrder = 'creation' | 'priority'
/** What the sidebar's headings are: what each session is doing, its project, or its machine. */
export type SidebarGroup = 'status' | 'project' | 'machine'
/** Where an agent's own terminals show: a tab of the side panel, or a drawer under its chat. */
export type TerminalDock = 'panel' | 'under'
/** The window style: one of four looks that share one layout (`theme/windowStyles.ts`). Kept by this window alone. */
export type StyleChoice = WindowStyle
/** App and folder icons keep their colours, or drop to neutral gray tiles. Independent of the style. */
export type IconColor = 'color' | 'grayscale'

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
  /** Reopen the panes and splits of last time at launch. Saved as `restoreTabs` before fleets; that name is still read. */
  restoreLayout: boolean
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
  /** Where a new session starts. Empty: your home folder. */
  startFolder: string
  /** Where a block saved from a chat goes; empty is the Downloads folder. */
  exportFolder: string
  /** How the sidebar groups sessions: by git repository, or by the folder they are in. */
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
  density: Density
  /** Text size of the sidebar's folder names and session rows, in px. */
  sidebarTextSize: number
  /** Text size of the tabs, in px. */
  tabTextSize: number
  /** The docked sidebar's width in px; dragged with the mouse. */
  sidebarWidth: number
  sidebarSide: SidebarSide
  sidebarOrder: SidebarOrder
  sidebarGroup: SidebarGroup
  terminalDock: TerminalDock
  iconSize: IconSize
  style: StyleChoice
  iconColor: IconColor
  /** How wide the chat's messages run; `auto` grows with the pane, so a large screen is used. */
  chatWidth: ChatWidth
  /** The size of everything in the chat, as a percentage (`CHAT_TEXT_SIZES`). */
  chatTextSize: number
  /** Which view a Claude or Codex session opens in. Terminal until you change it. */
  chatViewDefault: ChatViewDefaults
  /** The agent a new session starts. */
  defaultAgent: ChatAgent
  /** What a new session opens as: a chat with the default agent, or that agent typed into a new shell. */
  newSessionView: PaneView
  /** A new chat starts with the agent's full-access mode (no sandbox, no permission prompts). Off until you turn it on. */
  chatFullAccessDefault: boolean
  /** Yolo mode per agent, off by default: Claude may pick Bypass permissions; Codex and Gemini start with approvals and sandbox off. */
  yoloClaude: boolean
  yoloCodex: boolean
  yoloGemini: boolean
  /** The bottom line of the window: daemon, branch, counts and the focused session. */
  statusBar: boolean
  /** A small readout of frames and process load over the window, for finding what lags. */
  perfOverlay: boolean
  /** Draw terminals with xterm's DOM renderer instead of WebGL; for a web view that shows typed text late. */
  terminalPlainDrawing: boolean
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
export const SIDEBAR_WIDTH_MIN = 208
export const SIDEBAR_WIDTH_MAX = 440
export const SIDEBAR_WIDTH_DEFAULT = 264
/** The sidebar and tab text sizes the controls offer, in px; any size in range that was saved is kept. */
export const UI_TEXT_SIZE_SMALL = 12
export const UI_TEXT_SIZE_DEFAULT = 13.5
export const UI_TEXT_SIZE_LARGE = 15
const UI_TEXT_SIZE_MIN = 11
const UI_TEXT_SIZE_MAX = 17

const STORAGE_KEY = 'maitty.gui.settings.v1'

// The theme was stored on its own before the desktop theme browser; carry that choice over.
const THEME_FROM_PREFERENCE: Record<ThemePreference, string> = { night: DEFAULT_THEME, abyss: DEFAULT_THEME, day: DEFAULT_LIGHT_THEME }

function defaults(): SettingsPreferences {
  const legacy = THEME_FROM_PREFERENCE[readThemePreference()]
  return {
    accent: 'teal',
    spinner: 'braille',
    restoreLayout: true,
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
    exportFolder: '',
    // The theme chosen before there were two slots keeps its look; otherwise follow the system.
    themeMode: legacy === DEFAULT_LIGHT_THEME ? 'light' : 'system',
    darkTheme: DEFAULT_THEME,
    lightTheme: DEFAULT_LIGHT_THEME,
    cursorStyle: 'block',
    scrollback: SCROLLBACK_DEFAULT,
    scrollSpeed: SCROLL_NORMAL,
    terminalScreenReader: false,
    style: DEFAULT_WINDOW_STYLE,
    density: 'comfortable',
    sidebarTextSize: UI_TEXT_SIZE_DEFAULT,
    tabTextSize: UI_TEXT_SIZE_DEFAULT,
    sidebarWidth: SIDEBAR_WIDTH_DEFAULT,
    sidebarSide: 'left',
    sidebarOrder: 'creation',
    sidebarGroup: 'status',
    terminalDock: 'panel',
    iconSize: 'medium',
    iconColor: 'color',
    chatWidth: 'auto',
    chatTextSize: CHAT_TEXT_SIZE_DEFAULT,
    chatViewDefault: DEFAULT_CHAT_VIEWS,
    defaultAgent: 'claude',
    newSessionView: 'chat',
    chatFullAccessDefault: false,
    yoloClaude: false,
    yoloCodex: false,
    yoloGemini: false,
    statusBar: false,
    perfOverlay: false,
    // WebGL everywhere. The late letters it once showed on Linux came from a page drawn on the CPU, which the host now prevents (docs/PERFORMANCE.md).
    terminalPlainDrawing: false,
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
    const saved = value as { restoreTabs?: unknown }
    return {
      accent: isAccent(preferences.accent) ? preferences.accent : fallback.accent,
      spinner: isSpinnerStyle(preferences.spinner) ? preferences.spinner : fallback.spinner,
      restoreLayout: bool(preferences.restoreLayout ?? saved.restoreTabs, fallback.restoreLayout),
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
      exportFolder: typeof preferences.exportFolder === 'string' ? preferences.exportFolder.trim() : fallback.exportFolder,
      ...themeChoice(preferences, fallback),
      cursorStyle: isCursorStyle(preferences.cursorStyle) ? preferences.cursorStyle : fallback.cursorStyle,
      scrollback: isScrollback(preferences.scrollback) ? preferences.scrollback : fallback.scrollback,
      scrollSpeed: isScrollSpeed(preferences.scrollSpeed) ? preferences.scrollSpeed : fallback.scrollSpeed,
      terminalScreenReader: bool(preferences.terminalScreenReader, fallback.terminalScreenReader),
      density: isDensity(preferences.density) ? preferences.density : fallback.density,
      sidebarTextSize: clampNumber(preferences.sidebarTextSize, UI_TEXT_SIZE_MIN, UI_TEXT_SIZE_MAX, fallback.sidebarTextSize),
      tabTextSize: clampNumber(preferences.tabTextSize, UI_TEXT_SIZE_MIN, UI_TEXT_SIZE_MAX, fallback.tabTextSize),
      sidebarWidth: clampNumber(preferences.sidebarWidth, SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX, fallback.sidebarWidth),
      sidebarSide: preferences.sidebarSide === 'left' || preferences.sidebarSide === 'right' ? preferences.sidebarSide : fallback.sidebarSide,
      sidebarOrder: preferences.sidebarOrder === 'creation' || preferences.sidebarOrder === 'priority' ? preferences.sidebarOrder : fallback.sidebarOrder,
      sidebarGroup: preferences.sidebarGroup === 'status' || preferences.sidebarGroup === 'project' || preferences.sidebarGroup === 'machine' ? preferences.sidebarGroup : fallback.sidebarGroup,
      terminalDock: preferences.terminalDock === 'panel' || preferences.terminalDock === 'under' ? preferences.terminalDock : fallback.terminalDock,
      iconSize: preferences.iconSize === 'small' || preferences.iconSize === 'medium' || preferences.iconSize === 'large' ? preferences.iconSize : fallback.iconSize,
      style: styleOf(preferences.style) ?? fallback.style,
      chatWidth: CHAT_WIDTHS.find((width) => width === preferences.chatWidth) ?? fallback.chatWidth,
      chatTextSize: CHAT_TEXT_SIZES.find((size) => size === preferences.chatTextSize) ?? fallback.chatTextSize,
      chatViewDefault: chatViewDefault(preferences.chatViewDefault, fallback.chatViewDefault),
      defaultAgent: preferences.defaultAgent === 'claude' || preferences.defaultAgent === 'codex' ? preferences.defaultAgent : fallback.defaultAgent,
      newSessionView: paneView(preferences.newSessionView, fallback.newSessionView),
      chatFullAccessDefault: bool(preferences.chatFullAccessDefault, fallback.chatFullAccessDefault),
      yoloClaude: bool(preferences.yoloClaude, fallback.yoloClaude),
      yoloCodex: bool(preferences.yoloCodex, fallback.yoloCodex),
      yoloGemini: bool(preferences.yoloGemini, fallback.yoloGemini),
      statusBar: bool(preferences.statusBar, fallback.statusBar),
      perfOverlay: bool(preferences.perfOverlay, fallback.perfOverlay),
      terminalPlainDrawing: bool(preferences.terminalPlainDrawing, fallback.terminalPlainDrawing),
      terminalSeeThrough: bool(preferences.terminalSeeThrough, fallback.terminalSeeThrough),
      backgroundGlow: bool(preferences.backgroundGlow, fallback.backgroundGlow),
      cursorBlink: bool(preferences.cursorBlink, fallback.cursorBlink),
      autoDownloadUpdates: bool(preferences.autoDownloadUpdates, fallback.autoDownloadUpdates),
      restartWhenIdle: bool(preferences.restartWhenIdle, fallback.restartWhenIdle),
      monthlyBudget: validBudget(preferences.monthlyBudget),
      iconColor: preferences.iconColor === 'color' || preferences.iconColor === 'grayscale' ? preferences.iconColor : fallback.iconColor,
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

function isDensity(value: unknown): value is Density {
  return value === 'compact' || value === 'comfortable' || value === 'spacious'
}

function isAccent(value: unknown): value is SettingsPreferences['accent'] {
  return value === 'teal' || value === 'blue' || value === 'violet'
}

/**
 * "minimal" and "expressive" were the names before the window styles; they read as Flat and Glass, and
 * Flat (like the retired Tiled) reads as the Flat style, id `console`: the solid-panel look.
 */
function styleOf(value: unknown): StyleChoice | null {
  if (value === 'flat' || value === 'tiled' || value === 'minimal') return 'console'
  if (value === 'expressive' || value === 'frosted') return 'glass'
  return isWindowStyle(value) ? value : null
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

const paneView = (value: unknown, fallback: PaneView): PaneView => (value === 'chat' || value === 'terminal' ? value : fallback)

function chatViewDefault(value: unknown, fallback: ChatViewDefaults): ChatViewDefaults {
  if (typeof value !== 'object' || value === null) return fallback
  const saved = value as Partial<Record<keyof ChatViewDefaults, unknown>>
  return { claude: paneView(saved.claude, fallback.claude), codex: paneView(saved.codex, fallback.codex), gemini: paneView(saved.gemini, fallback.gemini) }
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
