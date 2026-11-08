import { useEffect, useRef, useState } from 'react'

/** How often the shown text catches up toward what has arrived. */
const TICK_MS = 24
/** Each tick shows this share of what is still unshown (at least one character), so a big chunk eases in and a trickle types one letter at a time. */
const CATCH_UP_DIVISOR = 14

/**
 * The text as if it were being typed. A reply arrives in chunks (the history is read every second or so), and
 * showing each chunk whole makes the text flash in. This shows only what was on screen when the row first
 * appeared, then types toward whatever arrives after it; a row that mounts already full (an old message, a
 * session you switched to) shows all of it at once. With `enabled` off everything shows at once.
 */
export function useTypedText(target: string, enabled: boolean): string {
  const [shown, setShown] = useState(target.length)
  const latest = useRef(target)
  useEffect(() => { latest.current = target })
  useEffect(() => {
    if (!enabled) return
    const timer = window.setInterval(() => {
      setShown((now) => {
        const left = latest.current.length - now
        return left > 0 ? now + Math.max(1, Math.ceil(left / CATCH_UP_DIVISOR)) : now
      })
    }, TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [enabled])
  if (!enabled) return target
  return target.slice(0, Math.min(shown, target.length))
}
