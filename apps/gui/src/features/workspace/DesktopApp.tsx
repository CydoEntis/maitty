import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  archiveConversationOnDaemon,
  markSeenOnDaemon,
  pinSessionOnDaemon,
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
import { readSettingsPreferences, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN, writeSettingsPreferences } from '@/features/settings/preferences'
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
import { runsProgram } from './components/useArmed'
import { rememberSessions, withEnded } from './model/ended'
import { HoverTips } from './components/HoverTips'
import { ShortcutOverlay } from '@/features/shortcuts/ShortcutOverlay'
import { StatusBar } from './components/StatusBar'
import { WelcomeDialog } from '@/features/welcome/WelcomeDialog'
import { isFirstRun, markWelcomed } from '@/features/welcome/welcomed'
import { repoDiff, repoLocate } from '@/features/inspect/api'
import { FindBar } from '@/features/find/FindBar'
import { RestoreBanner } from './components/RestoreBanner'
import { UpdateSlot } from '@/features/updates/UpdateSlot'
import { appVersion } from '@/features/updates/appVersion'
import { markRestarting, skippedUpdate, skipUpdate, takeRestarted } from '@/features/updates/updateMemory'
import { formatDroppedPaths } from '@/features/terminal/droppedPaths'
import { useFileDrop } from '@/features/terminal/useFileDrop'
import { updaterSummary, useUpdater } from '@/features/updates/useUpdater'
import { sendNativeNotice } from '@/features/notifications/native'
import { dismissSessionNotice, showSessionNotice } from '@/features/notifications/notify'
import { sessionNoticeFor } from '@/features/notifications/noticeForSession'
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
  removeSessions,
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
import { hotkeyRows, isMacPlatform, keyLabels, matchShortcut, type ShortcutAction } from './model/shortcuts'
import { folderExists, resolveStartFolder } from './model/startFolder'
import { markSeen, MOVED_HIGHLIGHT_MS, trackSessions, type SessionTrack } from './model/tracking'
import { folderColor } from './model/folderColor'
import type { PaletteItem } from './model/palette'
import { activeThemeName, findTheme, themeStyle, THEMES, xtermTheme } from './theme/themes'
import './workspace.css'

const SESSION_REFRESH_MS = 1500
const COMPACT_WIDTH_PX = 1100
const MIN_TABS_FOR_NUMBERS = 2
/** A tab's hover card lists its panes only when it has more than one. */
const MIN_PANES_FOR_LIST = 2
const NARROW_WIDTH_PX = 900
const TOAST_MS = 2600
/** What clicking the status bar's size resets the terminal text to. */
const DEFAULT_TEXT_SIZE = 14
/** How long a second ⌘W confirms closing a session that is running a program. */
const CLOSE_ARM_MS = 4000
/** How long "Restarting Maitty" shows before the window relaunches. */
const RESTART_SCREEN_MS = 1200
const TEST_NOTICE_MS = 6000
const SECOND_MS = 1000
const SLOW_TICK_MS = 30_000
const DIFF_STAT_REFRESH_MS = 8000
const DIFF_STAT_IDLE_MS = 30_000
/** A theme previews once the pointer rests on it, not as it passes over every row (each one repaints every terminal). */
const THEME_PREVIEW_DELAY_MS = 140
/** A notification's click brings the window forward; returning later than this is not that click. */
const NOTIFICATION_CLICK_WINDOW_MS = 15_000
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
/** What restarting an older service does to the sessions it owns, in words. */
function olderServiceNote(status: { canRestart: boolean; sessionCount: number; activeTurnCount: number | null } | undefined): string {
  if (!status || status.sessionCount === 0) return ''
  const sessions = `${String(status.sessionCount)} session${status.sessionCount === 1 ? '' : 's'}`
  if (!status.canRestart) return ` The older service owns ${sessions} but is not saving them, so Maitty leaves it running.`
  const midTurn = status.activeTurnCount ? ` ${String(status.activeTurnCount)} working or waiting agent${status.activeTurnCount === 1 ? '' : 's'} will offer a Resume.` : ''
  return ` Restarting it stops what is running in its ${sessions}. They reopen in the same folders.${midTurn}`
}

