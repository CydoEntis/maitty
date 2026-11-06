import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { z } from 'zod'
import {
  archiveConversationOnDaemon,
  closeChatSession,
  markSeenOnDaemon,
  pinSessionOnDaemon,
  defaultSessionFolder,
  conversationsFromSnapshot,
  isDesktop,
  listenSessionsChanged,
  loadConversations,
  loadDaemonRestartStatus,
  renameConversationOnDaemon,
  restartMismatchedDaemon,
  sendTerminalInput,
  submitMessage,
  startChatSession,
  startShellSession,
} from '@/api/daemon'
import { LOCAL_MACHINE, addMachine, cancelAddMachine, listSshHosts, listenAttention, listenMachineClipboard, listenMachineError, listenMachineNotice, listenMachineRemoved, listenMachineSessions, listenMachineState, loadMachinesOverview, removeMachine, retryMachine, setMachineGrant, startSessionOnMachine, terminalOf, unregisterMachine, type Attention, type Grant, type MachineState } from '@/api/machines'
import { TitleBar } from '@/components/TitleBar'
import { Toaster } from '@/components/ui/sonner'
import { markConversationsDisconnected } from '@/data/connection'
import { DesktopSettings, type DesktopSettingsSection, type UsageInfo } from '@/features/settings/components/DesktopSettings'
import { readSettingsPreferences, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN, writeSettingsPreferences, type SettingsPreferences } from '@/features/settings/preferences'
import type { Conversation } from '@/data/preview'
import { sizeVariables } from './model/layoutSize'
import { daemonConnectionError, daemonProtocolMismatch } from '@/lib/daemon-connection'
import { errorMessage } from '@/lib/error-message'
import { guardLocal } from '@/lib/localOnly'
import { markGpu } from '@/lib/gpu'
import { isWindowActive, subscribeWindowActivity } from '@/lib/window-activity'
import { styleUnderCap, type EffectStyle } from '@/lib/animation-level'
import { levelFor, useAutoState } from '@/lib/animation-auto'
import { useAdaptiveAnimations } from './useAdaptiveAnimations'
import { PerfOverlay } from './components/PerfOverlay'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { TerminalView } from '@/TerminalView'
import { EffectsContext, ReducedMotionContext } from './components/motion'
import { ColumnHandle } from './components/ColumnHandle'
import { CommandPalette } from './components/CommandPalette'
import { InboxPanel, type Answer, type InboxEntry } from './components/InboxPanel'
import { InspectPanel, type InspectView } from '@/features/inspect/InspectPanel'
import { runsProgram } from './components/useArmed'
import { rememberSessions, withEnded } from './model/ended'
import { isChatSession, isListedChatSession } from './model/chatSession'
import { advanceLoginWatch, LOGIN_CHECK_MS, startLoginWatch, type LoginWatch } from './model/loginWatch'
import { announcementFor, idsOfMachinesNotHeard, machineHeads, remoteStartRefusal, remoteTargetOf, remoteTerminals, type RemoteMachine } from './model/machines'
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
import { UpdatedNotice } from '@/features/updates/UpdatedNotice'
import { markRestarting, skippedUpdate, skipUpdate, takeRestarted, type RestartedUpdate } from '@/features/updates/updateMemory'
import { useIdleUpdate } from '@/features/updates/useIdleUpdate'
import { routeDrop } from '@/features/terminal/dropRoute'
import { dropOnComposer } from '@/features/chat/composerDrop'
import { useFileDrop } from '@/features/terminal/useFileDrop'
import { useFileSends } from '@/features/terminal/useFileSends'
import { useImagePaste } from '@/features/terminal/useImagePaste'
import { updaterSummary, useUpdater } from '@/features/updates/useUpdater'
import { sendNativeNotice } from '@/features/notifications/native'
import { chooseProjectFolder } from '@/api/folderPicker'
import { dismissSessionNotice, showSessionNotice } from '@/features/notifications/notify'
import { sessionNoticeFor } from '@/features/notifications/noticeForSession'
import { noticeForAttention, noticesFor, type Notice } from '@/features/notifications/transitions'
import { enableNotificationSound, playNotificationSound } from '@/data/notificationSound'
import { NewWorktreeDialog } from '@/features/worktrees/NewWorktreeDialog'
import { newWorktreeSession } from '@/features/worktrees/api'
import type { TerminalHandle } from '@/features/terminal/handle'
import type { LookAt } from '@/features/inspect/FilesTab'
import { PaneCard } from './components/PaneCard'
import { ChatPane } from '@/features/chat/components/ChatPane'
import { ANSWER_KEYS } from '@/features/chat/permissionAsk'
import { NoTerminalNote } from '@/features/chat/components/NoTerminalNote'
import { rememberConversationPermissions } from '@/data/conversationPermissions'
import { reserveOperation } from '@/features/chat/sendOperation'
import { usePaneViews } from './usePaneViews'
import { chatAvailability, chatSourceOf, effectiveView, type ChatAgent, type PaneView } from './model/chatView'
import { listFolders } from '@/api/folders'
import { RemoteSessionDialog } from './components/RemoteSessionDialog'
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
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { topmostOpen, type EscLayer } from './model/escLadder'
import { jumpToast, NOTHING_NEEDS_YOU, nextNeedsYou } from '@/features/needs-you/model/needsYou'
import { bindingFor, detectPlatform, matchShortcut, shortcutChords, shortcutText, type ShortcutAction } from './model/shortcuts'
import { folderExists, resolveStartFolder } from './model/startFolder'
import { markSeen, trackSessions, tracksEqual, type SessionTrack } from './model/tracking'
import type { PaletteItem } from './model/palette'
import { activeThemeName, findTheme, terminalMinimumContrast, themeStyle, THEMES, xtermTheme } from './theme/themes'
import { COMPACT_WIDTH_PX, NARROW_WIDTH_PX } from './model/windowSize'
import './workspace.css'

