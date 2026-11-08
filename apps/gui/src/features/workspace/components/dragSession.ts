import type { PointerEvent as ReactPointerEvent } from 'react'

/*
 * Moving a pane, or dropping a sidebar session into one, is done with pointer events and not with
 * the browser's drag and drop: the desktop window takes over HTML5 drags to receive files from the
 * OS, so a drag that starts inside the page shows a blocked cursor and never drops.
 * OS file drops still go through the window (useFileDrop) and are untouched.
 */

export type DragSource = { kind: 'pane' | 'session'; id: string; label: string }

export interface DragHandlers {
  start: (source: DragSource) => void
  /** The pane under the pointer, or null when it is over none. */
  over: (paneId: string | null) => void
  drop: (source: DragSource, paneId: string | null) => void
  end: () => void
}

/** How far the pointer must travel before a press becomes a drag, so a click stays a click. */
const DRAG_THRESHOLD_PX = 5

/** Keeps the label off the pointer so it never covers what is being dropped on. */
const GHOST_OFFSET_PX = 12

let handlers: DragHandlers | null = null
let active = false

/** The workspace registers who reacts to a drag; returns the way to unregister. */
export function registerDragHandlers(next: DragHandlers): () => void {
  handlers = next
  return () => { if (handlers === next) handlers = null }
}

function paneUnder(target: EventTarget | null): string | null {
  return target instanceof Element ? target.closest<HTMLElement>('[data-pane]')?.dataset.pane ?? null : null
}

function makeGhost(label: string): HTMLElement {
  const ghost = document.createElement('div')
  ghost.className = 'mx-drag-ghost'
  ghost.textContent = label
  document.body.appendChild(ghost)
  return ghost
}

/** Call from `onPointerDown` of whatever can be dragged. */
export function beginPointerDrag(event: ReactPointerEvent, source: DragSource): void {
  if (event.button !== 0 || active || !handlers) return
  const origin = { x: event.clientX, y: event.clientY }
  let ghost: HTMLElement | null = null

  const place = (x: number, y: number) => {
    if (ghost) ghost.style.transform = `translate(${String(x + GHOST_OFFSET_PX)}px, ${String(y + GHOST_OFFSET_PX)}px)`
  }
  const finish = (dropped: string | null | undefined) => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    window.removeEventListener('pointercancel', cancel)
    window.removeEventListener('keydown', key, true)
    ghost?.remove()
    if (ghost) {
      // The release is followed by a click on whatever is under the pointer; a drag must not trigger it.
      const swallow = (click: Event) => { click.stopPropagation(); click.preventDefault() }
      window.addEventListener('click', swallow, { capture: true, once: true })
      setTimeout(() => { window.removeEventListener('click', swallow, true) }, 0)
      if (dropped !== undefined) handlers?.drop(source, dropped)
      handlers?.end()
    }
    ghost = null
    active = false
  }
  const move = (e: PointerEvent) => {
    if (!ghost) {
      if (Math.hypot(e.clientX - origin.x, e.clientY - origin.y) < DRAG_THRESHOLD_PX) return
      ghost = makeGhost(source.label)
      handlers?.start(source)
    }
    place(e.clientX, e.clientY)
    handlers?.over(paneUnder(e.target))
  }
  const up = (e: PointerEvent) => { finish(ghost ? paneUnder(e.target) : undefined) }
  const cancel = () => { finish(undefined) }
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && ghost) { e.stopPropagation(); finish(undefined) }
  }

  active = true
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
  window.addEventListener('pointercancel', cancel)
  window.addEventListener('keydown', key, true)
}