export function DesktopApp() {
  const queryClient = useQueryClient()
  const mac = isMacPlatform(navigator.userAgent)
  const labels = keyLabels(mac)
  const width = useWindowWidth()
  const compact = width < COMPACT_WIDTH_PX
  const narrow = width < NARROW_WIDTH_PX
  const systemReducedMotion = useSystemReducedMotion()
  const systemDark = useSystemDark()
  const updater = useUpdater()
  const [runningVersion, setRunningVersion] = useState<string | null>(null)
  const [skippedVersion, setSkippedVersion] = useState(skippedUpdate)
  const [restarting, setRestarting] = useState(false)
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
  // After a restart into an update, the first thing said is what happened to the sessions.
  const [toastMessage, setToastMessage] = useState<{ text: string; key: number } | null>(() => {
    const version = takeRestarted()
    return version ? { text: `Maitty ${version} · every session kept running`, key: Date.now() } : null
  })
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

  const everySession = useMemo(() => {
    const terminals = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal')
    return (online ? terminals : markConversationsDisconnected(terminals)).map((conversation) => toSessionView(conversation, home))
  }, [sessionsQuery.data, online, home])
  const sessions = everySession
  const sessionMap = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions])
  // A session the daemon stops listing keeps its pane, shown as ended, so a refresh that misses one
  // never closes a tab or moves your focus. A pane goes only when you close it.
  const [remembered, setRemembered] = useState<ReadonlyMap<string, SessionView>>(() => new Map())
  const [seenSessions, setSeenSessions] = useState(sessions)
  if (seenSessions !== sessions) {
    setSeenSessions(sessions)
    setRemembered((before) => rememberSessions(before, sessions, layout))
  }
  const paneSessions = useMemo(() => withEnded(sessionMap, remembered), [sessionMap, remembered])
  const focusedId = focusedSessionId(layout)
  // A click anywhere outside the panes takes the border off the focused pane; a click in a pane, or focus moving to another pane, puts it back.
  const focusedPaneId = activeTab(layout)?.focus ?? null
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
  const focusedSession = focusedId ? sessionMap.get(focusedId) : undefined
  // Files dropped on a pane type their paths into it; the pane under the pointer, else the focused one.
  useFileDrop((paths, at) => {
    const target = document.elementFromPoint(at.x, at.y)?.closest<HTMLElement>('[data-session-id]')?.dataset.sessionId ?? focusedId
    const handle = target ? terminalHandles.current.get(target) : undefined
    if (!handle) return
    if (!online) {
      toast.error('Reconnect to the Maitty service before dropping files on a pane')
      return
    }
    handle.paste(formatDroppedPaths(paths, /Windows/i.test(navigator.userAgent)))
    handle.focus()
  })
  const visibleKey = [...visibleSessionIds(layout)].sort().join(' ')
  // For a session whose tab has other panes, all of that tab's sessions (the hover card lists them).
  const tabPanes = useMemo(() => {
    const panes = new Map<string, readonly SessionView[]>()
    for (const candidate of layout.tabs) {
      if (candidate.panes.length < MIN_PANES_FOR_LIST) continue
      const shown = candidate.panes.flatMap((pane) => sessionMap.get(pane.sessionId) ?? [])
      for (const session of shown) panes.set(session.id, shown)
    }
    return panes
  }, [layout.tabs, sessionMap])
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
    const session = sessionId ? paneSessions.get(sessionId) : undefined
    return session ? groupKeyOf(session, preferences.groupBy) : ''
  }, [paneSessions, preferences.groupBy])

  const reducedMotion = preferences.reduceMotion || systemReducedMotion
  const theme = findTheme(previewTheme ?? activeThemeName(preferences, systemDark))
  const look = useMemo(() => ({ cursorStyle: preferences.cursorStyle, scrollback: preferences.scrollback, screenReader: preferences.terminalScreenReader, theme: xtermTheme(theme.tokens) }), [preferences.cursorStyle, preferences.scrollback, preferences.terminalScreenReader, theme])
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

  // The first list after launch: drop panes of sessions that are gone since last time, and open
  // something when there is nothing to restore. Later lists never remove a pane: one that stopped
  // being listed stays, as an ended pane. `sessions` also changes when the home folder arrives; only a
  // new list counts.
  const reconciledAt = useRef(0)
  useEffect(() => {
    if (!sessionsQuery.isSuccess || reconciledAt.current === sessionsQuery.dataUpdatedAt) return
    reconciledAt.current = sessionsQuery.dataUpdatedAt
    const live = new Set(sessions.map((session) => session.id))
    const first = [...sessions].sort((a, b) => a.pane - b.pane)[0]
    const openFirst = !openedFirst.current
    openedFirst.current = true
    setLayout((current) => {
      const reconciled = openFirst ? reconcileLayout(current, live, tabGroup) : current
      return openFirst && reconciled.tabs.length === 0 && first ? openInNewTab(reconciled, first.id, makeId) : reconciled
    })
  }, [sessions, sessionsQuery.dataUpdatedAt, sessionsQuery.isSuccess, tabGroup])

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
    runStart(async () => cwd ?? resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => {
      // If the tab changed or filled up while the shell started, the shell still gets a tab: it must not be left with no pane.
      setLayout((current) => {
        const next = splitPane(current, source?.id ?? null, direction, id, makeId)
        return next === current ? openInNewTab(current, id, makeId) : next
      })
    })
  }, [layout, sessionMap, runStart, preferences.startFolder, home])

  // Closing a pane, tab or session ends it; there is no way back, so a program that is running asks for a second click first.
  const [armedClose, setArmedClose] = useState<{ key: string; at: number } | null>(null)
  const endSessions = useCallback((ids: string[], what: string) => {
    const gone = new Set(ids)
    const running = sessions.filter((session) => gone.has(session.id))
    // Panes of sessions that already ended have nothing to stop: they just close.
    if (running.length === 0) {
      setLayout((current) => removeSessions(current, gone, tabGroup))
      return
    }
    void Promise.all(running.map((session) => archiveConversationOnDaemon({ id: session.id }, true)))
      .then(async () => {
        setLayout((current) => removeSessions(current, gone, tabGroup))
        flash(`${what} closed · ${running.length === 1 ? 'session' : `${String(running.length)} sessions`} ended`)
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not end the session')) })
  }, [sessions, flash, queryClient, tabGroup])

  const sessionsOf = useCallback((tabs: readonly Tab[]) => [...new Set(tabs.flatMap((tab) => tab.panes.map((pane) => pane.sessionId)))], [])

  const closePaneById = useCallback((paneId: string) => {
    const pane = layout.tabs.flatMap((tab) => tab.panes).find((candidate) => candidate.id === paneId)
    if (pane) endSessions([pane.sessionId], 'Pane')
  }, [endSessions, layout])

  const closeTabById = useCallback((tabId: string) => {
    const tab = layout.tabs.find((candidate) => candidate.id === tabId)
    if (tab) endSessions(sessionsOf([tab]), 'Tab')
  }, [endSessions, layout, sessionsOf])

  const closeOtherTabs = useCallback((tabId: string) => {
    const keep = layout.tabs.find((candidate) => candidate.id === tabId)
    if (!keep) return
    const others = layout.tabs.filter((tab) => tab.id !== tabId && tabGroup(tab) === tabGroup(keep))
    endSessions(sessionsOf(others), others.length === 1 ? 'Tab' : `${String(others.length)} tabs`)
  }, [endSessions, layout, sessionsOf, tabGroup])

  const restartPane = useCallback((paneId: string) => {
    const pane = layout.tabs.flatMap((tab) => tab.panes).find((candidate) => candidate.id === paneId)
    const old = pane ? paneSessions.get(pane.sessionId) : undefined
    if (!old) return
    runStart(() => Promise.resolve(old.cwd), (id) => { setLayout((current) => dropSession(current, paneId, id)) })
  }, [layout, paneSessions, runStart])

  const endSession = useCallback((sessionId: string) => { endSessions([sessionId], 'Session') }, [endSessions])
  const endProject = useCallback((ids: string[], name: string) => { endSessions(ids, name) }, [endSessions])

  const pinSession = useCallback((sessionId: string, pinned: boolean) => {
    void pinSessionOnDaemon({ id: sessionId }, pinned)
      .then(async () => { await queryClient.invalidateQueries({ queryKey: ['conversations'] }) })
      .catch((error: unknown) => { toast.error(errorMessage(error, pinned ? 'Could not pin the session' : 'Could not unpin the session')) })
  }, [queryClient])

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
  useEffect(() => {
    let active = true
    void appVersion().then((version) => { if (active) setRunningVersion(version) }).catch(() => undefined)
    return () => { active = false }
  }, [])

  const restartNow = useCallback(() => {
    if (updater.state.status !== 'ready') return
    markRestarting(updater.state.version)
    setRestarting(true)
    window.setTimeout(() => { updater.restart() }, RESTART_SCREEN_MS)
  }, [updater])

  // With the setting on, a new release is fetched as soon as it is found. Installing still waits for a restart.
  const updateStatus = updater.state.status
  const updateOffered = updater.state.status === 'available' ? updater.state.version : null
  const downloadUpdate = updater.download
  useEffect(() => {
    if (!preferences.autoDownloadUpdates || updateStatus !== 'available' || updateOffered === skippedVersion) return
    const timer = window.setTimeout(downloadUpdate, 0)
    return () => { window.clearTimeout(timer) }
  }, [preferences.autoDownloadUpdates, updateStatus, updateOffered, skippedVersion, downloadUpdate])

  // Optionally, a downloaded update restarts the window once nothing is working or waiting on you.
  const busyCount = sessions.filter((session) => session.status === 'working' || session.status === 'needs').length
  useEffect(() => {
    if (!preferences.restartWhenIdle || updateStatus !== 'ready' || !online || busyCount > 0 || restarting) return
    const timer = window.setTimeout(restartNow, 0)
    return () => { window.clearTimeout(timer) }
  }, [preferences.restartWhenIdle, updateStatus, online, busyCount, restarting, restartNow])

  const showTestNotice = useCallback(() => {
    showSessionNotice('test', {
      kind: 'done',
      color: 'var(--accent)',
      title: 'Finished · a test',
      body: 'A card like this appears when a session you are not looking at finishes.',
      actions: [{ label: 'Open', primary: true, onClick: () => undefined }],
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
        const tab = activeTab(layout)
        if (!tab) return
        const single = tab.panes.length > 1
        const targets = single ? tab.panes.filter((pane) => pane.id === tab.focus) : tab.panes
        const shown = targets.flatMap((pane) => paneSessions.get(pane.sessionId) ?? [])
        const agent = shown.find((session) => runsProgram(session))
        // Ctrl+W is also a shell's delete-a-word, and closing ends the session, so the key asks twice,
        // for this pane or tab only. Panes that already ended have nothing to lose.
        const key = targets.map((pane) => pane.id).join(' ')
        const confirmed = armedClose !== null && armedClose.key === key && Date.now() - armedClose.at <= CLOSE_ARM_MS
        if (shown.some((session) => !session.ended) && !confirmed) {
          setArmedClose({ key, at: Date.now() })
          const chord = mac ? '⌘W' : 'Ctrl+W'
          flash(agent ? `Press ${chord} again to close and stop ${agent.process}` : `Press ${chord} again to close this ${single ? 'pane' : 'tab'}`)
          return
        }
        setArmedClose(null)
        endSessions(targets.map((pane) => pane.sessionId), single ? 'Pane' : 'Tab')
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
      case 'text-larger':
        setTextSize(preferences.terminalFontSize + 1)
        return
      case 'text-smaller':
        setTextSize(preferences.terminalFontSize - 1)
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
  }, [newSession, split, layout, flash, toggleSidebar, tabGroup, focusedSession, online, preferences.terminalFontSize, setTextSize, endSessions, mac, paneSessions, armedClose])

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
        // What Esc closes is not also sent on to the terminal, where it would interrupt an agent.
        const consume = () => { event.preventDefault(); event.stopPropagation() }
        if (state.overlayOpen) {
          consume()
          setPaletteOpen(false)
          setInboxOpen(false)
          return
        }
        if (state.settingsOpen) {
          consume()
          setSettingsSection(null)
          return
        }
        if (state.drawerShown) {
          consume()
          setDrawerOpen(false)
          return
        }
        if (state.focusMode) {
          consume()
          setFocusMode(false)
        }
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
    // The counts come from a full diff, so they are read slowly unless Inspect is open.
    refetchInterval: inspectOpen ? DIFF_STAT_REFRESH_MS : DIFF_STAT_IDLE_MS,
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
    // A card can outlast the question. Typing into a session that is not asking would land in its shell.
    if (target.status !== 'needs') {
      flash(`${displayName(target)} is not asking any more`)
      return
    }
    // The answer is typed into the session's own prompt; the GUI never decides for it.
    void sendTo(target, answer)
      .then(() => { flash(`Sent “${answer}” to ${displayName(target)}`) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not answer the session')) })
  }, [sessionMap, sendTo, flash])

  // A session that just started needing you, finished or failed: tell the system, if you are looking elsewhere.
  const { notifyNeedsYou, notifyFinished, notifyError } = preferences
  useEffect(() => {
    const previousStatuses = lastStatuses.current
    const notices = online
      ? noticesFor(lastStatuses.current, sessions, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError }, document.hasFocus() ? focusedId : null)
      : []
    lastStatuses.current = new Map(sessions.map((item) => [item.id, item.status]))
    // In the window but on another pane: a toast you can click. Away from the window: the system's.
    const inWindow = document.hasFocus()
    // A card that said "Needs you" goes when the session stops needing you.
    for (const item of sessions) if (item.status !== 'needs' && previousStatuses?.get(item.id) === 'needs') dismissSessionNotice(item.id)
    for (const notice of notices) {
      if (inWindow) {
        const target = sessionMap.get(notice.sessionId)
        const card = target && sessionNoticeFor(target, projectOf.get(target.id), {
          open: () => { showSessionById(notice.sessionId) },
          allow: online ? () => { answerSession(notice.sessionId, 'y') } : null,
        })
        if (card) showSessionNotice(notice.sessionId, card)
        continue
      }
      lastNotified.current = { sessionId: notice.sessionId, at: Date.now() }
      sendNativeNotice(notice.title, notice.body).catch((error: unknown) => { console.warn('Could not show a system notification', error) })
    }
  }, [sessions, online, focusedId, notifyNeedsYou, notifyFinished, notifyError, showSessionById, sessionMap, projectOf, answerSession])

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
      action('sidebar-side', preferences.sidebarSide === 'left' ? 'Move sidebar to the right' : 'Move sidebar to the left', () => { setPreferences((current) => ({ ...current, sidebarSide: current.sidebarSide === 'left' ? 'right' : 'left' })) }),
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
  }, [labels.primary, labels.shift, layout, activeGroupKey, projectName, newProjectSession, newSession, split, runShortcut, startAgent, sessions, projectOf, groups, focusedId, focusedSession, showSessionById, preferences.sidebarSide])

  const restartOffer: RestartOffer | null = mismatch
    ? {
        label: restartStatusQuery.isPending ? 'Checking service…' : restartStatusQuery.isError ? 'Restart check failed' : restartStatusQuery.data.canRestart ? (restartStatusQuery.data.sessionCount > 0 ? `Restart and restore ${String(restartStatusQuery.data.sessionCount)} session${restartStatusQuery.data.sessionCount === 1 ? '' : 's'}` : 'Restart service') : 'Service in use',
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
  const offlineDetail = mismatch ? `${daemonConnectionError(sessionsQuery.error)}${olderServiceNote(restartStatusQuery.data)}` : null

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
      side={preferences.sidebarSide}
      width={sidebarWidth}
      tabOf={tabOf}
      tabPanes={tabPanes}
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
      onSetName={saveName}
      onRenameSession={setRenameId}
      onPinSession={pinSession}
      onOpenSettings={() => { afterPick(); setSettingsSection('general') }}
      updateSlot={updateSlot('footer')}
      railUpdateSlot={updateSlot('rail')}
    />
  )

  return (
    <ReducedMotionContext.Provider value={reducedMotion}>
      <EffectsContext.Provider value={effects}>
        <div ref={setRoot} className={`mx-app${narrow ? ' is-narrow' : ''}`} style={rootStyle} data-motion={reducedMotion ? 'reduced' : 'full'} data-side={preferences.sidebarSide}>
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
                  sessions={paneSessions}
                  primary={labels.primary}
                  canStart={canStart}
                  portalContainer={root}
                  onSelect={(tabId) => { setLayout((current) => selectTab(current, tabId)) }}
                  onClose={closeTabById}
                  onCloseOthers={closeOtherTabs}
                  onRenameSession={saveName}
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
                      <Fragment key={column[0]?.id ?? columnIndex}>
                        {columnIndex > 0 && tab.max === null && !narrow && (
                          <ColumnHandle
                            index={columnIndex - 1}
                            onResize={(widths) => { setLayout((current) => setColumnWidths(current, tab.id, widths)) }}
                            onReset={() => { setLayout((current) => setColumnWidths(current, tab.id, null)) }}
                          />
                        )}
                      <div className="mx-pane-column" style={tab.widths && tab.max === null && !narrow && tab.widths.length === columns.length ? { flexGrow: tab.widths[columnIndex] } : undefined}>
                        {column.map((pane) => {
                          const session = paneSessions.get(pane.sessionId)
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
                              focused={focused && !outsidePanes}
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
                                  ended={session.ended}
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
                              onRestart={restartPane}
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
                {!focusMode && preferences.statusBar && (
                  <StatusBar
                    online={online}
                    themeMode={preferences.themeMode}
                    lightInUse={theme.light}
                    activeTheme={theme.name}
                    style={preferences.style}
                    textSize={preferences.terminalFontSize}
                    minTextSize={TERMINAL_FONT_SIZE_MIN}
                    maxTextSize={TERMINAL_FONT_SIZE_MAX}
                    shortcutChord={hotkeyRows(mac).find((row) => row.action === 'Keyboard shortcuts')?.chords[0] ?? []}
                    portalContainer={root}
                    onThemeMode={(themeMode) => { setPreferences((current) => ({ ...current, themeMode })) }}
                    onPickTheme={(name) => { setPreferences((current) => (findTheme(name).light ? { ...current, lightTheme: name } : { ...current, darkTheme: name })) }}
                    onAllThemes={() => { setSettingsSection('appearance') }}
                    onStyle={(style) => { setPreferences((current) => ({ ...current, style })) }}
                    onTextSize={setTextSize}
                    onResetTextSize={() => { setTextSize(DEFAULT_TEXT_SIZE) }}
                    onOpenShortcuts={() => { setShortcutsOpen(true) }}
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
                onPreviewTheme={previewThemeSoon}
                onTestNotice={showTestNotice}
                updates={{ summary: updaterSummary(updater.state), status: updater.state.status, onCheck: updater.check, onDownload: updater.download, onRestart: restartNow }}
                onShowWelcome={() => { setPreviewTheme(null); setSettingsSection(null); setWelcomeOpen(true) }}
                mac={mac}
                daemon={{ online, sessionCount: sessions.length }}
                projects={groups.map((group) => ({ key: group.key, name: group.name, colorId: group.colorId }))}
                validateFolder={folderExists}
                onClose={() => { setPreviewTheme(null); setSettingsSection(null) }}
              />
            )}
            {toastMessage && <Toast key={toastMessage.key} message={toastMessage.text} />}
            {restarting && (
              <div className="mx-restarting" role="status">
                <strong>Restarting Maitty</strong>
                <span>Reopening {layout.tabs.length === 1 ? '1 tab' : `${String(layout.tabs.length)} tabs`}. Your sessions never stopped.</span>
              </div>
            )}
          </div>
          <Toaster position={preferences.toastPosition} theme={theme.light ? 'light' : 'dark'} />
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