/** The daemon pushes every change (`sessions-changed`); this slow poll only catches a stream that went quiet. */
const SESSION_REFRESH_MS = 15_000
/** How long a new session on another machine is waited for, and how often its list is looked at. */
const REMOTE_START_WAIT_MS = 8000
const REMOTE_START_POLL_MS = 250
const MIN_TABS_FOR_NUMBERS = 2
/** A tab's hover card lists its panes only when it has more than one. */
const MIN_PANES_FOR_LIST = 2
const TOAST_MS = 2600
/** What clicking the status bar's size resets the terminal text to. */
const DEFAULT_TEXT_SIZE = 14
/** How long a second ⌘W confirms closing a session that is running a program. */
const CLOSE_ARM_MS = 4000
/** How long "Restarting Maitty" shows before the window relaunches. */
const RESTART_SCREEN_MS = 1200
const ABSOLUTE_PATH = /^(\/|[A-Za-z]:[\\/])/
const TEST_NOTICE_MS = 6000
const SECOND_MS = 1000
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
  const [query] = useState(() => (typeof window.matchMedia === 'function' ? window.matchMedia(REDUCED_MOTION_QUERY) : null))
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
  const [query] = useState(() => (typeof window.matchMedia === 'function' ? window.matchMedia(DARK_SCHEME_QUERY) : null))
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
 * The desktop app: a terminal multiplexer window over sessions maittyd owns. Tabs and
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
  const platform = detectPlatform(navigator.userAgent)
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
  const paneViews = usePaneViews()
  const [layout, setLayout] = useState<Layout>(() => (preferences.restoreTabs ? readLayout() : EMPTY_LAYOUT))
  // The width while the sidebar's edge is being dragged; it is saved when the pointer is released.
  const [liveSidebarWidth, setLiveSidebarWidth] = useState<number | null>(null)
  const [settingsSection, setSettingsSection] = useState<DesktopSettingsSection | null>(null)
  const [renameId, setRenameId] = useState<string | null>(null)
  const [paletteOpen, setPaletteOpen] = useState(false)
  const [inboxOpen, setInboxOpen] = useState(false)
  const [focusMode, setFocusMode] = useState(false)
  const [inspectOpen, setInspectOpen] = useState(false)
  // The docked panel's width, so the Inspect drawer that overlays the window stays beside it and not under it.
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
  const [windowFocused, setWindowFocused] = useState(() => document.hasFocus())
  // Inbox entries the user cleared, by the status they had; an entry comes back if the session changes state.
  const [cleared, setCleared] = useState<ReadonlyMap<string, SessionView['status']>>(new Map())
  const [toastMessage, setToastMessage] = useState<{ text: string; key: number } | null>(null)
  // After a restart into an update, the first thing said is which version this is and what changed.
  const [updatedTo, setUpdatedTo] = useState<RestartedUpdate | null>(takeRestarted)
  const [tracks, setTracks] = useState<ReadonlyMap<string, SessionTrack>>(new Map())
  const [dragId, setDragId] = useState<string | null>(null)
  const [dropId, setDropId] = useState<string | null>(null)
  const [starting, setStarting] = useState(false)
  // A new session on another machine asks for a folder there first.
  const [remoteStart, setRemoteStart] = useState<{ machine: number; name: string } | null>(null)
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

  // Other machines (ADR 0007): the host holds their connections and tells the window each one's state
  // and, when it changes, the sessions it lists. The window never connects.
  const [machines, setMachines] = useState<ReadonlyMap<number, RemoteMachine>>(new Map())
  const machinesRef = useRef(machines)
  useEffect(() => { machinesRef.current = machines }, [machines])
  // The first answer about the machines has come, so a Machines page can tell loading from empty.
  const [machinesKnown, setMachinesKnown] = useState(false)
  // Said to a screen reader when a machine goes down or comes back.
  const [machineSaid, setMachineSaid] = useState('')
  useEffect(() => {
    const stops: Array<() => void> = []
    let gone = false
    const keep = (stop: () => void) => { if (gone) stop(); else stops.push(stop) }
    const change = (id: number, update: (before: RemoteMachine | undefined) => RemoteMachine | undefined) => {
      setMachines((current) => {
        const next = update(current.get(id))
        if (next === undefined) return current
        return new Map(current).set(id, next)
      })
    }
    // The host names a machine in its state, and only by number in its snapshots.
    const names = new Map<number, string>()
    const settle = (state: MachineState) => {
      if (state.machine === LOCAL_MACHINE) return
      names.set(state.machine, state.name)
      const said = announcementFor(machinesRef.current.get(state.machine)?.state, state)
      if (said !== null) setMachineSaid(said)
      change(state.machine, (before) => ({ conversations: before?.conversations ?? [], heard: before?.heard ?? false, state }))
      // Typing the machine could not take while it was down: say so once it is back.
      if (state.state === 'connected' && state.unsent) {
        toast.message(`${String(state.unsent)} ${state.unsent === 1 ? 'keystroke' : 'keystrokes'} to ${state.name} ${state.unsent === 1 ? 'was' : 'were'} not sent while it was disconnected`)
      }
    }
    const list = (id: number, name: string, snapshot: unknown) => {
      let conversations: Conversation[]
      try {
        conversations = conversationsFromSnapshot(snapshot, { id, name })
      } catch (error: unknown) {
        if (!(error instanceof z.ZodError)) throw error
        console.warn(`${name} sent a session list this window cannot read`, error)
        return
      }
      change(id, (before) => (before ? { ...before, conversations, heard: true } : undefined))
    }
    void loadMachinesOverview().then((overview) => {
      if (gone) return
      for (const machine of overview) {
        settle(machine)
        if (machine.snapshot !== null) list(machine.machine, machine.name, machine.snapshot)
      }
      setMachinesKnown(true)
    }, (error: unknown) => {
      console.warn('Could not read the state of the machines', error)
      if (!gone) setMachinesKnown(true)
    })
    for (const listening of [
      listenMachineState(settle),
      listenMachineError((text) => { toast.error(text) }),
      // A machine that was allowed to raise notices: in the window as a toast, away from it as the system's.
      // Text a machine that was allowed to asks to put on the clipboard.
      listenMachineClipboard((id, text) => {
        const name = names.get(id) ?? 'A machine'
        navigator.clipboard.writeText(text).then(
          () => { toast.message(`${name} copied ${String(text.length)} characters to your clipboard`) },
          (error: unknown) => { toast.error(errorMessage(error, `Could not copy what ${name} sent`)) },
        )
      }),
      listenMachineNotice((text) => {
        if (document.hasFocus()) toast.message(text)
        else sendNativeNotice(text, '').catch((error: unknown) => { console.warn('Could not show a system notification', error) })
      }),
      listenMachineRemoved((id) => {
        const name = names.get(id)
        names.delete(id)
        if (name !== undefined) unregisterMachine(name)
        setMachines((current) => {
          if (!current.has(id)) return current
          const next = new Map(current)
          next.delete(id)
          return next
        })
      }),
      listenMachineSessions((id, snapshot) => {
        const name = names.get(id)
        if (snapshot !== null && name !== undefined) list(id, name, snapshot)
      }),
    ]) {
      listening.then(keep, (error: unknown) => { toast.error(errorMessage(error, 'Could not follow the machines')) })
    }
    return () => {
      gone = true
      for (const stop of stops) stop()
    }
  }, [])
  const everySession = useMemo(() => {
    // Terminal sessions, and the chat sessions the daemon owns that are not put away (ADR 0003).
    const terminals = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal' || isListedChatSession(item))
    const here = online ? terminals : markConversationsDisconnected(terminals)
    return [...here, ...remoteTerminals(machines)].map((conversation) => toSessionView(conversation, home))
  }, [sessionsQuery.data, online, home, machines])
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
  // A path the machine kept a sent file at is typed into the pane the drop was for, if that pane is still open.
  const typeIntoPane = (sessionId: string, text: string) => {
    const handle = terminalHandles.current.get(sessionId)
    if (!handle) return false
    handle.paste(text)
    return true
  }
  const { sendFiles, sendImage } = useFileSends({ typeInto: typeIntoPane })
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
  // Every session some tab shows, so the hover card can say a session is open even with a single tab.
  const openIds = useMemo(() => new Set(layout.tabs.flatMap((candidate) => candidate.panes.map((pane) => pane.sessionId))), [layout.tabs])
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
  const heads = useMemo(() => machineHeads(machines), [machines])
  const retryNamed = useCallback((name: string) => {
    retryMachine(name).catch((error: unknown) => { toast.error(errorMessage(error, `Could not retry ${name}`)) })
  }, [])
  // A first login runs in a session on this computer; the machine is dialled again when it closes.
  const localTermsRef = useRef<ReadonlySet<number>>(new Set())
  const loginsRef = useRef<LoginWatch[]>([])
  const [watchingLogins, setWatchingLogins] = useState(false)
  const checkLogins = useCallback(() => {
    const next: LoginWatch[] = []
    for (const watch of loginsRef.current) {
      const step = advanceLoginWatch(watch, localTermsRef.current, Date.now())
      if (step.dial) retryNamed(watch.machine)
      if (step.watch) next.push(step.watch)
    }
    loginsRef.current = next
    setWatchingLogins(next.length > 0)
  }, [retryNamed])
  useEffect(() => {
    const terms = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal').map((item) => terminalOf(item.id))
    localTermsRef.current = new Set(terms.filter((term): term is number => term !== null))
  }, [sessionsQuery.data])
  useEffect(() => {
    if (!watchingLogins) return
    const timer = setInterval(checkLogins, LOGIN_CHECK_MS)
    return () => { clearInterval(timer) }
  }, [watchingLogins, checkLogins])
  const addNamed = useCallback(async (target: string, name?: string) => {
    const added = await addMachine(target, name)
    if (added.outcome === 'login') {
      loginsRef.current = [...loginsRef.current, startLoginWatch(added.name, localTermsRef.current, Date.now())]
      setWatchingLogins(true)
    }
    return added
  }, [])
  const grantNamed = useCallback((name: string, grant: Grant, on: boolean) => {
    setMachineGrant(name, grant, on).catch((error: unknown) => { toast.error(errorMessage(error, `Could not change ${name}`)) })
  }, [])
  const copyCommand = useCallback((command: string) => {
    navigator.clipboard.writeText(command).then(
      () => { toast.message('Copied. Run it in a terminal on this computer.') },
      (error: unknown) => { toast.error(errorMessage(error, 'Could not copy the command')) },
    )
  }, [])
  const cancelAdding = useCallback(() => {
    cancelAddMachine().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not cancel')) })
  }, [])
  const removeNamed = useCallback((name: string) => {
    removeMachine(name).then(
      () => { toast.message(`Removed ${name}. Its sessions keep running on ${name}.`) },
      (error: unknown) => { toast.error(errorMessage(error, `Could not remove ${name}`)) },
    )
  }, [])
  const groups = useMemo(() => groupSessions(sessions, preferences.groupBy, focusedId, home), [sessions, preferences.groupBy, focusedId, home])
  // A tab belongs to the project of the session in its focused pane, so it moves when that shell cds into another repo.
  const tabGroup = useCallback((candidate: Tab): string => {
    const sessionId = candidate.panes.find((pane) => pane.id === candidate.focus)?.sessionId ?? candidate.panes[0]?.sessionId
    const session = sessionId ? paneSessions.get(sessionId) : undefined
    return session ? groupKeyOf(session, preferences.groupBy) : ''
  }, [paneSessions, preferences.groupBy])

  const reducedMotion = preferences.reduceMotion || systemReducedMotion
  const theme = findTheme(previewTheme ?? activeThemeName(preferences, systemDark))
  const look = useMemo(() => ({ fontFamily: preferences.terminalFontFamily, fontWeight: preferences.terminalFontWeight, fontWeightBold: preferences.terminalFontWeightBold, cursorStyle: preferences.cursorStyle, scrollback: preferences.scrollback, scrollSpeed: preferences.scrollSpeed, screenReader: preferences.terminalScreenReader, seeThrough: preferences.terminalSeeThrough, cursorBlink: preferences.cursorBlink, plainDrawing: preferences.terminalPlainDrawing, theme: xtermTheme(theme.tokens), minimumContrast: terminalMinimumContrast(theme) }), [preferences.terminalFontFamily, preferences.terminalFontWeight, preferences.terminalFontWeightBold, preferences.cursorStyle, preferences.cursorBlink, preferences.scrollback, preferences.scrollSpeed, preferences.terminalScreenReader, preferences.terminalSeeThrough, preferences.terminalPlainDrawing, theme])
  const auto = useAutoState()
  const cap = levelFor(preferences.animations, auto)
  const effects = useMemo(() => {
    const allowed = online && !reducedMotion
    const styled = (on: boolean, style: EffectStyle) => (allowed && on ? styleUnderCap(style, cap) : null)
    return {
      working: styled(preferences.effectWorking, preferences.effectStyleWorking),
      needs: styled(preferences.effectNeedsYou, preferences.effectStyleNeedsYou),
      done: styled(preferences.effectDone, preferences.effectStyleDone),
      workingScope: preferences.workingScope,
      cap,
    }
  }, [online, reducedMotion, cap, preferences.effectWorking, preferences.effectNeedsYou, preferences.effectDone, preferences.effectStyleWorking, preferences.effectStyleNeedsYou, preferences.effectStyleDone, preferences.workingScope])
  // Waves are running when a session in a status that has a Waves effect is listed.
  const wavesRunning = useMemo(() => sessions.some((session) => (session.status === 'working' || session.status === 'needs' || session.status === 'done') && effects[session.status] === 'waves'), [sessions, effects])
  useAdaptiveAnimations(preferences.animations === 'auto' && !reducedMotion && cap !== 'none', wavesRunning)

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

  useEffect(() => { markGpu() }, [])

  // Looping effects pause in the stylesheet while the window is hidden or in the background. It is an
  // attribute on <html>, not React state, so a focus change repaints nothing and re-renders nothing.
  useEffect(() => {
    const mark = () => { document.documentElement.dataset.window = isWindowActive() ? 'active' : 'inactive' }
    mark()
    return subscribeWindowActivity(mark)
  }, [])

  useEffect(() => {
    // Flat and Tiled share one finish (neutral, no glow); Tiled changes the layout on top of it.
    document.documentElement.dataset.style = preferences.style
    document.documentElement.dataset.finish = preferences.style === 'glass' ? 'glass' : 'flat'
  }, [preferences.style])

  useEffect(() => {
    document.documentElement.dataset.glow = preferences.backgroundGlow ? 'on' : 'off'
  }, [preferences.backgroundGlow])

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

  // The host forwards the daemon's snapshots as they change, so the list is current without asking.
  // A snapshot that does not read, or a dropped stream, sends the query to ask the daemon itself.
  useEffect(() => {
    const askAgain = () => { void queryClient.invalidateQueries({ queryKey: ['conversations'] }) }
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
  }, [queryClient])

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
      const reconciled = openFirst ? reconcileLayout(current, live, tabGroup) : current
      return openFirst && reconciled.tabs.length === 0 && first ? openInNewTab(reconciled, first.id, makeId) : reconciled
    })
  }, [sessions, sessionsQuery.data, sessionsQuery.isSuccess, tabGroup])

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

  // Lists again and opens the chat session the daemon just started, with its first prompt if there is one.
  const openStartedChat = useCallback(async (started: { id: number; generation: number }, prompt: string) => {
    const id = String(started.id)
    await queryClient.invalidateQueries({ queryKey: ['conversations'] })
    await queryClient.query({ queryKey: ['conversations'], queryFn: loadConversations, staleTime: 0 })
    setLayout((current) => openInNewTab(current, id, makeId))
    if (prompt !== '') {
      // The start has answered, so the agent is up: the first prompt is an ordinary send, number 1.
      const operation = reserveOperation(started.id, { receipt: null, queued: [] })
      await submitMessage({ id, generation: started.generation }, operation, prompt)
    }
  }, [queryClient])

  // A chat session in `path`, opened in a new tab. Chats are this computer's: another machine's shell asks first.
  const startChat = useCallback((path: string, agent: ChatAgent, fullAccess: boolean) =>
    startChatSession(path, agent, null, fullAccess).then((started) => {
      if (fullAccess) rememberConversationPermissions(String(started.id), 'full-access')
      return openStartedChat(started, '')
    }).then(() => { flash(`${agent} chat started`) }), [openStartedChat, flash])

  // "Start a Claude chat" on a shell's empty chat card: a chat session in that shell's folder.
  const startChatHere = useCallback((from: SessionView, agent: ChatAgent, fullAccess: boolean) => {
    const refusal = remoteStartRefusal(from.machine !== LOCAL_MACHINE, from.machineName, 'A chat session')
    if (refusal) {
      toast.message(refusal)
      return
    }
    if (!online || starting) return
    setStarting(true)
    startChat(from.cwd, agent, fullAccess)
      .catch((error: unknown) => { toast.error(errorMessage(error, `Could not start a ${agent} chat`)) })
      .finally(() => { setStarting(false) })
  }, [online, starting, startChat])

  const runStart = useCallback((folder: () => Promise<string>, place: (sessionId: string) => void) => {
    if (!online || starting) return
    setStarting(true)
    void folder()
      .then((path) => startShell(path))
      .then((id) => { if (id) place(id) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not start a shell')) })
      .finally(() => { setStarting(false) })
  }, [online, starting, startShell])

  // A new session is the default agent, running in a terminal the daemon owns, shown as its chat or
  // as its terminal as Settings says. It is one session either way: the view can be switched at any
  // time without restarting the agent (ADR 0003). A plain shell is `runStart` and has no chat view.
  const startAgentSession = useCallback((folder: () => Promise<string>) => {
    const agent = preferences.defaultAgent
    const view = preferences.newSessionView
    runStart(folder, (id) => {
      setLayout((current) => openInNewTab(current, id, makeId))
      if (view === 'chat') paneViews.choose(id, 'chat')
      void sendTerminalInput({ id, generation: 0 }, `${agent}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${agent}`)) })
    })
  }, [preferences.defaultAgent, preferences.newSessionView, runStart, paneViews])

  const newSession = useCallback(() => {
    setSettingsSection(null)
    // A new session goes where the focused one is: on another machine the folder is typed there,
    // because the folder picker and the start folder are this computer's.
    const remote = remoteTargetOf(focusedSession)
    if (remote !== null) {
      setRemoteStart(remote)
      return
    }
    startAgentSession(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())))
  }, [startAgentSession, preferences.startFolder, home, focusedSession])

  // A plain shell in the start folder: terminal view only, whatever the default agent is.
  const newTerminal = useCallback(() => {
    setSettingsSection(null)
    const remote = remoteTargetOf(focusedSession)
    if (remote !== null) {
      setRemoteStart(remote)
      return
    }
    runStart(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => { setLayout((current) => openInNewTab(current, id, makeId)) })
  }, [runStart, preferences.startFolder, home, focusedSession])

  // The sidebar's header for a machine and the palette both choose where the session goes.
  const newSessionOn = useCallback((machine: number, name: string) => {
    setSettingsSection(null)
    setRemoteStart({ machine, name })
  }, [])

  // What the folder dialog asks the machine, and the folders its sessions are in.
  const remoteMachine = remoteStart?.machine ?? LOCAL_MACHINE
  const listOnRemote = useCallback((path: string) => listFolders(remoteMachine, path), [remoteMachine])
  const recentOnRemote = useMemo(() => {
    const folders = (machines.get(remoteMachine)?.conversations ?? []).flatMap((item) => (item.kind === 'terminal' && item.cwd ? [item.cwd] : []))
    return [...new Set(folders)].reverse()
  }, [machines, remoteMachine])

  const startOnMachine = useCallback(async (target: { machine: number; name: string }, folder: string) => {
    const known = new Set((machinesRef.current.get(target.machine)?.conversations ?? []).map((item) => item.id))
    await startSessionOnMachine(target.machine, folder)
    setRemoteStart(null)
    // The machine tells its new session in its next list; the tab opens when it does.
    const deadline = Date.now() + REMOTE_START_WAIT_MS
    while (Date.now() < deadline) {
      const fresh = machinesRef.current.get(target.machine)?.conversations.find((item) => item.kind === 'terminal' && !known.has(item.id))
      if (fresh) {
        setLayout((current) => openInNewTab(current, fresh.id, makeId))
        return
      }
      await new Promise((resolve) => { setTimeout(resolve, REMOTE_START_POLL_MS) })
    }
    toast.error(`${target.name} did not report the new session`)
  }, [])

  // The + after the tabs starts a shell in the project you are in; ⌘T and the sidebar's New use the start folder.
  const activeGroupKey = (() => {
    const showing = activeTab(layout)
    return showing ? tabGroup(showing) : ''
  })()
  const startIn = useCallback((folder: string) => {
    setSettingsSection(null)
    startAgentSession(() => Promise.resolve(folder))
  }, [startAgentSession])
  const startTerminalIn = useCallback((folder: string, machineName: string | null = null) => {
    setSettingsSection(null)
    if (machineName !== null) {
      // The project is on another machine: the shell starts there, in its folder.
      const head = heads.find((candidate) => candidate.name?.toLowerCase() === machineName.toLowerCase())
      if (head?.name == null) return
      void startOnMachine({ machine: head.machine, name: head.name }, folder).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start a shell on ${machineName}`)) })
      return
    }
    runStart(() => Promise.resolve(folder), (id) => { setLayout((current) => openInNewTab(current, id, makeId)) })
  }, [runStart, heads, startOnMachine])
  const explainLocalOnly = useCallback((machineName: string) => {
    toast.message(remoteStartRefusal(true, machineName, 'A chat session') ?? `Chats run on this computer only; use New terminal here for a shell on ${machineName}.`)
  }, [])
  const activeGroup = groups.find((group) => group.key === activeGroupKey)
  const projectName = activeGroup?.name
  // A project of another machine is keyed by that machine and its folder: not a folder to start a shell in here.
  const remoteGroup = activeGroup?.machineName ?? null
  const newProjectSession = useCallback(() => {
    if (!activeGroupKey) {
      newSession()
      return
    }
    const refusal = remoteStartRefusal(remoteGroup !== null, remoteGroup, 'A new session here')
    if (refusal) {
      toast.message(refusal)
      return
    }
    setSettingsSection(null)
    startAgentSession(() => Promise.resolve(activeGroupKey))
  }, [activeGroupKey, remoteGroup, newSession, startAgentSession])

  const split = useCallback((paneId: string | null, direction: SplitDirection) => {
    const tab = activeTab(layout)
    if (!tab || tab.panes.length >= MAX_PANES) return
    const source = tab.panes.find((pane) => pane.id === (paneId ?? tab.focus))
    const sourceSession = source ? sessionMap.get(source.sessionId) : undefined
    const refusal = sourceSession ? remoteStartRefusal(sourceSession.machine !== LOCAL_MACHINE, sourceSession.machineName, 'Splitting this pane') : null
    if (refusal) {
      toast.message(refusal)
      return
    }
    const cwd = sourceSession?.cwd
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
    void Promise.all(running.map((session) => (isChatSession(session.conversation) ? closeChatSession(session.conversation) : archiveConversationOnDaemon({ id: session.id }, true))))
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
    if (isChatSession(old.conversation)) {
      toast.message('A chat session cannot be restarted. Close it and start a new one.')
      return
    }
    const refusal = remoteStartRefusal(old.machine !== LOCAL_MACHINE, old.machineName, 'Restarting this pane')
    if (refusal) {
      toast.message(refusal)
      return
    }
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
  }, [showSessionById])

  // In a window too narrow for the sidebar, the same key opens and closes its drawer.
  useEffect(() => {
    let active = true
    void appVersion().then((version) => { if (active) setRunningVersion(version) }).catch(() => undefined)
    return () => { active = false }
  }, [])

  const restartInto = useCallback((automatic: boolean) => {
    if (updater.state.status !== 'ready') return
    markRestarting({ version: updater.state.version, previous: runningVersion, notes: updater.releaseNotes(), automatic })
    setRestarting(true)
    // If the install fails, the screen must not stay up: it goes, and so does the "kept running" note for the next launch.
    window.setTimeout(() => { updater.restart(() => { setRestarting(false); takeRestarted() }) }, RESTART_SCREEN_MS)
  }, [updater, runningVersion])
  const restartNow = useCallback(() => { restartInto(false) }, [restartInto])
  const installWhenIdle = useCallback(() => { restartInto(true) }, [restartInto])

  // With the setting on, a new release is fetched as soon as it is found. Installing still waits for a restart.
  const updateStatus = updater.state.status
  const updateOffered = updater.state.status === 'available' ? updater.state.version : null
  const downloadUpdate = updater.download
  useEffect(() => {
    if (!preferences.autoDownloadUpdates || updateStatus !== 'available' || updateOffered === skippedVersion) return
    const timer = window.setTimeout(downloadUpdate, 0)
    return () => { window.clearTimeout(timer) }
  }, [preferences.autoDownloadUpdates, updateStatus, updateOffered, skippedVersion, downloadUpdate])

  // With the setting on, an idle app fetches and installs an update by itself (docs/stack/tauri-updater.md).
  const busyCount = sessions.filter((session) => session.status === 'working' || session.status === 'needs').length
  useIdleUpdate({
    enabled: preferences.restartWhenIdle && online && !restarting,
    agentBusy: busyCount > 0,
    status: updateStatus,
    offeredIsSkipped: updateOffered !== null && updateOffered === skippedVersion,
    onDownload: downloadUpdate,
    onInstall: installWhenIdle,
  })

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

  // The tab bar and status bar are memoised, so what they are handed keeps its identity.
  const selectTabById = useCallback((tabId: string) => { setLayout((current) => selectTab(current, tabId)) }, [])
  const openInbox = useCallback(() => { setInboxOpen(true) }, [])
  // The repository view reads this computer's checkouts: on a pane of another machine it says so.
  const inspectRefusal = guardLocal(focusedSession, 'inspect')
  const toggleInspect = useCallback(() => {
    if (inspectRefusal !== null) toast.message(inspectRefusal)
    else setInspectOpen((open) => !open)
  }, [inspectRefusal])
  const openDrawer = useCallback(() => { setDrawerOpen(true) }, [])
  // An edit block's header: show the repository's changes in the Inspect panel.
  const showChanges = useCallback(() => {
    if (inspectRefusal !== null) {
      toast.message(inspectRefusal)
      return
    }
    setInspectView('changes')
    setInspectOpen(true)
  }, [inspectRefusal])
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

  const runShortcut = useCallback((action: ShortcutAction, tab?: number) => {
    switch (action) {
      case 'new-session':
        newSession()
        return
      case 'new-terminal':
        newTerminal()
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
          const chord = shortcutText('close', platform)
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
      case 'jump-needs': {
        const target = nextNeedsYou(sessions, focusedSession?.id ?? null)
        if (!target) flash(NOTHING_NEEDS_YOU)
        else {
          showSessionById(target.session.id)
          flash(jumpToast(target))
        }
        return
      }
      case 'go-to-tab': {
        const inProject = layout.tabs.filter((candidate) => tabGroup(candidate) === activeGroupKey)
        const target = inProject[(tab ?? 0) - 1]
        if (target) selectTabById(target.id)
        return
      }
      case 'performance':
        setSettingsSection('performance')
        return
      // Registered in the table but not built (or not undoable): the key does nothing until the feature ships.
      case 'undo-close':
      case 'usage':
        setSettingsSection('usage')
        return
      case 'chat-toggle': {
        // A chat session has only its chat; there is nothing to switch to.
        if (!focusedSession || isChatSession(focusedSession.conversation)) return
        const next: PaneView = chatOf(focusedSession).view === 'chat' ? 'terminal' : 'chat'
        paneViews.choose(focusedSession.id, next)
        return
      }
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
        toggleInspect()
        return
      case 'find':
        setFindOpen((open) => !open)
        return
      case 'new-worktree': {
        const refusal = guardLocal(focusedSession, 'worktrees')
        if (refusal !== null) toast.message(refusal)
        else if (focusedSession && online) setWorktreeFrom(focusedSession)
        return
      }
      case 'broadcast':
        if (layout.activeTabId) {
          const turningOn = !activeTab(layout)?.broadcast
          setLayout((current) => toggleBroadcast(current, current.activeTabId ?? ''))
          flash(turningOn ? 'Broadcast on · typing goes to every idle pane in this tab' : 'Broadcast off')
        }
        return
    }
  }, [newSession, newTerminal, split, layout, flash, toggleSidebar, tabGroup, focusedSession, online, preferences.terminalFontSize, setTextSize, endSessions, platform, paneSessions, armedClose, toggleInspect, sessions, showSessionById, activeGroupKey, selectTabById, chatOf, paneViews])

  // Captured on the window before xterm sees the key, so only these few keys leave the shell.
  const overlayOpen = paletteOpen || inboxOpen
  const dialogOpen = settingsSection !== null || renameId !== null || remoteStart !== null
  const modalOpen = dialogOpen || overlayOpen
  const drawerShown = compact && drawerOpen
  const shortcutState = useRef({ runShortcut, settingsOpen: dialogOpen, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen })
  useEffect(() => { shortcutState.current = { runShortcut, settingsOpen: dialogOpen, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen } })
  useEffect(() => {
    // How each layer of the Esc ladder closes. A new dialog adds its closer here and its id to ESC_LADDER.
    const closers: Partial<Record<EscLayer, () => void>> = {
      palette: () => { setPaletteOpen(false) },
      inbox: () => { setInboxOpen(false) },
      settings: () => { setSettingsSection(null) },
      drawer: () => { setDrawerOpen(false) },
      'focus-mode': () => { setFocusMode(false) },
    }
    const onKeyDown = (event: KeyboardEvent) => {
      const state = shortcutState.current
      if (state.welcomeOpen) return
      const match = matchShortcut(event, platform)
      if (state.shortcutsOpen) {
        // Only its own key closes it; Esc is the dialog's.
        if (match?.action === 'shortcuts') {
          event.preventDefault()
          event.stopPropagation()
          state.runShortcut('shortcuts')
        }
        return
      }
      if (event.key === 'Escape') {
        // A menu or list that is open (the spinner picker in Settings) takes Esc itself.
        if (event.target instanceof Element && event.target.closest('[role="menu"]')) return
        const top = topmostOpen({ palette: state.paletteOpen, inbox: state.inboxOpen, settings: state.settingsOpen, drawer: state.drawerShown, 'focus-mode': state.focusMode })
        if (top === null) return
        // What Esc closes is not also sent on to the terminal, where it would interrupt an agent.
        event.preventDefault()
        event.stopPropagation()
        closers[top]?.()
        return
      }
      if (state.overlayOpen) {
        if (match?.action === 'palette' || match?.action === 'inbox') {
          event.preventDefault()
          event.stopPropagation()
          state.runShortcut(match.action)
        }
        return
      }
      if (state.settingsOpen) return
      if (!match) return
      event.preventDefault()
      event.stopPropagation()
      state.runShortcut(match.action, match.tab)
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => { window.removeEventListener('keydown', onKeyDown, true) }
  }, [platform])

  // The project each session sits in, for the inbox and the palette.
  const projectOf = useMemo(() => {
    const map = new Map<string, { name: string; key: string }>()
    for (const group of groups) for (const member of group.sessions) map.set(member.id, { name: group.name, key: group.key })
    return map
  }, [groups])

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
  const needsCount = inboxEntries.filter((entry) => entry.session.status === 'needs').length
  const needsElsewhere = sessions.filter((item) => item.status === 'needs' && item.id !== focusedId).length

  // Inspect looks at the focused pane's own repository; the tab bar's +N −N comes from the same daemon question.
  // A pane of another machine has no checkout on this computer, so there is nothing to inspect.
  const inspectTerm = online && isDesktop() && focusedSession && focusedSession.machine === LOCAL_MACHINE && !isChatSession(focusedSession.conversation) ? focusedSession.pane : null
  const diffStat = useQuery({
    queryKey: ['repo', inspectTerm, 'diff-stat'],
    queryFn: async () => {
      const diff = await repoDiff(inspectTerm ?? 0)
      return { additions: diff.additions, deletions: diff.deletions }
    },
    enabled: inspectTerm !== null,
    // The counts come from a full diff, so they are read slowly unless Inspect is open, and not
    // again just because the window came to the front.
    refetchInterval: inspectOpen ? DIFF_STAT_REFRESH_MS : DIFF_STAT_IDLE_MS,
    refetchOnWindowFocus: false,
    retry: false,
  })
  const inspect = useMemo(() => {
    if (inspectTerm !== null) return { open: inspectOpen, additions: diffStat.data?.additions ?? 0, deletions: diffStat.data?.deletions ?? 0 }
    return inspectRefusal === null ? null : { open: false, additions: 0, deletions: 0, refusal: inspectRefusal }
  }, [inspectTerm, inspectOpen, diffStat.data, inspectRefusal])

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

  const sendTo = useCallback((target: SessionView, data: string) => {
    // A chat session has no terminal to type into; its answers go through the chat, by request id.
    if (isChatSession(target.conversation)) return Promise.reject(new Error('A chat session has no terminal. Use its chat.'))
    return sendTerminalInput({ id: target.id, generation: target.conversation.generation ?? 0 }, data)
  }, [])

  const answerSession = useCallback((sessionId: string, answer: Answer) => {
    const target = sessionMap.get(sessionId)
    if (!target) return
    // A card can outlast the question. Typing into a session that is not asking would land in its shell.
    if (target.status !== 'needs') {
      flash(`${displayName(target)} is not asking any more`)
      return
    }
    // The answer is typed into the session's own prompt; the GUI never decides for it.
    // Claude Code's permission menu numbers its choices and ignores a bare `y`; only that key is verified.
    const keys = target.process === 'claude' && answer === 'y' ? ANSWER_KEYS.approve : answer
    void sendTo(target, keys)
      .then(() => { flash(`Sent “${answer}” to ${displayName(target)}`) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not answer the session')) })
  }, [sessionMap, sendTo, flash])

  // In the window but on another pane: a toast you can click. Away from the window: the system's.
  const raiseNotice = useCallback((notice: Notice) => {
    if (document.hasFocus()) {
      const target = sessionMap.get(notice.sessionId)
      const card = target && sessionNoticeFor(target, projectOf.get(target.id), {
        open: () => { showSessionById(notice.sessionId) },
        // A chat session's approval is answered in its chat, by the request's id, never by typing a letter.
        allow: online && !isChatSession(target.conversation) ? () => { answerSession(notice.sessionId, 'y') } : null,
      })
      if (card) showSessionNotice(notice.sessionId, card)
      return
    }
    lastNotified.current = { sessionId: notice.sessionId, at: Date.now() }
    sendNativeNotice(notice.title, notice.body).catch((error: unknown) => { console.warn('Could not show a system notification', error) })
  }, [sessionMap, projectOf, showSessionById, online, answerSession])

  const { notifyNeedsYou, notifyFinished, notifyError, notificationSound } = preferences

  // A chat session that just started needing you, finished or failed. A terminal session's alert comes from
  // the daemon (below), which decides it once per event; diffing statuses here as well would alert twice.
  useEffect(() => {
    const previousStatuses = lastStatuses.current
    // What another machine's agents do raises no system notice unless its entry says notices = true.
    const mine = sessions.filter((item) => item.machine === LOCAL_MACHINE || machines.get(item.machine)?.state.notices === true)
    const chats = mine.filter((item) => isChatSession(item.conversation))
    const notices = online
      ? noticesFor(lastStatuses.current, chats, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError }, document.hasFocus() ? focusedId : null)
      : []
    lastStatuses.current = new Map(mine.map((item) => [item.id, item.status]))
    // A card that said "Needs you" goes when the session stops needing you.
    for (const item of sessions) if (item.status !== 'needs' && previousStatuses?.get(item.id) === 'needs') dismissSessionNotice(item.id)
    for (const notice of notices) raiseNotice(notice)
  }, [sessions, online, focusedId, notifyNeedsYou, notifyFinished, notifyError, machines, raiseNotice])

  // A terminal session the daemon says needs you, or finished. Kept in a ref so the one listener below always
  // reads the latest sessions and settings.
  const onAttention = useRef<(attention: Attention) => void>(() => undefined)
  useEffect(() => {
    onAttention.current = (attention) => {
      const target = sessions.find((item) => item.machine === attention.machine && item.pane === attention.term)
      if (!target || isChatSession(target.conversation)) return
      if (target.machine !== LOCAL_MACHINE && machines.get(target.machine)?.state.notices !== true) return
      if (document.hasFocus() && target.id === focusedId) return
      const notice = noticeForAttention(target, attention.status, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError })
      if (!notice) return
      raiseNotice(notice)
      if (notificationSound) playNotificationSound()
    }
  }, [sessions, machines, focusedId, notifyNeedsYou, notifyFinished, notifyError, notificationSound, raiseNotice])

  useEffect(() => {
    if (!isDesktop()) return undefined
    let gone = false
    let stop: (() => void) | null = null
    listenAttention((attention) => { onAttention.current(attention) })
      .then((off) => { if (gone) off(); else stop = off })
      .catch((error: unknown) => { console.warn('Could not listen for alerts from the daemon', error) })
    return () => { gone = true; stop?.() }
  }, [])

  // A browser only lets a tone play once the person has clicked or typed in the page.
  useEffect(() => {
    if (!notificationSound) return
    const enable = () => { enableNotificationSound() }
    window.addEventListener('pointerdown', enable, { once: true })
    window.addEventListener('keydown', enable, { once: true })
    return () => {
      window.removeEventListener('pointerdown', enable)
      window.removeEventListener('keydown', enable)
    }
  }, [notificationSound])

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
      if (other?.status === 'idle' && !isChatSession(other.conversation)) void sendTo(other, data).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not broadcast to a pane')) })
    }
  }, [layout, sessionMap, sendTo])

  const startAgent = useCallback((program: string) => {
    const refusal = remoteStartRefusal(remoteGroup !== null, remoteGroup, `Starting ${program} here`)
    if (refusal) {
      toast.message(refusal)
      return
    }
    setSettingsSection(null)
    runStart(async () => activeGroupKey || resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => {
      setLayout((current) => openInNewTab(current, id, makeId))
      void sendTerminalInput({ id, generation: 0 }, `${program}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${program}`)) })
    })
  }, [runStart, activeGroupKey, remoteGroup, preferences.startFolder, home])

  const paletteItems: PaletteItem[] = useMemo(() => {
    // Thirty-odd items with closures, built only while the palette can show them.
    if (!paletteOpen) return []
    const keys = (shortcut: ShortcutAction) => shortcutText(shortcut, platform)
    const action = (id: string, label: string, run: () => void, shortcut?: string): PaletteItem => ({ id, label, kind: 'Action', run, ...(shortcut ? { shortcut } : {}) })
    const showing = activeTab(layout)
    const items: PaletteItem[] = [
      action('new-here', activeGroupKey ? `New session in ${projectName ?? 'this project'}` : 'New session', newProjectSession, keys('new-session')),
      action('new-start', 'New session in the start folder', newSession),
      action('new-terminal', 'New terminal (a plain shell)', newTerminal),
      ...heads.filter((head) => head.name !== null && head.chip?.tone === 'ok' && online).map((head) => action(`new-on-${head.key}`, `New terminal on ${head.label}…`, () => { newSessionOn(head.machine, head.label) })),
      action('sidebar-side', preferences.sidebarSide === 'left' ? 'Move sidebar to the right' : 'Move sidebar to the left', () => { setPreferences((current) => ({ ...current, sidebarSide: current.sidebarSide === 'left' ? 'right' : 'left' })) }),
      action('split-right', 'Split right', () => { split(null, 'right') }, keys('split-right')),
      action('split-down', 'Split down', () => { split(null, 'down') }, keys('split-down')),
      action('inbox', 'Open the inbox', () => { setInboxOpen(true) }, keys('inbox')),
      action('usage', 'Open usage and cost', () => { setSettingsSection('usage') }, keys('usage')),
      action('jump-needs', 'Jump to the next session that needs you', () => { runShortcut('jump-needs') }, keys('jump-needs')),
      action('inspect', 'Inspect the repository', () => { if (inspectRefusal !== null) toast.message(inspectRefusal); else setInspectOpen(true) }, keys('inspect')),
      ...(focusedSession ? [action('new-worktree', 'New worktree', () => { runShortcut('new-worktree') }, keys('new-worktree'))] : []),
      action('find', 'Find in the pane or every session', () => { setFindOpen(true) }, keys('find')),
      action('broadcast', showing?.broadcast ? 'Stop broadcasting' : 'Broadcast typing to this tab', () => { runShortcut('broadcast') }, keys('broadcast')),
      action('focus', 'Focus mode', () => { setFocusMode(true) }, keys('focus-mode')),
      action('settings', 'Open settings', () => { setSettingsSection('general') }, keys('settings')),
      action('performance', 'Open performance', () => { setSettingsSection('performance') }, keys('performance')),
      action('machines', 'Machines', () => { setSettingsSection('machines') }),
      action('shortcuts', 'Keyboard shortcuts', () => { setShortcutsOpen(true) }, keys('shortcuts')),
      ...(showing?.widths ? [action('reset-widths', 'Reset pane widths', () => { setLayout((current) => setColumnWidths(current, showing.id, null)) })] : []),
      action('start-codex', 'Start codex', () => { startAgent('codex') }),
    ]
    for (const member of sessions) {
      const project = projectOf.get(member.id)
      items.push({
        id: `session-${member.id}`,
        label: `${displayName(member)} · ${member.process}`,
        kind: 'Session',
        ...(project ? { detail: project.name } : {}),
        ...(member.status === 'needs' ? { badge: 'needs you' } : {}),
        run: () => { showSessionById(member.id) },
      })
    }
    for (const group of groups) {
      const target = group.sessions.find((member) => member.id === focusedId) ?? group.sessions[0]
      if (target) items.push({ id: `project-${group.key}`, label: group.name, detail: group.path, kind: 'Project', run: () => { showSessionById(target.id) } })
    }
    for (const candidate of THEMES) items.push({ id: `theme-${candidate.name}`, label: candidate.name, detail: candidate.family, kind: 'Theme', needsQuery: true, run: () => { setPreferences((current) => ({ ...current, themeMode: candidate.light ? 'light' : 'dark', ...(candidate.light ? { lightTheme: candidate.name } : { darkTheme: candidate.name }) })) } })
    return items
  }, [paletteOpen, platform, layout, activeGroupKey, projectName, newProjectSession, newSession, newTerminal, split, runShortcut, startAgent, sessions, projectOf, groups, focusedId, focusedSession, showSessionById, preferences.sidebarSide, inspectRefusal, heads, online, newSessionOn])

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
      machineHeads={heads}
      focusedId={focusedId}
      tracks={tracks}
      mode={mode}
      online={online}
      sessionCount={sessions.length}
      startFolderLabel={startFolderLabel}
      canStart={canStart}
      portalContainer={root}
      onToggleRail={toggleSidebar}
      canExpand={!compact}
      side={preferences.sidebarSide}
      width={sidebarWidth}
      tabOf={tabOf}
      tabPanes={tabPanes}
      openIds={openIds}
      paneOf={paneOf}
      trayStyle={preferences.trayStyle}
      onResize={setLiveSidebarWidth}
      onResizeEnd={(width) => {
        setPreferences((current) => ({ ...current, sidebarWidth: width }))
        setLiveSidebarWidth(null)
      }}
      onNewSession={() => { afterPick(); newSession() }}
      onNewSessionIn={(folder) => { afterPick(); startIn(folder) }}
      onNewTerminal={() => { afterPick(); newTerminal() }}
      onNewTerminalIn={(folder, machineName) => { afterPick(); startTerminalIn(folder, machineName) }}
      onLocalOnly={(machineName) => { afterPick(); explainLocalOnly(machineName) }}
      onNewSessionOn={(machine, name) => { afterPick(); newSessionOn(machine, name) }}
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
        <div ref={setRoot} className={`mx-app${narrow ? ' is-narrow' : ''}`} style={rootStyle} data-motion={reducedMotion ? 'reduced' : 'full'} data-theme-kind={theme.light ? 'light' : 'dark'} data-side={preferences.sidebarSide}>
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
                              canStart={canStart}
                  portalContainer={root}
                  onSelect={selectTabById}
                  onClose={closeTabById}
                  onCloseOthers={closeOtherTabs}
                  onRenameSession={saveName}
                  newTabLabel={projectName ? `New session in ${projectName}` : 'New tab'}
                  onNewTab={newProjectSession}
                  inboxCount={inboxEntries.length}
                  inboxNeeds={needsCount > 0}
                  onOpenInbox={openInbox}
                  inspect={inspect}
                  onToggleInspect={toggleInspect}
                  foldPanels={narrow}
                  {...(compact ? { onOpenSessions: openDrawer } : {})}
                />}
                {findOpen && (
                  <FindBar
                    online={online}
                    otherMachines={heads.length > 0}
                    findInPane={(query, direction) => (focusedId ? terminalHandles.current.get(focusedId)?.find(query, direction) ?? false : false)}
                    clearPane={() => { if (focusedId) terminalHandles.current.get(focusedId)?.clearFind() }}
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
                          const chat = chatOf(session)
                          const chatOnly = isChatSession(session.conversation)
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
                              view={chat.view}
                              {...(chatOnly ? {} : { onViewChange: (view: PaneView) => { paneViews.choose(session.id, view) } })}
                              chat={chat.view === 'chat' ? (
                                <ChatPane
                                  availability={chat.availability}
                                  hasTerminal={!chatOnly}
                                  root={session.repoRoot ?? session.cwd}
                                  onOpenDiff={showChanges}
                                  {...(chatOnly ? {} : { typeInto: (text: string) => sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, text) })}
                                  ask={session.ask}
                                  sessionId={session.id}
                                  paneStatus={session.status}
                                  process={session.process}
                                  online={online}
                                  onBackToTerminal={() => { paneViews.choose(session.id, 'terminal') }}
                                  onStartAgent={(agent) => {
                                    paneViews.choose(session.id, 'terminal')
                                    void sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${agent}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${agent}`)) })
                                  }}
                                  defaultFullAccess={preferences.chatFullAccessDefault}
                                  onStartChat={(agent, fullAccess) => { startChatHere(session, agent, fullAccess) }}
                                />
                              ) : undefined}
                              online={online}
                              portalContainer={root}
                              terminal={chatOnly ? <NoTerminalNote /> :
                                <TerminalView
                                  key={session.id}
                                  conversation={session.conversation}
                                  desktop
                                  fontSize={preferences.terminalFontSize}
                                  look={look}
                                  label={`Shell in ${session.cwdLabel}`}
                                  inputDisabled={!online}
                                  ended={session.ended}
                                  focused={focused && !modalOpen && chat.view === 'terminal'}
                                  onInput={(data) => { mirrorInput(session.id, data) }}
                                  onOpenPath={(path, line) => { openPath(session, path, line) }}
                                  onPasteImage={(image) => { pasteImage(session.id, image) }}
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
            {shortcutsOpen && <ShortcutOverlay platform={platform} portalContainer={root} onClose={() => { setShortcutsOpen(false) }} />}
            {welcomeOpen && (
              <WelcomeDialog
                platform={platform}
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
            {remoteStart !== null && (
              <RemoteSessionDialog
                machineName={remoteStart.name}
                list={listOnRemote}
                recent={recentOnRemote}
                onStart={(folder) => startOnMachine(remoteStart, folder)}
                onCancel={() => { setRemoteStart(null) }}
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
                platform={platform}
                daemon={{ online, sessionCount: sessions.length }}
                appVersion={runningVersion}
                machines={{ known: machinesKnown, list: heads.filter((head) => head.name !== null), onRetry: retryNamed, onRemove: removeNamed, onGrant: grantNamed, onCopyCommand: copyCommand, add: { windows: navigator.userAgent.includes('Windows'), loadHosts: listSshHosts, onAdd: addNamed, onCancel: cancelAdding } }}
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
                <span>Reopening {layout.tabs.length === 1 ? '1 tab' : `${String(layout.tabs.length)} tabs`}. Your sessions never stopped.</span>
              </div>
            )}
          </div>
          <Toaster position={preferences.toastPosition} theme={theme.light ? 'light' : 'dark'} />
          <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">{machineSaid}</div>
        </div>
      </EffectsContext.Provider>
    </ReducedMotionContext.Provider>
  )
}

