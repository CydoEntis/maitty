import { useCallback, useState } from 'react'

/** The docked panel's open state is remembered for the window; the overlay's is not (closed by default, v18 C.2). */
export const PANEL_STORAGE_KEY = 'maitty.gui.sessionPanel.v1'

function readDocked(): boolean {
  try {
    return localStorage.getItem(PANEL_STORAGE_KEY) === 'open'
  } catch {
    return false
  }
}

function writeDocked(open: boolean): void {
  try {
    localStorage.setItem(PANEL_STORAGE_KEY, open ? 'open' : 'closed')
  } catch (error) {
    console.warn('Could not remember the session panel', error)
  }
}

export type SessionPanelState = {
  /** The panel is on screen: docked on a wide window, an overlay on a compact one. */
  open: boolean
  mode: 'docked' | 'overlay'
  toggle: () => void
  close: () => void
}

/**
 * Whether the session panel is open. A wide window docks it and remembers the choice; a compact
 * one (under 1100px) shows it as an overlay that starts closed and is not remembered, so narrowing
 * the window never opens a panel over the panes by itself.
 */
export function useSessionPanelState(compact: boolean): SessionPanelState {
  const [docked, setDocked] = useState(readDocked)
  const [overlay, setOverlay] = useState(false)
  const toggle = useCallback(() => {
    if (compact) {
      setOverlay((open) => !open)
      return
    }
    setDocked(!docked)
    writeDocked(!docked)
  }, [compact, docked])
  const close = useCallback(() => {
    if (compact) {
      setOverlay(false)
      return
    }
    setDocked(false)
    writeDocked(false)
  }, [compact])
  return { open: compact ? overlay : docked, mode: compact ? 'overlay' : 'docked', toggle, close }
}
