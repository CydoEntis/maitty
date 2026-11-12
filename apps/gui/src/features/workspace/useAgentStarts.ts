import { movePanes } from './components/paneTransition'
import { type Dispatch, type SetStateAction, useCallback } from 'react'
import { toast } from '@/lib/toast'
import { defaultSessionFolder, sendTerminalInput } from '@/api/daemon'
import { LOCAL_MACHINE } from '@/api/machines'
import { type AgentTerminals, withAgentTerminal } from '@/data/agentTerminals'
import type { DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import { remoteStartRefusal } from './model/machines'
import { type Layout, openArrangement } from './model/layout'
import type { SessionView } from './model/session'
import { resolveStartFolder } from './model/startFolder'
import { makeId } from './makeId'

/** Starts for what is in front: another terminal for an agent, or an agent in the focused folder. */
export function useAgentStarts({ sessionMap, endSessions, setAgentTerminals, onTerminalAdded, runStart, setLayout, focusedSession, setSettingsSection, preferences, home }: {
  sessionMap: Map<string, SessionView>
  endSessions: (ids: string[]) => void
  setAgentTerminals: Dispatch<SetStateAction<AgentTerminals>>
  /** A terminal was started for an agent: the window shows it where terminals open. */
  onTerminalAdded: (agentId: string, terminalId: string) => void
  runStart: (folder: () => Promise<string>, place: (sessionId: string) => void, launch?: (folder: string) => Promise<void>) => Promise<boolean>
  setLayout: Dispatch<SetStateAction<Layout>>
  focusedSession: SessionView | undefined
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  preferences: SettingsPreferences
  home: string
}) {
  // "New terminal" on an agent: a shell in its folder, as another of its terminal tabs.
  const addAgentTerminal = useCallback((agentId: string) => {
    const agent = sessionMap.get(agentId)
    if (!agent) return
    const refusal = remoteStartRefusal(agent.machine !== LOCAL_MACHINE, agent.machineName, 'Opening a terminal here')
    if (refusal) {
      toast.message(refusal)
      return
    }
    void runStart(() => Promise.resolve(agent.cwd), (id) => {
      setAgentTerminals((current) => withAgentTerminal(current, agentId, id, new Set([...sessionMap.keys(), id])))
      onTerminalAdded(agentId, id)
    })
  }, [sessionMap, runStart, setAgentTerminals, onTerminalAdded])

  // Closing a tab ends its shell. It stays in the agent's list, so Undo brings the tab back and not a stray session.
  const closeAgentTerminal = useCallback((terminalId: string) => { endSessions([terminalId]) }, [endSessions])

  const startAgent = useCallback((program: string) => {
    const refusal = remoteStartRefusal(focusedSession !== undefined && focusedSession.machine !== LOCAL_MACHINE, focusedSession?.machineName ?? null, `Starting ${program} here`)
    if (refusal) {
      toast.message(refusal)
      return
    }
    setSettingsSection(null)
    void runStart(async () => focusedSession?.cwd ?? resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => {
      movePanes(() => { setLayout((current) => openArrangement(current, id, makeId)) })
      void sendTerminalInput({ id, generation: 0 }, `${program}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${program}`)) })
    })
  }, [focusedSession, setSettingsSection, runStart, preferences.startFolder, home, setLayout])
  return { addAgentTerminal, closeAgentTerminal, startAgent }
}
