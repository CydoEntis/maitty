import { useRef, type KeyboardEvent } from 'react'
import { SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN } from '@/features/settings/preferences'

const KEY_STEP_PX = 16

const clamp = (width: number) => Math.min(SIDEBAR_WIDTH_MAX, Math.max(SIDEBAR_WIDTH_MIN, Math.round(width)))

type Drag = { startX: number; startWidth: number }

/** The docked sidebar's right edge: drag it, or focus it and use the arrow keys; double-click resets. */
export function SidebarResizer(props: { width: number; onResize: (width: number) => void; onResizeEnd: (width: number) => void; onDragging: (dragging: boolean) => void }) {
  const { width, onResize, onResizeEnd, onDragging } = props
  const drag = useRef<Drag | null>(null)
  const onKeyDown = (event: KeyboardEvent) => {
    const next = event.key === 'ArrowLeft' ? width - KEY_STEP_PX : event.key === 'ArrowRight' ? width + KEY_STEP_PX : event.key === 'Home' ? SIDEBAR_WIDTH_DEFAULT : null
    if (next === null) return
    event.preventDefault()
    onResizeEnd(clamp(next))
  }
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize sidebar"
      aria-valuenow={width}
      aria-valuemin={SIDEBAR_WIDTH_MIN}
      aria-valuemax={SIDEBAR_WIDTH_MAX}
      tabIndex={0}
      className="mx-sidebar-resizer"
      title="Drag to resize · double-click to reset"
      onKeyDown={onKeyDown}
      onDoubleClick={() => { onResizeEnd(SIDEBAR_WIDTH_DEFAULT) }}
      onPointerDown={(event) => {
        // The handle keeps receiving the pointer even when it moves off the thin edge.
        if ('setPointerCapture' in event.currentTarget) event.currentTarget.setPointerCapture(event.pointerId)
        drag.current = { startX: event.clientX, startWidth: width }
        onDragging(true)
      }}
      onPointerMove={(event) => {
        const current = drag.current
        if (current) onResize(clamp(current.startWidth + event.clientX - current.startX))
      }}
      onPointerUp={(event) => {
        const current = drag.current
        if (!current) return
        drag.current = null
        onDragging(false)
        onResizeEnd(clamp(current.startWidth + event.clientX - current.startX))
      }}
      onPointerCancel={() => {
        drag.current = null
        onDragging(false)
        onResizeEnd(width)
      }}
    />
  )
}
