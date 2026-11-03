import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  archiveConversationOnDaemon,
  markSeenOnDaemon,
  defaultSessionFolder,
  isDesktop,
  loadConversations,
  loadDaemonRestartStatus,
  renameConversationOnDaemon,
  restartMismatchedDaemon,
  sendTerminalInput,
  startShellSession,
} from '@/api/daemon'
import { TitleBar } from '@/components/TitleBar'
import { Toaster } from '@/components/ui/sonner'
import { markConversationsDisconnected } from '@/data/connection'
import { DesktopSettings, type DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import { readSettingsPreferences, writeSettingsPreferences } from '@/features/settings/preferences'
import { sizeVariables } from './model/layoutSize'
import { daemonConnectionError, daemonProtocolMismatch } from '@/lib/daemon-connection'
import { errorMessage } from '@/lib/error-message'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { TerminalView } from '@/TerminalView'
import { EffectsContext, ReducedMotionContext } from './components/motion'
import { ColumnHandle } from './components/ColumnHandle'
import { CommandPalette } from './components/CommandPalette'
import { InboxPanel, type Answer, type InboxEntry } from './components/InboxPanel'
import { InspectPanel, type InspectView } from '@/features/inspect/InspectPanel'
import { ConfirmEndDialog, type EndRequest } from './components/ConfirmEndDialog'
import { HoverTips } from './components/HoverTips'
import { ShortcutOverlay } from '@/features/shortcuts/ShortcutOverlay'
import { WelcomeDialog } from '@/features/welcome/WelcomeDialog'
import { isFirstRun, markWelcomed } from '@/features/welcome/welcomed'
import { repoDiff, repoLocate } from '@/features/inspect/api'
import { FindBar } from '@/features/find/FindBar'
import { RestoreBanner } from './components/RestoreBanner'
import { sendNativeNotice } from '@/features/notifications/native'
import { noticesFor } from '@/features/notifications/transitions'
import { NewWorktreeDialog } from '@/features/worktrees/NewWorktreeDialog'
import { newWorktreeSession } from '@/features/worktrees/api'
import type { TerminalHandle } from '@/features/terminal/handle'
import type { LookAt } from '@/features/inspect/FilesTab'
import { PaneCard } from './components/PaneCard'
import { RenameDialog } from './components/RenameDialog'
import { Sidebar } from './components/Sidebar'
import { EmptyState, OfflineBanner, Toast, type RestartOffer } from './components/states'
import { TabBar } from './components/TabBar'
import {
  activeTab,
  closeFocused,
  closePane,
  closeTab,
  dropSession,
  cycleFocus,
  cycleTab,
  EMPTY_LAYOUT,
  focusedSessionId,
  focusPane,
  MAX_PANES,
  openInNewTab,
  paneColumns,
  reconcileLayout,
  selectTab,
  setColumnWidths,
  toggleBroadcast,
  showSession,
  splitPane,
  swapSessions,
  toggleMaximize,
  visibleSessionIds,
  type Layout,
  type MakeId,
  type SplitDirection,
  type Tab,
} from './model/layout'
import { readLayout, writeLayout } from './model/layoutStorage'
import { byPriority } from './model/stack'
import { displayName, groupKeyOf, groupSessions, tildePath, toSessionView, type SessionView } from './model/session'
import { isMacPlatform, keyLabels, matchShortcut, type ShortcutAction } from './model/shortcuts'
import { folderExists, resolveStartFolder } from './model/startFolder'
import { markSeen, MOVED_HIGHLIGHT_MS, trackSessions, type SessionTrack } from './model/tracking'
import { folderColor } from './model/folderColor'
import type { PaletteItem } from './model/palette'
import { activeThemeName, findTheme, themeStyle, THEMES, xtermTheme } from './theme/themes'
import './workspace.css'

const SESSION_REFRESH_MS = 1500
const COMPACT_WIDTH_PX = 1100
const MIN_TABS_FOR_NUMBERS = 2
const NARROW_WIDTH_PX = 900
const TOAST_MS = 2600
const SECOND_MS = 1000
const SLOW_TICK_MS = 30_000
const DIFF_STAT_REFRESH_MS = 8000
const NOTIFICATION_CLICK_WINDOW_MS = 60_000
const MAX_RETRY_EXPONENT = 3
const BACKOFF_BASE = 2
const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)'

const noop = () => undefined

const makeId: MakeId = (prefix) => `${prefix}-${crypto.randomUUID()}`

function useWindowWidth(): number {
  const [width, setWidth] = useState(() => window.innerWidth)
  useEffect(() => {
    const onResize = () => { setWidth(window.innerWidth) }
    window.addEventListener('resize', onResize)
    return () => { window.removeEventListener('resize', onResize) }
  }, [])
  return width
}

function useSystemReducedMotion(): boolean {
  const query = typeof window.matchMedia === 'function' ? window.matchMedia(REDUCED_MOTION_QUERY) : null
  const [reduced, setReduced] = useState(() => query?.matches ?? false)
  useEffect(() => {
    if (!query) return
    const onChange = () => { setReduced(query.matches) }
    query.addEventListener('change', onChange)
    return () => { query.removeEventListener('change', onChange) }
  }, [query])
  return reduced
}

const DARK_SCHEME_QUERY = '(prefers-color-scheme: dark)'

/** Whether the operating system is set to dark, following it as it changes (System mode needs it). */
function useSystemDark(): boolean {
  const query = typeof window.matchMedia === 'function' ? window.matchMedia(DARK_SCHEME_QUERY) : null
  const [dark, setDark] = useState(() => query?.matches ?? true)
  useEffect(() => {
    if (!query) return
    const onChange = () => { setDark(query.matches) }
    query.addEventListener('change', onChange)
    return () => { query.removeEventListener('change', onChange) }
  }, [query])
  return dark
}

/**
 * The desktop app: a terminal multiplexer window over sessions maittyd owns (ADR 0046). Tabs and
 * splits are this window's arrangement; closing them only detaches.
 */
