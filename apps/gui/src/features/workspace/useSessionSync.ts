import { type Dispatch, type RefObject, type SetStateAction, useEffect, useRef, useState } from 'react'
import type { QueryClient, UseQueryResult } from '@tanstack/react-query'
import { idsOfMachinesNotHeard, type RemoteMachine } from './model/machines'
import { toast } from '@/lib/toast'
import { z } from 'zod'
import { conversationsFromSnapshot, listenSessionsChanged } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import { errorMessage } from '@/lib/error-message'
import { type Layout, openArrangement, reconcileLayout } from './model/layout'
import type { SessionView } from './model/session'
import { markSeen, type SessionTrack, tracksEqual, trackSessions } from './model/tracking'
import { makeId } from './makeId'

/** Keeps the window in step with the service: retries a lost read, hears its changes, and tracks what each session did while out of sight. */
export function useSessionSync({ sessionsQuery, retryDelay, queryClient, setFailures, sessions, setLayout, machinesRef, visibleKey, flash }: {
  sessionsQuery: UseQueryResult<Conversation[]>
  retryDelay: number
  queryClient: QueryClient
  setFailures: Dispatch<SetStateAction<number>>
  sessions: SessionView[]
  setLayout: Dispatch<SetStateAction<Layout>>
  machinesRef: RefObject<ReadonlyMap<number, RemoteMachine>>
  visibleKey: string
  flash: (text: string) => void
}) {
  const [tracks, setTracks] = useState<ReadonlyMap<string, SessionTrack>>(new Map())
  const tracksRef = useRef<ReadonlyMap<string, SessionTrack>>(new Map())
  const openedFirst = useRef(false)
  const refetchSessions = sessionsQuery.refetch
  const retryAt = sessionsQuery.isError ? sessionsQuery.errorUpdatedAt + retryDelay : null

  // The host forwards the daemon's snapshots as they change, so the list is current without asking.
  // A snapshot that does not read, or a dropped stream, sends the query to ask the daemon itself.
  useEffect(() => {
    const askAgain = () => {
      void queryClient.invalidateQueries({ queryKey: ['conversations'] })
    }
    const applySnapshot = (snapshot: unknown) => {
      if (snapshot === null) {
        askAgain()
        return
      }
      let conversations: Conversation[]
      try {
        conversations = conversationsFromSnapshot(snapshot)
      } catch (error: unknown) {
        if (!(error instanceof z.ZodError)) throw error
        console.warn('The daemon sent a session list this window cannot read; asking for it again', error)
        askAgain()
        return
      }
      // A poll still in flight answers with an older list than this push; it must not win.
      void queryClient.cancelQueries({ queryKey: ['conversations'] })
      queryClient.setQueryData(['conversations'], conversations)
      setFailures(0)
    }
    let stop: (() => void) | null = null
    let gone = false
    listenSessionsChanged(applySnapshot).then(
      (unlisten) => {
        if (gone) unlisten()
        else stop = unlisten
      },
      (error: unknown) => { toast.error(errorMessage(error, 'Could not follow the session list')) },
    )
    return () => {
      gone = true
      stop?.()
    }
  }, [queryClient, setFailures])

  // The first list after launch: drop panes of sessions that are gone since last time, and open
  // something when there is nothing to restore. Later lists never remove a pane: one that stopped
  // being listed stays, as an ended pane. `sessions` also changes when the home folder arrives; only a
  // new list counts.
  const reconciled = useRef<readonly Conversation[] | null>(null)
  useEffect(() => {
    // An unchanged list keeps its identity (structural sharing), so only a real change counts here.
    if (!sessionsQuery.isSuccess || reconciled.current === sessionsQuery.data) return
    reconciled.current = sessionsQuery.data
    const listed = sessions.map((session) => session.id)
    // This computer's first pane, else the first of another machine.
    const first = [...sessions].sort((a, b) => a.machine - b.machine || a.pane - b.pane)[0]
    const openFirst = !openedFirst.current
    openedFirst.current = true
    setLayout((current) => {
      // A machine that has not answered yet has not lost its panes: they are unknown, not gone.
      const live = new Set([...listed, ...idsOfMachinesNotHeard(current, machinesRef.current)])
      const reconciled = openFirst ? reconcileLayout(current, live) : current
      return openFirst && reconciled.arrangements.length === 0 && first ? openArrangement(reconciled, first.id, makeId) : reconciled
    })
  }, [sessions, sessionsQuery.data, sessionsQuery.isSuccess, machinesRef, setLayout])

  useEffect(() => {
    const { tracks: next, moves } = trackSessions(tracksRef.current, sessions, new Set(visibleKey.split(' ').filter(Boolean)), Date.now())
    if (!tracksEqual(tracksRef.current, next)) {
      tracksRef.current = next
      setTracks(next)
    }
    const move = moves.at(-1)
    if (move) {
      const name = (path: string | null) => path?.split(/[\\/]/).filter(Boolean).at(-1) ?? ''
      flash(move.to ? `Session moved to ${name(move.to)}` : `Session left ${name(move.from)}`)
    }
  }, [sessions, visibleKey, flash])

  useEffect(() => {
    const next = markSeen(tracksRef.current, new Set(visibleKey.split(' ').filter(Boolean)))
    if (!tracksEqual(tracksRef.current, next)) {
      tracksRef.current = next
      setTracks(next)
    }
  }, [visibleKey])
  return { refetchSessions, retryAt, tracks }
}
