/** The terminals pinned under a chat (layout 2e): how tall the drawer may be, and what it remembers. */
export const DRAWER_MIN_PX = 120
export const DRAWER_MAX_PX = 640
export const DRAWER_DEFAULT_PX = 250
/** The drawer never takes more than this share of its pane, so the chat keeps the larger part. */
export const DRAWER_MAX_SHARE = 0.6
/** A window shorter than this opens the drawer folded: there is no room for a chat and a terminal. */
export const SHORT_WINDOW_PX = 760

const HEIGHT_KEY = 'maitty.gui.terminal-drawer.height'

/** A height the drawer can have where the pane and it share `room` px. */
export function clampDrawerHeight(height: number, room: number): number {
  const most = Math.max(DRAWER_MIN_PX, Math.min(DRAWER_MAX_PX, Math.floor(room * DRAWER_MAX_SHARE)))
  return Math.round(Math.min(Math.max(height, DRAWER_MIN_PX), most))
}

export function readDrawerHeight(): number {
  try {
    const stored = Number(window.localStorage.getItem(HEIGHT_KEY))
    return Number.isFinite(stored) && stored >= DRAWER_MIN_PX && stored <= DRAWER_MAX_PX ? stored : DRAWER_DEFAULT_PX
  } catch {
    return DRAWER_DEFAULT_PX
  }
}

export function writeDrawerHeight(height: number): void {
  try {
    window.localStorage.setItem(HEIGHT_KEY, String(height))
  } catch {
    // The height is only remembered, never needed.
  }
}

export function startsFolded(): boolean {
  return window.innerHeight < SHORT_WINDOW_PX
}
