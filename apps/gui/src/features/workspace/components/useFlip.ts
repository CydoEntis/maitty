import { useEffect, useLayoutEffect, useRef, type RefObject } from 'react'

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

/** The property a folding section animates (`Collapse`): its height, as a grid row. */
const FOLD_PROPERTY = 'grid-template-rows'

/**
 * Whether something in the list is folding open or shut right now. A fold moves what is under it by itself, a little
 * each frame; a glide started from a place half way through it would carry the rows a second time, which showed as
 * the list bouncing whenever a session's subagents opened or closed while the list was being drawn again.
 */
function folding(root: HTMLElement): boolean {
  if (typeof root.getAnimations !== 'function') return false
  return root.getAnimations({ subtree: true }).some((animation) => (
    'transitionProperty' in animation && animation.transitionProperty === FOLD_PROPERTY && animation.playState !== 'finished' && animation.playState !== 'idle'
  ))
}

/** Where everything in the list is now, by its `data-flip` name. */
function places(root: HTMLElement): Map<string, number> {
  const now = new Map<string, number>()
  for (const item of Array.from(root.querySelectorAll<HTMLElement>('[data-flip]'))) now.set(item.dataset.flip ?? '', layoutTop(item, root))
  return now
}

/**
 * Everything in the list that changes place slides to it instead of snapping (FLIP: the layout is settled first,
 * then each thing that moved is drawn at its old place and carried to the new one). Rows and the headings between
 * them move together, so a row that changes section never leaves the rest of the list to jump. Something new slides
 * in where it will stay. A list that changes again mid-glide carries on from where each thing is drawn, not from
 * where it started. Things are found by `data-flip`; nothing moves when motion is reduced. While a section folds
 * nothing glides (the fold is the movement), and where things are when it ends is where the next glide starts from.
 */
export function useFlip(container: RefObject<HTMLElement | null>, reduced: boolean): void {
  const previous = useRef(new Map<string, number>())
  const glides = useRef(new WeakMap<HTMLElement, Animation>())
  useLayoutEffect(() => {
    const root = container.current
    if (root === null) return
    const items = Array.from(root.querySelectorAll<HTMLElement>('[data-flip]'))
    const now = places(root)
    const first = previous.current.size === 0
    if (!reduced && !first && typeof items[0]?.animate === 'function' && !folding(root)) {
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
  // A fold that ends leaves everything where it now belongs, with no draw of the list to notice it.
  useEffect(() => {
    const root = container.current
    if (root === null) return
    const settle = (event: TransitionEvent) => {
      if (event.propertyName === FOLD_PROPERTY) previous.current = places(root)
    }
    root.addEventListener('transitionend', settle)
    root.addEventListener('transitioncancel', settle)
    return () => {
      root.removeEventListener('transitionend', settle)
      root.removeEventListener('transitioncancel', settle)
    }
  }, [container])
}
