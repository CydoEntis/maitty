import { EMPTY_LAYOUT, parseLayout, type Layout } from './layout'

/**
 * The split layouts of this window (see `layout.ts`). Earlier versions saved tabs under the keys
 * below; a tab layout means nothing in a flat window, so it is dropped once and the person is told.
 */
export const STORAGE_KEY = 'maitty.gui.layout.v3'
const SUPERSEDED_KEYS = ['maitty.gui.layout.v2', 'maitty.gui.layout.v1'] as const

export type SavedLayout = {
  layout: Layout
  /** An older tab layout was found and discarded: say so once. */
  reset: boolean
}

export const LAYOUT_RESET_NOTICE = 'Maitty is one list of sessions now, so your saved tabs were reset. Your sessions are all still here.'

/** The splits from last time. Sessions are the daemon's; only the arrangement is local. */
export function readLayout(): SavedLayout {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored !== null) return { layout: parseLayout(JSON.parse(stored)), reset: false }
    const older = SUPERSEDED_KEYS.filter((key) => localStorage.getItem(key) !== null)
    for (const key of older) localStorage.removeItem(key)
    return { layout: EMPTY_LAYOUT, reset: older.length > 0 }
  } catch (error) {
    console.warn('Ignoring the saved layout', error)
    return { layout: EMPTY_LAYOUT, reset: false }
  }
}

export function writeLayout(layout: Layout): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout))
  } catch (error) {
    console.warn('Could not save the layout', error)
  }
}
