import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { displayName, type SessionView } from '../model/session'
import type { SessionMenuActions } from './SessionMenu'
import { Icon } from './glyphs'
import { runsAgent, useArmed } from './useArmed'

const MENU_WIDTH_PX = 260
const MENU_HEIGHT_PX = 190
const EDGE_PX = 8

type Props = { session: SessionView; actions: SessionMenuActions; at: { x: number; y: number }; onClose: () => void }

export function SessionMenuPopup({ session, actions, at, onClose }: Props) {
  const root = useRef<HTMLDivElement>(null)
  const close = useRef(onClose)
  const closing = useArmed(runsAgent(session.process))
  useEffect(() => { close.current = onClose })

  useEffect(() => {
    // Focus goes back to where it was when the menu closes.
    const returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null
    root.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus()
    const onPointerDown = (event: PointerEvent) => {
      if (root.current && event.target instanceof Node && !root.current.contains(event.target)) close.current()
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Home' || event.key === 'End') {
        const items = Array.from(root.current?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? [])
        if (items.length === 0) return
        event.preventDefault()
        event.stopPropagation()
        const at = items.indexOf(document.activeElement as HTMLElement)
        const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (at + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
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

  const run = (action: () => void) => () => { onClose(); action() }
  const style = {
    left: Math.max(EDGE_PX, Math.min(at.x, window.innerWidth - MENU_WIDTH_PX - EDGE_PX)),
    top: Math.max(EDGE_PX, Math.min(at.y, window.innerHeight - MENU_HEIGHT_PX - EDGE_PX)),
  }
  const menu = (
    <div ref={root} role="dialog" aria-label={`Menu for ${displayName(session)}`} className="mx-sessionmenu" style={style}>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(actions.onRename)}><Icon name="pencil" size={13} />Rename</button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(() => { actions.onPin(!session.pinned) })}>
        <Icon name="pin" size={13} />{session.pinned ? 'Unpin' : 'Pin to top'}
      </button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(actions.onShow)}><span className="mx-sm-gap" />Open</button>
      <div className="mx-menu-sep" />
      <button
        type="button"
        role="menuitem"
        className={`mx-menu-item is-danger${closing.armed ? ' is-armed' : ''}`}
        disabled={!actions.online}
        onClick={() => { if (closing.press()) run(actions.onEnd)() }}
      >
        <span className="mx-sm-gap" />{closing.armed ? `Close and stop ${session.process}? Click again` : 'Close session'}<span className="mx-menu-hint" aria-hidden="true">{closing.armed ? '' : 'ends it'}</span>
      </button>
    </div>
  )
  return createPortal(menu, actions.portalContainer ?? document.body)
}
