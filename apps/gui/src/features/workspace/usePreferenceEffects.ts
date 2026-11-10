import { useEffect } from 'react'
import { toast } from '@/lib/toast'
import { type SettingsPreferences, writeSettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import { markGpu } from '@/lib/gpu'
import { isWindowShown, subscribeWindowActivity } from '@/lib/window-activity'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { finishOf } from './theme/windowStyles'

/** Saves the settings and marks them on the page, where the stylesheet reads them. */
export function usePreferenceEffects({ preferences, reducedMotion }: {
  preferences: SettingsPreferences
  reducedMotion: boolean
}) {
  useEffect(() => {
    try {
      writeSettingsPreferences(preferences)
    } catch (error) {
      toast.error(errorMessage(error, 'Could not save settings'))
    }
    setSpinnerStyle(preferences.spinner)
  }, [preferences])

  useEffect(() => {
    document.documentElement.dataset.reducedMotion = String(reducedMotion)
  }, [reducedMotion])

  useEffect(() => { markGpu() }, [])

  // Looping effects pause in the stylesheet while the window is hidden or in the background. It is an
  // attribute on <html>, not React state, so a focus change repaints nothing and re-renders nothing.
  useEffect(() => {
    const mark = () => { document.documentElement.dataset.window = isWindowShown() ? 'active' : 'inactive' }
    mark()
    return subscribeWindowActivity(mark)
  }, [])

  useEffect(() => {
    document.documentElement.dataset.style = preferences.style
    document.documentElement.dataset.finish = finishOf(preferences.style)
  }, [preferences.style])

  useEffect(() => {
    document.documentElement.dataset.glow = preferences.backgroundGlow ? 'on' : 'off'
  }, [preferences.backgroundGlow])

  useEffect(() => {
    document.documentElement.dataset.icons = preferences.iconColor
  }, [preferences.iconColor])
}
