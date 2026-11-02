import { EMPTY_LAYOUT, parseLayout, type Layout } from './layout'

const STORAGE_KEY = 'maitty.gui.layout.v1'

/** The tabs and splits from last time. Sessions are the daemon's; only the arrangement is local. */
export function readLayout(): Layout {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    return stored === null ? EMPTY_LAYOUT : parseLayout(JSON.parse(stored))
  } catch (error) {
    console.warn('Ignoring the saved tab layout', error)
    return EMPTY_LAYOUT
  }
}

export function writeLayout(layout: Layout): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout))
  } catch (error) {
    console.warn('Could not save the tab layout', error)
  }
}
