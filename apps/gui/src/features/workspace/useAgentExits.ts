import { markClosed } from './closedSessions'
import { useEffect, useRef, type Dispatch, type SetStateAction } from 'react'
import type { AgentTerminals } from '@/data/agentTerminals'
import { LOCAL_MACHINE } from '@/api/machines'
import { movePanes } from './components/paneTransition'
import { type Layout, removeSessions } from './model/layout'
import type { SessionView } from './model/session'
import { startedAsShell } from './model/shellStarts'
import { isShell } from './model/sidebar'

/**
 * An agent that ended by itself (`/exit`) takes its pane with it. The daemon ends such a pane a moment after the
 * agent said its session was over, and until then lists it as the bare shell the agent ran in. A session that goes
 * from agent to bare shell to gone is that case: its pane leaves the layout instead of staying as an ended one, and
 * the terminals the agent opened for itself are ended, as when the agent is stopped by hand.
 *
 * A pane that vanishes with its agent still in it stays as an ended pane (it may show why), and so does a session
 * the person opened as a terminal and typed an agent into.
 */
export function useAgentExits({ sessions, listed, setLayout, agentTerminals, endSessions }: {
  sessions: readonly SessionView[]
  /** The list is the daemon's, read just now: an empty list while it is offline says nothing about what ended. */
  listed: boolean
  setLayout: Dispatch<SetStateAction<Layout>>
  /** The terminals each agent opened for itself, as the window keeps them (the gone agent's among them). */
  agentTerminals: AgentTerminals
  endSessions: (ids: string[]) => void
}): void {
  const wasAgent = useRef(new Set<string>())
  const lastSeen = useRef<ReadonlyMap<string, SessionView>>(new Map())
  useEffect(() => {
    if (!listed) return
    const now = new Map(sessions.map((session) => [session.id, session]))
    const ended = [...lastSeen.current.values()].filter((session) => !now.has(session.id)
      && wasAgent.current.has(session.id)
      && isShell(session)
      && session.machine === LOCAL_MACHINE
      && !startedAsShell(session.id))
    for (const session of sessions) if (!isShell(session)) wasAgent.current.add(session.id)
    for (const id of wasAgent.current) if (!now.has(id)) wasAgent.current.delete(id)
    lastSeen.current = now
    if (ended.length === 0) return
    const gone = ended.map((session) => session.id)
    // The agent ended on purpose: its row goes now, not after the grace a lost row gets.
    markClosed(gone)
    const theirs = ended.flatMap((session) => agentTerminals.get(session.id) ?? []).filter((id) => now.has(id))
    // With terminals to end, the close takes the agents' panes out as well, in the one layout change it makes when
    // the daemon has answered: a change made here first would be undone by it.
    if (theirs.length > 0) endSessions([...gone, ...theirs])
    else movePanes(() => { setLayout((current) => removeSessions(current, new Set(gone))) })
  }, [sessions, listed, setLayout, agentTerminals, endSessions])
}
