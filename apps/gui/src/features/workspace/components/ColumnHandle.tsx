import type { PointerEvent } from 'react'
import { dragWidths } from '../model/columns'

/** The columns' current widths in px, left to right. */
function columnWidths(container: HTMLElement): number[] {
  return Array.from(container.querySelectorAll<HTMLElement>(':scope > .mx-pane-column')).map((column) => column.getBoundingClientRect().width)
}

/** The gap between two columns: invisible until hovered, drag to resize, double-click for equal widths. */
export function ColumnHandle({ index, onResize, onReset }: { index: number; onResize: (widths: number[]) => void; onReset: () => void }) {
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    const container = event.currentTarget.parentElement
    if (!container) return
    event.preventDefault()
    const handle = event.currentTarget
    handle.setPointerCapture(event.pointerId)
    const startX = event.clientX
    const start = columnWidths(container)
    handle.classList.add('is-dragging')
    const move = (moveEvent: globalThis.PointerEvent) => { onResize(dragWidths(start, index, moveEvent.clientX - startX)) }
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
  return <div role="separator" aria-orientation="vertical" aria-label="Resize columns" title="Drag to resize · double-click to reset" className="mx-col-handle" onPointerDown={onPointerDown} onDoubleClick={onReset} />
}
