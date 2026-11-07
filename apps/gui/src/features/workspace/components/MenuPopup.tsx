import { useEffect, useLayoutEffect, useRef, type ReactNode } from 'react'
import { createPortal } from 'react-dom'

const MENU_WIDTH_PX = 260
const EDGE_PX = 8
const ITEM_SELECTOR = '[role^="menuitem"]:not(:disabled)'

type Props = {
  label: string
  at: { x: number; y: number }
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  onClose: () => void
  children: ReactNode
}

/**
 * The shell of a right-click menu: placed at the pointer and kept inside the window, focus on the
 * first item, arrow keys move, Esc or a click outside closes it and focus returns to where it was.
 */
export function MenuPopup({ label, at, portalContainer, onClose, children }: Props) {
  const root = useRef<HTMLDivElement>(null)
  const close = useRef(onClose)
  useEffect(() => { close.current = onClose })

  // The menu's height depends on its items, so it is placed once it is measured.
  useLayoutEffect(() => {
    const element = root.current
    if (!element) return
    element.style.left = `${String(Math.max(EDGE_PX, Math.min(at.x, window.innerWidth - MENU_WIDTH_PX - EDGE_PX)))}px`
    element.style.top = `${String(Math.max(EDGE_PX, Math.min(at.y, window.innerHeight - element.offsetHeight - EDGE_PX)))}px`
  }, [at])

  useEffect(() => {
    const returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null
    root.current?.querySelector<HTMLElement>(ITEM_SELECTOR)?.focus()
    const onPointerDown = (event: PointerEvent) => {
      if (root.current && event.target instanceof Node && !root.current.contains(event.target)) close.current()
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Home' || event.key === 'End') {
        const items = Array.from(root.current?.querySelectorAll<HTMLElement>(ITEM_SELECTOR) ?? [])
        if (items.length === 0) return
        event.preventDefault()
        event.stopPropagation()
        const current = items.indexOf(document.activeElement as HTMLElement)
        const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (current + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
        items[next]?.focus()
        return
      }
      if (event.key !== 'Escape') return
      event.preventDefault()
      event.stopPropagation()
      close.current()
    }
    document.addEventListener('pointerdown', onPointerDown, true)
    document.addEventListener('keydown', onKeyDown, true)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true)
      document.removeEventListener('keydown', onKeyDown, true)
      if (returnTo?.isConnected) returnTo.focus()
    }
  }, [])

  return createPortal(<div ref={root} role="dialog" aria-label={label} className="mx-sessionmenu">{children}</div>, portalContainer ?? document.body)
}
