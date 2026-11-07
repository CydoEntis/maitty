import { useState, type MouseEvent } from 'react'
import type { Fleet } from '@/api/fleets'
import type { FleetColor } from '../model/fleets'
import { FleetMenuPopup } from './FleetMenuPopup'

export type FleetMenuActions = {
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  onRename: () => void
  onRecolor: (color: FleetColor) => void
}

/** A right-click menu for a fleet: rename it, or recolour it with one of the six fleet colours. */
export function useFleetMenu(fleet: Fleet, actions: FleetMenuActions | null) {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null)
  return {
    onContextMenu: (event: MouseEvent) => {
      if (actions === null) return
      event.preventDefault()
      setAt({ x: event.clientX, y: event.clientY })
    },
    element: at && actions ? <FleetMenuPopup fleet={fleet} actions={actions} at={at} onClose={() => { setAt(null) }} /> : null,
  }
}
