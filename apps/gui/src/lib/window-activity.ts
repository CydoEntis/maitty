import { useSyncExternalStore } from 'react'

/**
 * Whether the window can be seen: it is not minimized or hidden. Spinners keep turning while it is,
 * even with focus elsewhere, because the point of them is to be glanced at from another window.
 */
export function isWindowShown(): boolean {
  return !document.hidden
}

/**
 * Whether the window is shown and in front: the document is visible and has focus. The cursor blink
 * and the idle check for an update follow this, since they only matter to someone typing here.
 */
export function isWindowActive(): boolean {
  return isWindowShown() && document.hasFocus()
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
