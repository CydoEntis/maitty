import { EMPTY_LAYOUT, parseLayout, type Layout } from './layout'

/**
 * v2 may name a pane's session by machine (`build/t12`); v1 never did. Every v1 id (`t12`, a
 * chat's number) means the same in v2, so a v1 layout is read as it is and kept under the v2 key
 * from then on. The v1 entry is left alone, so an older app still finds its own.
 */
export const STORAGE_KEY = 'maitty.gui.layout.v2'
export const STORAGE_KEY_V1 = 'maitty.gui.layout.v1'

/** A layout saved by v1, as v2 reads it. Nothing in it names a machine, so every session is this computer's. */
export function migrateLayoutV1(saved: unknown): Layout {
  return parseLayout(saved)
}

/** The tabs and splits from last time. Sessions are the daemon's; only the arrangement is local. */
export function readLayout(): Layout {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored !== null) return parseLayout(JSON.parse(stored))
    const before = localStorage.getItem(STORAGE_KEY_V1)
    if (before === null) return EMPTY_LAYOUT
    const migrated = migrateLayoutV1(JSON.parse(before))
    localStorage.setItem(STORAGE_KEY, JSON.stringify(migrated))
    return migrated
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
