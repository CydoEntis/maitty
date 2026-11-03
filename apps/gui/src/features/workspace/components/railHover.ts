import { useCallback, useEffect, useMemo, useRef, useState, type FocusEvent, type MouseEvent } from 'react'

const OPEN_DELAY_MS = 280
const CLOSE_DELAY_MS = 160

export type RailTarget = { kind: 'folder'; key: string } | { kind: 'session'; id: string }
type Shown = { target: RailTarget; anchor: DOMRect }

export type RailHover = {
  shown: Shown | null
  enter: (target: RailTarget, element: HTMLElement) => void
  /** The pointer or focus left the item: close soon, unless it reaches the popover. */
  leave: () => void
  /** The pointer is on the popover: keep it open. */
  hold: () => void
  close: () => void
}

/**
 * When a rail popover shows: after a short hover, instantly once one is already open, and a beat
 * after the pointer leaves, so it can be reached and used.
 */
export function useRailHover(): RailHover {
  const [shown, setShown] = useState<Shown | null>(null)
  const isOpen = useRef(false)
  const openTimer = useRef(0)
  const closeTimer = useRef(0)

  const enter = useCallback((target: RailTarget, element: HTMLElement) => {
    window.clearTimeout(closeTimer.current)
    window.clearTimeout(openTimer.current)
    const anchor = element.getBoundingClientRect()
    const show = () => {
      isOpen.current = true
      setShown({ target, anchor })
    }
    if (isOpen.current) show()
    else openTimer.current = window.setTimeout(show, OPEN_DELAY_MS)
  }, [])
  const close = useCallback(() => {
    window.clearTimeout(openTimer.current)
    window.clearTimeout(closeTimer.current)
    isOpen.current = false
    setShown(null)
  }, [])
  const leave = useCallback(() => {
    window.clearTimeout(openTimer.current)
    window.clearTimeout(closeTimer.current)
    closeTimer.current = window.setTimeout(close, CLOSE_DELAY_MS)
  }, [close])
  const hold = useCallback(() => { window.clearTimeout(closeTimer.current) }, [])
  useEffect(() => close, [close])

  return useMemo(() => ({ shown, enter, leave, hold, close }), [shown, enter, leave, hold, close])
}

/** The pointer and focus handlers that make an item open its popover; the item's own click and menu stay its own. */
export function hoverHandlers(hover: RailHover, target: RailTarget) {
  return {
    onMouseEnter: (event: MouseEvent<HTMLElement>) => { hover.enter(target, event.currentTarget) },
    onMouseLeave: hover.leave,
    onFocus: (event: FocusEvent<HTMLElement>) => { hover.enter(target, event.currentTarget) },
    onBlur: hover.leave,
  }
}
