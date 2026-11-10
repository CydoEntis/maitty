import { type Dispatch, type SetStateAction, useCallback, useState } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import { toast } from '@/lib/toast'
import { closeChatSession, closeSessionWithUndo, undoCloseSession } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { useLatest } from '@/lib/useLatest'
import { runsProgram } from './components/runsProgram'
import { isChatSession } from './model/chatSession'
import { type Layout, removeSessions, showSession } from './model/layout'
import { displayName, type SessionView } from './model/session'
import { isShell } from './model/sidebar'
import { makeId } from './makeId'

/** How long a closed session can be brought back; the daemon keeps its process running this long. */
const UNDO_CLOSE_MS = 6000

/** How long a closing pane shrinks before its session ends (matches `.mx-pane-cell.is-closing` in pane.css), and how long it is kept marked if the close did not happen. */
const PANE_EXIT_MS = 240

const PANE_CLOSE_SETTLE_MS = 1500

/** Closing panes and sessions: asking first when something would be lost, and keeping the last close undoable. */
export function useClosing({ setLayout, queryClient, layout, sessions, flash, paneSessions }: {
  setLayout: Dispatch<SetStateAction<Layout>>
  queryClient: QueryClient
  layout: Layout
  sessions: SessionView[]
  flash: (text: string) => void
  paneSessions: ReadonlyMap<string, SessionView>
}) {
  // Closing a pane or session ends it, with a few seconds to take it back: the daemon hides the session and
  // stops its program only when the window passes. A session that is still running asks first, in a dialog.
  const [closeAsk, setCloseAsk] = useState<{ title: string; body: string; confirmLabel: string; run: () => void } | null>(null)
  // The last close, while it can still be undone: the layout it came out of is put back with it.
  const [lastClose, setLastClose] = useState<{ toastId: string | number; sessions: SessionView[]; before: Layout; after: Layout } | null>(null)
  const undoLastClose = useCallback(() => {
    const closed = lastClose
    if (closed === null) return
    setLastClose(null)
    toast.dismiss(closed.toastId)
    void Promise.all(closed.sessions.map((session) => undoCloseSession(session.conversation)))
      .then(async () => {
        setLayout((current) => (current === closed.after ? closed.before : closed.sessions.reduce((layoutNow, session) => showSession(layoutNow, session.id, makeId), current)))
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not bring the session back')) })
  }, [lastClose, setLayout, queryClient])
  // The toast's button outlives the render that made it, so it calls whichever undo is current.
  const undoNow = useLatest(undoLastClose)
  // Ending a session takes a moment, and the layout may change meanwhile (a split, a resize, another close):
  // the pane is taken out of the layout as it is when the daemon answers, not as it was when the close began.
  const layoutNow = useLatest(() => layout)
  const endSessions = useCallback((ids: string[], what: string) => {
    const gone = new Set(ids)
    const running = sessions.filter((session) => gone.has(session.id))
    // Panes of sessions that already ended have nothing to stop: they just close.
    if (running.length === 0) {
      setLayout((current) => removeSessions(current, gone))
      return
    }
    // A chat session has no terminal to hide: it is ended for good, as before. Terminals get the undo window.
    const chats = running.filter((session) => isChatSession(session.conversation))
    const terminals = running.filter((session) => !isChatSession(session.conversation))
    void Promise.all([
      ...chats.map((session) => closeChatSession(session.conversation)),
      ...terminals.map((session) => closeSessionWithUndo(session.conversation)),
    ])
      .then(async () => {
        const before = layoutNow()
        const after = removeSessions(before, gone)
        setLayout(after)
        if (terminals.length > 0) {
          const label = terminals.length === 1 && terminals[0] ? displayName(terminals[0]) : `${String(terminals.length)} sessions`
          const toastId = toast(`Closed ${label}`, { duration: UNDO_CLOSE_MS, action: { label: 'Undo', onClick: undoNow } })
          setLastClose({ toastId, sessions: terminals, before, after })
          window.setTimeout(() => { setLastClose((current) => (current?.toastId === toastId ? null : current)) }, UNDO_CLOSE_MS)
        } else {
          flash(`${what} closed · session ended`)
        }
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not end the session')) })
  }, [sessions, setLayout, layoutNow, queryClient, undoNow, flash])

  /**
   * Runs `close` now when no session in `ids` has anything to lose: it has ended, or it is a shell with nothing
   * running in it, which Undo brings back as it was. Otherwise asks first.
   */
  const askToClose = useCallback((ids: string[], close: () => void) => {
    const live = ids.flatMap((id) => paneSessions.get(id) ?? []).filter((session) => !session.ended && !(isShell(session) && !isChatSession(session.conversation)))
    const first = live[0]
    if (!first) {
      close()
      return
    }
    const only = live.length === 1
    const name = only ? displayName(first) : `${String(live.length)} sessions`
    setCloseAsk({
      title: `Close ${name}?`,
      body: only && runsProgram(first)
        ? `This stops ${first.process}. You can undo it for a few seconds afterwards.`
        : 'This ends the session. You can undo it for a few seconds afterwards.',
      confirmLabel: runsProgram(first) ? `Stop ${first.process}` : 'Close',
      run: close,
    })
  }, [paneSessions])

  // A pane that is closing shrinks away first (its neighbours grow into the room as it goes); the session ends
  // when that is done. If the close does not happen it grows back.
  const [closingPanes, setClosingPanes] = useState<ReadonlySet<string>>(() => new Set())
  const closePaneNow = useCallback((paneId: string) => {
    const tab = layout.arrangements.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
    const pane = tab?.panes.find((candidate) => candidate.id === paneId)
    if (!tab || !pane) return
    if (tab.panes.length <= 1) {
      endSessions([pane.sessionId], 'Pane')
      return
    }
    setClosingPanes((current) => new Set(current).add(paneId))
    window.setTimeout(() => {
      endSessions([pane.sessionId], 'Pane')
      window.setTimeout(() => {
        setClosingPanes((current) => {
          const next = new Set(current)
          next.delete(paneId)
          return next
        })
      }, PANE_CLOSE_SETTLE_MS)
    }, PANE_EXIT_MS)
  }, [endSessions, layout])
  const closePaneById = useCallback((paneId: string) => {
    const pane = layout.arrangements.flatMap((tab) => tab.panes).find((candidate) => candidate.id === paneId)
    if (!pane) return
    askToClose([pane.sessionId], () => { closePaneNow(paneId) })
  }, [layout, askToClose, closePaneNow])
  return { askToClose, endSessions, undoLastClose, lastClose, closePaneById, closingPanes, closeAsk, setCloseAsk }
}
