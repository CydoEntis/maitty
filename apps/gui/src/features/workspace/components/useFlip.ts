import { useLayoutEffect, useRef, type RefObject } from 'react'

const MOVE_MS = 240
const ENTER_MS = 160
const EASE = 'cubic-bezier(.2, .8, .2, 1)'
/** A row that moved less than this has not moved. */
const MOVED_PX = 1

/**
 * Rows that change place slide to it instead of snapping (FLIP: the layout is settled first, then each row that
 * moved is drawn at its old place and carried to the new one). A row that is new fades in where it will stay: it does not travel, so it never crosses a heading. The sidebar
 * reorders as a chat starts working, and a row that jumped there read as a flicker. Rows are found by `data-flip`;
 * nothing moves when motion is reduced.
 */
export function useFlip(container: RefObject<HTMLElement | null>, reduced: boolean): void {
  const previous = useRef(new Map<string, number>())
  useLayoutEffect(() => {
    const root = container.current
    if (root === null) return
    const rows = Array.from(root.querySelectorAll<HTMLElement>('[data-flip]'))
    // Measured against the list, so scrolling it is not a move.
    const base = root.getBoundingClientRect().top - root.scrollTop
    const now = new Map<string, number>()
    for (const row of rows) now.set(row.dataset.flip ?? '', row.getBoundingClientRect().top - base)
    const first = previous.current.size === 0
    if (!reduced && !first && typeof rows[0]?.animate === 'function') {
      for (const row of rows) {
        const key = row.dataset.flip ?? ''
        const was = previous.current.get(key)
        const at = now.get(key) ?? 0
        if (was === undefined) {
          row.animate([{ opacity: 0 }, { opacity: 1 }], { duration: ENTER_MS, easing: EASE })
        } else if (Math.abs(was - at) > MOVED_PX) {
          row.animate([{ transform: `translateY(${String(was - at)}px)` }, { transform: 'none' }], { duration: MOVE_MS, easing: EASE })
        }
      }
    }
    previous.current = now
  })
}
