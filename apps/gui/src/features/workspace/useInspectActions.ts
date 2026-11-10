import { type Dispatch, type SetStateAction, useCallback } from 'react'
import { toast } from '@/lib/toast'
import { guardLocal } from '@/lib/localOnly'
import type { InspectView } from '@/features/inspect/InspectPanel'
import { focusPane, type Layout } from './model/layout'
import type { SessionView } from './model/session'

/** Opens and closes the repository view. It reads this computer's checkouts, so a pane of another machine is told so instead. */
export function useInspectActions({ focusedSession, setInspectOpen, setInspectView, sessionMap, showSessionById, inspectOpen, inspectView, focusedId, setLayout }: {
  focusedSession: SessionView | undefined
  setInspectOpen: Dispatch<SetStateAction<boolean>>
  setInspectView: Dispatch<SetStateAction<InspectView>>
  sessionMap: Map<string, SessionView>
  showSessionById: (sessionId: string) => void
  inspectOpen: boolean
  inspectView: InspectView
  focusedId: string | null
  setLayout: Dispatch<SetStateAction<Layout>>
}) {
  // The repository view reads this computer's checkouts: on a pane of another machine it says so.
  const inspectRefusal = guardLocal(focusedSession, 'inspect')
  const toggleInspect = useCallback(() => {
    if (inspectRefusal !== null) toast.message(inspectRefusal)
    else setInspectOpen((open) => !open)
  }, [inspectRefusal, setInspectOpen])
  // An edit block's header: show the repository's changes in the Inspect panel.
  const showChanges = useCallback(() => {
    if (inspectRefusal !== null) {
      toast.message(inspectRefusal)
      return
    }
    setInspectView('changes')
    setInspectOpen(true)
  }, [inspectRefusal, setInspectOpen, setInspectView])
  // A session menu's Show changes: that session in front, its repository's changes beside it.
  const showChangesOf = useCallback((sessionId: string) => {
    const refusal = guardLocal(sessionMap.get(sessionId), 'inspect')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    showSessionById(sessionId)
    setInspectView('changes')
    setInspectOpen(true)
  }, [sessionMap, setInspectOpen, setInspectView, showSessionById])
  // A pane's side-panel button: the pane takes the focus and the panel opens for it, on `view` when one is asked for; it closes when it already showed that.
  const togglePanel = useCallback((paneId: string, sessionId: string, view?: InspectView) => {
    const refusal = guardLocal(sessionMap.get(sessionId), 'inspect')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    const showing = inspectOpen && focusedId === sessionId && (view === undefined || inspectView === view)
    setLayout((current) => focusPane(current, paneId))
    if (view !== undefined) setInspectView(view)
    setInspectOpen(!showing)
  }, [sessionMap, inspectOpen, inspectView, focusedId, setLayout, setInspectView, setInspectOpen])
  return { toggleInspect, inspectRefusal, showChangesOf, showChanges, togglePanel }
}
