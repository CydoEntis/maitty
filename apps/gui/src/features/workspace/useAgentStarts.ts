import { type Dispatch, type SetStateAction, useCallback } from 'react'
import { toast } from '@/lib/toast'
import { defaultSessionFolder, sendTerminalInput } from '@/api/daemon'
import { LOCAL_MACHINE } from '@/api/machines'
import type { DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import { remoteStartRefusal } from './model/machines'
import { type Layout, openArrangement, showSession, splitPane } from './model/layout'
import type { SessionView } from './model/session'
import { resolveStartFolder } from './model/startFolder'
import { makeId } from './makeId'

/** Starts beside what is in front: a shell next to an agent ("Open terminal here"), or an agent in the focused folder. */
export function useAgentStarts({ agentTerminals, sessionMap, endSessions, setAgentTerminals, runStart, setLayout, focusedSession, setSettingsSection, preferences, home }: {
  agentTerminals: ReadonlyMap<string, string>
  sessionMap: Map<string, SessionView>
  endSessions: (ids: string[], what: string) => void
  setAgentTerminals: Dispatch<SetStateAction<ReadonlyMap<string, string>>>
  runStart: (folder: () => Promise<string>, place: (sessionId: string) => void, launch?: (folder: string) => Promise<void>) => Promise<boolean>
  setLayout: Dispatch<SetStateAction<Layout>>
  focusedSession: SessionView | undefined
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  preferences: SettingsPreferences
  home: string
}) {
  // "Open terminal here" on an agent: a shell in its folder beside it. Again ("Close its terminal") ends that shell.
  const toggleAgentTerminal = useCallback((agentId: string) => {
    const opened = agentTerminals.get(agentId)
    if (opened !== undefined && sessionMap.has(opened)) {
      endSessions([opened], 'Terminal')
      setAgentTerminals((current) => { const next = new Map(current); next.delete(agentId); return next })
      return
    }
    const agent = sessionMap.get(agentId)
    if (!agent) return
    const refusal = remoteStartRefusal(agent.machine !== LOCAL_MACHINE, agent.machineName, 'Opening a terminal here')
    if (refusal) {
      toast.message(refusal)
      return
    }
    void runStart(() => Promise.resolve(agent.cwd), (id) => {
      setAgentTerminals((current) => new Map(current).set(agentId, id))
      setLayout((current) => {
        const shown = showSession(current, agentId, makeId)
        const next = splitPane(shown, null, 'right', id, makeId)
        return next === shown ? openArrangement(shown, id, makeId) : next
      })
    })
  }, [agentTerminals, sessionMap, runStart, endSessions, setAgentTerminals, setLayout])

  const startAgent = useCallback((program: string) => {
    const refusal = remoteStartRefusal(focusedSession !== undefined && focusedSession.machine !== LOCAL_MACHINE, focusedSession?.machineName ?? null, `Starting ${program} here`)
    if (refusal) {
      toast.message(refusal)
      return
    }
    setSettingsSection(null)
    void runStart(async () => focusedSession?.cwd ?? resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => {
      setLayout((current) => openArrangement(current, id, makeId))
      void sendTerminalInput({ id, generation: 0 }, `${program}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${program}`)) })
    })
  }, [focusedSession, setSettingsSection, runStart, preferences.startFolder, home, setLayout])
  return { toggleAgentTerminal, startAgent }
}