export function DesktopApp() {
  const queryClient = useQueryClient()
  const mac = isMacPlatform(navigator.userAgent)
  const labels = keyLabels(mac)
  const width = useWindowWidth()
  const compact = width < COMPACT_WIDTH_PX
  const narrow = width < NARROW_WIDTH_PX
  const systemReducedMotion = useSystemReducedMotion()
  const systemDark = useSystemDark()
  // A theme being hovered in Settings shows on the whole window until the pointer leaves or one is picked.
  const [previewTheme, setPreviewTheme] = useState<string | null>(null)
  const [welcomeOpen, setWelcomeOpen] = useState(isFirstRun)
  const [shortcutsOpen, setShortcutsOpen] = useState(false)
  const [preferences, setPreferences] = useState(readSettingsPreferences)
  const [layout, setLayout] = useState<Layout>(() => (preferences.restoreTabs ? readLayout() : EMPTY_LAYOUT))
  // The width while the sidebar's edge is being dragged; it is saved when the pointer is released.
  const [liveSidebarWidth, setLiveSidebarWidth] = useState<number | null>(null)
  const [settingsSection, setSettingsSection] = useState<DesktopSettingsSection | null>(null)
  const [renameId, setRenameId] = useState<string | null>(null)
  const [paletteOpen, setPaletteOpen] = useState(false)
  const [inboxOpen, setInboxOpen] = useState(false)
  const [focusMode, setFocusMode] = useState(false)
  const [inspectOpen, setInspectOpen] = useState(false)
  const [findOpen, setFindOpen] = useState(false)
  // Under 1100px the sidebar is a drawer over the panes, opened from the tab bar's Sessions button.
  const [drawerOpen, setDrawerOpen] = useState(false)
  // Widening the window past the breakpoint puts the drawer away, so it does not come back open.
  if (!compact && drawerOpen) setDrawerOpen(false)
  // Interrupted agents whose banner you closed; the daemon stops reporting one once an agent runs again.
  const [dismissedRestores, setDismissedRestores] = useState<ReadonlySet<string>>(new Set())
  // The session a new worktree is made from, while the dialog is open.
  const [worktreeFrom, setWorktreeFrom] = useState<SessionView | null>(null)
  const [inspectView, setInspectView] = useState<InspectView>('changes')
  const [lookAt, setLookAt] = useState<LookAt | null>(null)
  // Inbox entries the user cleared, by the status they had; an entry comes back if the session changes state.
  const [cleared, setCleared] = useState<ReadonlyMap<string, SessionView['status']>>(new Map())
  const [toastMessage, setToastMessage] = useState<{ text: string; key: number } | null>(null)
  const [tracks, setTracks] = useState<ReadonlyMap<string, SessionTrack>>(new Map())
  const [dragId, setDragId] = useState<string | null>(null)
  const [dropId, setDropId] = useState<string | null>(null)
  const [starting, setStarting] = useState(false)
  const [failures, setFailures] = useState(0)
  const [restartPending, setRestartPending] = useState(false)
  const [root, setRoot] = useState<HTMLDivElement | null>(null)
  const tracksRef = useRef<ReadonlyMap<string, SessionTrack>>(new Map())
  const openedFirst = useRef(false)
  const terminalHandles = useRef(new Map<string, TerminalHandle>())
  const lastStatuses = useRef<ReadonlyMap<string, SessionView['status']> | null>(null)
  const lastNotified = useRef<{ sessionId: string; at: number } | null>(null)

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
  const online = !sessionsQuery.isError
  const mismatch = sessionsQuery.isError ? daemonProtocolMismatch(sessionsQuery.error) : null
  const restartStatusQuery = useQuery({ queryKey: ['daemon-restart-status', mismatch?.daemonProtocol], queryFn: loadDaemonRestartStatus, enabled: Boolean(mismatch), retry: false })

  const sessions = useMemo(() => {
    const terminals = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal')
    return (online ? terminals : markConversationsDisconnected(terminals)).map((conversation) => toSessionView(conversation, home))
  }, [sessionsQuery.data, online, home])
  const sessionMap = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions])
  const focusedId = focusedSessionId(layout)
  const focusedSession = focusedId ? sessionMap.get(focusedId) : undefined
  const visibleKey = [...visibleSessionIds(layout)].sort().join(' ')
  // The tab each session is open in; the sidebar shows it only once there is more than one tab.
  const tabOf = useMemo(() => {
    const numbers = new Map<string, number>()
    if (layout.tabs.length < MIN_TABS_FOR_NUMBERS) return numbers
    layout.tabs.forEach((candidate, index) => {
      for (const pane of candidate.panes) numbers.set(pane.sessionId, index + 1)
    })
    return numbers
  }, [layout.tabs])
  // The pane (counting from 1) each session has in its tab, for the tray's tooltips.
  const paneOf = useMemo(() => {
    const numbers = new Map<string, number>()
    for (const candidate of layout.tabs) candidate.panes.forEach((pane, index) => { numbers.set(pane.sessionId, index + 1) })
    return numbers
  }, [layout.tabs])
  const groups = useMemo(() => groupSessions(sessions, preferences.groupBy, focusedId, home, preferences.projectColors), [sessions, preferences.groupBy, focusedId, home, preferences.projectColors])
  // A tab belongs to the project of the session in its focused pane, so it moves when that shell cds into another repo.
  const tabGroup = useCallback((candidate: Tab): string => {
    const sessionId = candidate.panes.find((pane) => pane.id === candidate.focus)?.sessionId ?? candidate.panes[0]?.sessionId
    const session = sessionId ? sessionMap.get(sessionId) : undefined
    return session ? groupKeyOf(session, preferences.groupBy) : ''
  }, [sessionMap, preferences.groupBy])

  const reducedMotion = preferences.reduceMotion || systemReducedMotion
  const theme = findTheme(previewTheme ?? activeThemeName(preferences, systemDark))
  const look = useMemo(() => ({ cursorStyle: preferences.cursorStyle, scrollback: preferences.scrollback, theme: xtermTheme(theme.tokens) }), [preferences.cursorStyle, preferences.scrollback, theme])
  const effects = useMemo(() => {
    const allowed = online && !reducedMotion
    return { working: allowed && preferences.effectWorking, needs: allowed && preferences.effectNeedsYou, done: allowed && preferences.effectDone }
  }, [online, reducedMotion, preferences.effectWorking, preferences.effectNeedsYou, preferences.effectDone])

  const flash = useCallback((text: string) => { setToastMessage({ text, key: Date.now() }) }, [])
  useEffect(() => {
    if (!toastMessage) return
    const timer = window.setTimeout(() => { setToastMessage(null) }, TOAST_MS)
    return () => { window.clearTimeout(timer) }
  }, [toastMessage])

  useEffect(() => {
    try {
      writeSettingsPreferences(preferences)
    } catch (error) {
      toast.error(errorMessage(error, 'Could not save settings'))
    }
    setSpinnerStyle(preferences.spinner)
  }, [preferences])

  useEffect(() => {
    document.documentElement.dataset.reducedMotion = String(reducedMotion)
  }, [reducedMotion])

  useEffect(() => {
    // Flat and Tiled share one finish (neutral, no glow); Tiled changes the layout on top of it.
    document.documentElement.dataset.style = preferences.style
    document.documentElement.dataset.finish = preferences.style === 'glass' ? 'glass' : 'flat'
  }, [preferences.style])

  useEffect(() => {
    document.documentElement.dataset.icons = preferences.iconColor
  }, [preferences.iconColor])

  useEffect(() => { writeLayout(layout) }, [layout])

  // A drag from the sidebar ends wherever it is released; the pane's highlight must not outlast it.
  useEffect(() => {
    const clear = () => { setDropId(null) }
    window.addEventListener('dragend', clear)
    window.addEventListener('drop', clear)
    return () => {
      window.removeEventListener('dragend', clear)
      window.removeEventListener('drop', clear)
    }
  }, [])

  const refetchSessions = sessionsQuery.refetch
  const retryAt = sessionsQuery.isError ? sessionsQuery.errorUpdatedAt + retryDelay : null

  // Each fresh list: drop panes whose session ended, and open something on the first load when
  // there is nothing to restore. `sessions` also changes when the home folder arrives; only a new
  // list counts.
  const reconciledAt = useRef(0)
  useEffect(() => {
    if (!sessionsQuery.isSuccess || reconciledAt.current === sessionsQuery.dataUpdatedAt) return
    reconciledAt.current = sessionsQuery.dataUpdatedAt
    const live = new Set(sessions.map((session) => session.id))
    const first = [...sessions].sort((a, b) => a.pane - b.pane)[0]
    const openFirst = !openedFirst.current
    openedFirst.current = true
    setLayout((current) => {
      const reconciled = reconcileLayout(current, live)
      return openFirst && reconciled.tabs.length === 0 && first ? openInNewTab(reconciled, first.id, makeId) : reconciled
    })
  }, [sessions, sessionsQuery.dataUpdatedAt, sessionsQuery.isSuccess])

  useEffect(() => {
    const { tracks: next, moves } = trackSessions(tracksRef.current, sessions, new Set(visibleKey.split(' ').filter(Boolean)), Date.now())
    tracksRef.current = next
    setTracks(next)
    const move = moves.at(-1)
    if (move) {
      const name = (path: string | null) => path?.split(/[\\/]/).filter(Boolean).at(-1) ?? ''
      flash(move.to ? `Session moved to ${name(move.to)}` : `Session left ${name(move.from)}`)
    }
  }, [sessions, visibleKey, flash])

  useEffect(() => {
    const next = markSeen(tracksRef.current, new Set(visibleKey.split(' ').filter(Boolean)))
    tracksRef.current = next
    setTracks(next)
  }, [visibleKey])

  const highlightUntil = Math.max(0, ...[...tracks.values()].map((track) => (track.movedAt ?? 0) + MOVED_HIGHLIGHT_MS))
  const now = useClock(sessions.some((session) => session.status === 'working'), highlightUntil)

  // The session the daemon made since `known` was the whole list, if one appeared.
  const newSessionSince = useCallback(async (known: ReadonlySet<string>): Promise<string | null> => {
    await queryClient.invalidateQueries({ queryKey: ['conversations'] })
    const fresh = await queryClient.query({ queryKey: ['conversations'], queryFn: loadConversations, staleTime: 0 })
    const created = fresh.filter((item) => item.kind === 'terminal' && !known.has(item.id)).sort((a, b) => (a.nativePane ?? 0) - (b.nativePane ?? 0)).at(-1)
    return created?.id ?? null
  }, [queryClient])

  const startShell = useCallback(async (folder: string): Promise<string | null> => {
    const known = new Set(sessions.map((session) => session.id))
    await startShellSession(folder)
    return newSessionSince(known)
  }, [sessions, newSessionSince])

  const createWorktree = useCallback(async (from: SessionView, branch: string, base: string | null) => {
    const known = new Set(sessions.map((session) => session.id))
    await newWorktreeSession(from.pane, branch, base)
    const id = await newSessionSince(known)
    setWorktreeFrom(null)
    if (id) setLayout((current) => openInNewTab(current, id, makeId))
    flash(`Worktree ${branch} ready`)
  }, [sessions, newSessionSince, flash])

  const runStart = useCallback((folder: () => Promise<string>, place: (sessionId: string) => void) => {
    if (!online || starting) return
    setStarting(true)
    void folder()
      .then((path) => startShell(path))
      .then((id) => { if (id) place(id) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not start a shell')) })
      .finally(() => { setStarting(false) })
  }, [online, starting, startShell])

  const newSession = useCallback(() => {
    setSettingsSection(null)
    runStart(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => { setLayout((current) => openInNewTab(current, id, makeId)) })
  }, [runStart, preferences.startFolder, home])

  // The + after the tabs starts a shell in the project you are in; ⌘T and the sidebar's New use the start folder.
  const activeGroupKey = (() => {
    const showing = activeTab(layout)
    return showing ? tabGroup(showing) : ''
  })()
  const startIn = useCallback((folder: string) => {
    setSettingsSection(null)
    runStart(() => Promise.resolve(folder), (id) => { setLayout((current) => openInNewTab(current, id, makeId)) })
  }, [runStart])
  const projectName = groups.find((group) => group.key === activeGroupKey)?.name
  const newProjectSession = useCallback(() => {
    if (!activeGroupKey) {
      newSession()
      return
    }
    setSettingsSection(null)
    runStart(() => Promise.resolve(activeGroupKey), (id) => { setLayout((current) => openInNewTab(current, id, makeId)) })
  }, [activeGroupKey, newSession, runStart])

  const split = useCallback((paneId: string | null, direction: SplitDirection) => {
    const tab = activeTab(layout)
    if (!tab || tab.panes.length >= MAX_PANES) return
    const source = tab.panes.find((pane) => pane.id === (paneId ?? tab.focus))
    const cwd = source ? sessionMap.get(source.sessionId)?.cwd : undefined
    runStart(async () => cwd ?? resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => { setLayout((current) => splitPane(current, source?.id ?? null, direction, id, makeId)) })
  }, [layout, sessionMap, runStart, preferences.startFolder, home])

  const closePaneById = useCallback((paneId: string) => {
    setLayout((current) => closePane(current, paneId))
    flash('Pane closed · the session keeps running')
  }, [flash])

  const closeTabById = useCallback((tabId: string) => {
    setLayout((current) => closeTab(current, tabId, tabGroup))
    flash('Tab closed · sessions keep running')
  }, [flash, tabGroup])

  // Ending a session kills its shell, so every path that ends one (row, pane, menus, whole project) asks first.
  const [endRequest, setEndRequest] = useState<EndRequest | null>(null)

  const endSession = useCallback((sessionId: string) => {
    const session = sessions.find((candidate) => candidate.id === sessionId)
    setEndRequest({ ids: [sessionId], label: session ? displayName(session) : 'this session' })
  }, [sessions])

  const endProject = useCallback((ids: string[], name: string) => { setEndRequest({ ids, label: name }) }, [])

  const confirmEnd = useCallback(() => {
    if (!endRequest) return
    const { ids } = endRequest
    setEndRequest(null)
    void Promise.all(ids.map((id) => archiveConversationOnDaemon({ id }, true)))
      .then(async () => {
        const ended = new Set(ids)
        setLayout((current) => reconcileLayout(current, new Set(sessions.map((session) => session.id).filter((id) => !ended.has(id)))))
        flash(ids.length === 1 ? 'Session ended' : `${String(ids.length)} sessions ended`)
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not end the session')) })
  }, [endRequest, flash, queryClient, sessions])

  const saveName = useCallback((sessionId: string, name: string) => {
    setRenameId(null)
    void renameConversationOnDaemon({ id: sessionId }, name)
      .then(async () => { await queryClient.invalidateQueries({ queryKey: ['conversations'] }) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not rename the session')) })
  }, [queryClient])

  const showSessionById = useCallback((sessionId: string) => {
    setLayout((current) => showSession(current, sessionId, makeId))
  }, [])

  // Cmd/Ctrl-click on a file name in a pane's output: find it in that pane's repo, then show it in Inspect.
  const openPath = useCallback((session: SessionView, path: string, line?: number) => {
    void repoLocate(session.pane, path)
      .then((relative) => {
        showSessionById(session.id)
        setLookAt({ path: relative, ...(line === undefined ? {} : { line }), key: Date.now() })
        setInspectView('files')
        setInspectOpen(true)
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not open that file')) })
  }, [showSessionById])

  // In a window too narrow for the sidebar, the same key opens and closes its drawer.
  const toggleSidebar = useCallback(() => {
    if (compact) setDrawerOpen((open) => !open)
    else setPreferences((current) => ({ ...current, compactSidebar: !current.compactSidebar }))
  }, [compact])

  const runShortcut = useCallback((action: ShortcutAction) => {
    switch (action) {
      case 'new-session':
        newSession()
        return
      case 'split-right':
        split(null, 'right')
        return
      case 'split-down':
        split(null, 'down')
        return
      case 'focus-previous':
        setLayout((current) => cycleFocus(current, -1))
        return
      case 'focus-next':
        setLayout((current) => cycleFocus(current, 1))
        return
      case 'close': {
        const result = closeFocused(layout, tabGroup)
        setLayout(result.layout)
        if (result.closed === 'pane') flash('Pane closed · the session keeps running')
        if (result.closed === 'tab') flash('Tab closed · sessions keep running')
        return
      }
      case 'next-tab':
        setLayout((current) => cycleTab(current, 1, tabGroup))
        return
      case 'previous-tab':
        setLayout((current) => cycleTab(current, -1, tabGroup))
        return
      case 'toggle-sidebar':
        toggleSidebar()
        return
      case 'settings':
        setSettingsSection((current) => current ?? 'general')
        return
      case 'palette':
        setPaletteOpen((open) => !open)
        return
      case 'inbox':
        setInboxOpen((open) => !open)
        return
      case 'focus-mode':
        setFocusMode((on) => !on)
        return
      case 'shortcuts':
        setShortcutsOpen((open) => !open)
        return
      case 'inspect':
        setInspectOpen((open) => !open)
        return
      case 'find':
        setFindOpen((open) => !open)
        return
      case 'new-worktree':
        if (focusedSession && online) setWorktreeFrom(focusedSession)
        return
      case 'broadcast':
        if (layout.activeTabId) {
          const turningOn = !activeTab(layout)?.broadcast
          setLayout((current) => toggleBroadcast(current, current.activeTabId ?? ''))
          flash(turningOn ? 'Broadcast on · typing goes to every idle pane in this tab' : 'Broadcast off')
        }
        return
    }
  }, [newSession, split, layout, flash, toggleSidebar, tabGroup, focusedSession, online])

  // Captured on the window before xterm sees the key, so only these few keys leave the shell.
  const overlayOpen = paletteOpen || inboxOpen
  const modalOpen = settingsSection !== null || renameId !== null || overlayOpen
  const drawerShown = compact && drawerOpen
  const shortcutState = useRef({ runShortcut, settingsOpen: settingsSection !== null || renameId !== null, overlayOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen })
  useEffect(() => { shortcutState.current = { runShortcut, settingsOpen: settingsSection !== null || renameId !== null, overlayOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen } })
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const state = shortcutState.current
      if (state.welcomeOpen) return
      if (state.shortcutsOpen) {
        // Only its own key closes it; Esc is the dialog's.
        if (matchShortcut(event, mac) === 'shortcuts') {
          event.preventDefault()
          event.stopPropagation()
          state.runShortcut('shortcuts')
        }
        return
      }
      if (event.key === 'Escape') {
        // Innermost first: the palette or inbox, then focus mode, then Settings.
        if (state.overlayOpen) {
          setPaletteOpen(false)
          setInboxOpen(false)
          return
        }
        if (state.settingsOpen) {
          setSettingsSection(null)
          return
        }
        if (state.drawerShown) {
          setDrawerOpen(false)
          return
        }
        if (state.focusMode) setFocusMode(false)
        return
      }
      if (state.overlayOpen) {
        const toggle = matchShortcut(event, mac)
        if (toggle === 'palette' || toggle === 'inbox') {
          event.preventDefault()
          event.stopPropagation()
          state.runShortcut(toggle)
        }
        return
      }
      if (state.settingsOpen) return
      const action = matchShortcut(event, mac)
      if (!action) return
      event.preventDefault()
      event.stopPropagation()
      state.runShortcut(action)
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => { window.removeEventListener('keydown', onKeyDown, true) }
  }, [mac])

  // The project each session sits in, for the inbox and the palette.
  const projectOf = useMemo(() => {
    const map = new Map<string, { name: string; color: string; key: string }>()
    for (const group of groups) for (const member of group.sessions) map.set(member.id, { name: group.name, color: folderColor(group.key, group.colorId).from, key: group.key })
    return map
  }, [groups])

  const [windowFocused, setWindowFocused] = useState(() => document.hasFocus())
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
      .map((item) => ({ session: item, projectName: projectOf.get(item.id)?.name ?? '', projectColor: projectOf.get(item.id)?.color ?? 'var(--accent)' }))
  }, [sessions, cleared, tracks, projectOf, focusedId, windowFocused])
  const needsCount = inboxEntries.filter((entry) => entry.session.status === 'needs').length
  const needsElsewhere = sessions.filter((item) => item.status === 'needs' && item.id !== focusedId).length

  // Inspect looks at the focused pane's own repository; the tab bar's +N −N comes from the same daemon question.
  const inspectTerm = online && isDesktop() && focusedSession ? focusedSession.pane : null
  const diffStat = useQuery({
    queryKey: ['repo', inspectTerm, 'diff-stat'],
    queryFn: async () => {
      const diff = await repoDiff(inspectTerm ?? 0)
      return { additions: diff.additions, deletions: diff.deletions }
    },
    enabled: inspectTerm !== null,
    refetchInterval: DIFF_STAT_REFRESH_MS,
    retry: false,
  })

  // A session that just started needing you, finished or failed: tell the system, if you are looking elsewhere.
  const { notifyNeedsYou, notifyFinished, notifyError } = preferences
  useEffect(() => {
    const notices = online
      ? noticesFor(lastStatuses.current, sessions, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError }, document.hasFocus() ? focusedId : null)
      : []
    lastStatuses.current = new Map(sessions.map((item) => [item.id, item.status]))
    // In the window but on another pane: a toast you can click. Away from the window: the system's.
    const inWindow = document.hasFocus()
    for (const notice of notices) {
      if (inWindow) {
        toast(notice.title, { description: notice.body, action: { label: 'Open', onClick: () => { showSessionById(notice.sessionId) } } })
        continue
      }
      lastNotified.current = { sessionId: notice.sessionId, at: Date.now() }
      sendNativeNotice(notice.title, notice.body).catch((error: unknown) => { console.warn('Could not show a system notification', error) })
    }
  }, [sessions, online, focusedId, notifyNeedsYou, notifyFinished, notifyError, showSessionById])

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

  // Clicking a notification brings the window forward; that focus shows the session it was about.
  useEffect(() => {
    const onFocus = () => {
      const notified = lastNotified.current
      lastNotified.current = null
      if (notified && Date.now() - notified.at < NOTIFICATION_CLICK_WINDOW_MS) showSessionById(notified.sessionId)
    }
    window.addEventListener('focus', onFocus)
    return () => { window.removeEventListener('focus', onFocus) }
  }, [showSessionById])

  const sendTo = useCallback((target: SessionView, data: string) => sendTerminalInput({ id: target.id, generation: target.conversation.generation ?? 0 }, data), [])

  const answerSession = useCallback((sessionId: string, answer: Answer) => {
    const target = sessionMap.get(sessionId)
    if (!target) return
    // The answer is typed into the session's own prompt; the GUI never decides for it.
    void sendTo(target, answer)
      .then(() => { flash(`Sent “${answer}” to ${displayName(target)}`) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not answer the session')) })
  }, [sessionMap, sendTo, flash])

  // Types the resume command into the session, as you would. Nothing resumes without this click.
  const resumeInterrupted = useCallback((target: SessionView) => {
    const command = target.interrupted?.resume
    if (!command) return
    showSessionById(target.id)
    void sendTo(target, `${command}
`)
      .then(() => { setDismissedRestores((current) => new Set(current).add(target.id)) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not resume the agent')) })
  }, [sendTo, showSessionById])

  const clearEntry = useCallback((sessionId: string) => {
    const target = sessionMap.get(sessionId)
    if (target) setCleared((current) => new Map(current).set(sessionId, target.status))
  }, [sessionMap])

  const clearSettled = useCallback(() => {
    setCleared((current) => {
      const next = new Map(current)
      for (const entry of inboxEntries) if (entry.session.status !== 'needs') next.set(entry.session.id, entry.session.status)
      return next
    })
  }, [inboxEntries])

  // With broadcast on, what is typed in one pane also goes to the tab's other panes, but only the idle ones: never into a busy agent.
  const mirrorInput = useCallback((fromSessionId: string, data: string | Uint8Array) => {
    const showing = activeTab(layout)
    if (!showing?.broadcast || typeof data !== 'string') return
    for (const pane of showing.panes) {
      const other = pane.sessionId === fromSessionId ? undefined : sessionMap.get(pane.sessionId)
      if (other?.status === 'idle') void sendTo(other, data).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not broadcast to a pane')) })
    }
  }, [layout, sessionMap, sendTo])

  const startAgent = useCallback((program: string) => {
    setSettingsSection(null)
    runStart(async () => activeGroupKey || resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => {
      setLayout((current) => openInNewTab(current, id, makeId))
      void sendTerminalInput({ id, generation: 0 }, `${program}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${program}`)) })
    })
  }, [runStart, activeGroupKey, preferences.startFolder, home])

  const paletteItems: PaletteItem[] = useMemo(() => {
    const keys = (letter: string, shifted = false) => `${labels.primary}${shifted ? labels.shift : ''}${letter}`
    const action = (id: string, label: string, run: () => void, shortcut?: string): PaletteItem => ({ id, label, kind: 'Action', run, ...(shortcut ? { shortcut } : {}) })
    const showing = activeTab(layout)
    const items: PaletteItem[] = [
      action('new-here', activeGroupKey ? `New session in ${projectName ?? 'this project'}` : 'New session', newProjectSession, keys('T')),
      action('new-start', 'New session in the start folder', newSession),
      action('split-right', 'Split right', () => { split(null, 'right') }, keys('D')),
      action('split-down', 'Split down', () => { split(null, 'down') }, keys('D', true)),
      action('inbox', 'Open the inbox', () => { setInboxOpen(true) }, keys('J')),
      action('inspect', 'Inspect the repository', () => { setInspectOpen(true) }, keys('I')),
      ...(focusedSession ? [action('new-worktree', 'New worktree', () => { setWorktreeFrom(focusedSession) }, keys('N', true))] : []),
      action('find', 'Find in the pane or every session', () => { setFindOpen(true) }, keys('F')),
      action('broadcast', showing?.broadcast ? 'Stop broadcasting' : 'Broadcast typing to this tab', () => { runShortcut('broadcast') }, keys('B', true)),
      action('focus', 'Focus mode', () => { setFocusMode(true) }, keys('.')),
      action('settings', 'Open settings', () => { setSettingsSection('general') }, keys(',')),
      action('shortcuts', 'Keyboard shortcuts', () => { setShortcutsOpen(true) }, keys('/')),
      ...(showing?.widths ? [action('reset-widths', 'Reset pane widths', () => { setLayout((current) => setColumnWidths(current, showing.id, null)) })] : []),
      action('start-codex', 'Start codex', () => { startAgent('codex') }),
    ]
    for (const member of sessions) {
      const project = projectOf.get(member.id)
      items.push({
        id: `session-${member.id}`,
        label: `${displayName(member)} · ${member.process}`,
        kind: 'Session',
        ...(project ? { detail: project.name, dot: project.color } : {}),
        ...(member.status === 'needs' ? { badge: 'needs you' } : {}),
        run: () => { showSessionById(member.id) },
      })
    }
    for (const group of groups) {
      const target = group.sessions.find((member) => member.id === focusedId) ?? group.sessions[0]
      if (target) items.push({ id: `project-${group.key}`, label: group.name, detail: group.path, kind: 'Project', dot: folderColor(group.key, group.colorId).from, run: () => { showSessionById(target.id) } })
    }
    for (const candidate of THEMES) items.push({ id: `theme-${candidate.name}`, label: candidate.name, detail: candidate.family, kind: 'Theme', needsQuery: true, run: () => { setPreferences((current) => ({ ...current, themeMode: candidate.light ? 'light' : 'dark', ...(candidate.light ? { lightTheme: candidate.name } : { darkTheme: candidate.name }) })) } })
    return items
  }, [labels.primary, labels.shift, layout, activeGroupKey, projectName, newProjectSession, newSession, split, runShortcut, startAgent, sessions, projectOf, groups, focusedId, focusedSession, showSessionById])

  const restartOffer: RestartOffer | null = mismatch
    ? {
        label: restartStatusQuery.isPending ? 'Checking service…' : restartStatusQuery.isError ? 'Restart check failed' : restartStatusQuery.data.canRestart ? 'Restart service' : 'Service in use',
        canRestart: Boolean(restartStatusQuery.data?.canRestart),
        pending: restartPending,
        onRestart: () => {
          setRestartPending(true)
          void restartMismatchedDaemon(mismatch.daemonProtocol)
            .then(async () => { await refetchSessions() })
            .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not restart the Maitty service')) })
            .finally(() => { setRestartPending(false) })
        },
      }
    : null
  const offlineDetail = mismatch
    ? `${daemonConnectionError(sessionsQuery.error)}${restartStatusQuery.data && !restartStatusQuery.data.canRestart && restartStatusQuery.data.sessionCount > 0 ? ` The older service owns ${String(restartStatusQuery.data.sessionCount)} sessions, so Maitty leaves it running.` : ''}`
    : null

  const tab = activeTab(layout)
  const visibleTabs = useMemo(() => layout.tabs.filter((candidate) => tabGroup(candidate) === activeGroupKey), [layout.tabs, tabGroup, activeGroupKey])
  const startFolderLabel = preferences.startFolder.trim() ? tildePath(resolveStartFolder(preferences.startFolder, home), home) : '~'
  const canStart = online && !starting
  const sidebarMode = compact || preferences.compactSidebar ? 'rail' : 'docked'
  const sidebarWidth = liveSidebarWidth ?? preferences.sidebarWidth
  const rootStyle = { ...themeStyle(theme), ...sizeVariables(preferences, sidebarWidth) } as CSSProperties

  // The sidebar, as the docked column or rail, or inside the drawer of a window under 1100px.
  const renderSidebar = (mode: 'docked' | 'rail', afterPick: () => void) => (
    <Sidebar
      groups={groups}
      focusedId={focusedId}
      tracks={tracks}
      now={now}
      mode={mode}
      online={online}
      sessionCount={sessions.length}
      primary={labels.primary}
      startFolderLabel={startFolderLabel}
      canStart={canStart}
      portalContainer={root}
      onToggleRail={toggleSidebar}
      canExpand={!compact}
      width={sidebarWidth}
      tabOf={tabOf}
      paneOf={paneOf}
      trayStyle={preferences.trayStyle}
      leadMode={preferences.leadMode}
      onResize={setLiveSidebarWidth}
      onResizeEnd={(width) => {
        setPreferences((current) => ({ ...current, sidebarWidth: width }))
        setLiveSidebarWidth(null)
      }}
      onNewSession={() => { afterPick(); newSession() }}
      onNewSessionIn={(folder) => { afterPick(); startIn(folder) }}
      onNewWorktree={(groupKey) => {
        const group = groups.find((candidate) => candidate.key === groupKey)
        const lead = group?.sessions.find((member) => member.id === focusedId) ?? group?.sessions[0]
        if (lead) {
          afterPick()
          setWorktreeFrom(lead)
        }
      }}
      onShowSession={(id) => { afterPick(); showSessionById(id) }}
      onEndSession={endSession}
      onEndProject={endProject}
      onRenameSession={setRenameId}
      onOpenSettings={() => { afterPick(); setSettingsSection('general') }}
    />
  )

  return (
    <ReducedMotionContext.Provider value={reducedMotion}>
      <EffectsContext.Provider value={effects}>
        <div ref={setRoot} className={`mx-app${narrow ? ' is-narrow' : ''}`} style={rootStyle} data-motion={reducedMotion ? 'reduced' : 'full'}>
          <TitleBar />
          <div className="mx-window" role="application" aria-label="Maitty">
            {!focusMode && !compact && renderSidebar(sidebarMode, noop)}
            {!focusMode && drawerShown && (
              <div className="mx-drawer-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) setDrawerOpen(false) }}>
                <div className="mx-drawer" role="dialog" aria-label="Sessions" onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); setDrawerOpen(false) } }}>
                  {renderSidebar('docked', () => { setDrawerOpen(false) })}
                </div>
              </div>
            )}
            <div className="mx-main-wrap">
              <main className="mx-main" aria-label="Terminals">
                {!focusMode && <TabBar
                  tabs={visibleTabs}
                  activeTabId={layout.activeTabId}
                  sessions={sessionMap}
                  primary={labels.primary}
                  canStart={canStart}
                  portalContainer={root}
                  onSelect={(tabId) => { setLayout((current) => selectTab(current, tabId)) }}
                  onClose={closeTabById}
                  newTabLabel={projectName ? `New session in ${projectName}` : 'New tab'}
                  onNewTab={newProjectSession}
                  inboxCount={inboxEntries.length}
                  inboxNeeds={needsCount > 0}
                  onOpenInbox={() => { setInboxOpen(true) }}
                  inspect={inspectTerm === null ? null : { open: inspectOpen, additions: diffStat.data?.additions ?? 0, deletions: diffStat.data?.deletions ?? 0 }}
                  onToggleInspect={() => { setInspectOpen((open) => !open) }}
                  {...(compact ? { onOpenSessions: () => { setDrawerOpen(true) } } : {})}
                />}
                {findOpen && (
                  <FindBar
                    online={online}
                    findInPane={(query, direction) => (focusedId ? terminalHandles.current.get(focusedId)?.find(query, direction) ?? false : false)}
                    clearPane={() => { if (focusedId) terminalHandles.current.get(focusedId)?.clearFind() }}
                    nameOf={(term) => { const found = sessions.find((item) => item.pane === term); return found ? displayName(found) : null }}
                    onOpenSession={(term) => { const found = sessions.find((item) => item.pane === term); if (found) showSessionById(found.id) }}
                    onClose={() => { setFindOpen(false); if (focusedId) terminalHandles.current.get(focusedId)?.focus() }}
                  />
                )}
                <RestoreBanner
                  sessions={sessions.filter((item) => item.interrupted !== null && !dismissedRestores.has(item.id))}
                  online={online}
                  onResume={resumeInterrupted}
                  onDismiss={(id) => { setDismissedRestores((current) => new Set(current).add(id)) }}
                />
                {!online && <OfflineBanner retryAt={retryAt} detail={offlineDetail} restart={restartOffer} onRetry={() => { void refetchSessions() }} />}
                {tab ? (
                  <div className="mx-panes">
                    {paneColumns(tab, narrow).map((column, columnIndex, columns) => (
                      <Fragment key={column.map((pane) => pane.id).join(' ')}>
                        {columnIndex > 0 && tab.max === null && !narrow && (
                          <ColumnHandle
                            index={columnIndex - 1}
                            onResize={(widths) => { setLayout((current) => setColumnWidths(current, tab.id, widths)) }}
                            onReset={() => { setLayout((current) => setColumnWidths(current, tab.id, null)) }}
                          />
                        )}
                      <div className="mx-pane-column" style={tab.widths && tab.max === null && !narrow && tab.widths.length === columns.length ? { flexGrow: tab.widths[columnIndex] } : undefined}>
                        {column.map((pane) => {
                          const session = sessionMap.get(pane.sessionId)
                          if (!session) return null
                          const focused = pane.id === tab.focus
                          return (
                            <PaneCard
                              key={pane.id}
                              pane={pane}
                              index={tab.panes.findIndex((candidate) => candidate.id === pane.id)}
                              session={session}
                              paneCount={tab.panes.length}
                              broadcast={tab.broadcast === true}
                              onToggleBroadcast={() => { runShortcut('broadcast') }}
                              focused={focused}
                              maximized={tab.max === pane.id}
                              dropTarget={dropId === pane.id && dragId !== pane.id}
                              workingSince={tracks.get(session.id)?.workingSince ?? null}
                              now={now}
                              primary={labels.primary}
                              shift={labels.shift}
                              online={online}
                              portalContainer={root}
                              terminal={
                                <TerminalView
                                  key={session.id}
                                  conversation={session.conversation}
                                  desktop
                                  fontSize={preferences.terminalFontSize}
                                  look={look}
                                  label={`Shell in ${session.cwdLabel}`}
                                  inputDisabled={!online}
                                  focused={focused && !modalOpen}
                                  onInput={(data) => { mirrorInput(session.id, data) }}
                                  onOpenPath={(path, line) => { openPath(session, path, line) }}
                                  onHandle={(handle) => {
                                    if (handle) terminalHandles.current.set(session.id, handle)
                                    else terminalHandles.current.delete(session.id)
                                  }}
                                />
                              }
                              onFocus={(paneId) => { setLayout((current) => focusPane(current, paneId)) }}
                              onSplit={(paneId, direction) => {
                                setLayout((current) => focusPane(current, paneId))
                                split(paneId, direction)
                              }}
                              onToggleMaximize={(paneId) => { setLayout((current) => toggleMaximize(current, paneId)) }}
                              onClose={closePaneById}
                              onEndSession={endSession}
                              onRenameSession={setRenameId}
                              onDragStart={setDragId}
                              onDragEnd={() => {
                                setDragId(null)
                                setDropId(null)
                              }}
                              onDragOver={(paneId, fromSidebar) => { if (dragId || fromSidebar) setDropId(paneId) }}
                              onDrop={(paneId, sessionId) => {
                                if (sessionId) {
                                  setLayout((current) => dropSession(current, paneId, sessionId))
                                  flash('Session moved into this pane')
                                } else if (dragId && dragId !== paneId) setLayout((current) => swapSessions(current, dragId, paneId))
                                setDragId(null)
                                setDropId(null)
                              }}
                            />
                          )
                        })}
                      </div>
                      </Fragment>
                    ))}
                  </div>
                ) : (
                  <EmptyState
                    hasSessions={sessions.length > 0}
                    startFolderLabel={startFolderLabel}
                    primary={labels.primary}
                    canStart={canStart}
                    onNewSession={newSession}
                    onChangeStartFolder={() => { setSettingsSection('general') }}
                  />
                )}
              </main>
            </div>
            {inspectOpen && !focusMode && inspectTerm !== null && focusedSession && (
              <InspectPanel
                term={inspectTerm}
                title={`${displayName(focusedSession)} · ${focusedSession.cwdLabel}`}
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
            {shortcutsOpen && <ShortcutOverlay mac={mac} portalContainer={root} onClose={() => { setShortcutsOpen(false) }} />}
            {welcomeOpen && (
              <WelcomeDialog
                mac={mac}
                startFolder={preferences.startFolder}
                portalContainer={root}
                validateFolder={folderExists}
                onStartFolder={(path) => { setPreferences((current) => ({ ...current, startFolder: path })) }}
                onClose={() => { markWelcomed(); setWelcomeOpen(false) }}
                onStartSession={newSession}
              />
            )}
            <ConfirmEndDialog request={endRequest} portalContainer={root} onCancel={() => { setEndRequest(null) }} onConfirm={confirmEnd} />
            {paletteOpen && <CommandPalette items={paletteItems} onClose={() => { setPaletteOpen(false) }} />}
            {inboxOpen && (
              <InboxPanel
                entries={inboxEntries}
                tracks={tracks}
                now={now}
                online={online}
                onAnswer={answerSession}
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
                <button type="button" className="mx-focus-exit" onClick={() => { setFocusMode(false) }}>Exit <kbd className="mx-keycap">{labels.primary}.</kbd></button>
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
            {renameId !== null && sessionMap.get(renameId) && (
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
                onPreviewTheme={setPreviewTheme}
                onShowWelcome={() => { setPreviewTheme(null); setSettingsSection(null); setWelcomeOpen(true) }}
                mac={mac}
                daemon={{ online, sessionCount: sessions.length }}
                projects={groups.map((group) => ({ key: group.key, name: group.name, colorId: group.colorId }))}
                validateFolder={folderExists}
                onClose={() => { setPreviewTheme(null); setSettingsSection(null) }}
              />
            )}
            {toastMessage && <Toast key={toastMessage.key} message={toastMessage.text} />}
          </div>
          <Toaster position="bottom-right" theme={theme.light ? 'light' : 'dark'} />
        </div>
      </EffectsContext.Provider>
    </ReducedMotionContext.Provider>
  )
}

/**
 * The current time: every second while something shows elapsed time, else slowly, plus once when
 * a move highlight starts and once when it ends.
 */
function useClock(working: boolean, highlightUntil: number): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = window.setInterval(() => { setNow(Date.now()) }, working ? SECOND_MS : SLOW_TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [working])
  useEffect(() => {
    const wait = highlightUntil - Date.now()
    if (wait <= 0) return
    const start = window.setTimeout(() => { setNow(Date.now()) }, 0)
    const end = window.setTimeout(() => { setNow(Date.now()) }, wait)
    return () => {
      window.clearTimeout(start)
      window.clearTimeout(end)
    }
  }, [highlightUntil])
  return now
}

