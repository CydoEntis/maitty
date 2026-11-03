import { useState, type MouseEvent } from 'react'
import type { SessionView } from '../model/session'
import { SessionMenuPopup } from './SessionMenuPopup'

export type SessionMenuActions = {
  online: boolean
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  onShow: () => void
  /** An empty name clears it. */
  onSetName: (name: string) => void
  onPin: (pinned: boolean) => void
  onArchive: () => void
  onEnd: () => void
}

/** A right-click menu for a session: name it, pin it, open it, put it away, or end it. */
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
