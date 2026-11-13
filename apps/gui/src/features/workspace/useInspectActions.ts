import { type Dispatch, type SetStateAction, useCallback, useState } from 'react'
import { toast } from '@/lib/toast'
import { guardLocal } from '@/lib/localOnly'
import type { InspectView } from '@/features/inspect/InspectPanel'
import type { TurnSpan } from '@/features/inspect/turnRange'
import { focusPane, type Layout } from './model/layout'
import type { SessionView } from './model/session'

/** A turn whose changes the panel shows instead of the whole working tree; `turn` is null when its times are unknown. */
export type TurnDiff = { sessionId: string; turn: TurnSpan | null }

/** The turn the panel shows. It belongs to the session it was opened for: when the focus moves to another, it goes. */
export function useTurnDiff(focusedId: string | null) {
  const [turnDiff, setTurnDiff] = useState<TurnDiff | null>(null)
  const [seenFocus, setSeenFocus] = useState(focusedId)
  if (seenFocus !== focusedId) {
    setSeenFocus(focusedId)
    if (turnDiff !== null && turnDiff.sessionId !== focusedId) setTurnDiff(null)
  }
  return [turnDiff, setTurnDiff] as const
}

/** Opens and closes the repository view. It reads this computer's checkouts, so a pane of another machine is told so instead. */
export function useInspectActions({ focusedSession, setInspectOpen, setInspectView, setTurnDiff, sessionMap, showSessionById, inspectOpen, inspectView, focusedId, setLayout }: {
  focusedSession: SessionView | undefined
  setInspectOpen: Dispatch<SetStateAction<boolean>>
  setInspectView: Dispatch<SetStateAction<InspectView>>
  setTurnDiff: Dispatch<SetStateAction<TurnDiff | null>>
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
  // The session's repository changes beside it, once `bringForward` has put it in front; a turn's own when one is given.
  const openChanges = useCallback((sessionId: string, bringForward: () => void, turn?: TurnSpan | null) => {
    const refusal = guardLocal(sessionMap.get(sessionId), 'inspect')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    bringForward()
    setTurnDiff(turn === undefined ? null : { sessionId, turn })
    setInspectView('changes')
    setInspectOpen(true)
  }, [sessionMap, setInspectOpen, setInspectView, setTurnDiff])
  // A chat's edit block or changes row: its own pane takes the focus, so a helper agent's chat open there stays open.
  const showChanges = useCallback((paneId: string, sessionId: string, _file?: string, turn?: TurnSpan | null) => {
    openChanges(sessionId, () => { setLayout((current) => focusPane(current, paneId)) }, turn)
  }, [openChanges, setLayout])
  // A session menu's Show changes: the session may be out of sight, so it is brought to the front.
  const showChangesOf = useCallback((sessionId: string) => { openChanges(sessionId, () => { showSessionById(sessionId) }) }, [openChanges, showSessionById])
  // A pane's side-panel button: the pane takes the focus and the panel opens for it, on `view` when one is asked for; it closes when it already showed that.
  const togglePanel = useCallback((paneId: string, sessionId: string, view?: InspectView) => {
    const refusal = guardLocal(sessionMap.get(sessionId), 'inspect')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    const showing = inspectOpen && focusedId === sessionId && (view === undefined || inspectView === view)
    setLayout((current) => focusPane(current, paneId))
    setTurnDiff(null)
    if (view !== undefined) setInspectView(view)
    setInspectOpen(!showing)
  }, [sessionMap, inspectOpen, inspectView, focusedId, setLayout, setInspectView, setInspectOpen, setTurnDiff])
  return { toggleInspect, inspectRefusal, showChangesOf, showChanges, togglePanel }
}
