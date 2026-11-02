import type { SpinnerStyle } from '@/lib/spinner-clock'

export type ToastPosition = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right'

export type SettingsPreferences = {
  accent: 'teal' | 'blue' | 'violet'
  spinner: SpinnerStyle
  restoreTabs: boolean
  reduceMotion: boolean
  animateSidebarActivity: boolean
  compactSidebar: boolean
  toastPosition: ToastPosition
  notifyNeedsYou: boolean
  notifyFinished: boolean
  notificationSound: boolean
}

const STORAGE_KEY = 'maitty.gui.settings.v1'
const DEFAULTS: SettingsPreferences = {
  accent: 'teal',
  spinner: 'braille',
  restoreTabs: true,
  reduceMotion: false,
  animateSidebarActivity: true,
  compactSidebar: false,
  toastPosition: 'bottom-right',
  notifyNeedsYou: true,
  notifyFinished: true,
  notificationSound: false,
}

export function readSettingsPreferences(): SettingsPreferences {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === null) return DEFAULTS
  try {
    const value: unknown = JSON.parse(stored)
    if (typeof value !== 'object' || value === null) return DEFAULTS
    const preferences = value as Partial<SettingsPreferences>
    return {
      accent: isAccent(preferences.accent) ? preferences.accent : DEFAULTS.accent,
      spinner: isSpinner(preferences.spinner) ? preferences.spinner : DEFAULTS.spinner,
      restoreTabs: typeof preferences.restoreTabs === 'boolean' ? preferences.restoreTabs : DEFAULTS.restoreTabs,
      reduceMotion: typeof preferences.reduceMotion === 'boolean' ? preferences.reduceMotion : DEFAULTS.reduceMotion,
      animateSidebarActivity: typeof preferences.animateSidebarActivity === 'boolean' ? preferences.animateSidebarActivity : DEFAULTS.animateSidebarActivity,
      compactSidebar: typeof preferences.compactSidebar === 'boolean' ? preferences.compactSidebar : DEFAULTS.compactSidebar,
      toastPosition: isToastPosition(preferences.toastPosition) ? preferences.toastPosition : DEFAULTS.toastPosition,
      notifyNeedsYou: typeof preferences.notifyNeedsYou === 'boolean' ? preferences.notifyNeedsYou : DEFAULTS.notifyNeedsYou,
      notifyFinished: typeof preferences.notifyFinished === 'boolean' ? preferences.notifyFinished : DEFAULTS.notifyFinished,
      notificationSound: typeof preferences.notificationSound === 'boolean' ? preferences.notificationSound : DEFAULTS.notificationSound,
    }
  } catch {
    console.warn('Ignoring invalid Maitty notification preferences')
    return DEFAULTS
  }
}

export function writeSettingsPreferences(preferences: SettingsPreferences): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
}

function isToastPosition(value: unknown): value is ToastPosition {
  return value === 'top-left' || value === 'top-center' || value === 'top-right' || value === 'bottom-left' || value === 'bottom-center' || value === 'bottom-right'
}

function isAccent(value: unknown): value is SettingsPreferences['accent'] {
  return value === 'teal' || value === 'blue' || value === 'violet'
}

function isSpinner(value: unknown): value is SpinnerStyle {
  return value === 'braille' || value === 'snake' || value === 'bars'
}
