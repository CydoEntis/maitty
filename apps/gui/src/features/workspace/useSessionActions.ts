import { movePanes } from './components/paneTransition'
import { type Dispatch, type SetStateAction, useCallback } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import { toast } from '@/lib/toast'
import { renameConversationOnDaemon } from '@/api/daemon'
import { LOCAL_MACHINE } from '@/api/machines'
import type { DesktopSettingsSection, UsageInfo } from '@/features/settings/components/DesktopSettings'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import { guardLocal } from '@/lib/localOnly'
import type { InspectView } from '@/features/inspect/InspectPanel'
import { repoLocate } from '@/features/inspect/api'
import type { LookAt } from '@/features/inspect/FilesTab'
import { usePaneViews } from './usePaneViews'
import type { OpenSubagent } from './components/Sidebar'
import { activeArrangement, type Layout, MAX_PANES, showSession, splitPane } from './model/layout'
import { displayName, type SessionView } from './model/session'
import { makeId } from './makeId'
import { copyText as writeClipboard } from '@/lib/clipboard'

/** What a session's row and menu do: show it, rename it, split it, copy its path, open one of its files. */
export function useSessionActions({ setRenameId, queryClient, setOpenSubagent, setLayout, layout, flash, sessionMap, paneViews, sessions, preferences, setPreferences, setSettingsSection, setLookAt, setInspectView, setInspectOpen }: {
  setRenameId: Dispatch<SetStateAction<string | null>>
  queryClient: QueryClient
  setOpenSubagent: Dispatch<SetStateAction<OpenSubagent | null>>
  setLayout: Dispatch<SetStateAction<Layout>>
  layout: Layout
  flash: (text: string) => void
  sessionMap: Map<string, SessionView>
  paneViews: ReturnType<typeof usePaneViews>
  sessions: SessionView[]
  preferences: SettingsPreferences
  setPreferences: Dispatch<SetStateAction<SettingsPreferences>>
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  setLookAt: Dispatch<SetStateAction<LookAt | null>>
  setInspectView: Dispatch<SetStateAction<InspectView>>
  setInspectOpen: Dispatch<SetStateAction<boolean>>
}) {
  const saveName = useCallback((sessionId: string, name: string) => {
    setRenameId(null)
    void renameConversationOnDaemon({ id: sessionId }, name)
      .then(async () => { await queryClient.invalidateQueries({ queryKey: ['conversations'] }) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not rename the session')) })
  }, [queryClient, setRenameId])

  // Showing a session shows its own chat: a subagent's chat is left by picking any session, its parent too.
  const showSessionById = useCallback((sessionId: string) => {
    setOpenSubagent(null)
    movePanes(() => { setLayout((current) => showSession(current, sessionId, makeId)) })
  }, [setLayout, setOpenSubagent])

  // "Open in split": the session joins the layout beside the pane that has focus. One already in it just takes the focus.
  const openInSplit = useCallback((sessionId: string, direction: 'right' | 'down') => {
    const showing = activeArrangement(layout)
    if (!showing) {
      showSessionById(sessionId)
      return
    }
    if (showing.panes.some((pane) => pane.sessionId === sessionId)) {
      showSessionById(sessionId)
      return
    }
    if (showing.panes.length >= MAX_PANES) {
      toast.message(`A window holds ${String(MAX_PANES)} panes at most. Close one to open another beside it.`)
      return
    }
    movePanes(() => { setLayout((current) => splitPane(current, null, direction, sessionId, makeId)) })
  }, [layout, setLayout, showSessionById])

  const copyText = useCallback((text: string, what: string) => {
    writeClipboard(text).then(
      () => { flash(`${what} copied`) },
      (error: unknown) => { toast.error(errorMessage(error, `Could not copy the ${what.toLowerCase()}`)) },
    )
  }, [flash])

  const copyPathOf = useCallback((sessionId: string) => {
    const target = sessionMap.get(sessionId)
    if (target) copyText(target.cwd, 'Path')
  }, [sessionMap, copyText])

  // A subagent's own chat shows in its session's pane, in the chat view.
  const showSubagent = useCallback((sessionId: string, subagentId: string) => {
    movePanes(() => { setLayout((current) => showSession(current, sessionId, makeId)) })
    paneViews.choose(sessionId, 'chat')
    setOpenSubagent({ sessionId, subagentId })
  }, [paneViews, setLayout, setOpenSubagent])

  // A workflow run's steps show in its session's pane, in the chat view, as a subagent's chat does.
  const showRun = useCallback((sessionId: string, runKey: string) => {
    movePanes(() => { setLayout((current) => showSession(current, sessionId, makeId)) })
    paneViews.choose(sessionId, 'chat')
    setOpenSubagent({ sessionId, runKey })
  }, [paneViews, setLayout, setOpenSubagent])

  const splitSubagent = useCallback((sessionId: string, subagentId: string) => {
    openInSplit(sessionId, 'right')
    paneViews.choose(sessionId, 'chat')
    setOpenSubagent({ sessionId, subagentId })
  }, [openInSplit, paneViews, setOpenSubagent])

  // The usage report names panes of this computer's daemon by number; only a pane this window has can be opened.
  const sessionForUsageTerm = useCallback((term: number) => {
    const found = sessions.find((member) => member.machine === LOCAL_MACHINE && member.pane === term)
    return found ? { id: found.id, name: displayName(found) } : null
  }, [sessions])

  const usageInfo: UsageInfo = {
    budget: preferences.monthlyBudget,
    onBudget: (budget: number | null) => { setPreferences((current) => ({ ...current, monthlyBudget: budget })) },
    sessionForTerm: sessionForUsageTerm,
    onOpenSession: (id: string) => { setSettingsSection(null); showSessionById(id) },
  }

  // Cmd/Ctrl-click on a file name in a pane's output: find it in that pane's repo, then show it in Inspect.
  const openPath = useCallback((session: SessionView, path: string, line?: number) => {
    // Files and Changes look at this computer's own checkouts; a pane of another machine has none here.
    const refusal = guardLocal(session, 'path-links')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    void repoLocate(session.pane, path)
      .then((relative) => {
        showSessionById(session.id)
        setLookAt({ path: relative, ...(line === undefined ? {} : { line }), key: Date.now() })
        setInspectView('files')
        setInspectOpen(true)
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not open that file')) })
  }, [setInspectOpen, setInspectView, setLookAt, showSessionById])
  return { showSessionById, openPath, copyPathOf, showSubagent, showRun, saveName, openInSplit, splitSubagent, copyText, usageInfo }
}
