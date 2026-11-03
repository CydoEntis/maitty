import { useState, type MouseEvent } from 'react'
import type { SessionView } from '../model/session'
import { SessionMenuPopup } from './SessionMenuPopup'

export type SessionMenuActions = {
  online: boolean
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  onShow: () => void
  /** Starts renaming it, where it is shown. */
  onRename: () => void
  onPin: (pinned: boolean) => void
  onEnd: () => void
}

/** A right-click menu for a session: name it, pin it, open it, or end it. */
export function useSessionMenu(session: SessionView, actions: SessionMenuActions) {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null)
  return {
    onContextMenu: (event: MouseEvent) => {
      event.preventDefault()
      setAt({ x: event.clientX, y: event.clientY })
    },
    element: at ? <SessionMenuPopup session={session} actions={actions} at={at} onClose={() => { setAt(null) }} /> : null,
  }
}
