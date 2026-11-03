import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { displayName, MAX_SESSION_NAME_LENGTH, type SessionView } from '../model/session'
import type { SessionMenuActions } from './SessionMenu'
import { Icon } from './glyphs'

const MENU_WIDTH_PX = 260
const MENU_HEIGHT_PX = 236
const EDGE_PX = 8

type Props = { session: SessionView; actions: SessionMenuActions; at: { x: number; y: number }; onClose: () => void }

export function SessionMenuPopup({ session, actions, at, onClose }: Props) {
  const root = useRef<HTMLDivElement>(null)
  const name = useRef<HTMLInputElement>(null)
  const close = useRef(onClose)
  useEffect(() => { close.current = onClose })

  useEffect(() => {
    name.current?.focus()
    name.current?.select()
    const onPointerDown = (event: PointerEvent) => {
      if (root.current && event.target instanceof Node && !root.current.contains(event.target)) close.current()
    }
    const onKeyDown = (event: KeyboardEvent) => {
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
    }
  }, [])

  const run = (action: () => void) => () => { onClose(); action() }
  const style = {
    left: Math.max(EDGE_PX, Math.min(at.x, window.innerWidth - MENU_WIDTH_PX - EDGE_PX)),
    top: Math.max(EDGE_PX, Math.min(at.y, window.innerHeight - MENU_HEIGHT_PX - EDGE_PX)),
  }
  const menu = (
    <div ref={root} role="dialog" aria-label={`Menu for ${displayName(session)}`} className="mx-sessionmenu" style={style}>
      <label className="mx-sm-name">
        <span className="mx-menu-caps">Name</span>
        <input
          ref={name}
          aria-label="Session name"
          placeholder={session.title}
          defaultValue={session.name ?? ''}
          maxLength={MAX_SESSION_NAME_LENGTH}
          spellCheck={false}
          onKeyDown={(event) => {
            // Enter saves; an empty field clears the name. Keys typed here are the field's, not the menu's.
            if (event.key !== 'Enter') return
            event.preventDefault()
            run(() => { actions.onSetName(event.currentTarget.value.trim()) })()
          }}
        />
      </label>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(() => { actions.onPin(!session.pinned) })}>
        <Icon name="pin" size={13} />{session.pinned ? 'Unpin' : 'Pin to top'}
      </button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(actions.onShow)}><span className="mx-sm-gap" />Open</button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(actions.onArchive)}>
        <Icon name="archive" size={13} />Archive<span className="mx-menu-hint" aria-hidden="true">keeps running</span>
      </button>
      <div className="mx-menu-sep" />
      <button type="button" role="menuitem" className="mx-menu-item is-danger" disabled={!actions.online} onClick={run(actions.onEnd)}>
        <span className="mx-sm-gap" />Close session<span className="mx-menu-hint" aria-hidden="true">ends it</span>
      </button>
    </div>
  )
  return createPortal(menu, actions.portalContainer ?? document.body)
}
