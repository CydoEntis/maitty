import type { PointerEvent } from 'react'
import { dragWidths } from '../model/columns'

type Axis = 'columns' | 'rows'

const CHILD: Record<Axis, string> = { columns: ':scope > .mx-pane-column', rows: ':scope > .mx-pane-cell' }

/** The sizes in px of the columns (left to right) or the panes of a column (top to bottom). */
function sizesOf(container: HTMLElement, axis: Axis): number[] {
  return Array.from(container.querySelectorAll<HTMLElement>(CHILD[axis])).map((child) => {
    const box = child.getBoundingClientRect()
    return axis === 'columns' ? box.width : box.height
  })
}

/**
 * The gap between two columns, or two panes of a column: invisible until hovered, drag to resize,
 * double-click to even them out.
 */
export function SplitHandle({ axis, index, onResize, onReset }: { axis: Axis; index: number; onResize: (sizes: number[]) => void; onReset: () => void }) {
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    const container = event.currentTarget.parentElement
    if (!container) return
    event.preventDefault()
    const handle = event.currentTarget
    handle.setPointerCapture(event.pointerId)
    const start = axis === 'columns' ? event.clientX : event.clientY
    const sizes = sizesOf(container, axis)
    handle.classList.add('is-dragging')
    const move = (moveEvent: globalThis.PointerEvent) => { onResize(dragWidths(sizes, index, (axis === 'columns' ? moveEvent.clientX : moveEvent.clientY) - start)) }
    const end = () => {
      handle.classList.remove('is-dragging')
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', end)
      handle.removeEventListener('pointercancel', end)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', end)
    handle.addEventListener('pointercancel', end)
  }
  return (
    <div
      role="separator"
      aria-orientation={axis === 'columns' ? 'vertical' : 'horizontal'}
      aria-label={axis === 'columns' ? 'Resize columns' : 'Resize panes'}
      title="Drag to resize · double-click to even out"
      className={`mx-split-handle is-${axis}`}
      onPointerDown={onPointerDown}
      onDoubleClick={onReset}
    />
  )
}
