import { useCallback, useEffect, useState } from 'react'
import { closedSessions, type ClosedSession } from '@/api/daemon'
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

/** Why a closed session cannot be reopened from the list, or null when it can. */
export function reopenRefusal(closed: ClosedSession): string | null {
  if (!closed.folder_present) return 'folder removed'
  const agent = resumableAgent(closed.agent)
  if (agent === null) return 'cannot be reopened here'
  // The id is typed on the agent's command line, so one the shared rule refuses is never started.
  if (resumeWords(agent, closed.session) === null) return 'no conversation to resume'
  return null
}

/** The start that reopens a closed session: its agent in its folder with its conversation, or null when it cannot be. */
export function reopenRequest(closed: ClosedSession): NewChatRequest | null {
  const agent = resumableAgent(closed.agent)
  if (agent === null || reopenRefusal(closed) !== null) return null
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
