import { useCallback } from 'react'
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
  const bringBack = useCallback((sessionId: string) => {
    archiveSessionOnDaemon({ id: sessionId }, false)
      .then(() => queryClient.invalidateQueries({ queryKey: ['conversations'] }))
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not bring the session back')) })
  }, [queryClient])
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
  // The daemon refuses a session that needs you, and says so; that line is the toast.
  const snooze = useCallback((sessionId: string, until: number) => {
    const session = sessionMap.get(sessionId)
    snoozeSessionOnDaemon({ id: sessionId }, until)
      .then(async () => {
        toast(`Snoozed ${session ? displayName(session) : 'the session'} ${untilWord(until * MS_PER_SECOND, Date.now())}`, { duration: UNDO_PUT_AWAY_MS, action: { label: 'Undo', onClick: () => { unsnooze(sessionId) } } })
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not snooze the session')) })
  }, [queryClient, sessionMap, unsnooze])
  return { putAway, bringBack, snooze, unsnooze }
}
