import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { Panel } from '@/components/ui/panel'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from '@/lib/toast'
import { z } from 'zod'
import {
  closeChatSession,
  closeSessionWithUndo,
  markSeenOnDaemon,
  defaultSessionFolder,
  conversationsFromSnapshot,
  isDesktop,
  listenSessionsChanged,
  loadConversations,
  loadDaemonRestartStatus,
  openDaemonLog,
  renameConversationOnDaemon,
  restartMismatchedDaemon,
  sendTerminalInput,
  submitMessage,
  startChatSession,
  addWorktree,
  switchBranch,
  startAgentSession as launchAgentSession,
  startShellSession,
  undoCloseSession,
} from '@/api/daemon'
import { LOCAL_MACHINE, addMachine, cancelAddMachine, listSshHosts, listenAttention, listenMachineClipboard, listenMachineError, listenMachineNotice, listenMachineRemoved, listenMachineSessions, listenMachineState, listenMachineUpdateProgress, loadMachinesOverview, machineActivity, removeMachine, retryMachine, setMachineGrant, startSessionOnMachine, terminalOf, unregisterMachine, type Attention, type Grant, type MachineState } from '@/api/machines'
import { TitleBar } from '@/components/TitleBar'
import { Toaster } from '@/components/ui/sonner'
import { markConversationsDisconnected } from '@/data/connection'
import { DesktopSettings, type DesktopSettingsSection, type UsageInfo } from '@/features/settings/components/DesktopSettings'
import { readSettingsPreferences, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN, writeSettingsPreferences, type SettingsPreferences } from '@/features/settings/preferences'
import type { Conversation } from '@/data/preview'
import { sizeVariables } from './model/layoutSize'
import { serviceStateOf } from './model/serviceState'
import { daemonConnectionError, daemonProtocolMismatch } from '@/lib/daemon-connection'
import { errorMessage } from '@/lib/error-message'
import { useLatest } from '@/lib/useLatest'
import { guardLocal } from '@/lib/localOnly'
import { markGpu } from '@/lib/gpu'
import { isWindowShown, subscribeWindowActivity } from '@/lib/window-activity'
import { PerfOverlay } from './components/PerfOverlay'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { TerminalView } from '@/TerminalView'
import { ReducedMotionContext } from './components/motion'
import { SplitHandle } from './components/SplitHandle'
import { CommandPalette } from './components/CommandPalette'
import { InboxPanel, type InboxEntry } from './components/InboxPanel'
import { InspectPanel, type InspectView } from '@/features/inspect/InspectPanel'
import { runsProgram } from './components/runsProgram'
import { rememberSessions, withEnded } from './model/ended'
import { isChatSession, isListedChatSession } from './model/chatSession'
import { advanceLoginWatch, LOGIN_CHECK_MS, startLoginWatch, type LoginWatch } from './model/loginWatch'
import { announcementFor, idsOfMachinesNotHeard, LOCAL_HEAD, machineHeads, remoteStartRefusal, remoteTargetOf, remoteTerminals, type RemoteMachine, type UpdateAction } from './model/machines'
import { HoverTips } from './components/HoverTips'
import { ShortcutOverlay } from '@/features/shortcuts/ShortcutOverlay'
import { StatusBar, type FocusMachine } from './components/StatusBar'
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
import { indexRemoteFiles, listRemoteFolder } from '@/api/fileGets'
import { useFileGets } from '@/features/terminal/useFileGets'
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
import { PaneCard, type PaneTool } from './components/PaneCard'
import { registerDragHandlers } from './components/dragSession'
import { ChatPane } from '@/features/chat/components/ChatPane'
import { ANSWER_KEYS } from '@/features/chat/permissionAsk'
import { NoTerminalNote } from '@/features/chat/components/NoTerminalNote'
import { rememberConversationPermissions } from '@/data/conversationPermissions'
import { reserveOperation } from '@/features/chat/sendOperation'
import { holdFirstMessage } from '@/features/chat/firstMessage'
import { connectAgentOnce } from './model/agentHooks'
import { machineProjects, nameFromMessage } from './model/newSession'
import { usePaneViews } from './usePaneViews'
import { chatAvailability, chatSourceOf, effectiveView, type ChatAgent, type PaneView, type StructuredAgent } from './model/chatView'
import { NewSessionHost } from './components/NewSessionHost'
import type { NewSessionRequest } from './components/NewSessionModal'
import { NewChatPage, type NewChatRequest } from './components/NewChatPage'
import { type WireChoice } from '@/features/chat/agentChoice'
import { launchFlags, yoloFlags } from './model/newChatHost'
import { GetFileDialog } from './components/GetFileDialog'
import { MachineUpdateDialog } from './components/MachineUpdateDialog'
import { runMachineUpdate } from './machineUpdate'
import { ConfirmCloseDialog } from './components/ConfirmCloseDialog'
import { movePanes } from './components/paneTransition'
import { RenameDialog } from './components/RenameDialog'
import { Sidebar, type OpenSubagent } from './components/Sidebar'
import { EmptyState, Toast, type RestartOffer } from './components/states'
import { ServiceStatus } from './components/ServiceStatus'
import { useSharedSettings } from '@/features/settings/sharedSettings'
import {
  activeArrangement,
  dropSession,
  cycleFocus,
  EMPTY_LAYOUT,
  focusedSessionId,
  focusPane,
  MAX_PANES,
  openArrangement,
  paneColumns,
  reconcileLayout,
  removeSessions,
  setColumnWidths,
  setPaneHeights,
  toggleBroadcast,
  showSession,
  splitPane,
  swapSessions,
  toggleMaximize,
  visibleSessionIds,
  type Arrangement,
  type Layout,
  type MakeId,
  type PaneSlot,
  type SplitDirection,
} from './model/layout'
import { LAYOUT_RESET_NOTICE, readLayout, writeLayout } from './model/layoutStorage'
import { byPriority } from './model/inboxOrder'
import { displayName, folderNameOf, inCreationOrder, tildePath, toSessionView, type SessionView } from './model/session'
import { byUrgency, isShell, rowTitle } from './model/sidebar'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { topmostOpen, type EscLayer } from './model/escLadder'
import { jumpToast, NOTHING_NEEDS_YOU, nextNeedsYou } from '@/features/needs-you/model/needsYou'
import { bindingFor, detectPlatform, matchShortcut, shortcutChords, shortcutText, type ShortcutAction } from './model/shortcuts'
import { folderExists, resolveStartFolder } from './model/startFolder'
import { markSeen, trackSessions, tracksEqual, type SessionTrack } from './model/tracking'
import type { PaletteItem } from './model/palette'
import { activeThemeName, findTheme, terminalMinimumContrast, themeStyle, THEMES, xtermTheme } from './theme/themes'
import { finishOf, withWindowStyle } from './theme/windowStyles'
import { COMPACT_WIDTH_PX, NARROW_WIDTH_PX } from './model/windowSize'
import './workspace.css'
import './components/sidebar.css'
import './components/pane.css'
import './components/menu.css'
import './components/dialogs.css'
import '@/features/chat/chat.css'

