import { useEffect, useState } from 'react'
import type { ChatAvailability } from '@/features/workspace/model/chatView'
import { PANEL_WIDE_WIDTH_PX, PANEL_WIDTH_PX, SessionPanel } from './SessionPanel'

type Props = {
  mode: 'docked' | 'overlay'
  title: string
  process: string
  availability: ChatAvailability
  root: string | null
  folder: string | null
  branch: string | null
  online: boolean
  chatOpen: boolean
  onOpenInFiles: (file: string) => void
  onClose: () => void
  /** Tells the window how wide the docked panel is (0 once it is gone), so other drawers can sit beside it. */
  onDockedWidth: (width: number) => void
}

/** The session panel plus its footprint in the window: a docked panel takes room from the panes, more while a diff is open. */
export function SessionPanelHost({ onDockedWidth, ...panel }: Props) {
  const [wide, setWide] = useState(false)
  const width = wide ? PANEL_WIDE_WIDTH_PX : PANEL_WIDTH_PX
  const docked = panel.mode === 'docked'
  useEffect(() => {
    onDockedWidth(docked ? width : 0)
    return () => { onDockedWidth(0) }
  }, [docked, width, onDockedWidth])
  return <SessionPanel {...panel} width={width} onWide={setWide} />
}
