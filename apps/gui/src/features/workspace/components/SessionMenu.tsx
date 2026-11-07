import { useState, type MouseEvent } from 'react'
import type { Fleet } from '@/api/fleets'
import type { SessionView } from '../model/session'
import { SessionMenuPopup } from './SessionMenuPopup'

export type SessionMenuActions = {
  online: boolean
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  /** Shows the session beside the pane that has focus. */
  onSplit: (direction: 'right' | 'down') => void
  onCopyPath: () => void
  /** Asks for the path of a file on the session's machine to pull here; left out for this computer's sessions. */
  onGetFile?: () => void
  /** Starts renaming it, where it is shown. */
  onRename: () => void
  onPin: (pinned: boolean) => void
  onEnd: () => void
  /** The other fleets on the session's machine it can move to; none leaves the section out. */
  moveTargets: readonly Fleet[]
  onMove: (fleet: Fleet) => void
}

/** A right-click menu for a session: name it, pin it, open it in a split, copy its path, or end it. */
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
