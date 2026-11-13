import { useCallback, useEffect, useRef, useState } from 'react'
import { closedSessions, reopenClosedSession, type ClosedSession } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { LOCAL_MACHINE } from '@/api/machines'
import { DEFAULT_PICK } from '@/features/chat/agentChoice'
import type { NewChatRequest } from '../components/NewChatPage'
import { resumeWords } from './moveToWorktree'

/** What the Recent sessions list knows: nothing yet, the list, or why there is none. */
export type RecentRead = { kind: 'loading' } | { kind: 'listed'; list: readonly ClosedSession[] } | { kind: 'older' } | { kind: 'failed' }

const RESUMABLE = ['claude', 'codex'] as const
type ResumableAgent = (typeof RESUMABLE)[number]

function resumableAgent(agent: string): ResumableAgent | null {
  return RESUMABLE.find((known) => known === agent) ?? null
}

/** Whether the service can make a closed session's removed folder again: it was a worktree, and its repository and branch are known. */
function remakeable(closed: ClosedSession): boolean {
  return closed.worktree !== null && closed.repo !== null && closed.branch !== null
}

/** Why a closed session cannot be reopened from the list, or null when it can. */
export function reopenRefusal(closed: ClosedSession): string | null {
  if (!closed.folder_present && !remakeable(closed)) return 'folder removed; cannot be made again'
  const agent = resumableAgent(closed.agent)
  if (agent === null) return 'cannot be reopened here'
  // The id is typed on the agent's command line, so one the shared rule refuses is never started.
  if (resumeWords(agent, closed.session) === null) return 'no conversation to resume'
  return null
}

/**
 * The start that reopens a closed session: its agent in its folder with its conversation. Null when it cannot be, or
 * when its folder is gone, since the service then makes the folder again and starts the agent itself.
 */
export function reopenRequest(closed: ClosedSession): NewChatRequest | null {
  const agent = resumableAgent(closed.agent)
  if (agent === null || !closed.folder_present || reopenRefusal(closed) !== null) return null
  return { folder: closed.cwd, agent, model: DEFAULT_PICK, reasoning: DEFAULT_PICK, mode: 'default', message: '', branch: null, machine: LOCAL_MACHINE, machineName: 'This computer', resume: closed.session }
}

/** This computer's closed sessions, read off the UI thread when the list opens; `retry` asks again. */
export function useClosedSessions(): { read: RecentRead; retry: () => void } {
  const [read, setRead] = useState<RecentRead>({ kind: 'loading' })
  const [again, setAgain] = useState(0)
  useEffect(() => {
    const stop = new AbortController()
    void (async () => {
      try {
        const list = await closedSessions()
        if (!stop.signal.aborted) setRead(list === null ? { kind: 'older' } : { kind: 'listed', list })
      } catch (error) {
        if (!stop.signal.aborted) setRead({ kind: 'failed' })
        console.warn('Could not read the closed sessions', error)
      }
    })()
    return () => { stop.abort() }
  }, [again])
  const retry = useCallback(() => { setRead({ kind: 'loading' }); setAgain((count) => count + 1) }, [])
  return { read, retry }
}

/** Where reopening a session whose folder is gone stands. */
export type RemakeState = { kind: 'idle' } | { kind: 'working' } | { kind: 'failed'; message: string }

/** Reopening a session whose folder is gone, through the service, which makes it again and starts the agent; `onReopened` runs once it has. */
export function useReopenRemoved(onReopened: () => void): { state: RemakeState; reopen: (closed: ClosedSession) => void } {
  const [state, setState] = useState<RemakeState>({ kind: 'idle' })
  // A second pick before the first answers would start the session twice.
  const busy = useRef(false)
  // The list can close before the service answers; that answer then changes nothing here.
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => { mounted.current = false }
  }, [])
  const reopen = useCallback((closed: ClosedSession) => {
    if (busy.current) return
    busy.current = true
    setState({ kind: 'working' })
    void (async () => {
      let outcome: RemakeState
      try {
        const asked = await reopenClosedSession(closed.agent, closed.session)
        outcome = asked ? { kind: 'idle' } : { kind: 'failed', message: 'Restart the Maitty service to reopen it.' }
      } catch (error) {
        outcome = { kind: 'failed', message: errorMessage(error, 'Could not reopen the session.') }
      }
      busy.current = false
      if (!mounted.current) return
      setState(outcome)
      if (outcome.kind === 'idle') onReopened()
    })()
  }, [onReopened])
  return { state, reopen }
}
