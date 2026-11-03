import { useSyncExternalStore } from 'react'

/**
 * Whether the window is shown and in front: the document is visible and has focus. Spinners and
 * the cursor blink stop while it is not, since nobody sees them and every frame costs a repaint.
 */
export function isWindowActive(): boolean {
  return !document.hidden && document.hasFocus()
}

/** Calls `listener` whenever the window may have changed between active and not. */
export function subscribeWindowActivity(listener: () => void): () => void {
  window.addEventListener('focus', listener)
  window.addEventListener('blur', listener)
  document.addEventListener('visibilitychange', listener)
  return () => {
    window.removeEventListener('focus', listener)
    window.removeEventListener('blur', listener)
    document.removeEventListener('visibilitychange', listener)
  }
}

export function useWindowActive(): boolean {
  return useSyncExternalStore(subscribeWindowActivity, isWindowActive, () => true)
}
