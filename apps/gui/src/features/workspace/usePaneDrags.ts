import { type Dispatch, type SetStateAction, useEffect, useState } from 'react'
import { registerDragHandlers } from './components/dragSession'
import { movePanes } from './components/paneTransition'
import { dropSession, type Layout, swapSessions } from './model/layout'

/** Dragging a pane onto another swaps them; dragging a session from the sidebar puts it in the pane. */
export function usePaneDrags({ setLayout, flash }: {
  setLayout: Dispatch<SetStateAction<Layout>>
  flash: (text: string) => void
}) {
  const [dragId, setDragId] = useState<string | null>(null)
  const [dropId, setDropId] = useState<string | null>(null)
  useEffect(() => registerDragHandlers({
    start: (source) => { if (source.kind === 'pane') setDragId(source.id) },
    over: setDropId,
    drop: (source, paneId) => {
      if (!paneId) return
      if (source.kind === 'session') {
        movePanes(() => { setLayout((current) => dropSession(current, paneId, source.id)) })
        flash('Session moved into this pane')
      } else if (source.id !== paneId) movePanes(() => { setLayout((current) => swapSessions(current, source.id, paneId)) })
    },
    end: () => { setDragId(null); setDropId(null) },
  }), [flash, setLayout])
  // A drag from the sidebar ends wherever it is released; the pane's highlight must not outlast it.
  useEffect(() => {
    const clear = () => { setDropId(null) }
    window.addEventListener('dragend', clear)
    window.addEventListener('drop', clear)
    return () => {
      window.removeEventListener('dragend', clear)
      window.removeEventListener('drop', clear)
    }
  }, [])
  return { dropId, dragId }
}
