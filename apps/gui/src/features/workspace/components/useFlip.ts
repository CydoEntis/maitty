import { useLayoutEffect, useRef, type RefObject } from 'react'

const MOVE_MS = 420
const ENTER_MS = 280
const ENTER_FROM_PX = -10
const EASE = 'cubic-bezier(.2, .8, .2, 1)'
/** Something that moved less than this has not moved. */
const MOVED_PX = 1

/** Where the layout puts an element inside `root`. A glide still under way does not count, nor does scrolling. */
function layoutTop(element: HTMLElement, root: HTMLElement): number {
  let top = 0
  let at: HTMLElement | null = element
  while (at !== null && at !== root && root.contains(at)) {
    top += at.offsetTop
    at = at.offsetParent instanceof HTMLElement ? at.offsetParent : null
  }
  return top
}

/** How far a glide under way still holds the element from its place. */
function glideOffset(element: HTMLElement): number {
  const transform = getComputedStyle(element).transform
  if (transform === '' || transform === 'none' || typeof DOMMatrixReadOnly === 'undefined') return 0
  return new DOMMatrixReadOnly(transform).m42
}

/**
 * Everything in the list that changes place slides to it instead of snapping (FLIP: the layout is settled first,
 * then each thing that moved is drawn at its old place and carried to the new one). Rows and the headings between
 * them move together, so a row that changes section never leaves the rest of the list to jump. Something new slides
 * in where it will stay. A list that changes again mid-glide carries on from where each thing is drawn, not from
 * where it started. Things are found by `data-flip`; nothing moves when motion is reduced.
 */
export function useFlip(container: RefObject<HTMLElement | null>, reduced: boolean): void {
  const previous = useRef(new Map<string, number>())
  const glides = useRef(new WeakMap<HTMLElement, Animation>())
  useLayoutEffect(() => {
    const root = container.current
    if (root === null) return
    const items = Array.from(root.querySelectorAll<HTMLElement>('[data-flip]'))
    const now = new Map<string, number>()
    for (const item of items) now.set(item.dataset.flip ?? '', layoutTop(item, root))
    const first = previous.current.size === 0
    if (!reduced && !first && typeof items[0]?.animate === 'function') {
      for (const item of items) {
        const key = item.dataset.flip ?? ''
        const was = previous.current.get(key)
        const at = now.get(key) ?? 0
        if (was === undefined) {
          item.animate([{ opacity: 0, transform: `translateX(${String(ENTER_FROM_PX)}px)` }, { opacity: 1, transform: 'none' }], { duration: ENTER_MS, easing: EASE })
          continue
        }
        if (Math.abs(was - at) <= MOVED_PX) continue
        const from = was + glideOffset(item) - at
        glides.current.get(item)?.cancel()
        glides.current.set(item, item.animate([{ transform: `translateY(${String(from)}px)` }, { transform: 'none' }], { duration: MOVE_MS, easing: EASE }))
      }
    }
    previous.current = now
  })
}
