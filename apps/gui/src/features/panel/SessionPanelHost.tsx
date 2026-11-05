import { useEffect } from 'react'
import type { ChatAvailability } from '@/features/workspace/model/chatView'
import { PANEL_WIDTH_PX, SessionPanel } from './SessionPanel'

type Props = {
  mode: 'docked' | 'overlay'
  title: string
  process: string
  availability: ChatAvailability
  onClose: () => void
  /** Tells the window how wide the docked panel is (0 once it is gone), so other drawers can sit beside it. */
  onDockedWidth: (width: number) => void
}

/** The session panel plus its footprint in the window: a docked panel takes room from the panes. */
export function SessionPanelHost({ onDockedWidth, ...panel }: Props) {
  const docked = panel.mode === 'docked'
  useEffect(() => {
    onDockedWidth(docked ? PANEL_WIDTH_PX : 0)
    return () => { onDockedWidth(0) }
  }, [docked, onDockedWidth])
  return <SessionPanel {...panel} />
}
