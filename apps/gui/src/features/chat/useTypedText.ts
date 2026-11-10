import { useEffect, useRef, useState } from 'react'

/** How often the shown text catches up toward what has arrived. */
const TICK_MS = 24
/** Each tick shows this share of what is still unshown (at least one character), so a big chunk eases in and a trickle types one letter at a time. */
const CATCH_UP_DIVISOR = 14

/**
 * The text as if it were being typed. A reply arrives in chunks (the history is read every second or so), and
 * showing each chunk whole makes the text flash in. This shows only what was on screen when the row first
 * appeared, then types toward whatever arrives after it; a row that mounts already full (an old message, a
 * session you switched to) shows all of it at once, unless `fromStart` says it arrived live, which types it from its
 * first letter. With `enabled` off everything shows at once.
 */
export function useTypedText(target: string, enabled: boolean, fromStart = false): string {
  const [shown, setShown] = useState(fromStart ? 0 : target.length)
  const latest = useRef(target)
  useEffect(() => { latest.current = target })
  // The timer runs only while text is waiting to be shown: every agent message on screen has one of these, and a
  // conversation at rest should wake nothing.
  const behind = enabled && shown < target.length
  useEffect(() => {
    if (!behind) return
    const timer = window.setInterval(() => {
      setShown((now) => {
        const left = latest.current.length - now
        return left > 0 ? now + Math.max(1, Math.ceil(left / CATCH_UP_DIVISOR)) : now
      })
    }, TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [behind])
  if (!enabled) return target
  return target.slice(0, Math.min(shown, target.length))
}
