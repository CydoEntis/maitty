import { useCallback } from 'react'
import { useLatest } from '@/lib/useLatest'
import type { QueryClient } from '@tanstack/react-query'
import { archiveSessionOnDaemon, snoozeSessionOnDaemon } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { displayName, untilWord, type SessionView } from './model/session'

/** As long as a close's Undo, so the two feel the same. */
const UNDO_PUT_AWAY_MS = 6000
const MS_PER_SECOND = 1000

/** Put away and Bring back, Snooze and Unsnooze: view settings the daemon keeps for every window. Nothing is closed, stopped or killed. */
export function usePutAway({ queryClient, sessionMap }: { queryClient: QueryClient; sessionMap: ReadonlyMap<string, SessionView> }) {
  // Bring back also wakes a snooze, or the row would stay under Put away and the click would look like it did nothing.
  const bringBack = useCallback((sessionId: string) => {
    const snoozed = sessionMap.get(sessionId)?.snoozedUntil != null
    archiveSessionOnDaemon({ id: sessionId }, false)
      .then(() => (snoozed ? snoozeSessionOnDaemon({ id: sessionId }, null) : undefined))
      .then(() => queryClient.invalidateQueries({ queryKey: ['conversations'] }))
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not bring the session back')) })
  }, [queryClient, sessionMap])
  const putAway = useCallback((sessionId: string) => {
    const session = sessionMap.get(sessionId)
    archiveSessionOnDaemon({ id: sessionId }, true)
      .then(async () => {
        toast(`Put away ${session ? displayName(session) : 'the session'}`, { duration: UNDO_PUT_AWAY_MS, action: { label: 'Undo', onClick: () => { bringBack(sessionId) } } })
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not put the session away')) })
  }, [queryClient, sessionMap, bringBack])
  const unsnooze = useCallback((sessionId: string) => {
    snoozeSessionOnDaemon({ id: sessionId }, null)
      .then(() => queryClient.invalidateQueries({ queryKey: ['conversations'] }))
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not unsnooze the session')) })
  }, [queryClient])
  // Undo wakes the session only while it is still snoozed until the time this snooze set: another window may have changed it since.
  const undoSnooze = useLatest((sessionId: string, until: number) => {
    if (sessionMap.get(sessionId)?.snoozedUntil === until * MS_PER_SECOND) unsnooze(sessionId)
  })
  // The daemon refuses a session that needs you, and says so; that line is the toast. A pane on another machine is asked without
  // waiting for its answer (as Put away is), so for it the toast says it asked rather than that it is done.
  const snooze = useCallback((sessionId: string, until: number) => {
    const session = sessionMap.get(sessionId)
    const name = session ? displayName(session) : 'the session'
    const when = untilWord(until * MS_PER_SECOND, Date.now())
    const said = session?.machineName ? `Asked to snooze ${name} ${when}` : `Snoozed ${name} ${when}`
    snoozeSessionOnDaemon({ id: sessionId }, until)
      .then(async () => {
        toast(said, { duration: UNDO_PUT_AWAY_MS, action: { label: 'Undo', onClick: () => { undoSnooze(sessionId, until) } } })
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not snooze the session')) })
  }, [queryClient, sessionMap, undoSnooze])
  return { putAway, bringBack, snooze, unsnooze }
}
