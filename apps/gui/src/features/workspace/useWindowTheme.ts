import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { activeThemeName, findTheme, terminalMinimumContrast, terminalSlab, themeStyle, xtermTheme } from './theme/themes'
import { withWindowStyle } from './theme/windowStyles'

/** A theme previews once the pointer rests on it, not as it passes over every row (each one repaints every terminal). */
const THEME_PREVIEW_DELAY_MS = 140

/** The theme in use (or the one being previewed in Settings), put on the page root, and the look every terminal draws with. */
export function useWindowTheme({ preferences, systemReducedMotion, systemDark }: {
  preferences: SettingsPreferences
  systemReducedMotion: boolean
  systemDark: boolean
}) {
  // A theme being hovered in Settings shows on the whole window until the pointer leaves or one is picked.
  const [previewTheme, setPreviewTheme] = useState<string | null>(null)
  const previewTimer = useRef<number | undefined>(undefined)
  const previewThemeSoon = useCallback((name: string | null) => {
    window.clearTimeout(previewTimer.current)
    if (name === null) {
      setPreviewTheme(null)
      return
    }
    previewTimer.current = window.setTimeout(() => { setPreviewTheme(name) }, THEME_PREVIEW_DELAY_MS)
  }, [])
  const reducedMotion = preferences.reduceMotion || systemReducedMotion
  const themeName = previewTheme ?? activeThemeName(preferences, systemDark)
  const theme = useMemo(() => withWindowStyle(findTheme(themeName), preferences.style), [themeName, preferences.style])
  // The theme's tokens are on the page root as well as on the window: a menu or popover that renders outside the window
  // (Radix portals to the page) then reads the same colours, and the shadcn names mapped on `:root` resolve to the theme.
  useEffect(() => {
    const names = Object.keys(themeStyle(theme)).filter((name) => name.startsWith('--'))
    const values = themeStyle(theme)
    for (const name of names) document.documentElement.style.setProperty(name, values[name] ?? '')
    return () => { for (const name of names) document.documentElement.style.removeProperty(name) }
  }, [theme])
  const look = useMemo(() => ({ fontFamily: preferences.terminalFontFamily, fontWeight: preferences.terminalFontWeight, fontWeightBold: preferences.terminalFontWeightBold, cursorStyle: preferences.cursorStyle, scrollback: preferences.scrollback, scrollSpeed: preferences.scrollSpeed, screenReader: preferences.terminalScreenReader, seeThrough: preferences.terminalSeeThrough, cursorBlink: preferences.cursorBlink, plainDrawing: preferences.terminalPlainDrawing, theme: xtermTheme(theme.tokens), slab: terminalSlab(theme.tokens), minimumContrast: terminalMinimumContrast(theme) }), [preferences.terminalFontFamily, preferences.terminalFontWeight, preferences.terminalFontWeightBold, preferences.cursorStyle, preferences.cursorBlink, preferences.scrollback, preferences.scrollSpeed, preferences.terminalScreenReader, preferences.terminalSeeThrough, preferences.terminalPlainDrawing, theme])
  return { reducedMotion, theme, previewThemeSoon, setPreviewTheme, look }
}
