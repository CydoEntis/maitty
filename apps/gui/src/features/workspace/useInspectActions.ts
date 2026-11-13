import { type Dispatch, type SetStateAction, useCallback } from 'react'
import { toast } from '@/lib/toast'
import { guardLocal } from '@/lib/localOnly'
import { isChatSession } from './model/chatSession'
import type { InspectView } from '@/features/inspect/InspectPanel'
import { focusPane, type Layout } from './model/layout'
import type { SessionView } from './model/session'

/**
 * The pane another machine's daemon is asked about for `session`'s changes, or null when it has
 * none: a chat there has no terminal of its own to stand for its folder.
 */
export function remoteRepoPane(session: SessionView): number | null {
  return isChatSession(session.conversation) ? (session.conversation.nativePane ?? null) : session.pane
}

/** Why the side panel has nothing for `session`, or null when it has: a session of another machine needs a pane its daemon can be asked about. */
function panelRefusal(session: SessionView | undefined): string | null {
  return session !== undefined && remoteRepoPane(session) === null ? guardLocal(session, 'inspect') : null
}

/** Opens and closes the side panel. For a session of another machine it shows that machine's changes; one with no pane to ask is told so instead. */
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
  const inspectRefusal = panelRefusal(focusedSession)
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
    const refusal = panelRefusal(sessionMap.get(sessionId))
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
    const refusal = panelRefusal(sessionMap.get(sessionId))
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
