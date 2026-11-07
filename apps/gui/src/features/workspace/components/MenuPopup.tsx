import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import { EDGE_PX, MENU_WIDTH_PX } from './menuPoint'
const ITEM_SELECTOR = '[role^="menuitem"]:not(:disabled)'

/** One row of a menu, or a separator line between groups of rows. */
export type MenuEntry =
  | 'separator'
  | {
      label: string
      /** A key or a word shown dim at the right. */
      hint?: string
      run?: () => void
      /** Red, and asks for a second click before it runs (`armedLabel` says what the second click does). */
      danger?: boolean
      armedLabel?: string
      disabled?: boolean
      /** Why it is disabled, as its tooltip. */
      why?: string
    }

type Props = {
  /** The menu's dim heading: what it is for (a session's name, a machine). */
  title: string
  at: { x: number; y: number }
  items: readonly MenuEntry[]
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  onClose: () => void
}

/**
 * A right-click or ⋯ menu: placed at the pointer and kept inside the window, focus on the first item,
 * arrow keys move, Esc or a click outside closes it and focus returns to where it was. A danger item
 * arms on its first click and runs on the second.
 */
export function MenuPopup({ title, at, items, portalContainer, onClose }: Props) {
  const root = useRef<HTMLDivElement>(null)
  const close = useRef(onClose)
  const [armed, setArmed] = useState<string | null>(null)
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
        const rows = Array.from(root.current?.querySelectorAll<HTMLElement>(ITEM_SELECTOR) ?? [])
        if (rows.length === 0) return
        event.preventDefault()
        event.stopPropagation()
        const current = rows.indexOf(document.activeElement as HTMLElement)
        const next = event.key === 'Home' ? 0 : event.key === 'End' ? rows.length - 1 : (current + (event.key === 'ArrowDown' ? 1 : -1) + rows.length) % rows.length
        rows[next]?.focus()
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

  const choose = (entry: Exclude<MenuEntry, 'separator'>) => {
    if (entry.danger === true && entry.armedLabel !== undefined && armed !== entry.label) {
      setArmed(entry.label)
      return
    }
    onClose()
    entry.run?.()
  }

  return createPortal(
    <div ref={root} role="menu" aria-label={title} className="mx-ctx" onContextMenu={(event) => { event.preventDefault() }}>
      <span className="mx-ctx-title">{title}</span>
      {items.map((entry, index) => entry === 'separator'
        ? <div key={`sep-${String(index)}`} className="mx-ctx-sep" role="separator" />
        : (
          <button
            key={entry.label}
            type="button"
            role="menuitem"
            className={`mx-ctx-item${entry.danger === true ? ' is-danger' : ''}${armed === entry.label ? ' is-armed' : ''}`}
            disabled={entry.disabled === true}
            title={entry.disabled === true ? entry.why : undefined}
            onClick={() => { choose(entry) }}
          >
            <span className="mx-ctx-label">{armed === entry.label ? entry.armedLabel : entry.label}</span>
            {entry.hint !== undefined && armed !== entry.label && <span className="mx-ctx-hint" aria-hidden="true">{entry.hint}</span>}
          </button>
        ))}
    </div>,
    portalContainer ?? document.body,
  )
}
