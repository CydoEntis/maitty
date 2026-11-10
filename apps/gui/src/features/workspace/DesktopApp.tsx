import { type CSSProperties, Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Panel } from '@/components/ui/panel'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from '@/lib/toast'
import { defaultSessionFolder, isDesktop, loadConversations, loadDaemonRestartStatus, markSeenOnDaemon, openDaemonLog, sendTerminalInput } from '@/api/daemon'
import { listSshHosts, LOCAL_MACHINE, machineActivity } from '@/api/machines'
import { TitleBar } from '@/components/TitleBar'
import { Toaster } from '@/components/ui/sonner'
import { markConversationsDisconnected } from '@/data/connection'
import { DesktopSettings, type DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import { readSettingsPreferences, type SettingsPreferences, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN } from '@/features/settings/preferences'
import { sizeVariables } from './model/layoutSize'
import { daemonConnectionError, daemonProtocolMismatch } from '@/lib/daemon-connection'
import { errorMessage } from '@/lib/error-message'
import { guardLocal } from '@/lib/localOnly'
import { PerfOverlay } from './components/PerfOverlay'
import { ReducedMotionContext } from './components/motion'
import { SplitHandle } from './components/SplitHandle'
import { CommandPalette } from './components/CommandPalette'
import { type InboxEntry, InboxPanel } from './components/InboxPanel'
import { InspectPanel, type InspectView } from '@/features/inspect/InspectPanel'
import { rememberSessions, withEnded } from './model/ended'
import { isChatSession, isListedChatSession } from './model/chatSession'
import { LOCAL_HEAD, machineHeads, remoteStartRefusal, remoteTerminals, type UpdateAction } from './model/machines'
import { HoverTips } from './components/HoverTips'
import { ShortcutOverlay } from '@/features/shortcuts/ShortcutOverlay'
import { type FocusMachine, StatusBar } from './components/StatusBar'
import { WelcomeDialog } from '@/features/welcome/WelcomeDialog'
import { isFirstRun, markWelcomed } from '@/features/welcome/welcomed'
import { repoDiff } from '@/features/inspect/api'
import { FindBar } from '@/features/find/FindBar'
import { RestoreBanner } from './components/RestoreBanner'
import { UpdateSlot } from '@/features/updates/UpdateSlot'
import { UpdatedNotice } from '@/features/updates/UpdatedNotice'
import { type RestartedUpdate, skipUpdate, takeRestarted } from '@/features/updates/updateMemory'
import { routeDrop } from '@/features/terminal/dropRoute'
import { dropOnComposer } from '@/features/chat/composerDrop'
import { chatFindOf } from '@/features/chat/chatSearch'
import { useFileDrop } from '@/features/terminal/useFileDrop'
import { indexRemoteFiles, listRemoteFolder } from '@/api/fileGets'
import { useFileGets } from '@/features/terminal/useFileGets'
import { useFileSends } from '@/features/terminal/useFileSends'
import { useImagePaste } from '@/features/terminal/useImagePaste'
import { updaterSummary, useUpdater } from '@/features/updates/useUpdater'
import { chooseProjectFolder } from '@/api/folderPicker'
import { showSessionNotice } from '@/features/notifications/notify'
import { NewWorktreeDialog } from '@/features/worktrees/NewWorktreeDialog'
import type { TerminalHandle } from '@/features/terminal/handle'
import type { LookAt } from '@/features/inspect/FilesTab'
import type { PaneTool } from './components/PaneCard'
import { connectAgentOnce } from './model/agentHooks'
import { machineProjects } from './model/newSession'
import { usePaneViews } from './usePaneViews'
import { type ChatAgent, chatAvailability, chatSourceOf, effectiveView } from './model/chatView'
import { NewSessionHost } from './components/NewSessionHost'
import { NewChatPage } from './components/NewChatPage'
import { GetFileDialog } from './components/GetFileDialog'
import { MachineUpdateDialog } from './components/MachineUpdateDialog'
import { useMachineActions, useRemoteMachines } from './useMachines'
import { useSessionAlerts } from './useSessionAlerts'
import { useTrayItem } from './useTrayItem'
import { useSystemDark, useSystemReducedMotion, useWindowNarrowerThan } from './useWindowQueries'
import { ConfirmCloseDialog } from './components/ConfirmCloseDialog'
import { RenameDialog, RenameMachineDialog } from './components/RenameDialog'
import { readSplitShells, withSplitShell, writeSplitShells } from '@/data/splitShells'
import { readMachineNicknames, withNickname, writeMachineNicknames } from '@/data/machineNicknames'
import { type OpenSubagent, Sidebar } from './components/Sidebar'
import { EmptyState, Toast } from './components/states'
import { ServiceStatus } from './components/ServiceStatus'
import { useSharedSettings } from '@/features/settings/sharedSettings'
import { activeArrangement, dropSession, EMPTY_LAYOUT, focusedSessionId, focusPane, type Layout, MAX_PANES, openArrangement, paneColumns, setColumnWidths, setPaneHeights, showSession, type SplitDirection, splitPane, visibleSessionIds } from './model/layout'
import { keptArrangements, rememberShown } from './model/keptTabs'
import { LAYOUT_RESET_NOTICE, readLayout, writeLayout } from './model/layoutStorage'
import { byPriority } from './model/inboxOrder'
import { displayName, folderNameOf, inCreationOrder, keepLabels, type SessionView, tildePath, toSessionView } from './model/session'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { bindingFor, detectPlatform, shortcutChords } from './model/shortcuts'
import { folderExists, resolveStartFolder } from './model/startFolder'
import { activeThemeName, findTheme, terminalMinimumContrast, themeStyle, xtermTheme } from './theme/themes'
import { withWindowStyle } from './theme/windowStyles'
import { COMPACT_WIDTH_PX, NARROW_WIDTH_PX } from './model/windowSize'
import { makeId } from './makeId'
import { useSessionStarts } from './useSessionStarts'
import { useClosing } from './useClosing'
import { useAppUpdate } from './useAppUpdate'
import { useShortcutKeys, useShortcutRunner } from './useShortcuts'
import { usePaletteItems } from './usePaletteItems'
import { useServiceOffers } from './useServiceOffers'
import { usePaneDrags } from './usePaneDrags'
import { usePreferenceEffects } from './usePreferenceEffects'
import { useSessionSync } from './useSessionSync'
import { useSessionActions } from './useSessionActions'
import { useInboxActions } from './useInboxActions'
import { WorkspacePane } from './components/WorkspacePane'
import './workspace.css'
import './components/sidebar.css'
import './components/pane.css'
import './components/menu.css'
import './components/dialogs.css'
import '@/features/chat/chat.css'

/** The daemon pushes every change (`sessions-changed`); this slow poll only catches a stream that went quiet. */
const SESSION_REFRESH_MS = 15_000

const TOAST_MS = 2600

/** What clicking the status bar's size resets the terminal text to. */
const DEFAULT_TEXT_SIZE = 14

const ABSOLUTE_PATH = /^(\/|[A-Za-z]:[\\/])/

const TEST_NOTICE_MS = 6000

const SECOND_MS = 1000

const DIFF_STAT_REFRESH_MS = 8000

/** A theme previews once the pointer rests on it, not as it passes over every row (each one repaints every terminal). */
const THEME_PREVIEW_DELAY_MS = 140

const MAX_RETRY_EXPONENT = 3

/** Failed reads in a row before the window says the service is unreachable. */
const OFFLINE_AFTER_FAILURES = 2

const BACKOFF_BASE = 2

const noop = () => undefined

export function DesktopApp() {
  const queryClient = useQueryClient()
  const platform = detectPlatform(navigator.userAgent)
  const compact = useWindowNarrowerThan(COMPACT_WIDTH_PX)
  const narrow = useWindowNarrowerThan(NARROW_WIDTH_PX)
  const systemReducedMotion = useSystemReducedMotion()
  const systemDark = useSystemDark()
  const updater = useUpdater()
  // A theme being hovered in Settings shows on the whole window until the pointer leaves or one is picked.
  const [previewTheme, setPreviewTheme] = useState<string | null>(null)
  const previewTimer = useRef<number | undefined>(undefined)
  const previewThemeSoon = useCallback((name: string | null) => {
    window.clearTimeout(previewTimer.current)
    if (name === null) {
      setPreviewTheme(null)
      return
    }
    previewTimer.current = window.setTimeout(() => { setPreviewTheme(name) }, THEME_PREVIEW_DELAY_MS)
  }, [])
  const [welcomeOpen, setWelcomeOpen] = useState(isFirstRun)
  const [shortcutsOpen, setShortcutsOpen] = useState(false)
  const [preferences, setPreferences] = useState(readSettingsPreferences)
  const paneViews = usePaneViews()
  const [saved] = useState(readLayout)
  const [layout, setLayout] = useState<Layout>(() => (preferences.restoreLayout ? saved.layout : EMPTY_LAYOUT))
  // A tab layout from an older version was dropped: say so once.
  useEffect(() => { if (saved.reset) toast.message(LAYOUT_RESET_NOTICE) }, [saved.reset])
  // The width while the sidebar's edge is being dragged; it is saved when the pointer is released.
  const [liveSidebarWidth, setLiveSidebarWidth] = useState<number | null>(null)
  const [settingsSection, setSettingsSection] = useState<DesktopSettingsSection | null>(null)
  const [renameId, setRenameId] = useState<string | null>(null)
  // The subagent whose own chat its session's pane shows instead of the session's (README: subagents).
  const [openSubagent, setOpenSubagent] = useState<OpenSubagent | null>(null)
  // The shell opened beside an agent with "Open terminal here", by the agent's session id.
  const [agentTerminals, setAgentTerminals] = useState<ReadonlyMap<string, string>>(new Map())
  const [getFileId, setGetFileId] = useState<string | null>(null)
  const [updateAsk, setUpdateAsk] = useState<{ name: string; action: UpdateAction } | null>(null)
  const [paletteOpen, setPaletteOpen] = useState(false)
  const [inboxOpen, setInboxOpen] = useState(false)
  const [focusMode, setFocusMode] = useState(false)
  const [inspectOpen, setInspectOpen] = useState(false)
  // The docked panel's width, so the Inspect drawer that overlays the window stays beside it and not under it.
  const [findOpen, setFindOpen] = useState(false)
  // Under 1100px the sidebar is the 60px rail; expanding it opens the full sidebar over the window.
  const [drawerOpen, setDrawerOpen] = useState(false)
  // Widening the window past the breakpoint puts the drawer away, so it does not come back open.
  if (!compact && drawerOpen) setDrawerOpen(false)
  // Interrupted agents whose banner you closed; the daemon stops reporting one once an agent runs again.
  const [dismissedRestores, setDismissedRestores] = useState<ReadonlySet<string>>(new Set())
  // The session a new worktree is made from, while the dialog is open.
  const [worktreeFrom, setWorktreeFrom] = useState<SessionView | null>(null)
  const [inspectView, setInspectView] = useState<InspectView>('changes')
  const [lookAt, setLookAt] = useState<LookAt | null>(null)
  const [windowFocused, setWindowFocused] = useState(() => document.hasFocus())
  // Inbox entries the user cleared, by the status they had; an entry comes back if the session changes state.
  const [cleared, setCleared] = useState<ReadonlyMap<string, SessionView['status']>>(new Map())
  const [toastMessage, setToastMessage] = useState<{ text: string; key: number } | null>(null)
  // After a restart into an update, the first thing said is which version this is and what changed.
  const [updatedTo, setUpdatedTo] = useState<RestartedUpdate | null>(takeRestarted)
  const [starting, setStarting] = useState(false)
  // The sessions there were when a start began: one the daemon lists before its tab is placed is kept off the sidebar until it is.
  const [startedFrom, setStartedFrom] = useState<ReadonlySet<string> | null>(null)
  // A new session on another machine asks for a folder there first.
  // The machine the New session modal opened on; null while it is closed.
  const [newSessionMachine, setNewSessionMachine] = useState<number | null>(null)
  // The page a new agent chat starts from (agent, folder, model, effort), over the panes.
  const [newChatOpen, setNewChatOpen] = useState(false)
  // Opening the new chat page is the moment to connect the default agent's hooks, so starting it does not wait for that.
  useEffect(() => {
    if (newChatOpen) connectAgentOnce(preferences.defaultAgent).catch(() => { /* the start itself reports a failure */ })
  }, [newChatOpen, preferences.defaultAgent])
  const [failures, setFailures] = useState(0)
  const [root, setRoot] = useState<HTMLDivElement | null>(null)
  const terminalHandles = useRef(new Map<string, TerminalHandle>())

  // While maittyd is unreachable, try again after 1, 2, 4, then every 8 seconds.
  const retryDelay = BACKOFF_BASE ** Math.min(MAX_RETRY_EXPONENT, Math.max(0, failures - 1)) * SECOND_MS
  const sessionsQuery = useQuery({
    queryKey: ['conversations'],
    queryFn: async () => {
      try {
        const loaded = await loadConversations()
        setFailures(0)
        return loaded
      } catch (error) {
        setFailures((count) => count + 1)
        throw error
      }
    },
    retry: false,
    refetchInterval: (query) => (query.state.status === 'error' ? retryDelay : SESSION_REFRESH_MS),
  })
  const homeQuery = useQuery({ queryKey: ['home-folder'], queryFn: defaultSessionFolder, staleTime: Infinity, retry: false })
  const home = homeQuery.data ?? ''
  const mismatch = sessionsQuery.isError ? daemonProtocolMismatch(sessionsQuery.error) : null
  // One read that failed while the sessions are known is tried again a second later before anything is called
  // unreachable: a single lost read is not an outage, and marking every session unknown for it is a false alarm.
  // Another version is certain at once.
  const oneLostRead = failures < OFFLINE_AFTER_FAILURES && mismatch === null && sessionsQuery.data !== undefined
  const online = !sessionsQuery.isError || oneLostRead
  // A service newer than this app cannot be restarted into agreement; only updating the app helps.
  const serviceNewer = mismatch !== null && mismatch.daemonProtocol > mismatch.guiProtocol
  const restartStatusQuery = useQuery({ queryKey: ['daemon-restart-status', mismatch?.daemonProtocol], queryFn: loadDaemonRestartStatus, enabled: Boolean(mismatch), retry: false })

  const { machines, machinesRef, machinesKnown, machineSaid } = useRemoteMachines()
  // Sessions started as a known agent, until the daemon says so itself (see `startAgentSession`).
  const [expectedAgents, setExpectedAgents] = useState<Record<string, ChatAgent>>({})
  const [shownLabels, setShownLabels] = useState<ReadonlyMap<string, SessionView>>(() => new Map())
  const liveSessions = useMemo(() => {
    // Terminal sessions, and the chat sessions the daemon owns that are not put away (ADR 0003).
    const terminals = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal' || isListedChatSession(item))
    const here = online ? terminals : markConversationsDisconnected(terminals)
    return [...here, ...remoteTerminals(machines)].map((conversation) => {
      const expected = expectedAgents[conversation.id]
      return toSessionView(expected !== undefined && !conversation.agent ? { ...conversation, agent: expected } : conversation, home)
    })
  }, [sessionsQuery.data, online, home, machines, expectedAgents])
  const everySession = useMemo(() => liveSessions.map((view) => keepLabels(view, shownLabels.get(view.id))), [liveSessions, shownLabels])
  // What each row shows is remembered, so a later refresh keeps it (see `keepLabels`); only a change is stored, which ends the loop.
  const labelsChanged = everySession.length !== shownLabels.size || everySession.some((view) => shownLabels.get(view.id)?.title !== view.title || shownLabels.get(view.id)?.cwd !== view.cwd || shownLabels.get(view.id)?.branch !== view.branch)
  if (labelsChanged) setShownLabels(new Map(everySession.map((view) => [view.id, view])))
  const sessions = everySession
  const sessionMap = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions])
  // A session the daemon stops listing keeps its pane, shown as ended, so a refresh that misses one
  // never closes a layout or moves your focus. A pane goes only when you close it.
  const [remembered, setRemembered] = useState<ReadonlyMap<string, SessionView>>(() => new Map())
  const [seenSessions, setSeenSessions] = useState(sessions)
  if (seenSessions !== sessions) {
    setSeenSessions(sessions)
    setRemembered((before) => rememberSessions(before, sessions, layout))
  }
  const paneSessions = useMemo(() => withEnded(sessionMap, remembered), [sessionMap, remembered])
  // A cleared Inbox entry stays hidden only while its session keeps the state it had when cleared;
  // once it moves on, coming back to that state is news again.
  const [seenForClear, setSeenForClear] = useState(sessions)
  if (seenForClear !== sessions) {
    setSeenForClear(sessions)
    const stillCleared = new Map([...cleared].filter(([id, status]) => sessionMap.get(id)?.status === status))
    if (stillCleared.size !== cleared.size) setCleared(stillCleared)
  }
  const focusedId = focusedSessionId(layout)
  // A click anywhere outside the panes takes the border off the focused pane; a click in a pane, or focus moving to another pane, puts it back.
  const focusedPaneId = activeArrangement(layout)?.focus ?? null
  const [outsidePanes, setOutsidePanes] = useState(false)
  const [seenFocusedPane, setSeenFocusedPane] = useState(focusedPaneId)
  if (seenFocusedPane !== focusedPaneId) {
    setSeenFocusedPane(focusedPaneId)
    setOutsidePanes(false)
  }
  useEffect(() => {
    const onMouseDown = (event: MouseEvent) => {
      setOutsidePanes(!(event.target instanceof Element && event.target.closest('[data-pane]')))
    }
    document.addEventListener('mousedown', onMouseDown, true)
    return () => { document.removeEventListener('mousedown', onMouseDown, true) }
  }, [])
  const focusedSession = useMemo(() => (focusedId ? sessionMap.get(focusedId) : undefined), [focusedId, sessionMap])
  // A path the machine kept a sent file at is typed into the pane the drop was for, if that pane is still open.
  const typeIntoPane = (sessionId: string, text: string) => {
    const handle = terminalHandles.current.get(sessionId)
    if (!handle) return false
    handle.paste(text)
    return true
  }
  const { sendFiles, sendImage } = useFileSends({ typeInto: typeIntoPane })
  const { getFile } = useFileGets()
  // A pasted image goes the same two ways (ticket 19): saved here and its path typed, or sent to the pane's machine.
  const pasteImage = useImagePaste({
    sendImage,
    typeInto: typeIntoPane,
    isConnected: (name) => [...machinesRef.current.values()].some((candidate) => candidate.state.name.toLowerCase() === name.toLowerCase() && candidate.state.state === 'connected'),
    online,
    windows: /Windows/i.test(navigator.userAgent),
  })
  // Files dropped on a pane type their paths into it; the pane under the pointer, else the focused one.
  // On another machine's pane a path here names nothing there, so the files are sent to it and the
  // paths they land at are typed instead (ADR 0007).
  useFileDrop((paths, at) => {
    const under = document.elementFromPoint(at.x, at.y)
    // A drop on a chat composer attaches the files to its message; it is not a drop on the pane around it (ticket 20).
    if (dropOnComposer(paths, under)) return
    const target = under?.closest<HTMLElement>('[data-session-id]')?.dataset.sessionId ?? focusedId
    const handle = target ? terminalHandles.current.get(target) : undefined
    if (!target || !handle) return
    const route = routeDrop(target, paths, /Windows/i.test(navigator.userAgent))
    if (route === null) return
    if (route.kind === 'refuse') {
      toast.error(route.message)
      return
    }
    if (route.kind === 'send') {
      const machine = [...machinesRef.current.values()].find((candidate) => candidate.state.name.toLowerCase() === route.machine.toLowerCase())
      if (machine === undefined || machine.state.state !== 'connected') {
        toast.error(`${route.machine} is not connected; reconnect before sending files to it`)
        return
      }
      sendFiles(target, route.machine, route.paths)
      handle.focus()
      return
    }
    if (!online) {
      toast.error('Reconnect to the Maitty service before dropping files on a pane')
      return
    }
    handle.paste(route.text)
    handle.focus()
  })
  // With the window in the background nothing is being looked at, so a session that finishes then keeps its unread dot.
  const visibleKey = windowFocused ? [...visibleSessionIds(layout)].sort().join(' ') : ''
  // Every session some layout shows: one the daemon lists before its pane is placed is kept off the sidebar until it is.
  const openIds = useMemo(() => new Set(layout.arrangements.flatMap((candidate) => candidate.panes.map((pane) => pane.sessionId))), [layout.arrangements])
  const [machineNicknames, setMachineNicknames] = useState(readMachineNicknames)
  const [renamingMachine, setRenamingMachine] = useState<string | null>(null)
  const heads = useMemo(() => machineHeads(machines, machineNicknames), [machines, machineNicknames])
  // The machine of the pane in front, named in the status bar once there is more than one machine to tell apart.
  const focusedMachine = focusedSession?.machine
  const focusMachine = useMemo<FocusMachine | null>(() => {
    if (heads.length === 0 || focusedMachine === undefined) return null
    if (focusedMachine === LOCAL_MACHINE) return { name: 'This computer', word: 'local', tone: 'ok' }
    const head = heads.find((candidate) => candidate.machine === focusedMachine)
    if (head === undefined) return null
    return { name: head.name ?? head.label, word: head.chip?.word ?? 'unknown', tone: head.chip?.tone ?? 'quiet' }
  }, [heads, focusedMachine])
  const { retryNamed, addNamed, grantNamed, updateNamed, updateProgress, copyCommand, cancelAdding, removeNamed } = useMachineActions(sessionsQuery.data)
  // The sidebar is one flat list in creation order; a status change, a pin or a focus never moves a row.
  const ordered = useMemo(() => inCreationOrder(sessions), [sessions])
  // A session the daemon lists a moment before its pane exists would show in the sidebar and then jump once placed.
  // A terminal opened by splitting a pane is part of that split, not a session of its own to list; one no pane shows is listed so it is never lost.
  const [splitShells, setSplitShells] = useState(readSplitShells)
  const sidebarSessions = useMemo(() => {
    const listed = ordered.filter((session) => !splitShells.has(session.id) || !openIds.has(session.id))
    if (startedFrom === null) return listed
    return listed.filter((session) => startedFrom.has(session.id) || openIds.has(session.id))
  }, [ordered, startedFrom, openIds, splitShells])

  // New sessions go to the machine of the session in front, else this computer.
  const focusedMachineNumber = focusedSession?.machine ?? LOCAL_MACHINE
  const sidebarMachineHeads = useMemo(() => (heads.length === 0 ? [LOCAL_HEAD] : heads), [heads])

  const reducedMotion = preferences.reduceMotion || systemReducedMotion
  const themeName = previewTheme ?? activeThemeName(preferences, systemDark)
  const theme = useMemo(() => withWindowStyle(findTheme(themeName), preferences.style), [themeName, preferences.style])
  // The theme's tokens are on the page root as well as on the window: a menu or popover that renders outside the window
  // (Radix portals to the page) then reads the same colours, and the shadcn names mapped on `:root` resolve to the theme.
  useEffect(() => {
    const names = Object.keys(themeStyle(theme)).filter((name) => name.startsWith('--'))
    const values = themeStyle(theme)
    for (const name of names) document.documentElement.style.setProperty(name, values[name] ?? '')
    return () => { for (const name of names) document.documentElement.style.removeProperty(name) }
  }, [theme])
  const look = useMemo(() => ({ fontFamily: preferences.terminalFontFamily, fontWeight: preferences.terminalFontWeight, fontWeightBold: preferences.terminalFontWeightBold, cursorStyle: preferences.cursorStyle, scrollback: preferences.scrollback, scrollSpeed: preferences.scrollSpeed, screenReader: preferences.terminalScreenReader, seeThrough: preferences.terminalSeeThrough, cursorBlink: preferences.cursorBlink, plainDrawing: preferences.terminalPlainDrawing, theme: xtermTheme(theme.tokens), minimumContrast: terminalMinimumContrast(theme) }), [preferences.terminalFontFamily, preferences.terminalFontWeight, preferences.terminalFontWeightBold, preferences.cursorStyle, preferences.cursorBlink, preferences.scrollback, preferences.scrollSpeed, preferences.terminalScreenReader, preferences.terminalSeeThrough, preferences.terminalPlainDrawing, theme])
  useSharedSettings(preferences, setPreferences, online)

  const flash = useCallback((text: string) => { setToastMessage({ text, key: Date.now() }) }, [])
  const { dropId, dragId } = usePaneDrags({ setLayout, flash })
  useEffect(() => {
    if (!toastMessage) return
    const timer = window.setTimeout(() => { setToastMessage(null) }, TOAST_MS)
    return () => { window.clearTimeout(timer) }
  }, [toastMessage])
  usePreferenceEffects({ preferences, reducedMotion })

  useEffect(() => { writeLayout(layout) }, [layout])
  const { refetchSessions, retryAt, tracks } = useSessionSync({ sessionsQuery, retryDelay, queryClient, setFailures, sessions, setLayout, machinesRef, visibleKey, flash })
  const { runStart, newSession, newTerminal, startAgentSession, newSessionOn, startChatHere, startNewChat, createWorktree, startFromModal } = useSessionStarts({ queryClient, sessions, setWorktreeFrom, setLayout, flash, focusedSession, setNewChatOpen, online, starting, setStarting, setStartedFrom, sessionMap, preferences, setExpectedAgents, paneViews, setSettingsSection, setNewSessionMachine, home, machinesRef })

  const split = useCallback((paneId: string | null, direction: SplitDirection) => {
    const tab = activeArrangement(layout)
    if (!tab) return
    if (tab.panes.length >= MAX_PANES) {
      toast.message(`A window holds ${String(MAX_PANES)} panes at most. Close one to split again.`)
      return
    }
    const source = tab.panes.find((pane) => pane.id === (paneId ?? tab.focus))
    const sourceSession = source ? sessionMap.get(source.sessionId) : undefined
    const refusal = sourceSession ? remoteStartRefusal(sourceSession.machine !== LOCAL_MACHINE, sourceSession.machineName, 'Splitting this pane') : null
    if (refusal) {
      toast.message(refusal)
      return
    }
    const cwd = sourceSession?.cwd
    void runStart(async () => cwd ?? resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => {
      setSplitShells((known) => {
        const next = withSplitShell(known, id, new Set([...sessionMap.keys(), id]))
        writeSplitShells(next)
        return next
      })
      // If the layout changed or filled up while the shell started, the shell still gets a layout: it must not be left with no pane.
      setLayout((current) => {
        const next = splitPane(current, source?.id ?? null, direction, id, makeId)
        return next === current ? openArrangement(current, id, makeId) : next
      })
    })
  }, [layout, sessionMap, runStart, preferences.startFolder, home])
  const { askToClose, endSessions, undoLastClose, lastClose, closePaneById, closingPanes, closeAsk, setCloseAsk } = useClosing({ setLayout, queryClient, layout, sessions, flash, paneSessions })

  const restartPane = useCallback((paneId: string) => {
    const pane = layout.arrangements.flatMap((tab) => tab.panes).find((candidate) => candidate.id === paneId)
    const old = pane ? paneSessions.get(pane.sessionId) : undefined
    if (!old) return
    if (isChatSession(old.conversation)) {
      toast.message('A chat session cannot be restarted. Close it and start a new one.')
      return
    }
    const refusal = remoteStartRefusal(old.machine !== LOCAL_MACHINE, old.machineName, 'Restarting this pane')
    if (refusal) {
      toast.message(refusal)
      return
    }
    void runStart(() => Promise.resolve(old.cwd), (id) => { setLayout((current) => dropSession(current, paneId, id)) })
  }, [layout, paneSessions, runStart])

  const endSession = useCallback((sessionId: string) => { askToClose([sessionId], () => { endSessions([sessionId], 'Session') }) }, [askToClose, endSessions])
  const { showSessionById, openPath, copyPathOf, showSubagent, saveName, openInSplit, splitSubagent, copyText, usageInfo } = useSessionActions({ setRenameId, queryClient, setOpenSubagent, setLayout, layout, flash, sessionMap, paneViews, sessions, preferences, setPreferences, setSettingsSection, setLookAt, setInspectView, setInspectOpen })
  const { runningVersion, skippedVersion, restartNow, setSkippedVersion, restarting } = useAppUpdate({ updater, serviceNewer, preferences, sessions, online })

  const chooseStartFolder = useCallback(async (current: string): Promise<string | null> => {
    try {
      // `~` and other relative text mean nothing to the picker; it opens where it likes then.
      return await chooseProjectFolder(ABSOLUTE_PATH.test(current) ? current : undefined)
    } catch (error) {
      toast.error(errorMessage(error, 'Could not open the folder picker'))
      return null
    }
  }, [])

  const showTestNotice = useCallback(() => {
    showSessionNotice('test', {
      kind: 'done',
      process: 'claude',
      status: 'done',
      title: 'A test session',
      state: 'finished',
      body: 'A card like this appears when a session you are not looking at finishes.',
      actions: [{ label: 'Open session', primary: true, onClick: () => undefined }],
      countdownMs: TEST_NOTICE_MS,
    })
  }, [])

  const updateSlot = (variant: 'footer' | 'rail') => (
    <UpdateSlot
      variant={variant}
      state={updater.state}
      version={runningVersion}
      skipped={skippedVersion}
      portalContainer={root}
      onInstall={updater.download}
      onRestartNow={restartNow}
      onCheck={updater.check}
      onSkip={(version) => { skipUpdate(version); setSkippedVersion(version) }}
    />
  )

  const setTextSize = useCallback((size: number) => {
    setPreferences((current) => ({ ...current, terminalFontSize: Math.min(TERMINAL_FONT_SIZE_MAX, Math.max(TERMINAL_FONT_SIZE_MIN, size)) }))
  }, [])

  // In a window too narrow for the sidebar, the same key opens and closes its drawer.
  const toggleSidebar = useCallback(() => {
    if (compact) setDrawerOpen((open) => !open)
    else setPreferences((current) => ({ ...current, compactSidebar: !current.compactSidebar }))
  }, [compact])

  // The repository view reads this computer's checkouts: on a pane of another machine it says so.
  const inspectRefusal = guardLocal(focusedSession, 'inspect')
  const toggleInspect = useCallback(() => {
    if (inspectRefusal !== null) toast.message(inspectRefusal)
    else setInspectOpen((open) => !open)
  }, [inspectRefusal])
  // An edit block's header: show the repository's changes in the Inspect panel.
  const showChanges = useCallback(() => {
    if (inspectRefusal !== null) {
      toast.message(inspectRefusal)
      return
    }
    setInspectView('changes')
    setInspectOpen(true)
  }, [inspectRefusal])
  // A session menu's Show changes: that session in front, its repository's changes beside it.
  const showChangesOf = useCallback((sessionId: string) => {
    const refusal = guardLocal(sessionMap.get(sessionId), 'inspect')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    showSessionById(sessionId)
    setInspectView('changes')
    setInspectOpen(true)
  }, [sessionMap, showSessionById])
  // A pane's Files and Changes tray: the pane takes the focus and the drawer shows that view, or closes when it already did.
  const togglePaneTool = useCallback((paneId: string, sessionId: string, tool: PaneTool) => {
    const refusal = guardLocal(sessionMap.get(sessionId), 'inspect')
    if (refusal !== null) {
      toast.message(refusal)
      return
    }
    const showing = inspectOpen && inspectView === tool && focusedId === sessionId
    setLayout((current) => focusPane(current, paneId))
    setInspectView(tool)
    setInspectOpen(!showing)
  }, [sessionMap, inspectOpen, inspectView, focusedId])
  const setThemeMode = useCallback((themeMode: SettingsPreferences['themeMode']) => { setPreferences((current) => ({ ...current, themeMode })) }, [])
  const pickTheme = useCallback((name: string) => { setPreferences((current) => (findTheme(name).light ? { ...current, lightTheme: name } : { ...current, darkTheme: name })) }, [])
  const openAppearance = useCallback(() => { setSettingsSection('appearance') }, [])
  const setStyle = useCallback((style: SettingsPreferences['style']) => { setPreferences((current) => ({ ...current, style })) }, [])
  const resetTextSize = useCallback(() => { setTextSize(DEFAULT_TEXT_SIZE) }, [setTextSize])
  const openShortcuts = useCallback(() => { setShortcutsOpen(true) }, [])
  const shortcutChord = useMemo(() => shortcutChords(bindingFor('shortcuts'), platform)[0] ?? [], [platform])
  const focusModeChord = useMemo(() => shortcutChords(bindingFor('focus-mode'), platform)[0] ?? [], [platform])

  // Which view a session shows, and whether it has a chat. Switching views only changes the window; no daemon command is sent.
  const chatOf = useCallback((session: SessionView) => {
    const availability = chatAvailability(chatSourceOf(session))
    // A chat session has no terminal to switch to: its pane is the chat, always.
    if (isChatSession(session.conversation)) return { availability, view: 'chat' as const }
    return { availability, view: effectiveView(paneViews.choices.get(session.id), availability, preferences.chatViewDefault) }
  }, [paneViews.choices, preferences.chatViewDefault])
  const { runShortcut } = useShortcutRunner({ newSession, focusedMachineNumber, newTerminal, setSettingsSection, startAgentSession, preferences, home, split, setLayout, layout, askToClose, endSessions, toggleSidebar, setPaletteOpen, setInboxOpen, sessions, focusedSession, flash, showSessionById, undoLastClose, chatOf, paneViews, setFocusMode, setShortcutsOpen, setTextSize, toggleInspect, setFindOpen, online, setWorktreeFrom })

  // Captured on the window before xterm sees the key, so only these few keys leave the shell.
  const overlayOpen = paletteOpen || inboxOpen
  const drawerShown = compact && drawerOpen
  const sidebarMode = compact || preferences.compactSidebar ? 'rail' : 'docked'
  // A name is typed in its row where the full sidebar shows; the rail and Focus mode ask in a dialog.
  const renameInline = !focusMode && (sidebarMode === 'docked' || drawerShown)
  const dialogOpen = settingsSection !== null || (renameId !== null && !renameInline) || getFileId !== null || newSessionMachine !== null || newChatOpen || renamingMachine !== null
  const modalOpen = dialogOpen || overlayOpen
  // Only the new chat page is up, with no dialog over it.
  const onNewChatPage = newChatOpen && settingsSection === null && (renameId === null || renameInline) && getFileId === null && newSessionMachine === null && renamingMachine === null
  useShortcutKeys({ runShortcut, canUndoClose: lastClose !== null, dialogOpen, onNewChatPage, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen, setPaletteOpen, setInboxOpen, setSettingsSection, setNewChatOpen, setDrawerOpen, setFocusMode, platform })

  // The folder each session sits in, for the inbox, the notices and the palette.
  const projectOf = useMemo(() => new Map(sessions.map((member) => [member.id, { name: folderNameOf(member), key: member.repoRoot ?? member.cwd }])), [sessions])

  useEffect(() => {
    const gained = () => { setWindowFocused(true) }
    const lost = () => { setWindowFocused(false) }
    window.addEventListener('focus', gained)
    window.addEventListener('blur', lost)
    return () => {
      window.removeEventListener('focus', gained)
      window.removeEventListener('blur', lost)
    }
  }, [])
  const inboxEntries: InboxEntry[] = useMemo(() => {
    const activity = (id: string) => tracks.get(id)?.changedAt ?? 0
    return sessions
      .filter((item) => (item.status === 'needs' || item.status === 'error' || item.status === 'done') && cleared.get(item.id) !== item.status && !(item.status === 'error' && item.id === focusedId && windowFocused))
      .sort(byPriority(activity))
      .map((item) => ({ session: item, projectName: projectOf.get(item.id)?.name ?? '' }))
  }, [sessions, cleared, tracks, projectOf, focusedId, windowFocused])
  const inboxSessions = useMemo(() => inboxEntries.map((entry) => entry.session), [inboxEntries])
  useTrayItem(inboxSessions, showSessionById)
  const needsCount = inboxEntries.filter((entry) => entry.session.status === 'needs').length
  const needsElsewhere = sessions.filter((item) => item.status === 'needs' && item.id !== focusedId).length

  // Inspect looks at the focused pane's own repository; the Inspector's changed-file count comes from the same daemon question.
  // A pane of another machine has no checkout on this computer, so there is nothing to inspect.
  const inspectTerm = online && isDesktop() && focusedSession && focusedSession.machine === LOCAL_MACHINE && !isChatSession(focusedSession.conversation) ? focusedSession.pane : null
  const diffStat = useQuery({
    queryKey: ['repo', inspectTerm, 'diff-stat'],
    queryFn: async () => {
      const diff = await repoDiff(inspectTerm ?? 0)
      return { additions: diff.additions, deletions: diff.deletions, files: diff.total_files }
    },
    // The count shows only in the Inspect panel, and it comes from a full diff: nothing is read while the panel is
    // closed, and not again just because the window came to the front.
    enabled: inspectTerm !== null && inspectOpen,
    refetchInterval: DIFF_STAT_REFRESH_MS,
    refetchOnWindowFocus: false,
    retry: false,
  })
  // Looking at a pane acknowledges it: a finished agent's done goes back to idle, as when the terminal app
  // focuses it, and an error in it leaves the inbox. Needs-you stays until the agent is answered.
  const viewedId = focusedSession?.id
  const viewedStatus = focusedSession?.status
  useEffect(() => {
    if (!online || !windowFocused || viewedId === undefined) return
    if (viewedStatus !== 'done') return
    markSeenOnDaemon({ id: viewedId })
      .then(() => queryClient.invalidateQueries({ queryKey: ['conversations'] }))
      .catch((error: unknown) => { console.warn('Could not mark the session seen', error) })
  }, [online, windowFocused, viewedId, viewedStatus, queryClient])
  const { answerSession, sendTo, resumeInterrupted, clearEntry, clearSettled } = useInboxActions({ sessionMap, flash, showSessionById, setDismissedRestores, setCleared, inboxEntries })

  useSessionAlerts({ sessions, sessionMap, machines, projectOf, online, focusedId, preferences, showSessionById, answerSession })
  // With broadcast on, what is typed in one pane also goes to the layout's other panes, but only the idle ones: never into a busy agent.
  const mirrorInput = useCallback((fromSessionId: string, data: string | Uint8Array) => {
    const showing = activeArrangement(layout)
    if (!showing?.broadcast || typeof data !== 'string') return
    for (const pane of showing.panes) {
      const other = pane.sessionId === fromSessionId ? undefined : sessionMap.get(pane.sessionId)
      if (other?.status === 'idle' && !isChatSession(other.conversation)) void sendTo(other, data).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not broadcast to a pane')) })
    }
  }, [layout, sessionMap, sendTo])

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
  }, [agentTerminals, sessionMap, endSessions, runStart])

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
  }, [runStart, focusedSession, preferences.startFolder, home])
  const { paletteItems } = usePaletteItems({ paletteOpen, platform, layout, sessions, projectOf, showSessionById, home, newSession, focusedMachineNumber, newTerminal, focusedSession, toggleAgentTerminal, runShortcut, heads, online, newSessionOn, preferences, setPreferences, split, setInboxOpen, setSettingsSection, inspectRefusal, setInspectOpen, setFindOpen, setFocusMode, setShortcutsOpen, setLayout, startAgent })
  const { serviceFooterText, offlineDetail, restartOffer, appBehindOffer } = useServiceOffers({ mismatch, serviceNewer, restartStatusQuery, refetchSessions, updater, restartNow, online, failures, sessionsQuery })

  const tab = activeArrangement(layout)
  // The tabs shown last stay mounted behind the one showing, so going back to one does not build its terminals again.
  const [recentTabs, setRecentTabs] = useState<readonly string[]>([])
  const shownTabs = rememberShown(recentTabs, layout.activeId)
  if (shownTabs !== recentTabs) setRecentTabs(shownTabs)
  const keptTabs = useMemo(() => keptArrangements(layout, shownTabs), [layout, shownTabs])
  // In the layout's own order, so a tab coming to the front never moves in the page.
  const mountedTabs = tab ? layout.arrangements.filter((candidate) => candidate.id === tab.id || keptTabs.includes(candidate)) : []
  const startFolderLabel = preferences.startFolder.trim() ? tildePath(resolveStartFolder(preferences.startFolder, home), home) : '~'
  const canStart = online && !starting
  const sidebarWidth = liveSidebarWidth ?? preferences.sidebarWidth
  const rootStyle = { ...themeStyle(theme), ...sizeVariables(preferences, sidebarWidth) } as CSSProperties
  const paneProps = { paneSessions, chatOf, openSubagent, agentTerminals, sessionMap, outsidePanes, dropId, dragId, paneViews, showChanges, online, setOpenSubagent, togglePaneTool, preferences, startChatHere, inspectOpen, inspectView, toggleAgentTerminal, root, look, modalOpen, mirrorInput, openPath, pasteImage, terminalHandles, setLayout, split, closePaneById, sendTo, copyPathOf, restartPane }

  // The sidebar, as the docked column or the 60px rail, or the full one inside the overlay of a narrow window.
  const renderSidebar = (mode: 'docked' | 'rail', afterPick: () => void, onToggle: () => void) => (
    <Sidebar
      sessions={sidebarSessions}
      machines={sidebarMachineHeads}
      focusedId={focusedId}
      openSubagent={openSubagent}
      mode={mode}
      online={online}
      canStart={canStart}
      renamingId={renameInline ? renameId : null}
      portalContainer={root}
      onToggleRail={onToggle}
      side={preferences.sidebarSide}
      order={preferences.sidebarOrder}
      width={sidebarWidth}
      onResize={setLiveSidebarWidth}
      onResizeEnd={(width) => {
        setPreferences((current) => ({ ...current, sidebarWidth: width }))
        setLiveSidebarWidth(null)
      }}
      inboxCount={inboxEntries.length}
      inboxNeeds={needsCount > 0}
      onOpenInbox={() => { afterPick(); setInboxOpen(true) }}
      onNewAgentChat={(machine) => { afterPick(); newSession(machine) }}
      onNewShell={(machine) => { afterPick(); newTerminal(machine) }}
      onAddMachine={() => { afterPick(); setSettingsSection('machines') }}
      onMachineSettings={() => { afterPick(); setSettingsSection('machines') }}
      onRenameMachine={setRenamingMachine}
      onUpdateMachine={(name, action) => { afterPick(); setUpdateAsk({ name, action }) }}
      onShowSession={(id) => { afterPick(); showSessionById(id) }}
      onShowSubagent={(sessionId, subagentId) => { afterPick(); showSubagent(sessionId, subagentId) }}
      onRenameSession={setRenameId}
      onSetName={saveName}
      onSplitSession={(id, direction) => { afterPick(); openInSplit(id, direction) }}
      onSplitSubagent={(sessionId, subagentId) => { afterPick(); splitSubagent(sessionId, subagentId) }}
      onOpenTerminal={(id) => { afterPick(); toggleAgentTerminal(id) }}
      onCopyPath={copyPathOf}
      onCopyText={copyText}
      onShowChanges={(id) => { afterPick(); showChangesOf(id) }}
      onGetFile={setGetFileId}
      onEndSession={endSession}
      onOpenSettings={() => { afterPick(); setSettingsSection('appearance') }}
      updateSlot={updateSlot('footer')}
      railUpdateSlot={updateSlot('rail')}
      draft={newChatOpen ? { machine: LOCAL_MACHINE } : null}
      serviceText={serviceFooterText}
      updateProgress={updateProgress}
    />
  )

  return (
    <ReducedMotionContext.Provider value={reducedMotion}>
        <div ref={setRoot} className={`mx-app${narrow ? ' is-narrow' : ''}`} style={rootStyle} data-motion={reducedMotion ? 'reduced' : 'full'} data-theme-kind={theme.light ? 'light' : 'dark'} data-side={preferences.sidebarSide}>
          <TitleBar />
          <div className="mx-window" role="application" aria-label="Maitty">
            {!focusMode && renderSidebar(sidebarMode, noop, toggleSidebar)}
            {!focusMode && drawerShown && (
              <Panel label="Sessions" onClose={() => { setDrawerOpen(false) }} scrimClassName="mx-drawer-scrim" className="mx-drawer">
                {renderSidebar('docked', () => { setDrawerOpen(false) }, () => { setDrawerOpen(false) })}
              </Panel>
            )}
            <div className="mx-main-wrap">
              <main className="mx-main" aria-label="Terminals">
                {newChatOpen && (
                  <NewChatPage
                    machines={sidebarMachineHeads.filter((head) => head.machine === LOCAL_MACHINE || head.chip?.tone === 'ok').map((head) => ({ machine: head.machine, label: head.label, local: head.machine === LOCAL_MACHINE }))}
                    onOtherMachine={(machine) => { setNewChatOpen(false); setNewSessionMachine(machine) }}
                    projects={machineProjects(sessions, LOCAL_MACHINE, focusedSession, resolveStartFolder(preferences.startFolder, home))}
                    defaultAgent={preferences.defaultAgent}
                    bypassAllowed={preferences.yoloClaude}
                    browse={() => chooseProjectFolder()}
                    onStart={startNewChat}
                    onAddMachine={() => { setNewChatOpen(false); setSettingsSection('machines') }}
                    onCancel={() => { setNewChatOpen(false) }}
                  />
                )}
                {findOpen && (
                  <FindBar
                    online={online}
                    otherMachines={heads.length > 0}
                    findInPane={(query, direction) => {
                      if (!focusedId) return false
                      // A pane that shows its chat is searched as the chat; its terminal is under it and out of sight.
                      const chat = chatFindOf(focusedId)
                      return chat ? chat.find(query, direction) : terminalHandles.current.get(focusedId)?.find(query, direction) ?? false
                    }}
                    clearPane={() => {
                      if (!focusedId) return
                      chatFindOf(focusedId)?.clear()
                      terminalHandles.current.get(focusedId)?.clearFind()
                    }}
                    nameOf={(term, machine) => { const found = sessions.find((item) => item.machine === machine && item.pane === term); return found ? displayName(found) : null }}
                    onOpenSession={(term, machine) => { const found = sessions.find((item) => item.machine === machine && item.pane === term); if (found) showSessionById(found.id) }}
                    onClose={() => { setFindOpen(false); if (focusedId) terminalHandles.current.get(focusedId)?.focus() }}
                  />
                )}
                <RestoreBanner
                  sessions={sessions.filter((item) => item.interrupted !== null && !dismissedRestores.has(item.id))}
                  online={online}
                  onResume={resumeInterrupted}
                  onDismiss={(id) => { setDismissedRestores((current) => new Set(current).add(id)) }}
                />
                <ServiceStatus online={online} failures={failures} mismatch={Boolean(mismatch)} retryAt={retryAt} detail={offlineDetail ?? (sessionsQuery.isError ? daemonConnectionError(sessionsQuery.error) : null)} restart={restartOffer} appBehind={appBehindOffer} sessions={sessions.length} onRetry={() => { void refetchSessions() }} onOpenLog={() => { openDaemonLog().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not open the log')) }) }} />
                {tab ? (
                  <div className="mx-pane-stack">
                  {mountedTabs.map((tab) => {
                  const kept = tab.id !== layout.activeId
                  return (
                  <div key={tab.id} className={`mx-panes${tab.panes.length > 1 && tab.max === null ? ' is-split' : ''}${kept ? ' is-kept' : ''}`} aria-hidden={kept ? true : undefined} inert={kept}>
                    {paneColumns(tab, narrow).map((column, columnIndex, columns) => (
                      <Fragment key={column[0]?.id ?? columnIndex}>
                        {columnIndex > 0 && tab.max === null && !narrow && (
                          <SplitHandle
                            axis="columns"
                            index={columnIndex - 1}
                            onResize={(widths) => { setLayout((current) => setColumnWidths(current, tab.id, widths)) }}
                            onReset={() => { setLayout((current) => setColumnWidths(current, tab.id, null)) }}
                          />
                        )}
                      <div className={`mx-pane-column${column.every((pane) => closingPanes.has(pane.id)) ? ' is-closing' : ''}`} style={tab.widths && tab.max === null && !narrow && tab.widths.length === columns.length ? { flexGrow: tab.widths[columnIndex] } : undefined}>
                        {column.map((pane, rowIndex) => (
                          <Fragment key={pane.id}>
                            {rowIndex > 0 && tab.max === null && !narrow && (
                              <SplitHandle
                                axis="rows"
                                index={rowIndex - 1}
                                onResize={(heights) => { setLayout((current) => setPaneHeights(current, tab.id, pane.col, heights)) }}
                                onReset={() => { setLayout((current) => setPaneHeights(current, tab.id, pane.col, null)) }}
                              />
                            )}
                            <div className={`mx-pane-cell${closingPanes.has(pane.id) ? ' is-closing' : ''}`} style={pane.h !== undefined && tab.max === null && !narrow ? { flexGrow: pane.h } : undefined}>
                              <WorkspacePane {...paneProps} shown={tab} pane={pane} kept={kept} />
                            </div>
                          </Fragment>
                        ))}
                      </div>
                      </Fragment>
                    ))}
                  </div>
                  )
                  })}
                  </div>
                ) : (
                  <EmptyState
                    hasSessions={sessions.length > 0}
                    startFolderLabel={startFolderLabel}
                                  canStart={canStart}
                    onNewSession={() => { newSession(focusedMachineNumber) }}
                    onChangeStartFolder={() => { setSettingsSection('general') }}
                  />
                )}
                {!focusMode && preferences.statusBar && (
                  <StatusBar
                    online={online}
                    machine={focusMachine}
                    themeMode={preferences.themeMode}
                    lightInUse={theme.light}
                    activeTheme={theme.name}
                    style={preferences.style}
                    textSize={preferences.terminalFontSize}
                    minTextSize={TERMINAL_FONT_SIZE_MIN}
                    maxTextSize={TERMINAL_FONT_SIZE_MAX}
                    shortcutChord={shortcutChord}
                    portalContainer={root}
                    onThemeMode={setThemeMode}
                    onPickTheme={pickTheme}
                    onAllThemes={openAppearance}
                    onStyle={setStyle}
                    onTextSize={setTextSize}
                    onResetTextSize={resetTextSize}
                    onOpenShortcuts={openShortcuts}
                  />
                )}
              </main>
            </div>

            {inspectOpen && !focusMode && inspectTerm !== null && focusedSession && (
              <InspectPanel
                key={inspectTerm}
                term={inspectTerm}
                project={projectOf.get(focusedSession.id)?.name ?? displayName(focusedSession)}
                branch={focusedSession.branch}
                changedFiles={diffStat.data?.files ?? 0}
                view={inspectView}
                agentWorking={focusedSession.status === 'working'}
                timeline={focusedSession.timeline}
                lookAt={lookAt}
                onView={setInspectView}
                onClose={() => { setInspectOpen(false) }}
                onNotice={flash}
              />
            )}
            <HoverTips />
            {shortcutsOpen && <ShortcutOverlay platform={platform} portalContainer={root} onClose={() => { setShortcutsOpen(false) }} />}
            {welcomeOpen && (
              <WelcomeDialog
                platform={platform}
                startFolder={preferences.startFolder}
                portalContainer={root}
                validateFolder={folderExists}
                onStartFolder={(path) => { setPreferences((current) => ({ ...current, startFolder: path })) }}
                onClose={() => { markWelcomed(); setWelcomeOpen(false) }}
                onStartSession={() => { newSession(focusedMachineNumber) }}
              />
            )}
            {paletteOpen && <CommandPalette items={paletteItems} onClose={() => { setPaletteOpen(false) }} />}
            {inboxOpen && (
              <InboxPanel
                entries={inboxEntries}
                tracks={tracks}
                onClear={clearEntry}
                onClearDone={clearSettled}
                onOpen={(id) => { setInboxOpen(false); showSessionById(id) }}
                onClose={() => { setInboxOpen(false) }}
              />
            )}
            {focusMode && (
              <div className="mx-focus-pill" role="status">
                <span className="mx-focus-tag">Focus</span>
                {focusedId && sessionMap.get(focusedId) && <span>{projectOf.get(focusedId)?.name} · {sessionMap.get(focusedId)?.cwdLabel} · {sessionMap.get(focusedId)?.process}</span>}
                <span className={needsElsewhere > 0 ? 'mx-focus-needs' : undefined}>{needsElsewhere > 0 ? `${String(needsElsewhere)} need you elsewhere` : 'Nothing else needs you'}</span>
                <button type="button" className="mx-focus-exit" onClick={() => { setFocusMode(false) }}>Exit <Keycaps chord={focusModeChord} /></button>
              </div>
            )}
            {worktreeFrom && (
              <NewWorktreeDialog
                term={worktreeFrom.pane}
                repoName={worktreeFrom.repoName ?? worktreeFrom.cwdLabel}
                onCreate={(branch, base) => createWorktree(worktreeFrom, branch, base)}
                onCancel={() => { setWorktreeFrom(null) }}
              />
            )}
            {newSessionMachine !== null && (
              <NewSessionHost
                machine={newSessionMachine}
                heads={heads}
                sessions={sessions}
                focused={focusedSession}
                startFolder={resolveStartFolder(preferences.startFolder, home)}
                defaultAgent={preferences.defaultAgent}
                portalContainer={root}
                onStart={startFromModal}
                onCancel={() => { setNewSessionMachine(null) }}
              />
            )}
            {getFileId !== null && sessionMap.get(getFileId)?.machineName != null && (
              <GetFileDialog
                session={sessionMap.get(getFileId) as SessionView}
                browse={{
                  list: (path) => listRemoteFolder(sessionMap.get(getFileId)?.machineName ?? '', path),
                  index: (folder) => indexRemoteFiles(sessionMap.get(getFileId)?.machineName ?? '', folder),
                }}
                onGet={(path) => {
                  const target = sessionMap.get(getFileId)
                  setGetFileId(null)
                  if (target?.machineName != null) getFile(target.machineName, path)
                }}
                onCancel={() => { setGetFileId(null) }}
              />
            )}
            {updateAsk !== null && (
              <MachineUpdateDialog
                name={updateAsk.name}
                action={updateAsk.action}
                checkActivity={machineActivity}
                onUpdate={(whenIdle) => { setUpdateAsk(null); void updateNamed(updateAsk.name, updateAsk.action, whenIdle) }}
                onCancel={() => { setUpdateAsk(null) }}
              />
            )}
            {closeAsk && (
              <ConfirmCloseDialog title={closeAsk.title} body={closeAsk.body} confirmLabel={closeAsk.confirmLabel} onCancel={() => { setCloseAsk(null) }} onConfirm={() => { const run = closeAsk.run; setCloseAsk(null); run() }} />
            )}
            {renamingMachine !== null && (
              <RenameMachineDialog
                machine={renamingMachine}
                nickname={machineNicknames[renamingMachine.toLowerCase()] ?? ''}
                onSave={(nickname) => {
                  const next = withNickname(machineNicknames, renamingMachine, nickname)
                  setMachineNicknames(next)
                  writeMachineNicknames(next)
                  setRenamingMachine(null)
                }}
                onCancel={() => { setRenamingMachine(null) }}
              />
            )}
            {renameId !== null && !renameInline && sessionMap.get(renameId) && (
              <RenameDialog session={sessionMap.get(renameId) as SessionView} onSave={(name) => { saveName(renameId, name) }} onCancel={() => { setRenameId(null) }} />
            )}
            {settingsSection && (
              <DesktopSettings
                section={settingsSection}
                onSection={setSettingsSection}
                preferences={preferences}
                onChange={setPreferences}
                systemReducedMotion={systemReducedMotion}
                systemDark={systemDark}
                onPreviewTheme={previewThemeSoon}
                onTestNotice={showTestNotice}
                updates={{ summary: updaterSummary(updater.state), status: updater.state.status, onCheck: updater.check, onDownload: updater.download, onRestart: restartNow }}
                onShowWelcome={() => { setPreviewTheme(null); setSettingsSection(null); setWelcomeOpen(true) }}
                platform={platform}
                daemon={{ online, sessionCount: sessions.length }}
                appVersion={runningVersion}
                machines={{ known: machinesKnown, list: heads.filter((head) => head.name !== null), onRetry: retryNamed, onRemove: removeNamed, onGrant: grantNamed, onUpdate: updateNamed, onCheckActivity: machineActivity, onCopyCommand: copyCommand, add: { windows: navigator.userAgent.includes('Windows'), loadHosts: listSshHosts, onAdd: addNamed, onCancel: cancelAdding } }}
                usage={usageInfo}
                validateFolder={folderExists}
                onChooseFolder={chooseStartFolder}
                onClose={() => { setPreviewTheme(null); setSettingsSection(null) }}
              />
            )}
            {toastMessage && <Toast key={toastMessage.key} message={toastMessage.text} />}
            {updatedTo && <UpdatedNotice update={updatedTo} onDone={() => { setUpdatedTo(null) }} />}
            {preferences.perfOverlay && <PerfOverlay />}
            {restarting && (
              <div className="mx-restarting" role="status">
                <strong>Restarting Maitty</strong>
                <span>Reopening your layout. Your sessions never stopped.</span>
              </div>
            )}
          </div>
          <Toaster position={preferences.toastPosition} theme={theme.light ? 'light' : 'dark'} />
          <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">{machineSaid}</div>
        </div>
    </ReducedMotionContext.Provider>
  )
}
