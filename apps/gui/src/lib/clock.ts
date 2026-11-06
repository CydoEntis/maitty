import { useLayoutEffect, useState, useSyncExternalStore } from 'react'
import { isWindowShown, subscribeWindowActivity } from './window-activity'

export const CLOCK_TICK_MS = 1000

/**
 * One shared second clock for every piece of text that shows a time. Only the components that
 * show one subscribe, so a tick re-renders them and not the window around them. It runs while
 * something is subscribed and the window is in front.
 */
const listeners = new Set<() => void>()
let current = Date.now()
let timer: ReturnType<typeof setInterval> | undefined
let unsubscribeWindow: (() => void) | undefined

function tick() {
  current = Date.now()
  for (const listener of listeners) listener()
}

function syncTimer() {
  const shouldRun = listeners.size > 0 && isWindowShown()
  if (shouldRun && timer === undefined) {
    // Coming back to the front, the stale minutes catch up at once rather than a second later.
    tick()
    timer = setInterval(tick, CLOCK_TICK_MS)
  }
  if (!shouldRun && timer !== undefined) {
    clearInterval(timer)
    timer = undefined
  }
}

function subscribe(listener: () => void): () => void {
  if (listeners.size === 0) current = Date.now()
  listeners.add(listener)
  unsubscribeWindow ??= subscribeWindowActivity(syncTimer)
  syncTimer()
  return () => {
    listeners.delete(listener)
    syncTimer()
    if (listeners.size === 0) {
      unsubscribeWindow?.()
      unsubscribeWindow = undefined
    }
  }
}

function now(): number {
  return current
}

/** The time, to the second, for text such as "0:42" or "4m". */
export function useNow(): number {
  return useSyncExternalStore(subscribe, now, now)
}

/**
 * True while `at` is less than `forMs` ago, turning false on time without a clock subscription.
 */
export function useRecent(at: number | null, forMs: number): boolean {
  // A moment already over when the component first renders starts out expired, so it never shows.
  const [expiredAt, setExpiredAt] = useState<number | null>(() => (at !== null && Date.now() - at >= forMs ? at : null))
  useLayoutEffect(() => {
    if (at === null) return
    const timeout = setTimeout(() => { setExpiredAt(at) }, Math.max(0, at + forMs - Date.now()))
    return () => { clearTimeout(timeout) }
  }, [at, forMs])
  return at !== null && expiredAt !== at
}
