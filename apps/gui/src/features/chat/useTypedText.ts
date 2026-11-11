import { useEffect, useRef, useState } from 'react'

/** How often the shown text moves on toward what has arrived: a few words at a time, not letter by letter. */
export const TICK_MS = 40
/** The steady pace: this many characters a tick, about 300 a second, a little faster than anyone reads. */
const PACE_CHARS = 12
/** A backlog larger than the steady pace clears in a second or two goes faster: each tick shows this share of it, so a long reply does not take a minute. */
const CATCH_UP_DIVISOR = 45
/** A step ends on a whole word when the word's end is within this many characters, so words appear, not halves of them. */
const WORD_REACH = 24

/**
 * How much of `text` shows after one more tick, from `shown` characters. It moves at a steady pace, faster while a
 * lot is waiting, and stops at the end of a word. A reply read from an agent's file arrives whole or in large
 * pieces, and shown by share alone most of it appeared within a quarter of a second, which read as a flash.
 */
export function nextShown(text: string, shown: number): number {
  const left = text.length - shown
  if (left <= 0) return text.length
  const reach = Math.min(text.length, shown + Math.max(PACE_CHARS, Math.ceil(left / CATCH_UP_DIVISOR)))
  const wordEnd = text.slice(reach, reach + WORD_REACH).search(/\s/)
  return wordEnd === -1 ? Math.min(text.length, reach) : reach + wordEnd
}

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
      setShown((now) => nextShown(latest.current, now))
    }, TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [behind])
  if (!enabled) return target
  return target.slice(0, Math.min(shown, target.length))
}