/** The daemon pushes every change (`sessions-changed`); this slow poll only catches a stream that went quiet. */
const SESSION_REFRESH_MS = 15_000
/** How long a new session on another machine is waited for, and how often its list is looked at. */
const REMOTE_START_WAIT_MS = 8000
/** How long a session started as an agent is taken to be it while the daemon has not said so; a command that never starts one stops claiming it. */
/** Whether a session can show a chat at all: an agent the app reads. A shell or another program has none, so it gets no switch. */
function hasChat(availability: ReturnType<typeof chatAvailability>): boolean {
  return availability.kind === 'ready' || availability.kind === 'no-events'
}

const EXPECTED_AGENT_MS = 30_000
function omitKey<T>(record: Record<string, T>, key: string): Record<string, T> {
  return Object.fromEntries(Object.entries(record).filter(([name]) => name !== key))
}
const REMOTE_START_POLL_MS = 250
const TOAST_MS = 2600
/** How long a closed session can be brought back; the daemon keeps its process running this long. */
const UNDO_CLOSE_MS = 6000
/** What clicking the status bar's size resets the terminal text to. */
const DEFAULT_TEXT_SIZE = 14
/** How long a closing pane shrinks before its session ends (matches `.mx-pane-cell.is-closing` in pane.css), and how long it is kept marked if the close did not happen. */
const PANE_EXIT_MS = 240
const PANE_CLOSE_SETTLE_MS = 1500
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
/** Ctrl+L: a shell clears its screen and keeps what was typed. */
const CLEAR_SCREEN = '\x0c'

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
  const [tracks, setTracks] = useState<ReadonlyMap<string, SessionTrack>>(new Map())
  const [dragId, setDragId] = useState<string | null>(null)
  const [dropId, setDropId] = useState<string | null>(null)
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
  // Sessions started as a known agent, until the daemon says so itself (see `startAgentSession`).
  const [expectedAgents, setExpectedAgents] = useState<Record<string, ChatAgent>>({})
  const everySession = useMemo(() => {
    // Terminal sessions, and the chat sessions the daemon owns that are not put away (ADR 0003).
    const terminals = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal' || isListedChatSession(item))
    const here = online ? terminals : markConversationsDisconnected(terminals)
    return [...here, ...remoteTerminals(machines)].map((conversation) => {
      const expected = expectedAgents[conversation.id]
      return toSessionView(expected !== undefined && !conversation.agent ? { ...conversation, agent: expected } : conversation, home)
    })
  }, [sessionsQuery.data, online, home, machines, expectedAgents])
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
  const focusedSession = focusedId ? sessionMap.get(focusedId) : undefined
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
  const heads = useMemo(() => machineHeads(machines), [machines])
  // The machine of the pane in front, named in the status bar once there is more than one machine to tell apart.
  const focusedMachine = focusedSession?.machine
  const focusMachine = useMemo<FocusMachine | null>(() => {
    if (heads.length === 0 || focusedMachine === undefined) return null
    if (focusedMachine === LOCAL_MACHINE) return { name: 'This computer', word: 'local', tone: 'ok' }
    const head = heads.find((candidate) => candidate.machine === focusedMachine)
    if (head === undefined) return null
    return { name: head.name ?? head.label, word: head.chip?.word ?? 'unknown', tone: head.chip?.tone ?? 'quiet' }
  }, [heads, focusedMachine])
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
  // The restarted daemon is dialled again once the update is in: the state moves on by itself.
  // One update per machine at a time: two would race to replace the same file there.
  const updatingRef = useRef<Set<string>>(new Set())
  // How far each machine's update has got. A machine is in it from the moment its update starts until it ends.
  const [updateProgress, setUpdateProgress] = useState<Readonly<Record<string, number>>>({})
  useEffect(() => {
    if (!isDesktop()) return undefined
    let stopped = false
    let unlisten: (() => void) | undefined
    listenMachineUpdateProgress((progress) => {
      setUpdateProgress((current) => (progress.name in current ? { ...current, [progress.name]: progress.percent } : current))
    }).then((stop) => { if (stopped) stop(); else unlisten = stop }, (error: unknown) => { console.warn('Could not listen for update progress', error) })
    return () => { stopped = true; unlisten?.() }
  }, [])
  const updateNamed = useCallback(async (name: string, action: UpdateAction, whenIdle: boolean) => {
    if (updatingRef.current.has(name)) {
      toast.message(`${name} is already being updated.`)
      return
    }
    updatingRef.current.add(name)
    setUpdateProgress((current) => ({ ...current, [name]: 0 }))
    try {
      await runMachineUpdate(name, action, whenIdle, () => { retryNamed(name) })
    } finally {
      updatingRef.current.delete(name)
      setUpdateProgress((current) => Object.fromEntries(Object.entries(current).filter(([key]) => key !== name)))
    }
  }, [retryNamed])
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
  // The sidebar is one flat list in creation order; a status change, a pin or a focus never moves a row.
  const ordered = useMemo(() => inCreationOrder(sessions), [sessions])
  // A session the daemon lists a moment before its pane exists would show in the sidebar and then jump once placed.
  const sidebarSessions = useMemo(() => {
    if (startedFrom === null) return ordered
    return ordered.filter((session) => startedFrom.has(session.id) || openIds.has(session.id))
  }, [ordered, startedFrom, openIds])

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
  useEffect(() => registerDragHandlers({
    start: (source) => { if (source.kind === 'pane') setDragId(source.id) },
    over: setDropId,
    drop: (source, paneId) => {
      if (!paneId) return
      if (source.kind === 'session') {
        setLayout((current) => dropSession(current, paneId, source.id))
        flash('Session moved into this pane')
      } else if (source.id !== paneId) movePanes(() => { setLayout((current) => swapSessions(current, source.id, paneId)) })
    },
    end: () => { setDragId(null); setDropId(null) },
  }), [flash])
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
    const mark = () => { document.documentElement.dataset.window = isWindowShown() ? 'active' : 'inactive' }
    mark()
    return subscribeWindowActivity(mark)
  }, [])

  useEffect(() => {
    document.documentElement.dataset.style = preferences.style
    document.documentElement.dataset.finish = finishOf(preferences.style)
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
    const askAgain = () => {
      void queryClient.invalidateQueries({ queryKey: ['conversations'] })
    }
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
      const reconciled = openFirst ? reconcileLayout(current, live) : current
      return openFirst && reconciled.arrangements.length === 0 && first ? openArrangement(reconciled, first.id, makeId) : reconciled
    })
  }, [sessions, sessionsQuery.data, sessionsQuery.isSuccess])

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

  const startShell = useCallback(async (folder: string, launch?: (folder: string) => Promise<void>): Promise<string | null> => {
    const known = new Set(sessions.map((session) => session.id))
    await (launch ?? startShellSession)(folder)
    return newSessionSince(known)
  }, [sessions, newSessionSince])

  const createWorktree = useCallback(async (from: SessionView, branch: string, base: string | null) => {
    const known = new Set(sessions.map((session) => session.id))
    await newWorktreeSession(from.pane, branch, base)
    const id = await newSessionSince(known)
    setWorktreeFrom(null)
    if (id) setLayout((current) => openArrangement(current, id, makeId))
    flash(`Worktree ${branch} ready`)
  }, [sessions, newSessionSince, flash])

  // Lists again and opens the chat session the daemon just started, with its first prompt if there is one.
  const openStartedChat = useCallback(async (started: { id: number; generation: number }, prompt: string, choice: WireChoice | null = null) => {
    const id = String(started.id)
    await queryClient.invalidateQueries({ queryKey: ['conversations'] })
    await queryClient.query({ queryKey: ['conversations'], queryFn: loadConversations, staleTime: 0 })
    setLayout((current) => openArrangement(current, id, makeId))
    if (prompt !== '') {
      // The start has answered, so the agent is up: the first prompt is an ordinary send, number 1.
      const operation = reserveOperation(started.id, { receipt: null, queued: [] })
      await submitMessage({ id, generation: started.generation }, operation, prompt, choice ?? undefined)
    }
  }, [queryClient])

  // A chat session in `path`, opened in a layout of its own. Chats are this computer's: another machine's shell asks first.
  const startChat = useCallback((path: string, agent: StructuredAgent, fullAccess: boolean) =>
    startChatSession(path, agent, null, fullAccess).then((started) => {
      if (fullAccess) rememberConversationPermissions(String(started.id), 'full-access')
      return openStartedChat(started, '')
    }).then(() => { flash(`${agent} chat started`) }), [openStartedChat, flash])


  // What the new chat page asks for: a structured chat in the folder, with the model and effort picked, and its first message.
  // Choosing another session (the sidebar, a shortcut) leaves the new chat page for it.
  const focusedSessionKey = focusedSession?.id
  const [seenFocusKey, setSeenFocusKey] = useState(focusedSessionKey)
  if (seenFocusKey !== focusedSessionKey) {
    setSeenFocusKey(focusedSessionKey)
    setNewChatOpen(false)
  }

  // Resolves once the session is placed (true) or the start did not happen or failed (false).
  const runStart = useCallback((folder: () => Promise<string>, place: (sessionId: string) => void, launch?: (folder: string) => Promise<void>): Promise<boolean> => {
    if (!online || starting) return Promise.resolve(false)
    setStarting(true)
    setStartedFrom(new Set(sessionMap.keys()))
    return folder()
      .then((path) => startShell(path, launch))
      .then((id) => {
        if (!id) return false
        place(id)
        return true
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not start a shell'))
        return false
      })
      .finally(() => { setStarting(false); setStartedFrom(null) })
  }, [online, starting, startShell, sessionMap])

  // A new session is the default agent, running in a terminal the daemon owns, shown as its chat or
  // as its terminal as Settings says. It is one session either way: the view can be switched at any
  // time without restarting the agent (ADR 0003). A plain shell is `runStart` and has no chat view.
  // The agent is the new session's first command, so it starts as soon as the shell does instead of being typed in once the
  // shell is ready. Its status hooks are connected first: an agent already running never loads them, and its chat would
  // wait on them until it was restarted.
  const yolo = useMemo(() => ({ claude: preferences.yoloClaude, codex: preferences.yoloCodex, gemini: preferences.yoloGemini }), [preferences.yoloClaude, preferences.yoloCodex, preferences.yoloGemini])
  const launchAgent = useCallback((agent: ChatAgent, flags: string) => async (folder: string): Promise<void> => {
    await connectAgentOnce(agent).catch((error: unknown) => { console.warn(`Could not connect ${agent}'s status hooks`, error) })
    await launchAgentSession(folder, agent, `${flags}${yoloFlags(agent, yolo)}`.split(' ').filter((word) => word !== ''))
  }, [yolo])

  const startAgentSession = useCallback((folder: () => Promise<string>, agent: ChatAgent, view: PaneView, firstMessage = '', flags = '', afterPlace?: () => void): Promise<boolean> => {
    return runStart(folder, (id) => {
      // A first message waits under the session and goes out once the agent takes input; its words name the session.
      if (firstMessage !== '') {
        holdFirstMessage(id, firstMessage)
        void renameConversationOnDaemon({ id }, nameFromMessage(firstMessage)).catch((error: unknown) => { console.warn('Could not name the new session', error) })
      }
      // The daemon learns which agent runs from the agent's own hook, a moment after it starts; until then the
      // session is known to be this agent, so it never shows as a shell.
      setExpectedAgents((current) => ({ ...current, [id]: agent }))
      window.setTimeout(() => { setExpectedAgents((current) => omitKey(current, id)) }, EXPECTED_AGENT_MS)
      setLayout((current) => openArrangement(current, id, makeId))
      if (view === 'chat') paneViews.choose(id, 'chat')
      // In the same update as the new session opens, so the window never shows the page between the two.
      afterPlace?.()
    }, launchAgent(agent, flags))
  }, [runStart, paneViews, launchAgent])

  // "Start a Claude chat" on a shell's empty chat card: a chat session in that shell's folder.
  const startChatHere = useCallback((from: SessionView, agent: ChatAgent, fullAccess: boolean) => {
    const refusal = remoteStartRefusal(from.machine !== LOCAL_MACHINE, from.machineName, 'A chat session')
    if (refusal) {
      toast.message(refusal)
      return
    }
    if (!online || starting) return
    // Gemini has no structured chat: it starts as a terminal session shown as its chat.
    if (agent === 'gemini') {
      void startAgentSession(() => Promise.resolve(from.cwd), agent, 'chat')
      return
    }
    setStarting(true)
    startChat(from.cwd, agent, fullAccess)
      .catch((error: unknown) => { toast.error(errorMessage(error, `Could not start a ${agent} chat`)) })
      .finally(() => { setStarting(false) })
  }, [online, starting, startChat, startAgentSession])

  // The new chat is the agent in a terminal the daemon owns, shown as its chat: one session whose view
  // switches to the terminal at any time (ADR 0003), the same for every agent.
  const startNewChat = useCallback(async (request: NewChatRequest): Promise<void> => {
    if (!online) throw new Error('The daemon is not reachable')
    // The branch is settled before the agent starts, and a failure stays on the page with its words.
    let folder = request.folder
    if (request.branch?.worktree) folder = await addWorktree(folder, request.branch.name, request.branch.create)
    else if (request.branch) await switchBranch(folder, request.branch.name, request.branch.create)
    // The page stays, with the message in it, until the chat that replaces it is there: closing it first showed the
    // empty window (or the last chat) for a moment between the two.
    await startAgentSession(() => Promise.resolve(folder), request.agent, 'chat', request.message, launchFlags(request.agent, request.model, request.reasoning), () => { setNewChatOpen(false) })
  }, [online, startAgentSession])

  // New session (the shortcut, the palette, the sidebar's chat button): on this computer it is the new chat page.
  // Chats are this computer's, so another machine's session is asked for in the dialog, on that machine.
  const newSession = useCallback((machine?: number) => {
    setSettingsSection(null)
    const target = machine ?? remoteTargetOf(focusedSession)?.machine ?? LOCAL_MACHINE
    if (target === LOCAL_MACHINE) {
      setNewChatOpen(true)
      return
    }
    setNewSessionMachine(target)
  }, [focusedSession])

  // A plain shell in the start folder: terminal view only, whatever the default agent is.
  // Another machine asks for a folder there first, so it opens the modal.
  const newTerminal = useCallback((machine?: number) => {
    setSettingsSection(null)
    const target = machine ?? remoteTargetOf(focusedSession)?.machine ?? LOCAL_MACHINE
    if (target !== LOCAL_MACHINE) {
      setNewSessionMachine(target)
      return
    }
    void runStart(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => { setLayout((current) => openArrangement(current, id, makeId)) })
  }, [runStart, preferences.startFolder, home, focusedSession])

  // The sidebar's header for a machine and the palette both choose where the session goes.
  const newSessionOn = useCallback((machine: number) => {
    setSettingsSection(null)
    setNewSessionMachine(machine)
  }, [])

  // Starts a shell on another machine and opens it once the machine lists it; resolves to its id.
  const startOnMachine = useCallback(async (target: { machine: number; name: string }, folder: string): Promise<string | null> => {
    const known = new Set((machinesRef.current.get(target.machine)?.conversations ?? []).map((item) => item.id))
    await startSessionOnMachine(target.machine, folder)
    // The machine tells its new session in its next list; its layout opens when it does.
    const deadline = Date.now() + REMOTE_START_WAIT_MS
    while (Date.now() < deadline) {
      const fresh = machinesRef.current.get(target.machine)?.conversations.find((item) => item.kind === 'terminal' && !known.has(item.id))
      if (fresh) {
        setLayout((current) => openArrangement(current, fresh.id, makeId))
        return fresh.id
      }
      await new Promise((resolve) => { setTimeout(resolve, REMOTE_START_POLL_MS) })
    }
    toast.error(`${target.name} did not report the new session`)
    return null
  }, [])

  // What the New session modal asks for: a shell or an agent, here or on another machine. The modal closes once it is started.
  const startFromModal = useCallback(async (request: NewSessionRequest) => {
    if (request.machine !== LOCAL_MACHINE) {
      const id = await startOnMachine({ machine: request.machine, name: request.machineName }, request.folder)
      if (id !== null && request.startAs !== 'terminal') {
        if (request.message !== '') holdFirstMessage(id, request.message)
        await sendTerminalInput({ id, generation: 0 }, `${request.startAs}\r`)
      }
    } else if (request.startAs === 'terminal') {
      void runStart(() => Promise.resolve(request.folder), (id) => { setLayout((current) => openArrangement(current, id, makeId)) })
    } else {
      void startAgentSession(() => Promise.resolve(request.folder), request.startAs, preferences.newSessionView, request.message)
    }
    setNewSessionMachine(null)
  }, [startOnMachine, runStart, startAgentSession, preferences.newSessionView])

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
      // If the layout changed or filled up while the shell started, the shell still gets a layout: it must not be left with no pane.
      setLayout((current) => {
        const next = splitPane(current, source?.id ?? null, direction, id, makeId)
        return next === current ? openArrangement(current, id, makeId) : next
      })
    })
  }, [layout, sessionMap, runStart, preferences.startFolder, home])

  // Closing a pane or session ends it, with a few seconds to take it back: the daemon hides the session and
  // stops its program only when the window passes. A session that is still running asks first, in a dialog.
  const [closeAsk, setCloseAsk] = useState<{ title: string; body: string; confirmLabel: string; run: () => void } | null>(null)
  // The last close, while it can still be undone: the layout it came out of is put back with it.
  const [lastClose, setLastClose] = useState<{ toastId: string | number; sessions: SessionView[]; before: Layout; after: Layout } | null>(null)
  const undoLastClose = useCallback(() => {
    const closed = lastClose
    if (closed === null) return
    setLastClose(null)
    toast.dismiss(closed.toastId)
    void Promise.all(closed.sessions.map((session) => undoCloseSession(session.conversation)))
      .then(async () => {
        setLayout((current) => (current === closed.after ? closed.before : closed.sessions.reduce((layoutNow, session) => showSession(layoutNow, session.id, makeId), current)))
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not bring the session back')) })
  }, [queryClient, lastClose])
  // The toast's button outlives the render that made it, so it calls whichever undo is current.
  const undoNow = useLatest(undoLastClose)
  const endSessions = useCallback((ids: string[], what: string) => {
    const gone = new Set(ids)
    const running = sessions.filter((session) => gone.has(session.id))
    // Panes of sessions that already ended have nothing to stop: they just close.
    if (running.length === 0) {
      setLayout((current) => removeSessions(current, gone))
      return
    }
    // A chat session has no terminal to hide: it is ended for good, as before. Terminals get the undo window.
    const chats = running.filter((session) => isChatSession(session.conversation))
    const terminals = running.filter((session) => !isChatSession(session.conversation))
    void Promise.all([
      ...chats.map((session) => closeChatSession(session.conversation)),
      ...terminals.map((session) => closeSessionWithUndo(session.conversation)),
    ])
      .then(async () => {
        const before = layout
        const after = removeSessions(before, gone)
        setLayout(after)
        if (terminals.length > 0) {
          const label = terminals.length === 1 && terminals[0] ? displayName(terminals[0]) : `${String(terminals.length)} sessions`
          const toastId = toast(`Closed ${label}`, { duration: UNDO_CLOSE_MS, action: { label: 'Undo', onClick: undoNow } })
          setLastClose({ toastId, sessions: terminals, before, after })
          window.setTimeout(() => { setLastClose((current) => (current?.toastId === toastId ? null : current)) }, UNDO_CLOSE_MS)
        } else {
          flash(`${what} closed · session ended`)
        }
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not end the session')) })
  }, [sessions, flash, queryClient, layout, undoNow])

  /** Runs `close` now when every session in `ids` has already ended; otherwise asks first. */
  const askToClose = useCallback((ids: string[], close: () => void) => {
    const live = ids.flatMap((id) => paneSessions.get(id) ?? []).filter((session) => !session.ended)
    const first = live[0]
    if (!first) {
      close()
      return
    }
    const only = live.length === 1
    const name = only ? displayName(first) : `${String(live.length)} sessions`
    setCloseAsk({
      title: `Close ${name}?`,
      body: only && runsProgram(first)
        ? `This stops ${first.process}. You can undo it for a few seconds afterwards.`
        : 'This ends the session. You can undo it for a few seconds afterwards.',
      confirmLabel: runsProgram(first) ? `Stop ${first.process}` : 'Close',
      run: close,
    })
  }, [paneSessions])

  // A pane that is closing shrinks away first (its neighbours grow into the room as it goes); the session ends
  // when that is done. If the close does not happen it grows back.
  const [closingPanes, setClosingPanes] = useState<ReadonlySet<string>>(() => new Set())
  const closePaneNow = useCallback((paneId: string) => {
    const tab = layout.arrangements.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
    const pane = tab?.panes.find((candidate) => candidate.id === paneId)
    if (!tab || !pane) return
    if (tab.panes.length <= 1) {
      endSessions([pane.sessionId], 'Pane')
      return
    }
    setClosingPanes((current) => new Set(current).add(paneId))
    window.setTimeout(() => {
      endSessions([pane.sessionId], 'Pane')
      window.setTimeout(() => {
        setClosingPanes((current) => {
          const next = new Set(current)
          next.delete(paneId)
          return next
        })
      }, PANE_CLOSE_SETTLE_MS)
    }, PANE_EXIT_MS)
  }, [endSessions, layout])
  const closePaneById = useCallback((paneId: string) => {
    const pane = layout.arrangements.flatMap((tab) => tab.panes).find((candidate) => candidate.id === paneId)
    if (!pane) return
    askToClose([pane.sessionId], () => { closePaneNow(paneId) })
  }, [layout, askToClose, closePaneNow])

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

  const saveName = useCallback((sessionId: string, name: string) => {
    setRenameId(null)
    void renameConversationOnDaemon({ id: sessionId }, name)
      .then(async () => { await queryClient.invalidateQueries({ queryKey: ['conversations'] }) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not rename the session')) })
  }, [queryClient])

  // Showing a session shows its own chat: a subagent's chat is left by picking any session, its parent too.
  const showSessionById = useCallback((sessionId: string) => {
    setOpenSubagent(null)
    setLayout((current) => showSession(current, sessionId, makeId))
  }, [])

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
    setLayout((current) => splitPane(current, null, direction, sessionId, makeId))
  }, [layout, showSessionById])

  const copyText = useCallback((text: string, what: string) => {
    navigator.clipboard.writeText(text).then(
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
    setLayout((current) => showSession(current, sessionId, makeId))
    paneViews.choose(sessionId, 'chat')
    setOpenSubagent({ sessionId, subagentId })
  }, [paneViews])

  const splitSubagent = useCallback((sessionId: string, subagentId: string) => {
    openInSplit(sessionId, 'right')
    paneViews.choose(sessionId, 'chat')
    setOpenSubagent({ sessionId, subagentId })
  }, [openInSplit, paneViews])

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

  const runShortcut = useCallback((action: ShortcutAction) => {
    switch (action) {
      case 'new-session':
        newSession(focusedMachineNumber)
        return
      case 'new-terminal':
        newTerminal(focusedMachineNumber)
        return
      case 'new-agent-chat':
        // A chat with the default agent in the start folder, without the dialog. On another machine the dialog asks for its folder.
        setSettingsSection(null)
        if (focusedMachineNumber !== LOCAL_MACHINE) newSession(focusedMachineNumber)
        else void startAgentSession(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), preferences.defaultAgent, 'chat')
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
        const tab = activeArrangement(layout)
        if (!tab) return
        const single = tab.panes.length > 1
        const targets = single ? tab.panes.filter((pane) => pane.id === tab.focus) : tab.panes
        const ids = targets.map((pane) => pane.sessionId)
        askToClose(ids, () => { endSessions(ids, single ? 'Pane' : 'Session') })
        return
      }
      case 'toggle-sidebar':
        toggleSidebar()
        return
      case 'settings':
        setSettingsSection((current) => current ?? 'appearance')
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
      case 'performance':
        setSettingsSection('performance')
        return
      case 'undo-close':
        undoLastClose()
        return
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
        if (layout.activeId) {
          const turningOn = !activeArrangement(layout)?.broadcast
          setLayout((current) => toggleBroadcast(current, current.activeId ?? ''))
          flash(turningOn ? 'Broadcast on · typing goes to every idle pane in this tab' : 'Broadcast off')
        }
        return
    }
  }, [newSession, newTerminal, focusedMachineNumber, startAgentSession, preferences.startFolder, preferences.defaultAgent, home, split, layout, flash, toggleSidebar, focusedSession, online, preferences.terminalFontSize, setTextSize, endSessions, askToClose, toggleInspect, sessions, showSessionById, chatOf, paneViews, undoLastClose])

  // Captured on the window before xterm sees the key, so only these few keys leave the shell.
  const overlayOpen = paletteOpen || inboxOpen
  const drawerShown = compact && drawerOpen
  const sidebarMode = compact || preferences.compactSidebar ? 'rail' : 'docked'
  // A name is typed in its row where the full sidebar shows; the rail and Focus mode ask in a dialog.
  const renameInline = !focusMode && (sidebarMode === 'docked' || drawerShown)
  const dialogOpen = settingsSection !== null || (renameId !== null && !renameInline) || getFileId !== null || newSessionMachine !== null || newChatOpen
  const modalOpen = dialogOpen || overlayOpen
  const shortcutState = useRef({ runShortcut, canUndoClose: () => lastClose !== null, settingsOpen: dialogOpen, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen })
  useEffect(() => { shortcutState.current = { runShortcut, canUndoClose: () => lastClose !== null, settingsOpen: dialogOpen, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen } })
  useEffect(() => {
    // How each layer of the Esc ladder closes. A new dialog adds its closer here and its id to ESC_LADDER.
    const closers: Partial<Record<EscLayer, () => void>> = {
      palette: () => { setPaletteOpen(false) },
      inbox: () => { setInboxOpen(false) },
      // The new chat page counts as a dialog here, so this layer is the one that closes it too.
      settings: () => { setSettingsSection(null); setNewChatOpen(false) },
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
      // Undo is the field's own inside a text field, and the terminal's own when nothing was just closed.
      if (match.action === 'undo-close' && (!state.canUndoClose() || (event.target instanceof Element && event.target.closest('input, textarea, [contenteditable="true"]')))) return
      event.preventDefault()
      event.stopPropagation()
      state.runShortcut(match.action)
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => { window.removeEventListener('keydown', onKeyDown, true) }
  }, [platform])

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
    enabled: inspectTerm !== null,
    // The counts come from a full diff, so they are read slowly unless Inspect is open, and not
    // again just because the window came to the front.
    refetchInterval: inspectOpen ? DIFF_STAT_REFRESH_MS : DIFF_STAT_IDLE_MS,
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

  const answerSession = useCallback((sessionId: string, answer: 'y' | 'a' | 'n') => {
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
    // Never while reconnecting, with the service down or a restart needed: the status itself is the message, and a card could not be answered.
    if (!online) return
    if (document.hasFocus()) {
      const target = sessionMap.get(notice.sessionId)
      const card = target && sessionNoticeFor(target, projectOf.get(target.id), {
        open: () => { showSessionById(notice.sessionId) },
        // A chat session's approval is answered in its chat, by the request's id, never by typing a letter.
        allow: !isChatSession(target.conversation) ? () => { answerSession(notice.sessionId, 'y') } : null,
      })
      if (card) showSessionNotice(notice.sessionId, card)
      return
    }
    lastNotified.current = { sessionId: notice.sessionId, at: Date.now() }
    sendNativeNotice(notice.title, notice.body).catch((error: unknown) => { console.warn('Could not show a system notification', error) })
  }, [sessionMap, projectOf, showSessionById, online, answerSession])

  const { notifyNeedsYou, notifyFinished, notifyError, notificationSound } = preferences

  // A session that just started needing you, finished or failed, and that you are not looking at. A chat session's
  // alert is raised here. A terminal session's sound and system notice come from the daemon (below), which decides
  // them once per event; but it tells only one window, and only sometimes, so the window also raises the card
  // itself for a session it can see change. The card is keyed by its session, so the two never stack.
  useEffect(() => {
    const previousStatuses = lastStatuses.current
    // What another machine's agents do raises no system notice unless its entry says notices = true.
    const mine = sessions.filter((item) => item.machine === LOCAL_MACHINE || machines.get(item.machine)?.state.notices === true)
    const notices = online
      ? noticesFor(lastStatuses.current, mine, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError }, document.hasFocus() ? focusedId : null)
      : []
    lastStatuses.current = new Map(mine.map((item) => [item.id, item.status]))
    // A card that said "Needs you" goes when the session stops needing you.
    for (const item of sessions) if (item.status !== 'needs' && previousStatuses?.get(item.id) === 'needs') dismissSessionNotice(item.id)
    for (const notice of notices) {
      const target = mine.find((item) => item.id === notice.sessionId)
      // Away from the window, a terminal session's system notice is the daemon's; raising it here too would say it twice.
      if (target !== undefined && !isChatSession(target.conversation) && !document.hasFocus()) continue
      raiseNotice(notice)
    }
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

  const paletteItems: PaletteItem[] = useMemo(() => {
    // Thirty-odd items with closures, built only while the palette can show them.
    if (!paletteOpen) return []
    const keys = (shortcut: ShortcutAction) => shortcutText(shortcut, platform)
    const action = (id: string, label: string, run: () => void, shortcut?: string): PaletteItem => ({ id, label, kind: 'Action', run, ...(shortcut ? { shortcut } : {}) })
    const showing = activeArrangement(layout)
    // Sessions first, most urgent first, then their projects, then actions: what the palette is mostly for is jumping.
    const urgent = byUrgency(sessions)
    const sessionItems: PaletteItem[] = urgent.map((member) => ({
      id: `session-${member.id}`,
      label: rowTitle(member),
      detail: `${member.process} · ${projectOf.get(member.id)?.name ?? member.cwdLabel}`,
      kind: 'Session',
      ...(member.status === 'needs' ? { badge: 'Needs you' } : {}),
      run: () => { showSessionById(member.id) },
    }))
    const projectItems: PaletteItem[] = [...new Map(urgent.map((member) => [projectOf.get(member.id)?.key ?? member.cwd, member])).entries()].map(([key, member]) => ({
      id: `project-${key}`,
      label: projectOf.get(member.id)?.name ?? member.cwdLabel,
      detail: tildePath(member.repoRoot ?? member.cwd, home),
      kind: 'Project',
      run: () => { showSessionById(member.id) },
    }))
    const items: PaletteItem[] = [
      ...sessionItems,
      ...projectItems,
      action('new-session', 'New agent chat…', () => { newSession(focusedMachineNumber) }, keys('new-session')),
      action('new-terminal', 'New shell', () => { newTerminal(focusedMachineNumber) }, keys('new-terminal')),
      ...(focusedSession && !isShell(focusedSession) ? [action('open-terminal', 'Open terminal here', () => { toggleAgentTerminal(focusedSession.id) })] : []),
      action('new-agent-chat', 'New agent chat', () => { runShortcut('new-agent-chat') }, keys('new-agent-chat')),
      ...heads.filter((head) => head.name !== null && head.chip?.tone === 'ok' && online).map((head) => action(`new-on-${head.key}`, `New shell on ${head.label}…`, () => { newSessionOn(head.machine) })),
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
      action('settings', 'Open settings', () => { setSettingsSection('appearance') }, keys('settings')),
      action('performance', 'Open performance', () => { setSettingsSection('performance') }, keys('performance')),
      action('machines', 'Machines', () => { setSettingsSection('machines') }),
      action('shortcuts', 'Keyboard shortcuts', () => { setShortcutsOpen(true) }, keys('shortcuts')),
      ...(showing?.widths ? [action('reset-widths', 'Reset pane widths', () => { setLayout((current) => setColumnWidths(current, showing.id, null)) })] : []),
      action('start-codex', 'Start codex', () => { startAgent('codex') }),
    ]
    for (const candidate of THEMES) items.push({ id: `theme-${candidate.name}`, label: candidate.name, detail: candidate.family, kind: 'Theme', needsQuery: true, run: () => { setPreferences((current) => ({ ...current, themeMode: candidate.light ? 'light' : 'dark', ...(candidate.light ? { lightTheme: candidate.name } : { darkTheme: candidate.name }) })) } })
    return items
  }, [paletteOpen, platform, layout, newSession, newTerminal, focusedMachineNumber, toggleAgentTerminal, split, runShortcut, startAgent, sessions, projectOf, focusedSession, showSessionById, preferences.sidebarSide, inspectRefusal, heads, online, newSessionOn, home])

  const restartOffer: RestartOffer | null = mismatch
    ? {
        label: restartStatusQuery.isPending ? 'Checking service…' : restartStatusQuery.isError ? 'Restart check failed' : restartStatusQuery.data.canRestart ? (restartStatusQuery.data.sessionCount > 0 ? `Restart and restore ${String(restartStatusQuery.data.sessionCount)} session${restartStatusQuery.data.sessionCount === 1 ? '' : 's'}` : 'Restart service') : 'Service in use',
        canRestart: Boolean(restartStatusQuery.data?.canRestart),
        pending: restartPending,
        from: mismatch.daemonProtocol,
        to: mismatch.guiProtocol,
        sessions: restartStatusQuery.data?.sessionCount ?? 0,
        working: restartStatusQuery.data?.activeTurnCount ?? 0,
        onRestart: () => {
          setRestartPending(true)
          void restartMismatchedDaemon(mismatch.daemonProtocol)
            .then(async () => {
              await refetchSessions()
              toast.success(`Service restarted · ${String(restartStatusQuery.data?.sessionCount ?? 0)} sessions back`)
            })
            .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not restart the Maitty service')) })
            .finally(() => { setRestartPending(false) })
        },
      }
    : null
  const serviceState = serviceStateOf({ online, failures, mismatch: Boolean(mismatch) })
  const serviceFooterText = serviceState === 'down' ? 'Service stopped' : serviceState === 'update' ? 'Restart needed' : undefined
  const offlineDetail = mismatch ? `${daemonConnectionError(sessionsQuery.error)}${olderServiceNote(restartStatusQuery.data)}` : null

  const tab = activeArrangement(layout)
  const startFolderLabel = preferences.startFolder.trim() ? tildePath(resolveStartFolder(preferences.startFolder, home), home) : '~'
  const canStart = online && !starting
  const sidebarWidth = liveSidebarWidth ?? preferences.sidebarWidth
  const rootStyle = { ...themeStyle(theme), ...sizeVariables(preferences, sidebarWidth) } as CSSProperties

  // One pane of the layout: its header, and the session's terminal with its chat over it when the chat shows.
  const renderPane = (shown: Arrangement, pane: PaneSlot) => {
    const session = paneSessions.get(pane.sessionId)
    if (!session) return null
    const focused = pane.id === shown.focus
    const chat = chatOf(session)
    const chatOnly = isChatSession(session.conversation)
    const subagent = openSubagent?.sessionId === session.id ? session.subagents.find((child) => child.id === openSubagent.subagentId) : undefined
    const typeInto = chatOnly ? undefined : (text: string) => sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, text)
    const terminalOpen = isShell(session) ? null : agentTerminals.has(session.id) && sessionMap.has(agentTerminals.get(session.id) ?? '')
    return (
      <PaneCard
        pane={pane}
        index={shown.panes.findIndex((candidate) => candidate.id === pane.id)}
        session={session}
        paneCount={shown.panes.length}
        broadcast={shown.broadcast === true}
        focused={focused && !outsidePanes}
        maximized={shown.max === pane.id}
        dropTarget={dropId === pane.id && dragId !== pane.id}
        view={chat.view}
        {...(chatOnly || !hasChat(chat.availability) ? {} : { onViewChange: (view: PaneView) => { paneViews.choose(session.id, view) } })}
        chat={chat.view === 'chat' ? (
          <ChatPane
            availability={chat.availability}
            hasTerminal={!chatOnly}
            root={session.repoRoot ?? session.cwd}
            onOpenDiff={showChanges}
            typeInto={typeInto}
            ask={session.ask}
            sessionId={session.id}
            paneStatus={session.status}
            process={session.process}
            online={online}
            folder={folderNameOf(session)}
            branch={session.branch}
            subagent={subagent ?? null}
            subagentCount={session.subagents.length}
            onBackToMain={() => { setOpenSubagent(null) }}
            onOpenFiles={() => { togglePaneTool(pane.id, session.id, 'files') }}
            onBackToTerminal={() => { paneViews.choose(session.id, 'terminal') }}
            onStartAgent={(agent) => {
              paneViews.choose(session.id, 'terminal')
              void sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${agent}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${agent}`)) })
            }}
            defaultFullAccess={preferences.chatFullAccessDefault}
            onStartChat={(agent, fullAccess) => { startChatHere(session, agent, fullAccess) }}
          />
        ) : undefined}
        tool={inspectOpen && focused && (inspectView === 'files' || inspectView === 'changes') ? inspectView : null}
        onTool={(tool) => { togglePaneTool(pane.id, session.id, tool) }}
        terminalOpen={terminalOpen}
        onToggleTerminal={() => { toggleAgentTerminal(session.id) }}
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
        onClear={() => { void sendTo(session, CLEAR_SCREEN).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not clear the screen')) }) }}
        onCopyPath={() => { copyPathOf(session.id) }}
        onRestart={restartPane}
      />
    )
  }

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
                <ServiceStatus online={online} failures={failures} mismatch={Boolean(mismatch)} retryAt={retryAt} detail={offlineDetail ?? (sessionsQuery.isError ? daemonConnectionError(sessionsQuery.error) : null)} restart={restartOffer} sessions={sessions.length} onRetry={() => { void refetchSessions() }} onOpenLog={() => { openDaemonLog().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not open the log')) }) }} />
                {tab ? (
                  <div className={`mx-panes${tab.panes.length > 1 && tab.max === null ? ' is-split' : ''}`}>
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
                              {renderPane(tab, pane)}
                            </div>
                          </Fragment>
                        ))}
                      </div>
                      </Fragment>
                    ))}
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

