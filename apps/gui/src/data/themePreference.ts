const STORAGE_KEY = 'maitty.gui.theme.v1'

export type ThemePreference = 'night' | 'abyss' | 'day'

const DEFAULT_THEME: ThemePreference = 'night'

export function readThemePreference(): ThemePreference {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === null) return DEFAULT_THEME
  if (stored === 'night' || stored === 'abyss' || stored === 'day') return stored

  console.warn('Ignoring unsupported Maitty theme preference')
  return DEFAULT_THEME
}

export function writeThemePreference(theme: ThemePreference): void {
  localStorage.setItem(STORAGE_KEY, theme)
}
