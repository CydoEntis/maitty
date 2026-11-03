import { hasSavedSettings } from '@/features/settings/preferences'

const KEY = 'maitty.gui.welcomed.v1'

/**
 * The first launch: no settings were ever saved and the welcome was never shown. A window that already
 * has settings from an earlier version is not a first run, so it is never greeted retroactively.
 */
export function isFirstRun(): boolean {
  try {
    return localStorage.getItem(KEY) === null && !hasSavedSettings()
  } catch {
    // Storage is unavailable (a private window): greeting every launch would be worse than never.
    return false
  }
}

export function markWelcomed(): void {
  try {
    localStorage.setItem(KEY, '1')
  } catch (error) {
    console.warn('Could not remember that the welcome was shown', error)
  }
}
