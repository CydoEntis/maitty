import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Activity,
  ArrowUpRight,
  ArrowUp,
  Check,
  CheckCheck,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  CircleHelp,
  Command,
  FileCode2,
  FolderOpen,
  FolderPlus,
  GitBranch,
  Keyboard,
  LoaderCircle,
  MessageSquarePlus,
  PanelRight,
  PanelLeft,
  Pin,
  Pencil,
  PenLine,
  Plus,
  Search,
  Settings2,
  ShieldCheck,
  Square,
  TerminalSquare,
  X,
} from 'lucide-react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/StatusMark'
import { Elapsed } from '@/components/Elapsed'
import { TitleBar } from '@/components/TitleBar'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
import { DiffStat } from '@/components/DiffStat'
import { useReviewStat } from '@/features/review/useReviewStat'
import { showNotice } from '@/features/notifications/notify'
import {
  showAttentionToast,
  showFinishedToast,
} from '@/features/notifications/backgroundToasts'
import { HighlightedLine } from '@/components/HighlightedLine'
import { HighlightedCode } from '@/components/HighlightedCode'
import { DiffText } from '@/components/DiffText'
import { FilesView } from '@/features/review/FilesView'
import { TerminalPanes } from '@/features/terminal/TerminalPanes'
import { DesktopApp } from '@/features/workspace/DesktopApp'
import { WorkingSinceContext, useWorkingSince } from '@/lib/working-since'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { Toaster } from '@/components/ui/sonner'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { TerminalView as DaemonTerminalView } from './TerminalView'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import {
  answerUserInput as answerDaemonInput,
  approveRequest,
  cancelTurn as cancelDaemonTurn,
  classifySubmissionFailure,
  createConversation as createDaemonConversation,
  archiveConversationOnDaemon,
  defaultSessionFolder,
  startShellSession,
  startTerminalSession,
  renameConversationOnDaemon,
  endConversation as endDaemonConversation,
  type SessionPermissions,
  isDesktop,
  loadClaudeExecutablePath,
  loadCodexExecutablePath,
  loadConversationDiff,
  loadConversations,
  loadDaemonRestartStatus,
  loadMessages,
  restartMismatchedDaemon,
  resumeConversation as resumeDaemonConversation,
  submitMessage,
} from '@/api/daemon'
import { chooseProjectFolder } from '@/api/folderPicker'
import {
  previewConversations,
  previewDiff,
  previewMessages,
  type Conversation,
  type Message,
  type SessionStatus,
  type UserInputQuestion,
} from '@/data/preview'
import { markConversationsDisconnected } from '@/data/connection'
import {
  planPreferenceMigration,
  readSidebarPreferences,
  writeSidebarPreferences,
} from '@/data/sidebarPreferences'
import {
  readConversationDrafts,
  writeConversationDrafts,
} from '@/data/conversationDrafts'
import {
  findConversationNotifications,
  findConversationStatusAnnouncements,
} from '@/data/conversationNotifications'
import { conversationDiffRefreshInterval } from '@/data/conversationDiffRefresh'
import {
  readThemePreference,
  writeThemePreference,
} from '@/data/themePreference'
import {
  readSettingsPreferences,
  writeSettingsPreferences,
} from '@/features/settings/preferences'
import { SettingsDialog } from '@/features/settings/components/SettingsDialog'
import type { SettingsPreferences } from '@/features/settings/preferences'
import {
  enableNotificationSound,
  playNotificationSound,
} from '@/data/notificationSound'
import {
  readSelectedConversation,
  writeSelectedConversation,
} from '@/data/selectedConversation'
import {
  closeConversationTab,
  openConversationTab,
  readOpenConversationTabs,
  reconcileConversationTabs,
  writeOpenConversationTabs,
} from '@/data/openConversationTabs'
import {
  readConversationPermissions,
  rememberConversationPermissions,
  PERMISSION_LABELS,
} from '@/data/conversationPermissions'
import { shortcutModifier } from '@/lib/shortcut-modifier'
import { errorMessage } from '@/lib/error-message'
import { daemonConnectionError, daemonProtocolMismatch } from '@/lib/daemon-connection'
import './App.css'

const OTHER_FOLDERS = 'Other folders'

/** The sidebar group a terminal session belongs to (ADR 0046). */
function sessionGroup(
  conversation: Conversation,
  groupBy: 'repo' | 'folder',
): string {
  if (groupBy === 'folder') {
    const folder = conversation.cwd ?? conversation.folder
    return folder.split(/[\\/]/).filter(Boolean).at(-1) ?? folder
  }
  return conversation.repo ?? OTHER_FOLDERS
}

function focusMainContent(event: MouseEvent<HTMLAnchorElement>) {
  event.preventDefault()
  document.getElementById('conversation-main')?.focus()
}

const PREVIEW_RESPONSE_DELAY_MS = 1500
const PREVIEW_APPROVAL_DELAY_MS = 1700
const PERMISSION_JSON_INDENT_SPACES = 2
const SESSION_REFRESH_INTERVAL_MS = 1500
const HISTORY_REFRESH_INTERVAL_MS = 2500
const APPROVAL_COMPACT_HEIGHT_PX = 760
const COMPACT_SIDEBAR_MAX_WIDTH_PX = 1099
const RECENT_PROJECT_LIMIT = 5
const ACTIVITY_PREVIEW_LINE_COUNT = 3
const CODE_FENCE_MARKER = '```'

type PanelTab = 'details' | 'diff' | 'files'
type SendFailure = { kind: 'not-delivered' | 'unknown'; operation: number }

const statusText: Record<SessionStatus, string> = {
  working: 'Working',
  'needs-you': 'Needs you',
  done: 'Done',
  idle: 'Idle',
  recovery: 'Recovery needed',
  unknown: 'Disconnected',
}

/** The desktop app is the terminal multiplexer; the browser preview keeps the older chat screens. */
function App() {
  return isDesktop() ? <DesktopApp /> : <PreviewApp />
}

function PreviewApp() {
  const desktop = isDesktop()
  const primaryModifier = shortcutModifier(navigator.userAgent)
  const sessionsQuery = useQuery({
    queryKey: ['conversations'],
    queryFn: loadConversations,
    retry: false,
    refetchInterval: (query) =>
      desktop && query.state.status === 'success'
        ? SESSION_REFRESH_INTERVAL_MS
        : false,
  })
  const mismatch =
    desktop && sessionsQuery.isError
      ? daemonProtocolMismatch(sessionsQuery.error)
      : null
  const daemonRestartStatusQuery = useQuery({
    queryKey: ['daemon-restart-status', mismatch?.daemonProtocol],
    queryFn: loadDaemonRestartStatus,
    enabled: Boolean(mismatch),
    retry: false,
  })
  const [daemonRestartDialogOpen, setDaemonRestartDialogOpen] = useState(false)
  const [daemonRestartPending, setDaemonRestartPending] = useState(false)
  const daemonDisconnected = desktop && sessionsQuery.isError
  const [previewSessions, setConversations] = useState(previewConversations)
  const [sidebarPreferences, setSidebarPreferences] = useState(
    readSidebarPreferences,
  )
  const [startedWith, setStartedWith] = useState(readConversationPermissions)
  const [settingsPreferences, setSettingsPreferences] = useState(
    readSettingsPreferences,
  )
  const conversations = useMemo(() => {
    // The desktop app is a terminal multiplexer: chat sessions are not shown (ADR 0046).
    const current = desktop
      ? (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal')
      : previewSessions
    const withNames = current.map((conversation) => {
      const started = startedWith[conversation.id]
      return {
        ...conversation,
        // Terminal sessions group by repository or folder (Settings); the browser preview keeps its own.
        ...(desktop
          ? { project: sessionGroup(conversation, settingsPreferences.groupBy) }
          : {}),
        // The daemon holds the name for every client; only the browser preview keeps its own.
        title: desktop
          ? conversation.title
          : (sidebarPreferences.renamedTitles[conversation.id] ??
            conversation.title),
        // The daemon does not report how a session was started; show what was chosen.
        ...(desktop && started
          ? { permission: PERMISSION_LABELS[started] }
          : {}),
      }
    })
    return daemonDisconnected
      ? markConversationsDisconnected(withNames)
      : withNames
  }, [
    desktop,
    daemonDisconnected,
    sessionsQuery.data,
    previewSessions,
    sidebarPreferences.renamedTitles,
    settingsPreferences.groupBy,
    startedWith,
  ])
  const archivedIds = useMemo(
    () =>
      desktop
        ? (sessionsQuery.data ?? [])
            .filter((conversation) => conversation.archived)
            .map((conversation) => conversation.id)
        : sidebarPreferences.archivedIds,
    [desktop, sessionsQuery.data, sidebarPreferences.archivedIds],
  )
  const [selectedId, setSelectedId] = useState(
    () =>
      (readSettingsPreferences().restoreTabs
        ? readSelectedConversation()
        : null) ?? (desktop ? '' : 'codex-43'),
  )
  const [savedOpenTabIds, setSavedOpenTabIds] = useState(() =>
    readSettingsPreferences().restoreTabs ? readOpenConversationTabs() : [],
  )
  const [messagesById, setMessagesById] = useState<Record<string, Message[]>>(
    () => structuredClone(previewMessages),
  )
  const [filter, setFilter] = useState('')
  const [drafts, setDrafts] = useState<Record<string, string>>(
    readConversationDrafts,
  )
  const [activePanel, setActivePanel] = useState<PanelTab | null>(null)
  const [terminalConversationId, setTerminalConversationId] = useState<
    string | null
  >(null)
  const [sidebarDrawerOpen, setSidebarDrawerOpen] = useState(false)
  const [sidebarCollapsed, setSidebarCollapsed] = useState(
    settingsPreferences.compactSidebar,
  )
  const [isCompactSidebar, setIsCompactSidebar] = useState(false)
  const [newDialogOpen, setNewDialogOpen] = useState(false)
  const [denyQueuedFor, setDenyQueuedFor] = useState<string | null>(null)
  const [workingSince, setWorkingSince] = useState<ReadonlyMap<string, number>>(
    new Map(),
  )
  const [commandDialogOpen, setCommandDialogOpen] = useState(false)
  const [keyboardDialogOpen, setKeyboardDialogOpen] = useState(false)
  const [settingsDialogOpen, setSettingsDialogOpen] = useState(false)
  const [newProject, setNewProject] = useState('')
  const [newProvider, setNewProvider] = useState<'Codex' | 'Claude'>('Codex')
  // Chosen when a session starts and kept for its whole life.
  const [newKind, setNewKind] = useState<'chat' | 'terminal'>('chat')
  const [endTarget, setEndTarget] = useState<Conversation | null>(null)
  const [startingSessions, setStartingSessions] = useState<
    { key: string; label: string; startedAt: number }[]
  >([])
  const [newPermissions, setNewPermissions] =
    useState<SessionPermissions>('workspace-write')
  const [conversationCreationPending, setConversationCreationPending] =
    useState(false)
  const [isChoosingFolder, setIsChoosingFolder] = useState(false)
  const [theme, setTheme] = useState(readThemePreference)
  const [approvalCollapsed, setApprovalCollapsed] = useState(false)
  const [terminalSwitchNotice, setTerminalSwitchNotice] = useState('')
  const [recoveryPendingId, setRecoveryPendingId] = useState<string | null>(
    null,
  )
  const [sendFailures, setSendFailures] = useState<Record<string, SendFailure>>(
    {},
  )
  const [statusAnnouncement, setStatusAnnouncement] = useState('')
  const queryClient = useQueryClient()
  const conversationCreationPendingRef = useRef(false)
  const previousConversationStatuses = useRef(new Map<string, SessionStatus>())
  const chooseFolder = useCallback(async (): Promise<string | null> => {
    if (!desktop || isChoosingFolder) return null
    setIsChoosingFolder(true)
    try {
      const folder = await chooseProjectFolder()
      if (folder) setNewProject(folder)
      return folder
    } catch {
      toast.error('Could not open the folder picker')
      return null
    } finally {
      setIsChoosingFolder(false)
    }
  }, [desktop, isChoosingFolder])

  useEffect(() => {
    setWorkingSince((current) => {
      const next = new Map(current)
      for (const conversation of conversations) {
        const previous = previousConversationStatuses.current.get(
          conversation.id,
        )
        if (conversation.status !== 'working') next.delete(conversation.id)
        else if (previous !== undefined && previous !== 'working')
          next.set(conversation.id, Date.now())
      }
      return next
    })
  }, [conversations])

  const activeConversations = useMemo(
    () =>
      conversations.filter(
        (conversation) => !archivedIds.includes(conversation.id),
      ),
    [conversations, archivedIds],
  )
  const selected =
    conversations.find((conversation) => conversation.id === selectedId) ??
    activeConversations[0] ??
    conversations[0]
  const selectedConversationId = selected?.id
  const availableConversationIds = useMemo(
    () => conversations.map((conversation) => conversation.id),
    [conversations],
  )
  const openTabIds = useMemo(
    () =>
      reconcileConversationTabs(
        savedOpenTabIds,
        availableConversationIds,
        selectedConversationId,
      ),
    [savedOpenTabIds, availableConversationIds, selectedConversationId],
  )
  const openTabs = useMemo(
    () =>
      openTabIds.flatMap((id) => {
        const conversation = conversations.find((item) => item.id === id)
        return conversation ? [conversation] : []
      }),
    [conversations, openTabIds],
  )
  const selectConversation = useCallback(
    (conversationId: string) => {
      setTerminalSwitchNotice('')
      setSelectedId(conversationId)
      setSavedOpenTabIds(
        openConversationTab(openTabIds, conversationId, selectedConversationId),
      )
    },
    [openTabIds, selectedConversationId],
  )
  const closeConversation = useCallback(
    (conversationId: string) => {
      if (!selectedConversationId) return
      const result = closeConversationTab(
        openTabIds,
        selectedConversationId,
        conversationId,
      )
      setSavedOpenTabIds(result.tabIds)
      if (result.selectedId && result.selectedId !== selectedConversationId) {
        setTerminalSwitchNotice('')
        setSelectedId(result.selectedId)
      }
    },
    [openTabIds, selectedConversationId],
  )
  // While the terminal view is open, choosing a session keeps it open and
  // shows that session's terminal.
  const selectInCurrentView = (conversationId: string) => {
    selectConversation(conversationId)
    if (terminalConversationId !== null)
      setTerminalConversationId(conversationId)
  }
  // The desktop app shows only terminal sessions (ADR 0046). The browser preview, which has no
  // daemon, still flips between its chat and terminal views.
  const showTerminal = Boolean(
    selected && (desktop ? true : terminalConversationId === selected.id),
  )
  // Changes, files and context belong to a chat; a terminal session has none.
  const shownPanel = selected?.kind === 'terminal' ? null : activePanel
  const historyQuery = useQuery({
    queryKey: ['conversation-history', selected?.id],
    queryFn: () =>
      loadMessages(selected?.id ?? '', selected?.provider ?? 'Codex'),
    enabled: desktop && Boolean(selected) && selected?.kind !== 'terminal',
    refetchInterval: (query) => {
      const waitingForApproval =
        selected?.status === 'needs-you' &&
        !query.state.data?.messages.some(
          (message) => message.role === 'approval',
        ) &&
        !query.state.data?.error
      const hasRunningActivity = query.state.data?.messages.some(
        (message) =>
          message.role === 'activity' && message.activityState === 'Running',
      )
      return selected?.status === 'working' ||
        waitingForApproval ||
        hasRunningActivity
        ? HISTORY_REFRESH_INTERVAL_MS
        : false
    },
  })
  const previousSelectedStatus = useRef<{
    conversationId: string
    status: SessionStatus
  } | null>(null)
  const selectedIdForHistory = selected?.id
  const selectedStatusForHistory = selected?.status
  const refetchHistory = historyQuery.refetch
  useEffect(() => {
    const previous = previousSelectedStatus.current
    if (
      selectedIdForHistory &&
      selectedStatusForHistory &&
      previous?.conversationId === selectedIdForHistory &&
      previous.status === 'working' &&
      selectedStatusForHistory !== 'working'
    ) {
      void refetchHistory()
    }
    previousSelectedStatus.current =
      selectedIdForHistory && selectedStatusForHistory
        ? {
            conversationId: selectedIdForHistory,
            status: selectedStatusForHistory,
          }
        : null
  }, [refetchHistory, selectedIdForHistory, selectedStatusForHistory])
  const codexExecutableQuery = useQuery({
    queryKey: ['codex-executable'],
    queryFn: loadCodexExecutablePath,
    enabled: desktop && (!selected || settingsDialogOpen),
    retry: false,
    staleTime: 30_000,
  })
  const claudeExecutableQuery = useQuery({
    queryKey: ['claude-executable'],
    queryFn: loadClaudeExecutablePath,
    enabled: desktop && (!selected || settingsDialogOpen || newDialogOpen),
    retry: false,
    staleTime: 30_000,
  })
  const messages = selected
    ? desktop
      ? (historyQuery.data?.messages ?? [])
      : (messagesById[selected.id] ?? [])
    : []
  const pendingApproval = [...messages]
    .reverse()
    .find((message) => message.role === 'approval' && !message.resolved)
  const draft = selected ? (drafts[selected.id] ?? '') : ''
  const sendFailure = selected ? (sendFailures[selected.id] ?? null) : null
  const deliveryNotice =
    sendFailure?.kind === 'unknown' ||
    (historyQuery.data?.receipt?.delivery === 'SubmissionUncertain' &&
      historyQuery.data.receipt.generation === selected?.generation)
      ? 'unknown'
      : sendFailure?.kind === 'not-delivered'
        ? 'not-delivered'
        : null
  const setStatus = useCallback(
    (conversationId: string, status: SessionStatus) => {
      setConversations((current) =>
        current.map((conversation) =>
          conversation.id === conversationId
            ? { ...conversation, status, updated: 'now' }
            : conversation,
        ),
      )
    },
    [],
  )
  const cancelTurn = useCallback(() => {
    if (daemonDisconnected || !selected) return
    if (desktop) {
      void cancelDaemonTurn(selected)
        .then(async () => {
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['conversations'] }),
            queryClient.invalidateQueries({
              queryKey: ['conversation-history', selected.id],
            }),
          ])
          toast.message(
            `Cancellation requested. Waiting for ${selected.provider} to stop.`,
          )
        })
        .catch((error: unknown) => {
          toast.error(
            error instanceof Error
              ? error.message
              : 'Could not cancel the turn',
          )
        })
      return
    }
    setStatus(selected.id, 'idle')
    toast.message('Preview turn stopped')
  }, [daemonDisconnected, desktop, queryClient, selected, setStatus])
  const toggleTerminal = useCallback(() => {
    if (!selected) return
    setTerminalSwitchNotice('')
    if (!desktop || !selected.execution) {
      setTerminalConversationId(showTerminal ? null : selected.id)
    }
  }, [desktop, selected, showTerminal])
  const openConversationInTerminal = useCallback(
    (conversationId: string) => {
      const conversation = activeConversations.find(
        (item) => item.id === conversationId,
      )
      if (!conversation) return
      setCommandDialogOpen(false)
      selectConversation(conversation.id)
      if (
        !desktop ||
        !conversation.execution ||
        conversation.execution === 'Native'
      ) {
        setTerminalConversationId(conversation.id)
        return
      }
      setTerminalSwitchNotice(
        'This is a chat session, so it has no terminal. Start a terminal session to use one.',
      )
    },
    [activeConversations, desktop, selectConversation],
  )
  const filteredConversations = useMemo(() => {
    const query = filter.trim().toLowerCase()
    return query
      ? activeConversations.filter((conversation) =>
          `${conversation.title} ${conversation.project} ${conversation.provider}`
            .toLowerCase()
            .includes(query),
        )
      : activeConversations
  }, [activeConversations, filter])

  // Once per start: hand the names and closed sessions this browser used to keep to the daemon,
  // so every client shows them. A failure keeps them here and tries again next start.
  const preferencesMigrated = useRef(false)
  useEffect(() => {
    if (!desktop || preferencesMigrated.current || !sessionsQuery.isSuccess)
      return
    preferencesMigrated.current = true
    const plan = planPreferenceMigration(sidebarPreferences, sessionsQuery.data)
    const hasLocal =
      sidebarPreferences.archivedIds.length > 0 ||
      Object.keys(sidebarPreferences.renamedTitles).length > 0
    if (!hasLocal) return
    void Promise.all([
      ...plan.archive.map((id) => archiveConversationOnDaemon({ id }, true)),
      ...plan.rename.map(([id, title]) =>
        renameConversationOnDaemon({ id }, title),
      ),
    ])
      .then(() => {
        setSidebarPreferences(plan.remaining)
        return queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => {
        console.warn('Could not move saved names to the daemon', error)
      })
  }, [
    desktop,
    queryClient,
    sessionsQuery.data,
    sessionsQuery.isSuccess,
    sidebarPreferences,
  ])

  useEffect(() => {
    try {
      writeSidebarPreferences(sidebarPreferences)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save sidebar preferences: ${error.message}`
          : 'Could not save sidebar preferences',
      )
    }
  }, [sidebarPreferences])

  useEffect(() => {
    try {
      writeConversationDrafts(drafts)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save conversation drafts: ${error.message}`
          : 'Could not save conversation drafts',
      )
    }
  }, [drafts])

  useEffect(() => {
    if (!selectedConversationId) return
    try {
      writeSelectedConversation(selectedConversationId)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save selected conversation: ${error.message}`
          : 'Could not save selected conversation',
      )
    }
  }, [selectedConversationId])

  useEffect(() => {
    try {
      writeOpenConversationTabs(openTabIds)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save open conversation tabs: ${error.message}`
          : 'Could not save open conversation tabs',
      )
    }
  }, [openTabIds])

  useEffect(() => {
    if (!desktop) return
    if (daemonDisconnected) {
      previousConversationStatuses.current.clear()
      return
    }
    if (!sessionsQuery.data) return

    const statuses = previousConversationStatuses.current
    const announcements =
      statuses.size === 0
        ? []
        : findConversationStatusAnnouncements(statuses, activeConversations)
    if (announcements.length > 0) {
      setStatusAnnouncement(
        announcements.map(({ title, kind }) => `${kind}: ${title}`).join('. '),
      )
    }
    const notifications =
      statuses.size === 0
        ? []
        : findConversationNotifications(
            statuses,
            activeConversations,
            selected?.id,
          )

    for (const notification of notifications) {
      const conversation = activeConversations.find(
        (item) => item.id === notification.conversationId,
      )
      const open = () => {
        selectConversation(notification.conversationId)
      }
      if (notification.kind === 'attention') {
        if (!settingsPreferences.notifyNeedsYou) continue
        if (conversation) {
          void showAttentionToast(conversation, {
            open,
            answered: () => {
              void queryClient.invalidateQueries({
                queryKey: ['conversations'],
              })
            },
          })
        } else {
          toast.warning(`${notification.title} needs your attention`, {
            action: { label: 'Open', onClick: open },
          })
        }
      } else {
        if (!settingsPreferences.notifyFinished) continue
        if (conversation) {
          void showFinishedToast(conversation, {
            open,
            review: () => {
              open()
              setActivePanel('diff')
            },
          })
        } else {
          toast.success(`${notification.title} is done`, {
            action: { label: 'Open', onClick: open },
          })
        }
      }
      if (settingsPreferences.notificationSound) playNotificationSound()
    }

    statuses.clear()
    for (const conversation of activeConversations)
      statuses.set(conversation.id, conversation.status)
  }, [
    activeConversations,
    daemonDisconnected,
    desktop,
    selected?.id,
    sessionsQuery.data,
    settingsPreferences.notificationSound,
    settingsPreferences.notifyFinished,
    settingsPreferences.notifyNeedsYou,
    selectConversation,
    queryClient,
  ])

  useEffect(() => {
    try {
      writeSettingsPreferences(settingsPreferences)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save notification preferences: ${error.message}`
          : 'Could not save notification preferences',
      )
    }
  }, [settingsPreferences])

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    document.documentElement.classList.toggle('dark', theme !== 'day')
    try {
      writeThemePreference(theme)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save theme preference: ${error.message}`
          : 'Could not save theme preference',
      )
    }
  }, [theme])

  useEffect(() => {
    document.documentElement.dataset.accent = settingsPreferences.accent
    document.documentElement.dataset.reducedMotion = String(
      settingsPreferences.reduceMotion,
    )
    document.documentElement.dataset.sidebarActivity =
      settingsPreferences.animateSidebarActivity ? 'on' : 'off'
    setSpinnerStyle(settingsPreferences.spinner)
  }, [
    settingsPreferences.spinner,
    settingsPreferences.accent,
    settingsPreferences.animateSidebarActivity,
    settingsPreferences.reduceMotion,
  ])

  useEffect(() => {
    const updateApprovalLayout = () => {
      setApprovalCollapsed(window.innerHeight < APPROVAL_COMPACT_HEIGHT_PX)
    }
    updateApprovalLayout()
    window.addEventListener('resize', updateApprovalLayout)
    return () => {
      window.removeEventListener('resize', updateApprovalLayout)
    }
  }, [])

  useEffect(() => {
    const updateSidebarMode = () => {
      const compact = window.innerWidth <= COMPACT_SIDEBAR_MAX_WIDTH_PX
      setIsCompactSidebar(compact)
      if (!compact) setSidebarDrawerOpen(false)
    }
    updateSidebarMode()
    window.addEventListener('resize', updateSidebarMode)
    return () => {
      window.removeEventListener('resize', updateSidebarMode)
    }
  }, [])

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      const hasModifier = event.ctrlKey || event.metaKey
      const key = event.key.toLowerCase()
      const target = event.target instanceof HTMLElement ? event.target : null
      const inTerminal = Boolean(target?.closest('.terminal-screen'))
      const inDialog = Boolean(target?.closest('[role="dialog"]'))
      const inTextEntry =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        Boolean(target?.isContentEditable)

      if (hasModifier && key === 'k') {
        event.preventDefault()
        setCommandDialogOpen(true)
        return
      }
      if (inDialog) return
      if (inTerminal && !(hasModifier && key === '`')) return
      if (hasModifier && event.key === 'Tab' && openTabIds.length > 1) {
        event.preventDefault()
        const currentIndex = openTabIds.indexOf(selected?.id ?? '')
        const direction = event.shiftKey ? -1 : 1
        const nextId =
          openTabIds[
            (currentIndex + direction + openTabIds.length) % openTabIds.length
          ]
        if (nextId) selectConversation(nextId)
        return
      }
      if (hasModifier && /^\d$/.test(event.key)) {
        const tabId = openTabIds[Number(event.key) - 1]
        if (tabId) {
          event.preventDefault()
          selectConversation(tabId)
        }
        return
      }
      if (hasModifier && key === 'w' && selected) {
        event.preventDefault()
        closeConversation(selected.id)
        return
      }
      if (hasModifier && event.key === ',') {
        event.preventDefault()
        setSettingsDialogOpen(true)
        return
      }
      if (hasModifier && key === 'o') {
        event.preventDefault()
        openNewSession()
        return
      }
      if (event.key === 'Escape' && !inTerminal) {
        if (activePanel) {
          event.preventDefault()
          setActivePanel(null)
        } else if (sidebarDrawerOpen) {
          event.preventDefault()
          setSidebarDrawerOpen(false)
        }
        return
      }
      if (hasModifier && key === '\\') {
        event.preventDefault()
        if (isCompactSidebar) setSidebarDrawerOpen((open) => !open)
        else setSidebarCollapsed((collapsed) => !collapsed)
        return
      }
      if (
        event.key === '?' &&
        !(target instanceof HTMLInputElement) &&
        !(target instanceof HTMLTextAreaElement) &&
        !target?.isContentEditable
      ) {
        event.preventDefault()
        setKeyboardDialogOpen(true)
        return
      }
      if (!hasModifier && !inTextEntry && !inTerminal) {
        const useAltArrow = event.altKey && !event.shiftKey
        const useJk = !event.altKey && !event.shiftKey
        const direction =
          (useAltArrow && event.key === 'ArrowDown') || (useJk && key === 'j')
            ? 1
            : (useAltArrow && event.key === 'ArrowUp') || (useJk && key === 'k')
              ? -1
              : 0
        if (direction !== 0) {
          const rows = Array.from(
            document.querySelectorAll<HTMLButtonElement>(
              '.session-list-scroll button.session-row',
            ),
          )
          if (rows.length === 0) return
          const currentIndex = rows.findIndex(
            (row) => row.getAttribute('aria-current') === 'page',
          )
          const nextIndex =
            currentIndex < 0
              ? direction > 0
                ? 0
                : rows.length - 1
              : (currentIndex + direction + rows.length) % rows.length
          const nextRow = rows[nextIndex]
          if (!nextRow) return
          event.preventDefault()
          nextRow.click()
          return
        }
        if (
          pendingApproval &&
          !daemonDisconnected &&
          pendingApproval.approvalKind !== 'UserInput'
        ) {
          const choices =
            pendingApproval.approvalChoices ??
            (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel'])
          const decision =
            event.shiftKey && key === 'n'
              ? 'Cancel'
              : !event.shiftKey && (key === 'y' || key === 'a')
                ? 'Accept'
                : !event.shiftKey && key === 'n'
                  ? 'Decline'
                  : null
          if (decision && choices.includes(decision)) {
            const action = document.querySelector<HTMLButtonElement>(
              `.approval-dock [data-approval-choice="${decision}"]`,
            )
            if (action && !action.disabled) {
              event.preventDefault()
              action.click()
              return
            }
          }
        }
      }
      if (!hasModifier) return

      if (key === 'n') {
        if (daemonDisconnected) return
        event.preventDefault()
        openNewSession()
      } else if (event.shiftKey && key === 'd') {
        event.preventDefault()
        setActivePanel((panel) => (panel === 'diff' ? null : 'diff'))
      } else if (key === 'i') {
        event.preventDefault()
        setActivePanel((panel) => (panel === 'details' ? null : 'details'))
      } else if (key === '`') {
        event.preventDefault()
        toggleTerminal()
      } else if (
        key === '.' &&
        selected &&
        (selected.status === 'working' || selected.status === 'needs-you')
      ) {
        event.preventDefault()
        cancelTurn()
      } else if (event.shiftKey && key === 'a' && !daemonDisconnected) {
        const blocked = activeConversations.filter(
          (conversation) =>
            conversation.status === 'needs-you' ||
            conversation.status === 'recovery',
        )
        if (blocked.length === 0) {
          toast.message('No conversations need your attention')
          return
        }
        const firstBlocked = blocked[0]
        if (!firstBlocked) return
        event.preventDefault()
        const selectedIndex = blocked.findIndex(
          (conversation) => conversation.id === selected?.id,
        )
        selectConversation(
          blocked[(selectedIndex + 1) % blocked.length]?.id ?? firstBlocked.id,
        )
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [
    activeConversations,
    activePanel,
    sidebarDrawerOpen,
    cancelTurn,
    chooseFolder,
    daemonDisconnected,
    desktop,
    isCompactSidebar,
    openTabIds,
    pendingApproval,
    selected,
    selectConversation,
    closeConversation,
    openNewSession,
    toggleTerminal,
  ])

  function togglePin(conversationId: string) {
    setSidebarPreferences((current) => ({
      ...current,
      pinnedIds: current.pinnedIds.includes(conversationId)
        ? current.pinnedIds.filter((id) => id !== conversationId)
        : [...current.pinnedIds, conversationId],
    }))
  }

  function updateSettingsPreferences(preferences: SettingsPreferences) {
    if (preferences.compactSidebar !== settingsPreferences.compactSidebar) {
      setSidebarCollapsed(preferences.compactSidebar)
    }
    setSettingsPreferences(preferences)
  }

  // Close or reopen sessions for every client: the daemon keeps the flag. Only the browser
  // preview, which has no daemon, remembers it itself.
  function putAway(ids: string[], archived: boolean) {
    if (!desktop) {
      setSidebarPreferences((current) => ({
        ...current,
        archivedIds: archived
          ? [...new Set([...current.archivedIds, ...ids])]
          : current.archivedIds.filter((id) => !ids.includes(id)),
      }))
      return
    }
    void Promise.all(
      ids.map((id) => archiveConversationOnDaemon({ id }, archived)),
    )
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error
            ? error.message
            : 'Could not change this session',
        )
      })
      .finally(() => {
        void queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
  }

  // Closing a session ends it: a chat asks first (its agent stops), a terminal session closes
  // its pane. There is no archive; a closed session leaves the list.
  function setArchived(conversationId: string, archived: boolean) {
    if (!archived) return
    const conversation = conversations.find(
      (item) => item.id === conversationId,
    )
    if (conversation?.kind !== 'terminal') {
      requestEndSession(conversationId)
      return
    }
    putAway([conversationId], true)
    if (openTabIds.includes(conversationId) && openTabIds.length > 1)
      closeConversation(conversationId)
    toast.message('Terminal session closed', {
      description:
        'Its program was stopped, as when it is closed in the terminal app.',
    })
  }

  function requestEndSession(conversationId: string) {
    const conversation = conversations.find(
      (item) => item.id === conversationId,
    )
    if (conversation) setEndTarget(conversation)
  }

  function confirmEndSession() {
    const target = endTarget
    if (!target) return
    setEndTarget(null)
    if (!desktop) {
      putAway([target.id], true)
      if (openTabIds.includes(target.id) && openTabIds.length > 1)
        closeConversation(target.id)
      return
    }
    void endDaemonConversation(target)
      .then(() => {
        // The agent is stopped; move the session out of the way, like an archived one.
        putAway([target.id], true)
        if (openTabIds.includes(target.id) && openTabIds.length > 1)
          closeConversation(target.id)
        void queryClient.invalidateQueries({ queryKey: ['conversations'] })
        toast.message('Session ended', {
          description:
            'The agent is stopped. Its history stays under Archived. Start a new session to continue with different permissions.',
        })
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not end this session'))
      })
  }

  function renameConversation(conversationId: string, title: string) {
    const cleanTitle = title.trim()
    if (!cleanTitle) {
      toast.error('Conversation name cannot be empty')
      return
    }
    if (desktop) {
      void renameConversationOnDaemon({ id: conversationId }, cleanTitle)
        .catch((error: unknown) => {
          toast.error(
            error instanceof Error ? error.message : 'Could not rename it',
          )
        })
        .finally(() => {
          void queryClient.invalidateQueries({ queryKey: ['conversations'] })
        })
      return
    }
    setSidebarPreferences((current) => ({
      ...current,
      renamedTitles: { ...current.renamedTitles, [conversationId]: cleanTitle },
    }))
  }


  const settingsDialog = (
    <SettingsDialog
      open={settingsDialogOpen}
      onClose={() => {
        setSettingsDialogOpen(false)
      }}
      theme={theme}
      onThemeChange={setTheme}
      preferences={settingsPreferences}
      onPreferencesChange={updateSettingsPreferences}
      onNotificationSoundChange={(enabled) => {
        if (!enabled || enableNotificationSound()) return true
        toast.error('Could not enable notification sound')
        return false
      }}
      onTestNotification={() => {
        showNotice({
          kind: 'info',
          title: 'Test notification',
          body: 'Maitty notifications are working.',
        })
        if (settingsPreferences.notificationSound) playNotificationSound()
      }}
      daemonConnectionState={
        !desktop
          ? 'preview'
          : daemonDisconnected
            ? 'disconnected'
            : sessionsQuery.isPending
              ? 'connecting'
              : 'connected'
      }
      onReconnect={() => {
        void sessionsQuery.refetch()
      }}
      codexExecutableState={
        !desktop
          ? 'preview'
          : codexExecutableQuery.isPending
            ? 'checking'
            : codexExecutableQuery.isError
              ? 'unavailable'
              : 'available'
      }
      codexExecutablePath={codexExecutableQuery.data}
      onCodexRecheck={() => {
        void codexExecutableQuery.refetch()
      }}
      claudeExecutableState={
        !desktop
          ? 'preview'
          : claudeExecutableQuery.isPending
            ? 'checking'
            : claudeExecutableQuery.isError
              ? 'unavailable'
              : 'available'
      }
      claudeExecutablePath={claudeExecutableQuery.data}
      onClaudeRecheck={() => {
        void claudeExecutableQuery.refetch()
      }}
      primaryModifier={primaryModifier}
    />
  )
  const daemonRestartAction = mismatch && (
    <Button
      variant="outline"
      size="sm"
      disabled={
        daemonRestartStatusQuery.isPending ||
        daemonRestartPending ||
        Boolean(
          daemonRestartStatusQuery.data &&
          !daemonRestartStatusQuery.data.canRestart,
        )
      }
      onClick={() => {
        if (daemonRestartStatusQuery.isError)
          void daemonRestartStatusQuery.refetch()
        else setDaemonRestartDialogOpen(true)
      }}
    >
      {daemonRestartStatusQuery.isPending
        ? 'Checking service…'
        : daemonRestartStatusQuery.isError
          ? 'Retry restart check'
          : daemonRestartStatusQuery.data.canRestart
            ? 'Restart service'
            : daemonRestartStatusQuery.data.sessionCount > 0 ||
                daemonRestartStatusQuery.data.activeTurnCount === null
              ? 'Service in use'
              : 'Restart unavailable'}
    </Button>
  )
  const daemonRestartStatus = daemonRestartStatusQuery.data
  const daemonRestartDialog = mismatch && (
    <Dialog
      open={daemonRestartDialogOpen}
      onOpenChange={(open) => {
        if (!daemonRestartPending) setDaemonRestartDialogOpen(open)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restart the older Maitty service?</DialogTitle>
          <DialogDescription>
            {daemonRestartStatus?.canRestart
              ? daemonRestartStatus.activeTurnCount === null
                ? `The old service has ${String(daemonRestartStatus.sessionCount)} saved session${daemonRestartStatus.sessionCount === 1 ? '' : 's'} but cannot report which agents are active. Restarting will interrupt active work, and output since the last save may be lost.`
                : daemonRestartStatus.activeTurnCount > 0
                  ? `The old service has ${String(daemonRestartStatus.sessionCount)} session${daemonRestartStatus.sessionCount === 1 ? '' : 's'} and ${String(daemonRestartStatus.activeTurnCount)} active agent turn${daemonRestartStatus.activeTurnCount === 1 ? '' : 's'}. Active turns will stop, and output since the last save may be lost.`
                  : `The old service has ${String(daemonRestartStatus.sessionCount)} saved session${daemonRestartStatus.sessionCount === 1 ? '' : 's'} and no active agent turns. Restarting lets this GUI reconnect to the updated service.`
              : daemonRestartStatusQuery.isError
                ? 'Maitty could not confirm the service status. No restart has happened; retry the check or reconnect.'
                : daemonRestartStatusQuery.isPending
                  ? 'Checking whether the local service can be safely replaced.'
                  : mismatch.daemonProtocol > mismatch.guiProtocol
                    ? 'This service is newer than the one bundled with this GUI. Update the GUI before reconnecting; Maitty will not replace a newer service.'
                    : daemonRestartStatus?.sessionCount
                      ? `This older service owns ${String(daemonRestartStatus.sessionCount)} sessions. Maitty will leave it running so your TUI work is not interrupted.`
                      : daemonRestartStatus?.activeTurnCount === null
                        ? 'Maitty could not verify that the older service is idle, so it will leave the service running.'
                        : 'This older service still reports active work, so Maitty will leave it running.'}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button
            variant="outline"
            disabled={daemonRestartPending}
            onClick={() => {
              setDaemonRestartDialogOpen(false)
            }}
          >
            Cancel
          </Button>
          <Button
            disabled={!daemonRestartStatus?.canRestart || daemonRestartPending}
            onClick={() => {
              if (!daemonRestartStatus?.canRestart) return
              setDaemonRestartPending(true)
              void restartMismatchedDaemon(mismatch.daemonProtocol)
                .then(async () => {
                  setDaemonRestartDialogOpen(false)
                  await sessionsQuery.refetch()
                })
                .catch((error: unknown) => {
                  toast.error(
                    error instanceof Error
                      ? error.message
                      : typeof error === 'string'
                        ? error
                        : 'Could not restart the Maitty service',
                  )
                  void daemonRestartStatusQuery.refetch()
                })
                .finally(() => {
                  setDaemonRestartPending(false)
                })
            }}
          >
            {daemonRestartPending ? (
              <>
                <LoaderCircle size={14} />
                Restarting…
              </>
            ) : (
              'Restart and reconnect'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )

  const approvalChoicesOffered = pendingApproval
    ? (pendingApproval.approvalChoices ??
      (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel']))
    : []
  // Only a choice the provider offered may be used; Cancel also ends the turn so
  // the instructions can follow right away.
  const denyDecision: 'Cancel' | 'Decline' | null =
    pendingApproval &&
    pendingApproval.approvalKind !== 'UserInput' &&
    !daemonDisconnected
      ? approvalChoicesOffered.includes('Cancel')
        ? 'Cancel'
        : approvalChoicesOffered.includes('Decline')
          ? 'Decline'
          : null
      : null

  useEffect(() => {
    if (
      !selected ||
      denyQueuedFor === null ||
      denyQueuedFor !== selected.id ||
      daemonDisconnected
    )
      return
    if (selected.status === 'idle' || selected.status === 'done') {
      sendMessage()
    }
  })

  if (!selected)
    return (
      <TooltipProvider>
        <div className="app-shell">
          <div
            className="sr-only"
            role="status"
            aria-live="polite"
            aria-atomic="true"
          >
            {statusAnnouncement}
          </div>
          <a
            className="skip-link"
            href="#conversation-main"
            onClick={focusMainContent}
          >
            Skip to main content
          </a>
          <TitleBar />
          <div className="workspace-shell empty-shell">
            <ConversationSidebar
              conversations={[]}
              selectedId=""
              filter={filter}
              pinnedIds={sidebarPreferences.pinnedIds}
              compact={isCompactSidebar}
              desktopCollapsed={sidebarCollapsed}
              disconnected={daemonDisconnected}
              drawerOpen={sidebarDrawerOpen}
              onFilter={setFilter}
              onNew={() => {
                openNewSession()
              }}
              onTogglePin={togglePin}
              onArchive={setArchived}
              onRename={renameConversation}
              onEnd={requestEndSession}
              onSelect={selectConversation}
              onToggleDrawer={() => {
                setSidebarDrawerOpen((open) => !open)
              }}
              onToggleCollapse={() => {
                setSidebarCollapsed((collapsed) => !collapsed)
              }}
              onNewInProject={() => {
                openNewSession()
              }}
              starting={startingSessions}
              onOpenProject={() => {
                openNewSession()
              }}
              daemonLabel={
                sessionsQuery.isLoading
                  ? 'Connecting to Maitty daemon'
                  : sessionsQuery.isError
                    ? 'Daemon disconnected'
                    : 'Daemon connected'
              }
              sessionCount={0}
              previewMode={false}
              onOpenSettings={() => {
                setSettingsDialogOpen(true)
              }}
              primaryModifier={primaryModifier}
            />
            <main
              className="empty-workspace"
              id="conversation-main"
              tabIndex={-1}
            >
              <div className="empty-mark">
                <MessageSquarePlus size={20} />
              </div>
              <h1>
                {sessionsQuery.isLoading
                  ? 'Connecting to Maitty'
                  : sessionsQuery.isError
                    ? 'Daemon disconnected'
                    : 'Run your coding agents side by side.'}
              </h1>
              <p
                className={
                  sessionsQuery.isError ? 'daemon-connection-error' : undefined
                }
              >
                {sessionsQuery.isLoading
                  ? 'Connecting to your local Maitty service.'
                  : sessionsQuery.isError
                    ? daemonConnectionError(sessionsQuery.error)
                    : 'Start an agent session in a folder, and switch between chat and its native terminal at any time. Sessions live in maittyd, so closing this window never stops an agent.'}
              </p>
              {!sessionsQuery.isLoading && !sessionsQuery.isError && (
                <div
                  className="first-run-checklist"
                  aria-label="First launch status"
                >
                  <span>
                    <i className="connection-dot is-connected" />
                    Maitty daemon connected
                  </span>
                  <span>
                    <i
                      className={`connection-dot ${codexExecutableQuery.isSuccess ? 'is-connected' : ''}`}
                    />
                    {!desktop
                      ? 'Codex check available in desktop app'
                      : codexExecutableQuery.isPending
                        ? 'Checking for Codex CLI…'
                        : codexExecutableQuery.isError
                          ? 'Codex CLI not found'
                          : 'Codex CLI ready'}
                  </span>
                  <span>
                    <i
                      className={`connection-dot ${claudeExecutableQuery.isSuccess ? 'is-connected' : ''}`}
                    />
                    {!desktop
                      ? 'Claude Code check available in desktop app'
                      : claudeExecutableQuery.isPending
                        ? 'Checking for Claude Code CLI…'
                        : claudeExecutableQuery.isError
                          ? 'Claude Code CLI not found'
                          : 'Claude Code CLI ready'}
                  </span>
                </div>
              )}
              {sessionsQuery.isLoading ? (
                <Button disabled>
                  <LoaderCircle size={14} />
                  Connecting
                </Button>
              ) : sessionsQuery.isError ? (
                <>
                  <div className="connection-actions">
                    <Button
                      onClick={() => {
                        void sessionsQuery.refetch()
                      }}
                    >
                      Reconnect
                    </Button>
                    {daemonRestartAction}
                  </div>
                  {mismatch &&
                    daemonRestartStatus &&
                    !daemonRestartStatus.canRestart && (
                      <p className="daemon-connection-error" role="status">
                        {mismatch.daemonProtocol > mismatch.guiProtocol
                          ? 'This service is newer than the GUI. Update the GUI before reconnecting.'
                          : daemonRestartStatus.sessionCount > 0
                            ? `The older service owns ${String(daemonRestartStatus.sessionCount)} sessions. Maitty will leave it running so your TUI work is not interrupted.`
                            : daemonRestartStatus.activeTurnCount === null
                              ? 'Maitty could not verify that the older service is idle, so it will leave the service running.'
                              : 'The older service still reports active work, so Maitty will leave it running.'}
                      </p>
                    )}
                </>
              ) : (
                <Button
                  onClick={() => {
                    openNewSession()
                  }}
                >
                  <FolderOpen size={14} />
                  New session
                  <span className="shortcut-hint">{primaryModifier} O</span>
                </Button>
              )}
            </main>
          </div>
          <NewConversationDialog
            open={newDialogOpen}
            project={newProject}
            onProjectChange={setNewProject}
            provider={newProvider}
            onProviderChange={setNewProvider}
            kind={newKind}
            onKindChange={setNewKind}
            permissions={newPermissions}
            onPermissionsChange={setNewPermissions}
            claudeAvailable={!desktop || Boolean(claudeExecutableQuery.data)}
            desktop={desktop}
            onChooseFolder={() => {
              void chooseFolder()
            }}
            isChoosingFolder={isChoosingFolder}
            onClose={() => {
              setNewDialogOpen(false)
            }}
            onCreate={createConversation}
            disabled={daemonDisconnected || conversationCreationPending}
            isCreating={conversationCreationPending}
            recentProjects={[]}
          />
          {settingsDialog}
            {daemonRestartDialog}
          <Toaster
            position={settingsPreferences.toastPosition}
            theme={theme === 'day' ? 'light' : 'dark'}
          />
        </div>
      </TooltipProvider>
    )

  const activeConversation = selected

  function updateDraft(value: string) {
    setDrafts((current) => ({ ...current, [activeConversation.id]: value }))
  }

  function clearSendFailure(conversationId: string) {
    setSendFailures((current) =>
      Object.fromEntries(
        Object.entries(current).filter(([id]) => id !== conversationId),
      ),
    )
  }

  function pushMessage(conversationId: string, message: Message) {
    setMessagesById((current) => ({
      ...current,
      [conversationId]: [...(current[conversationId] ?? []), message],
    }))
  }

  function sendMessage() {
    // A queued deny-and-send is spent the moment a send is attempted.
    setDenyQueuedFor(null)
    if (daemonDisconnected) return
    if (deliveryNotice === 'unknown') return
    const text = draft.trim()
    if (
      !text ||
      activeConversation.status === 'working' ||
      activeConversation.status === 'needs-you' ||
      activeConversation.status === 'unknown'
    )
      return

    const conversationId = activeConversation.id
    if (desktop) {
      const operation = historyQuery.data?.nextOperation ?? 1
      const conversation = activeConversation
      void submitMessage(conversation, operation, text)
        .then(async () => {
          setDrafts((current) => ({ ...current, [conversationId]: '' }))
          clearSendFailure(conversationId)
          await refreshAfterSubmission(conversationId)
        })
        .catch(async () => {
          let outcome: ReturnType<typeof classifySubmissionFailure> = 'unknown'
          try {
            const latest = await queryClient.query({
              queryKey: ['conversation-history', conversationId],
              queryFn: () =>
                loadMessages(conversationId, conversation.provider),
              staleTime: 0,
            })
            outcome = classifySubmissionFailure(
              latest,
              conversation.generation ?? -1,
              operation,
            )
          } catch {
            // A failed receipt read cannot prove whether the provider received the prompt.
          }
          if (outcome === 'delivered') {
            setDrafts((current) => ({ ...current, [conversationId]: '' }))
            clearSendFailure(conversationId)
            await refreshAfterSubmission(conversationId)
            toast.message('Message was received; refreshing conversation')
            return
          }
          setSendFailures((current) => ({
            ...current,
            [conversationId]: { kind: outcome, operation },
          }))
          toast.error(
            outcome === 'unknown'
              ? 'Delivery is unknown. Check the native terminal before continuing.'
              : 'Message was not delivered. Your draft is safe to retry.',
          )
        })
      return
    }
    updateDraft('')
    pushMessage(conversationId, {
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      time: 'now',
    })
    setStatus(conversationId, 'working')

    window.setTimeout(() => {
      pushMessage(conversationId, {
        id: crypto.randomUUID(),
        role: 'assistant',
        content:
          'I’ve picked that up. This preview uses sample data; the real conversation connection is still being wired in.',
        time: 'now',
      })
      setStatus(conversationId, 'done')
    }, PREVIEW_RESPONSE_DELAY_MS)
  }

  async function checkSubmissionDelivery() {
    if (!selected) return
    const uncertainReceipt =
      historyQuery.data?.receipt?.delivery === 'SubmissionUncertain'
        ? historyQuery.data.receipt
        : null
    const operation = sendFailure?.operation ?? uncertainReceipt?.operation
    if (operation === undefined || selected.generation === undefined) return
    try {
      const latest = await loadMessages(selected.id, selected.provider)
      queryClient.setQueryData(['conversation-history', selected.id], latest)
      const outcome = classifySubmissionFailure(
        latest,
        selected.generation,
        operation,
      )
      if (outcome === 'delivered') {
        setDrafts((current) => ({ ...current, [selected.id]: '' }))
        clearSendFailure(selected.id)
        toast.message('The daemon confirmed this message was received')
      } else if (outcome === 'not-delivered') {
        setSendFailures((current) => ({
          ...current,
          [selected.id]: { kind: 'not-delivered', operation },
        }))
        toast.message(
          'The daemon confirmed this message was not delivered. Your draft is safe to retry.',
        )
      } else {
        setSendFailures((current) => ({
          ...current,
          [selected.id]: { kind: 'unknown', operation },
        }))
        toast.warning(
          'Delivery is still unknown. Check the native terminal before continuing.',
        )
      }
    } catch (error: unknown) {
      toast.error(
        error instanceof Error
          ? `Could not check delivery: ${error.message}`
          : 'Could not check delivery',
      )
    }
  }

  function resumeSelectedConversation() {
    if (
      !selected ||
      !desktop ||
      daemonDisconnected ||
      selected.execution !== 'Structured' ||
      selected.status !== 'recovery'
    )
      return
    const conversation = selected
    setRecoveryPendingId(conversation.id)
    void resumeDaemonConversation(conversation)
      .then(async (resumed) => {
        selectConversation(String(resumed.id))
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['conversations'] }),
          queryClient.invalidateQueries({
            queryKey: ['conversation-history', conversation.id],
          }),
        ])
      })
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error
            ? error.message
            : 'Could not resume this conversation',
        )
      })
      .finally(() => {
        setRecoveryPendingId(null)
      })
  }

  async function refreshAfterSubmission(conversationId: string) {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['conversations'] }),
      queryClient.invalidateQueries({
        queryKey: ['conversation-history', conversationId],
      }),
    ])
  }

  function resolveApproval(
    message: Message,
    decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel',
  ) {
    if (daemonDisconnected) return
    if (desktop) {
      if (message.providerRequestId === undefined) return
      void approveRequest(
        activeConversation,
        message.providerRequestId,
        decision,
      )
        .then(async () => {
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['conversations'] }),
            queryClient.invalidateQueries({
              queryKey: ['conversation-history', activeConversation.id],
            }),
          ])
        })
        .catch((error: unknown) => {
          toast.error(
            error instanceof Error
              ? error.message
              : 'Could not answer approval request',
          )
        })
      return
    }
    const allow = decision === 'Accept' || decision === 'AcceptForSession'
    setMessagesById((current) => ({
      ...current,
      [activeConversation.id]: (current[activeConversation.id] ?? []).map(
        (item) => (item.id === message.id ? { ...item, resolved: true } : item),
      ),
    }))
    pushMessage(activeConversation.id, {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: allow
        ? 'Approved once. The test command is running in this preview.'
        : 'You declined the command. The agent did not run it.',
      time: 'now',
    })
    setStatus(activeConversation.id, allow ? 'working' : 'idle')
    if (allow) {
      window.setTimeout(() => {
        setStatus(activeConversation.id, 'done')
      }, PREVIEW_APPROVAL_DELAY_MS)
    }
  }

  function answerInput(message: Message, answers: Record<string, string[]>) {
    if (daemonDisconnected) return
    if (message.providerRequestId === undefined) return
    void answerDaemonInput(
      activeConversation,
      message.providerRequestId,
      answers,
    )
      .then(async () => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['conversations'] }),
          queryClient.invalidateQueries({
            queryKey: ['conversation-history', activeConversation.id],
          }),
        ])
        toast.success(`Answers sent to ${activeConversation.provider}`)
      })
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error ? error.message : 'Could not send answers',
        )
      })
  }

  // A new session is a plain shell in the start folder; the user types what to run in it (ADR
  // 0046). The dialog remains only for the browser preview, which has no daemon to start sessions in.
  function openNewSession() {
    if (!desktop) {
      setNewDialogOpen(true)
      return
    }
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    conversationCreationPendingRef.current = true
    setConversationCreationPending(true)
    const known = new Set(conversations.map((item) => item.id))
    const chosen = settingsPreferences.startFolder
    void (chosen ? Promise.resolve(chosen) : defaultSessionFolder())
      .then((folder) => startShellSession(folder))
      .then(async () => {
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
        const fresh = await queryClient.query({
          queryKey: ['conversations'],
          queryFn: loadConversations,
          staleTime: 0,
        })
        const created = fresh.find(
          (item) => item.kind === 'terminal' && !known.has(item.id),
        )
        if (created) selectConversation(created.id)
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not start a shell'))
      })
      .finally(() => {
        conversationCreationPendingRef.current = false
        setConversationCreationPending(false)
      })
  }

  function createConversation() {
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    const typed = newProject.trim()
    if (typed) {
      startConversation(typed)
      return
    }
    // No folder chosen: start in the home folder and let the user move on from there (cd), as in the terminal app.
    void defaultSessionFolder()
      .then((folder) => {
        if (folder) startConversation(folder)
        else toast.error('Choose a project folder first')
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not find your home folder'))
      })
  }

  function startConversation(project: string) {
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    if (desktop && newKind === 'terminal') {
      conversationCreationPendingRef.current = true
      setConversationCreationPending(true)
      const known = new Set(conversations.map((item) => item.id))
      void startTerminalSession(project, newProvider)
        .then(async () => {
          setNewDialogOpen(false)
          await queryClient.invalidateQueries({ queryKey: ['conversations'] })
          // Open the terminal session that just appeared.
          const fresh = await queryClient.query({
            queryKey: ['conversations'],
            queryFn: loadConversations,
            staleTime: 0,
          })
          const created = fresh.find(
            (item) => item.kind === 'terminal' && !known.has(item.id),
          )
          if (created) selectConversation(created.id)
        })
        .catch((error: unknown) => {
          toast.error(
            errorMessage(error, `Could not start ${newProvider} terminal`),
          )
        })
        .finally(() => {
          conversationCreationPendingRef.current = false
          setConversationCreationPending(false)
        })
      return
    }
    if (desktop) {
      conversationCreationPendingRef.current = true
      setConversationCreationPending(true)
      // The daemon starts the provider before it answers, which can take a few
      // seconds: close the dialog now and show progress in the sidebar instead.
      const startingKey = crypto.randomUUID()
      const folderName =
        project.split(/[\\/]/).filter(Boolean).at(-1) ?? project
      setStartingSessions((current) => [
        ...current,
        {
          key: startingKey,
          startedAt: Date.now(),
          label: `Starting ${newProvider} session in ${folderName}\u2026`,
        },
      ])
      setNewDialogOpen(false)
      void createDaemonConversation(project, newProvider, newPermissions)
        .then(async ({ id }) => {
          rememberConversationPermissions(String(id), newPermissions)
          setStartedWith(readConversationPermissions())
          selectConversation(String(id))
          await queryClient.invalidateQueries({ queryKey: ['conversations'] })
        })
        .catch((error: unknown) => {
          // Reopen the dialog with the same choices so nothing has to be retyped.
          setNewDialogOpen(true)
          toast.error(
            errorMessage(error, `Could not start ${newProvider} conversation`),
          )
        })
        .finally(() => {
          setStartingSessions((current) =>
            current.filter((item) => item.key !== startingKey),
          )
          conversationCreationPendingRef.current = false
          setConversationCreationPending(false)
        })
      return
    }
    const conversation: Conversation = {
      id: crypto.randomUUID(),
      title: `New ${newProvider} conversation`,
      project: project.split(/[\\/]/).filter(Boolean).at(-1) ?? 'project',
      folder: project,
      branch: 'working tree',
      provider: newProvider,
      status: 'idle',
      updated: 'now',
      model: `${newProvider} default`,
      permission: 'Read only · approvals on request',
      context: 0,
      nativePane: 0,
      hasDiff: false,
    }
    setConversations((current) => [conversation, ...current])
    setMessagesById((current) => ({ ...current, [conversation.id]: [] }))
    selectConversation(conversation.id)
    setActivePanel('details')
    setNewDialogOpen(false)
    toast.message('Created in local preview')
  }

  function denyAndSend() {
    if (!pendingApproval || !denyDecision || !draft.trim()) return
    setDenyQueuedFor(activeConversation.id)
    resolveApproval(pendingApproval, denyDecision)
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      if (denyDecision) denyAndSend()
      else sendMessage()
    }
  }

  return (
    <WorkingSinceContext.Provider value={workingSince}>
      <TooltipProvider>
        <div className="app-shell">
          <div
            className="sr-only"
            role="status"
            aria-live="polite"
            aria-atomic="true"
          >
            {statusAnnouncement}
          </div>
          <a
            className="skip-link"
            href="#conversation-main"
            onClick={focusMainContent}
          >
            Skip to conversation
          </a>
          <TitleBar />
          <div className={`workspace-shell ${shownPanel ? 'has-panel' : ''}`}>
            {isCompactSidebar && sidebarDrawerOpen && (
              <div
                className="drawer-scrim"
                aria-hidden="true"
                onClick={() => {
                  setSidebarDrawerOpen(false)
                }}
              />
            )}
            <ConversationSidebar
              conversations={filteredConversations}
              selectedId={selected.id}
              filter={filter}
              pinnedIds={sidebarPreferences.pinnedIds}
              compact={isCompactSidebar}
              desktopCollapsed={sidebarCollapsed}
              disconnected={daemonDisconnected}
              drawerOpen={sidebarDrawerOpen}
              onFilter={setFilter}
              onNew={() => {
                openNewSession()
              }}
              onTogglePin={togglePin}
              onArchive={setArchived}
              onRename={renameConversation}
              onEnd={requestEndSession}
              onSelect={(id) => {
                selectInCurrentView(id)
                setSidebarDrawerOpen(false)
              }}
              onToggleDrawer={() => {
                setSidebarDrawerOpen((open) => !open)
              }}
              onToggleCollapse={() => {
                setSidebarCollapsed((collapsed) => !collapsed)
              }}
              onNewInProject={(folder) => {
                setNewProject(folder)
                openNewSession()
              }}
              starting={startingSessions}
              onOpenProject={() => {
                openNewSession()
              }}
              daemonLabel={
                desktop
                  ? daemonDisconnected
                    ? 'Daemon disconnected'
                    : 'Connected to Maitty daemon'
                  : 'Local preview · daemon not connected'
              }
              sessionCount={conversations.length}
              previewMode={!desktop}
              onOpenSettings={() => {
                setSettingsDialogOpen(true)
              }}
              primaryModifier={primaryModifier}
            />

            <div className="main-card">
              <main
                className="conversation-workspace"
                id="conversation-main"
                tabIndex={-1}
              >
                <ConversationTabStrip
                  tabs={openTabs}
                  selectedId={selected.id}
                  onSelect={selectInCurrentView}
                  onClose={closeConversation}
                  onNewTab={() => {
                    openNewSession()
                  }}
                  conversation={selected}
                  panel={shownPanel}
                  showTerminal={showTerminal}
                  onPanel={setActivePanel}
                  onToggleTerminal={toggleTerminal}
                  showViewSwitch={!desktop}
                  threadsButton={isCompactSidebar}
                  onOpenThreads={() => {
                    setSidebarDrawerOpen((open) => !open)
                  }}
                  attentionCount={
                    activeConversations.filter(
                      (item) =>
                        item.status === 'needs-you' ||
                        item.status === 'recovery',
                    ).length
                  }
                />
                <ConversationHeader conversation={selected} />

                {terminalSwitchNotice && (
                  <div className="handoff-notice" role="status">
                    <CircleAlert size={14} />
                    <span>{terminalSwitchNotice}</span>
                  </div>
                )}

                {daemonDisconnected && (
                  <div className="connection-warning" role="alert">
                    <CircleAlert size={15} />
                    <span>
                      Disconnected from maittyd. Agents keep running in the
                      daemon. Statuses are the last known values; reconnect
                      before sending or approving.
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        void sessionsQuery.refetch()
                      }}
                    >
                      Reconnect
                    </Button>
                    {daemonRestartAction}
                  </div>
                )}
                {showTerminal ? (
                  <div className="terminal-workspace">
                    <TerminalPanes
                      conversations={activeConversations}
                      selectedId={selected.id}
                      onSelect={selectInCurrentView}
                      primaryModifier={primaryModifier}
                      renderTerminal={(item) => (
                        <TerminalView conversation={item} desktop={desktop} fontSize={settingsPreferences.terminalFontSize} />
                      )}
                    />
                  </div>
                ) : (
                  <>
                    {selected.status === 'needs-you' && !pendingApproval && (
                      <div className="attention-strip" role="status">
                        <CircleAlert size={15} />
                        <span>
                          {desktop
                            ? 'This conversation needs your attention.'
                            : 'This agent needs your approval before it can continue.'}
                        </span>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() =>
                            document
                              .querySelector<HTMLElement>('.approval-request')
                              ?.scrollIntoView({
                                behavior: 'smooth',
                                block: 'center',
                              })
                          }
                        >
                          Review request
                        </Button>
                      </div>
                    )}
                    {selected.status === 'recovery' && (
                      <RecoveryNotice
                        conversation={selected}
                        desktop={desktop}
                        disabled={
                          daemonDisconnected ||
                          recoveryPendingId === selected.id
                        }
                        pending={recoveryPendingId === selected.id}
                        onResume={resumeSelectedConversation}
                      />
                    )}

                    <ScrollArea className="timeline-scroll">
                      <div className="timeline-content">
                        <div className="conversation-date">
                          <span />
                          Today, 10:42 AM
                          <span />
                        </div>
                        {messages.length === 0 ? (
                          <EmptyConversation conversation={selected} />
                        ) : (
                          (() => {
                            let labelled = false
                            return messages.map((message, index) => {
                              if (message.role === 'user') labelled = false
                              if (message.role === 'activity') {
                                if (messages[index - 1]?.role === 'activity')
                                  return null
                                const group: Message[] = []
                                for (
                                  let cursor = index;
                                  messages[cursor]?.role === 'activity';
                                  cursor += 1
                                ) {
                                  const item = messages[cursor]
                                  if (!item || item.role !== 'activity') break
                                  group.push(item)
                                }
                                return (
                                  <ActivityGroup
                                    key={message.id}
                                    messages={group}
                                    provider={selected.provider}
                                  />
                                )
                              }
                              const showLabel =
                                message.role === 'assistant' && !labelled
                              if (message.role === 'assistant') labelled = true
                              return (
                                <MessageRow
                                  key={message.id}
                                  message={message}
                                  provider={selected.provider}
                                  showLabel={showLabel}
                                />
                              )
                            })
                          })()
                        )}
                        {selected.status === 'working' && (
                          <StreamingRow provider={selected.provider} />
                        )}
                        <FilesChangedButton
                          conversation={selected}
                          onReview={() => {
                            setActivePanel('diff')
                          }}
                        />
                      </div>
                    </ScrollArea>

                    {pendingApproval && (
                      <ApprovalDock
                        key={`${pendingApproval.id}:${selected.id}`}
                        message={pendingApproval}
                        provider={selected.provider}
                        desktop={desktop}
                        disabled={daemonDisconnected}
                        collapsed={approvalCollapsed}
                        onToggle={() => {
                          setApprovalCollapsed((collapsed) => !collapsed)
                        }}
                        onResolve={(decision) => {
                          resolveApproval(pendingApproval, decision)
                        }}
                        onUserInput={(answers) => {
                          answerInput(pendingApproval, answers)
                        }}
                      />
                    )}
                    <Composer
                      value={draft}
                      status={selected.status}
                      conversation={selected}
                      desktop={desktop}
                      connected={!daemonDisconnected}
                      cancelling={historyQuery.data?.cancelling ?? false}
                      deliveryNotice={deliveryNotice}
                      deliveryBlocked={deliveryNotice === 'unknown'}
                      onCheckDelivery={() => {
                        void checkSubmissionDelivery()
                      }}
                      onRetry={() => {
                        sendMessage()
                      }}
                      onDiscard={() => {
                        setDrafts((current) => ({
                          ...current,
                          [selected.id]: '',
                        }))
                        clearSendFailure(selected.id)
                      }}
                      onChange={updateDraft}
                      onKeyDown={handleComposerKeyDown}
                      onSend={sendMessage}
                      denyDecision={denyDecision}
                      denyQueued={denyQueuedFor === selected.id}
                      onDenyAndSend={denyAndSend}
                      onCancel={cancelTurn}
                    />
                  </>
                )}
              </main>

              {shownPanel && (
                <div
                  className="panel-scrim"
                  aria-hidden="true"
                  onClick={() => {
                    setActivePanel(null)
                  }}
                />
              )}
              {shownPanel && (
                <ContextPanel
                  tab={shownPanel}
                  conversation={selected}
                  desktop={desktop}
                  onTab={setActivePanel}
                  onClose={() => {
                    setActivePanel(null)
                  }}
                />
              )}
            </div>
          </div>
        </div>

        <NewConversationDialog
          open={newDialogOpen}
          project={newProject}
          onProjectChange={setNewProject}
          provider={newProvider}
          onProviderChange={setNewProvider}
          kind={newKind}
          onKindChange={setNewKind}
          permissions={newPermissions}
          onPermissionsChange={setNewPermissions}
          claudeAvailable={!desktop || Boolean(claudeExecutableQuery.data)}
          desktop={desktop}
          onChooseFolder={() => {
            void chooseFolder()
          }}
          isChoosingFolder={isChoosingFolder}
          onClose={() => {
            setNewDialogOpen(false)
          }}
          onCreate={createConversation}
          disabled={daemonDisconnected || conversationCreationPending}
          isCreating={conversationCreationPending}
          recentProjects={Array.from(
            new Map(conversations.map((item) => [item.folder, item])).values(),
          ).slice(0, RECENT_PROJECT_LIMIT)}
        />
        <CommandDialog
          key={commandDialogOpen ? 'open' : 'closed'}
          open={commandDialogOpen}
          conversations={activeConversations}
          onClose={() => {
            setCommandDialogOpen(false)
          }}
          onSelect={(id) => {
            selectConversation(id)
            setCommandDialogOpen(false)
          }}
          onNew={() => {
            setCommandDialogOpen(false)
            openNewSession()
          }}
          onSettings={() => {
            setSettingsDialogOpen(true)
            setCommandDialogOpen(false)
          }}
          onKeyboard={() => {
            setCommandDialogOpen(false)
            setKeyboardDialogOpen(true)
          }}
          onPanel={(tab) => {
            if (tab === 'terminal') toggleTerminal()
            else setActivePanel(tab)
            setCommandDialogOpen(false)
          }}
          primaryModifier={primaryModifier}
          onSelectInTerminal={openConversationInTerminal}
        />
        <KeyboardDialog
          open={keyboardDialogOpen}
          onClose={() => {
            setKeyboardDialogOpen(false)
          }}
          primaryModifier={primaryModifier}
        />
        {settingsDialog}
        <Dialog
          open={endTarget !== null}
          onOpenChange={(open) => {
            if (!open) setEndTarget(null)
          }}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>End this session?</DialogTitle>
              <DialogDescription>
                {endTarget?.title}: the agent stops now. Its history is kept,
                but you can’t continue this chat. To change permissions, end the
                session and start a new one.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="ghost"
                onClick={() => {
                  setEndTarget(null)
                }}
              >
                Cancel
              </Button>
              <Button variant="destructive" onClick={confirmEndSession}>
                End session
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
        {daemonRestartDialog}
        <Toaster
          position={settingsPreferences.toastPosition}
          theme={theme === 'day' ? 'light' : 'dark'}
        />
      </TooltipProvider>
    </WorkingSinceContext.Provider>
  )
}

type SidebarProps = {
  conversations: Conversation[]
  selectedId: string
  filter: string
  pinnedIds: string[]
  compact: boolean
  desktopCollapsed: boolean
  disconnected: boolean
  drawerOpen: boolean
  onFilter: (value: string) => void
  onNew: () => void
  onSelect: (id: string) => void
  onTogglePin: (id: string) => void
  onArchive: (id: string, archived: boolean) => void
  onRename: (id: string, title: string) => void
  onEnd: (id: string) => void
  primaryModifier: '⌘' | 'Ctrl'
  starting: { key: string; label: string; startedAt: number }[]
  onOpenProject: () => void
  onNewInProject: (folder: string) => void
  daemonLabel: string
  sessionCount: number
  previewMode: boolean
  onOpenSettings: () => void
  onToggleDrawer: () => void
  onToggleCollapse: () => void
}

function ConversationSidebar({
  conversations,
  selectedId,
  filter,
  pinnedIds,
  compact,
  desktopCollapsed,
  disconnected,
  drawerOpen,
  onFilter,
  onNew,
  onSelect,
  onTogglePin,
  onArchive,
  onRename,
  onEnd,
  onToggleDrawer,
  onToggleCollapse,
  primaryModifier,
  starting,
  onOpenProject,
  onNewInProject,
  daemonLabel,
  sessionCount,
  previewMode,
  onOpenSettings,
}: SidebarProps) {
  const projects = Array.from(
    new Set(conversations.map((conversation) => conversation.project)),
  ).sort((left, right) => {
    // Sessions outside any repository sit under Other folders, last.
    if (left === OTHER_FOLDERS) return 1
    if (right === OTHER_FOLDERS) return -1
    return left.localeCompare(right)
  })
  const rowActions = {
    pinnedIds,
    selectedId,
    onSelect,
    onTogglePin,
    onArchive,
    onRename,
    onEnd,
  }

  return (
    <aside
      className={`session-sidebar ${compact ? 'is-compact' : ''} ${desktopCollapsed ? 'is-desktop-collapsed' : ''} ${drawerOpen ? 'is-drawer-open' : ''}`}
      aria-label="Conversations"
    >
      <div className="sidebar-heading">
        <Button
          className="sidebar-rail-toggle"
          variant="ghost"
          size="icon-sm"
          aria-label={
            drawerOpen
              ? 'Close conversation sidebar'
              : 'Open conversation sidebar'
          }
          onClick={onToggleDrawer}
        >
          <PanelLeft />
        </Button>
        <Button
          className="sidebar-collapse-toggle"
          variant="ghost"
          size="icon-sm"
          aria-label={
            desktopCollapsed
              ? 'Expand conversation sidebar'
              : 'Collapse conversation sidebar'
          }
          onClick={onToggleCollapse}
        >
          <PanelLeft />
        </Button>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="sidebar-open-project"
              variant="ghost"
              size="icon-sm"
              aria-label="New session"
              onClick={onOpenProject}
              disabled={disconnected}
            >
              <FolderPlus />
            </Button>
          </TooltipTrigger>
          <TooltipContent>New session · {primaryModifier} O</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="sidebar-new"
              variant="ghost"
              size="icon-sm"
              aria-label="New conversation"
              onClick={onNew}
              disabled={disconnected}
            >
              <PenLine />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            New conversation · {primaryModifier} N
          </TooltipContent>
        </Tooltip>
      </div>
      <label className="session-search">
        <Search size={14} aria-hidden="true" />
        <input
          aria-label="Search conversations"
          placeholder="Search sessions"
          value={filter}
          onChange={(event) => {
            onFilter(event.target.value)
          }}
        />
      </label>
      {starting.map((item) => (
        <div className="starting-row" role="status" key={item.key}>
          <StatusMark status="working" />
          <span>{item.label}</span>
          <Elapsed since={item.startedAt} />
        </div>
      ))}
      <ScrollArea className="session-list-scroll">
        {projects.map((project) => (
          <ProjectSection
            key={project}
            project={project}
            conversations={conversations.filter(
              (conversation) => conversation.project === project,
            )}
            disconnected={disconnected}
            onNewInProject={onNewInProject}
            {...rowActions}
          />
        ))}
        {conversations.length === 0 && (
          <div className="no-sessions" role="status">
            {filter.trim()
              ? `No sessions match “${filter}”`
              : 'No sessions yet. Start one in a folder to get started.'}
          </div>
        )}
      </ScrollArea>
      <div className="sidebar-bottom">
        <div
          className="sidebar-connection"
          role="status"
          aria-live="polite"
          aria-label={daemonLabel}
        >
          <span
            className={`connection-dot ${disconnected ? 'is-offline' : 'is-connected'}`}
          />
          <span>
            {previewMode
              ? 'Local preview · sample data'
              : disconnected
                ? 'Reconnecting to maittyd…'
                : `maittyd · ${String(sessionCount)} ${sessionCount === 1 ? 'session' : 'sessions'}`}
          </span>
        </div>
        <Button
          className="sidebar-settings"
          variant="ghost"
          size="sm"
          aria-label="Open settings"
          onClick={onOpenSettings}
        >
          <Settings2 />
          <span>Settings</span>
        </Button>
      </div>
    </aside>
  )
}

type SessionRowActions = {
  pinnedIds: string[]
  selectedId: string
  onSelect: (id: string) => void
  onTogglePin: (id: string) => void
  onArchive: (id: string, archived: boolean) => void
  onRename: (id: string, title: string) => void
  onEnd: (id: string) => void
}

function ProjectSection({
  project,
  conversations,
  disconnected,
  onNewInProject,
  ...actions
}: {
  project: string
  conversations: Conversation[]
  disconnected: boolean
  onNewInProject: (folder: string) => void
} & SessionRowActions) {
  const needsYou = conversations.filter(
    (conversation) => conversation.status === 'needs-you',
  ).length
  const active = [...conversations]
    .sort((left, right) => {
      const attention = (conversation: Conversation) =>
        conversation.status === 'needs-you' ||
        conversation.status === 'recovery'
          ? 0
          : 1
      const attentionOrder = attention(left) - attention(right)
      if (attentionOrder !== 0) return attentionOrder
      const pinOrder =
        Number(actions.pinnedIds.includes(right.id)) -
        Number(actions.pinnedIds.includes(left.id))
      if (pinOrder !== 0) return pinOrder
      return 0
    })

  return (
    <section className="project-section" aria-label={`${project} project`}>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div className="project-heading">
            <span>{project}</span>
            {needsYou > 0 && (
              <span className="project-needs">{needsYou} needs you</span>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  className="project-new"
                  variant="ghost"
                  size="icon-xs"
                  aria-label={`New session in ${project}`}
                  disabled={disconnected}
                  onClick={() => {
                    onNewInProject(conversations[0]?.folder ?? '')
                  }}
                >
                  <Plus />
                </Button>
              </TooltipTrigger>
              <TooltipContent>New session in {project}</TooltipContent>
            </Tooltip>
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem
            disabled={disconnected}
            onSelect={() => {
              onNewInProject(conversations[0]?.folder ?? '')
            }}
          >
            <Plus size={14} />
            New session in {project}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      <div className="session-rows">
        {active.map((conversation) => (
          <SessionRow
            key={conversation.id}
            conversation={conversation}
            selected={conversation.id === actions.selectedId}
            isPinned={actions.pinnedIds.includes(conversation.id)}
            {...actions}
          />
        ))}
      </div>
      {disconnected && (
        <span className="project-offline">Last known statuses</span>
      )}
    </section>
  )
}

function RowMeta({ conversation }: { conversation: Conversation }) {
  const since = useWorkingSince(conversation.id)
  if (conversation.status === 'working' && since !== undefined)
    return <Elapsed since={since} />
  if (conversation.status === 'needs-you')
    return <span className="meta-needs">Needs you</span>
  if (conversation.status === 'recovery')
    return <span className="meta-recovery">Recovery</span>
  if (conversation.status === 'unknown') return <>—</>
  return <>{conversation.updated}</>
}

function rowSubline(conversation: Conversation): string {
  switch (conversation.status) {
    case 'needs-you':
      return 'Waiting for your reply'
    case 'recovery':
      return `${conversation.provider} exited mid-turn`
    case 'working':
      return 'Working…'
    case 'unknown':
      return conversation.lastKnownStatus
        ? `Last seen ${statusText[conversation.lastKnownStatus].toLowerCase()}`
        : 'Last seen —'
    case 'done':
    case 'idle':
      return `${conversation.project} · ${conversation.branch}`
  }
}

function SessionRow({
  conversation,
  selected,
  isPinned,
  isArchived = false,
  onSelect,
  onTogglePin,
  onArchive,
  onRename,
  onEnd,
}: {
  conversation: Conversation
  selected: boolean
  isPinned: boolean
  isArchived?: boolean
} & Pick<
  SessionRowActions,
  'onSelect' | 'onTogglePin' | 'onArchive' | 'onRename' | 'onEnd'
>) {
  const lastKnownStatus = conversation.lastKnownStatus
  const lastKnownText =
    conversation.status === 'unknown' && lastKnownStatus
      ? `, last known ${statusText[lastKnownStatus]}`
      : ''
  const [isRenaming, setIsRenaming] = useState(false)
  const [renameDraft, setRenameDraft] = useState(conversation.title)

  function beginRename() {
    setRenameDraft(conversation.title)
    setIsRenaming(true)
  }

  function saveRename() {
    onRename(conversation.id, renameDraft)
    setIsRenaming(false)
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div className={`session-row-shell ${selected ? 'is-selected' : ''}`}>
          {isRenaming ? (
            <div className="session-row-rename">
              <input
                autoFocus
                aria-label={`Rename ${conversation.title}`}
                value={renameDraft}
                onChange={(event) => {
                  setRenameDraft(event.target.value)
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault()
                    saveRename()
                  } else if (event.key === 'Escape') setIsRenaming(false)
                }}
              />
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label="Save conversation name"
                onClick={saveRename}
              >
                <Check />
              </Button>
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label="Cancel rename"
                onClick={() => {
                  setIsRenaming(false)
                }}
              >
                <X />
              </Button>
            </div>
          ) : (
            <button
              className={`session-row status-${conversation.status} ${selected ? 'is-selected' : ''} ${conversation.status === 'needs-you' ? 'is-attention' : ''} ${isArchived ? 'is-archived' : ''}`}
              type="button"
              onClick={() => {
                onSelect(conversation.id)
              }}
              onKeyDown={(event) => {
                if (event.key === 'F2') {
                  event.preventDefault()
                  beginRename()
                }
              }}
              aria-label={`${conversation.title}, ${statusText[conversation.status]}${lastKnownText}, ${conversation.project}`}
              aria-current={selected ? 'page' : undefined}
            >
              <StatusMark status={conversation.status} className="row-mark" />
              <span className="session-row-copy">
                <span className="session-row-top">
                  <span className="session-row-title">
                    {conversation.title}
                  </span>
                  <span className="session-updated">
                    <RowMeta conversation={conversation} />
                  </span>
                </span>
                <span className="session-row-bottom">
                  <span>{rowSubline(conversation)}</span>
                </span>
              </span>
            </button>
          )}
        </div>
      </ContextMenuTrigger>
      {!isRenaming && (
        <ContextMenuContent>
          <ContextMenuItem
            onSelect={() => {
              onTogglePin(conversation.id)
            }}
          >
            <Pin size={14} />
            {isPinned ? 'Unpin conversation' : 'Pin conversation'}
          </ContextMenuItem>
          <ContextMenuItem onSelect={beginRename}>
            <Pencil size={14} />
            Rename
          </ContextMenuItem>
          <ContextMenuItem
            onSelect={() => {
              onArchive(conversation.id, true)
            }}
          >
            <X size={14} />
            Close session
          </ContextMenuItem>
          <ContextMenuItem
            variant="destructive"
            disabled={conversation.execution !== 'Structured'}
            onSelect={() => {
              onEnd(conversation.id)
            }}
          >
            <Square size={14} />
            End session
          </ContextMenuItem>
        </ContextMenuContent>
      )}
    </ContextMenu>
  )
}

function ConversationTabStrip({
  tabs,
  selectedId,
  onSelect,
  onClose,
  onNewTab,
  conversation,
  panel,
  showTerminal,
  onPanel,
  onToggleTerminal,
  showViewSwitch,
  threadsButton,
  onOpenThreads,
  attentionCount,
}: {
  tabs: Conversation[]
  selectedId: string
  onSelect: (id: string) => void
  onClose: (id: string) => void
  onNewTab: () => void
  conversation: Conversation
  panel: PanelTab | null
  showTerminal: boolean
  onPanel: (tab: PanelTab | null) => void
  onToggleTerminal: () => void
  showViewSwitch: boolean
  threadsButton: boolean
  onOpenThreads: () => void
  attentionCount: number
}) {
  const reviewStat = useReviewStat(conversation)

  function handleTabKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    const index = tabs.findIndex((tab) => tab.id === selectedId)
    const next = tabs[index + (event.key === 'ArrowRight' ? 1 : -1)]
    if (!next) return
    event.preventDefault()
    onSelect(next.id)
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLElement>('.conversation-tab.is-active [role="tab"]')
        ?.focus()
    })
  }

  return (
    <div className="tab-strip">
      {threadsButton && (
        <Button variant="ghost" size="sm" onClick={onOpenThreads}>
          Threads
          {attentionCount > 0 && (
            <span className="threads-attention">
              <StatusMark status="needs-you" />
              {attentionCount}
            </span>
          )}
        </Button>
      )}
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="New tab"
        title="New conversation in a new tab (Ctrl+N)"
        onClick={onNewTab}
      >
        <Plus size={14} />
      </Button>
      <div
        className="conversation-tabs"
        role="tablist"
        aria-label="Open conversations"
        onKeyDown={handleTabKeyDown}
      >
        {tabs.map((tab) => (
          <ContextMenu key={tab.id}>
            <ContextMenuTrigger asChild>
              <div
                className={`conversation-tab ${tab.id === selectedId ? 'is-active' : ''} ${tab.status === 'working' && tab.id === selectedId ? 'is-shimmering' : ''}`}
                onAuxClick={(event) => {
                  if (event.button === 1 && tabs.length > 1) onClose(tab.id)
                }}
              >
                <button
                  type="button"
                  role="tab"
                  aria-selected={tab.id === selectedId}
                  aria-controls="conversation-main"
                  tabIndex={tab.id === selectedId ? 0 : -1}
                  title={`${tab.title} · ${statusText[tab.status]}`}
                  onClick={() => {
                    onSelect(tab.id)
                  }}
                >
                  <StatusMark status={tab.status} />
                  <span>{tab.title}</span>
                  {tab.id === selectedId && (
                    <span
                      className="sr-only"
                      role="status"
                      aria-label={`Current status: ${statusText[tab.status]}`}
                    />
                  )}
                </button>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      className="conversation-tab-close"
                      type="button"
                      aria-label={`Close ${tab.title} tab`}
                      disabled={tabs.length <= 1}
                      onClick={() => {
                        onClose(tab.id)
                      }}
                    >
                      <X size={12} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>
                    Close tab · the session keeps running in maittyd
                  </TooltipContent>
                </Tooltip>
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem
                disabled={tabs.length <= 1}
                onSelect={() => {
                  onClose(tab.id)
                }}
              >
                <X size={14} />
                Close tab
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        ))}
      </div>
      <div className="header-toolbar">
        {showViewSwitch && (
          <div
            className="view-switch"
            role="group"
            aria-label="Conversation view"
          >
            <Button
              variant={!showTerminal ? 'secondary' : 'ghost'}
              size="sm"
              disabled={!showTerminal}
              onClick={onToggleTerminal}
              aria-pressed={!showTerminal}
              aria-label={showTerminal ? 'Return to chat' : 'Chat view'}
            >
              Chat
            </Button>
            <Button
              variant={showTerminal ? 'secondary' : 'ghost'}
              size="sm"
              disabled={showTerminal}
              onClick={onToggleTerminal}
              aria-pressed={showTerminal}
              aria-label="Show terminal"
            >
              Terminal
            </Button>
          </div>
        )}
        {conversation.kind !== 'terminal' && (
          <Button
            className="review-button"
            variant="ghost"
            size="sm"
            title="Changes, files and context (Ctrl+Shift+D)"
            aria-pressed={panel !== null}
            onClick={() => {
              onPanel(panel ? null : 'diff')
            }}
          >
            Review
            {reviewStat && (
              <DiffStat
                additions={reviewStat.additions}
                deletions={reviewStat.deletions}
              />
            )}
          </Button>
        )}
      </div>
    </div>
  )
}

function ConversationHeader({ conversation }: { conversation: Conversation }) {
  const since = useWorkingSince(conversation.id)
  const sandbox = conversation.permission.split(' · ')[0]
  const lastSeen =
    conversation.status === 'unknown' && conversation.lastKnownStatus
      ? ` · last seen ${statusText[conversation.lastKnownStatus].toLowerCase()}`
      : ''
  return (
    <div className="conversation-heading">
      <h1>{conversation.title}</h1>
      <span className="provider-pill">
        <span aria-hidden="true">›_</span>
        {conversation.provider} · {conversation.model}
      </span>
      <span className="conversation-branch">{conversation.branch}</span>
      <span className="conversation-sandbox">{sandbox}</span>
      <span
        className={`conversation-status status-${conversation.status}`}
        role="status"
        aria-label={`Status: ${statusText[conversation.status]}`}
      >
        <StatusMark status={conversation.status} />
        {conversation.status === 'unknown'
          ? 'Unknown'
          : statusText[conversation.status]}
        {conversation.status === 'working' && since !== undefined && (
          <>
            {' · '}
            <Elapsed since={since} />
          </>
        )}
        {lastSeen}
      </span>
    </div>
  )
}

function MessageRow({
  message,
  provider,
  showLabel,
}: {
  message: Message
  provider: Conversation['provider']
  showLabel: boolean
}) {
  if (message.role === 'activity') {
    return <ActivityGroup messages={[message]} provider={provider} />
  }
  if (message.role === 'approval') return <ApprovalMarker message={message} />
  if (message.role === 'user')
    return (
      <div className="message-row user-message">
        <div className="user-bubble">{message.content}</div>
      </div>
    )
  return (
    <div className="message-row assistant-message">
      {showLabel && <span className="agent-label">{provider}</span>}
      <AssistantContent content={message.content} />
    </div>
  )
}

type AssistantBlock =
  | { kind: 'text'; content: string }
  | { kind: 'code'; language: string; content: string }

const POWERSHELL_COMMAND = /powershell|pwsh/i

/** Which highlighting suits a command line, judged by the shell it runs. */
function commandLanguage(command: string | undefined): string {
  return POWERSHELL_COMMAND.test(command ?? '') ? 'powershell' : 'sh'
}

function AssistantContent({ content }: { content: string }) {
  const blocks = splitAssistantCode(content)
  return (
    <div className="assistant-copy">
      {blocks.map((block, index) =>
        block.kind === 'code' ? (
          <section
            className="assistant-code-block"
            key={`code-${String(index)}`}
          >
            <div className="assistant-code-language">
              {block.language || 'Code'}
            </div>
            <pre>
              <code>
                <HighlightedCode
                  code={block.content}
                  language={block.language}
                />
              </code>
            </pre>
          </section>
        ) : (
          <p key={`text-${String(index)}`}>{renderInlineCode(block.content)}</p>
        ),
      )}
    </div>
  )
}

function splitAssistantCode(content: string): AssistantBlock[] {
  const blocks: AssistantBlock[] = []
  const textLines: string[] = []
  let codeLines: string[] = []
  let language = ''
  let inCode = false

  for (const line of content.split('\n')) {
    if (line.startsWith(CODE_FENCE_MARKER)) {
      if (inCode) {
        blocks.push({ kind: 'code', language, content: codeLines.join('\n') })
        codeLines = []
        language = ''
        inCode = false
      } else {
        if (textLines.length > 0) {
          blocks.push({ kind: 'text', content: textLines.join('\n') })
          textLines.length = 0
        }
        language = line.slice(CODE_FENCE_MARKER.length).trim()
        inCode = true
      }
      continue
    }

    if (inCode) codeLines.push(line)
    else textLines.push(line)
  }

  if (inCode) {
    textLines.push(`\`\`\`${language}`, ...codeLines)
  }
  if (textLines.length > 0) {
    blocks.push({ kind: 'text', content: textLines.join('\n') })
  }
  return blocks
}

function renderInlineCode(content: string): ReactNode[] {
  return content.split(/(`[^`\n]+`)/g).map((part, index) => {
    if (part.startsWith('`') && part.endsWith('`')) {
      return <code key={String(index)}>{part.slice(1, -1)}</code>
    }
    return part
  })
}

function FilesChangedButton({
  conversation,
  onReview,
}: {
  conversation: Conversation
  onReview: () => void
}) {
  const stat = useReviewStat(conversation)
  if (!stat || conversation.status === 'working') return null
  return (
    <button type="button" className="files-changed-button" onClick={onReview}>
      {stat.files} {stat.files === 1 ? 'file' : 'files'} changed
      <DiffStat additions={stat.additions} deletions={stat.deletions} />
      <span className="files-changed-review">Review →</span>
    </button>
  )
}

function ApprovalMarker({ message }: { message: Message }) {
  const resolved = message.resolved
  const text = resolved
    ? 'Request answered'
    : message.approvalKind === 'UserInput'
      ? 'Waiting for your answer below'
      : 'Waiting for your approval below'
  return (
    <div
      className={`approval-marker ${resolved ? 'is-resolved' : ''}`}
      role="status"
    >
      <span className="approval-marker-box" aria-hidden="true">
        <i />
      </span>
      <span>{text}</span>
    </div>
  )
}

function ApprovalDock({
  message,
  provider,
  desktop,
  disabled,
  collapsed,
  onToggle,
  onResolve,
  onUserInput,
}: {
  message: Message
  provider: Conversation['provider']
  desktop: boolean
  disabled: boolean
  collapsed: boolean
  onToggle: () => void
  onResolve: (
    decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel',
  ) => void
  onUserInput: (answers: Record<string, string[]>) => void
}) {
  const choices =
    message.approvalChoices ??
    (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel'])
  return (
    <section
      className={`approval-dock ${collapsed ? 'is-collapsed' : ''}`}
      aria-label={
        message.approvalKind === 'UserInput'
          ? 'Question needs an answer'
          : 'Approval needed'
      }
    >
      <div className="approval-dock-heading">
        <button
          className="approval-dock-toggle"
          type="button"
          onClick={onToggle}
          aria-expanded={!collapsed}
        >
          <span className="approval-dock-status">
            <ShieldCheck size={15} />
            {message.approvalKind === 'UserInput'
              ? 'Your answer needed'
              : 'Approval needed'}
          </span>
          <span className="approval-dock-summary">
            {message.approvalKind === 'Command'
              ? message.detail
              : message.content}
          </span>
          <ChevronDown size={15} />
        </button>
        {collapsed &&
          (message.approvalKind === 'UserInput' ? (
            <Button size="sm" onClick={onToggle} disabled={disabled}>
              Answer
            </Button>
          ) : (
            <div className="approval-dock-quick-actions">
              {choices.includes('Decline') && (
                <Button
                  data-approval-choice="Decline"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    onResolve('Decline')
                  }}
                  disabled={disabled}
                >
                  Deny
                </Button>
              )}
              {choices.includes('Accept') && (
                <Button
                  data-approval-choice="Accept"
                  size="sm"
                  onClick={() => {
                    onResolve('Accept')
                  }}
                  disabled={disabled}
                >
                  Allow once
                </Button>
              )}
            </div>
          ))}
      </div>
      {disabled && (
        <p className="approval-disconnected" role="status">
          Reconnect to the daemon before answering this request.
        </p>
      )}
      <div className="approval-dock-content" hidden={collapsed}>
        {message.approvalKind === 'UserInput' ? (
          <UserInputRequest
            message={message}
            provider={provider}
            questions={message.userQuestions ?? []}
            onSubmit={onUserInput}
            disabled={disabled}
          />
        ) : (
          <ApprovalRequest
            message={message}
            provider={provider}
            desktop={desktop}
            onResolve={onResolve}
            disabled={disabled}
          />
        )}
      </div>
    </section>
  )
}

function UserInputRequest({
  message,
  provider,
  questions,
  onSubmit,
  disabled,
}: {
  message: Message
  provider: Conversation['provider']
  questions: UserInputQuestion[]
  onSubmit: (answers: Record<string, string[]>) => void
  disabled: boolean
}) {
  const [selected, setSelected] = useState<Record<string, string[]>>({})
  const [other, setOther] = useState<Record<string, string>>({})
  const answers = Object.fromEntries(
    questions.map((question) => {
      const chosen = selected[question.id] ?? []
      const note = other[question.id]?.trim()
      const freeform = note
        ? [
            question.is_other && question.options.length > 0
              ? `user_note: ${note}`
              : note,
          ]
        : []
      return [question.id, [...chosen, ...freeform]]
    }),
  )
  const canSubmit =
    questions.length > 0 &&
    questions.every((question) => (answers[question.id]?.length ?? 0) > 0)

  function toggle(questionId: string, label: string, checked: boolean) {
    setSelected((current) => {
      const values = current[questionId] ?? []
      return {
        ...current,
        [questionId]: checked
          ? [...values, label]
          : values.filter((value) => value !== label),
      }
    })
  }

  return (
    <section
      className="approval-request user-input-request"
      aria-label={`${provider} asks for input`}
    >
      <div className="approval-topline">
        <div className="approval-icon">
          <CircleHelp size={16} />
        </div>
        <div>
          <h2>{message.content}</h2>
          <p>Answer these questions to continue the conversation.</p>
        </div>
        <span className="approval-badge">Needs input</span>
      </div>
      {questions.map((question) => (
        <fieldset className="user-input-question" key={question.id}>
          <legend>
            <span>{question.header}</span>
            {question.is_secret && (
              <span className="secret-label">Private answer</span>
            )}
          </legend>
          <p>{question.question}</p>
          {question.options.map((option) => (
            <label className="user-input-option" key={option.label}>
              <input
                type="checkbox"
                checked={(selected[question.id] ?? []).includes(option.label)}
                onChange={(event) => {
                  toggle(question.id, option.label, event.target.checked)
                }}
                disabled={disabled}
              />
              <span>
                <strong>{option.label}</strong>
                <small>{option.description}</small>
              </span>
            </label>
          ))}
          {(question.is_other || question.options.length === 0) && (
            <label className="user-input-other">
              {question.is_other ? 'Other answer' : 'Your answer'}
              <input
                type={question.is_secret ? 'password' : 'text'}
                autoComplete="off"
                aria-label={`${question.is_other ? 'Other answer' : 'Your answer'} for ${question.header}`}
                value={other[question.id] ?? ''}
                onChange={(event) => {
                  setOther((current) => ({
                    ...current,
                    [question.id]: event.target.value,
                  }))
                }}
                disabled={disabled}
              />
            </label>
          )}
        </fieldset>
      ))}
      {questions.length === 0 ? (
        <div className="approval-resolved">
          This input request has no supported questions. Maitty left it
          unanswered.
        </div>
      ) : (
        <div className="approval-actions user-input-actions">
          <span className="approval-scope">
            Answers are sent only to this {provider} request.
          </span>
          <Button
            size="sm"
            onClick={() => {
              onSubmit(answers)
            }}
            disabled={disabled || !canSubmit}
          >
            Send answers
          </Button>
        </div>
      )}
    </section>
  )
}

const TOOL_LABELS: Record<string, { done: string; running: string }> = {
  commandExecution: { done: 'Ran a command', running: 'Running a command' },
  fileChange: { done: 'Edited files', running: 'Editing files' },
  mcpToolCall: { done: 'Used a tool', running: 'Using a tool' },
  webSearch: { done: 'Searched the web', running: 'Searching the web' },
}

/** The daemon reports a tool by its internal name; show it in plain words. */
function toolLabel(kind: string, running: boolean): string {
  const known = TOOL_LABELS[kind]
  if (known) return running ? known.running : known.done
  if (!/^[a-z]+(?:[A-Z][a-z0-9]*)+$/.test(kind)) return kind
  const words = kind.replace(/([A-Z])/g, ' $1').toLowerCase()
  return words.charAt(0).toUpperCase() + words.slice(1)
}

function ActivityGroup({
  messages,
  provider,
}: {
  messages: Message[]
  provider: Conversation['provider']
}) {
  const [expanded, setExpanded] = useState(false)
  const statePriority = [
    'Completed',
    'Interrupted',
    'Failed',
    'Running',
  ] as const
  const activityState = statePriority.reduce<(typeof statePriority)[number]>(
    (current, state) =>
      messages.some(
        (message) => (message.activityState ?? 'Completed') === state,
      )
        ? state
        : current,
    'Completed',
  )
  const latestActivity = messages.at(-1)
  const runningActivity = messages.findLast(
    (message) => message.activityState === 'Running',
  )
  const summary =
    activityState === 'Running'
      ? toolLabel(
          runningActivity?.content ?? latestActivity?.content ?? 'Tool',
          true,
        )
      : messages.length === 1
        ? toolLabel(latestActivity?.content ?? 'Tool activity', false)
        : `${String(messages.length)} actions ${activityState.toLowerCase()}`
  const preview = (runningActivity?.detail ?? '')
    .split(/\r?\n/)
    .filter(Boolean)
    .slice(-ACTIVITY_PREVIEW_LINE_COUNT)
  const stateIcon =
    activityState === 'Running' ? (
      <StatusMark status="working" />
    ) : activityState === 'Failed' || activityState === 'Interrupted' ? (
      <X size={13} />
    ) : (
      <Check size={13} />
    )
  return (
    <section
      className={`activity-row activity-${activityState.toLowerCase()} ${expanded ? 'is-expanded' : ''}`}
      aria-label={`${provider} tool activity`}
    >
      <button
        className="activity-trigger"
        type="button"
        onClick={() => {
          setExpanded(!expanded)
        }}
        aria-expanded={expanded}
        aria-label={`${provider} · ${summary} · ${activityState}`}
      >
        <span className="activity-marker" aria-label={activityState}>
          {stateIcon}
        </span>
        <span className="activity-label">{summary}</span>
        <span className="activity-count">
          {messages.length} {messages.length === 1 ? 'step' : 'steps'}
        </span>
        {activityState !== 'Completed' && (
          <span className="activity-state">{activityState}</span>
        )}
        <ChevronRight size={12} />
      </button>
      {!expanded && activityState === 'Running' && preview.length > 0 && (
        <pre className="activity-preview">{preview.join('\n')}</pre>
      )}
      {expanded && (
        <div className="activity-expanded">
          {messages.map((message) => (
            <article className="activity-step" key={message.id}>
              <div className="activity-step-heading">
                <strong>
                  {toolLabel(
                    message.content,
                    message.activityState === 'Running',
                  )}
                </strong>
                <span
                  className={`activity-state activity-state-${(message.activityState ?? 'Completed').toLowerCase()}`}
                >
                  {message.activityState ?? 'Completed'}
                </span>
              </div>
              {message.detail ? (
                message.content === 'fileChange' ? (
                  <DiffText text={message.detail} />
                ) : (
                  <pre>{message.detail}</pre>
                )
              ) : (
                <span className="activity-empty">No output captured.</span>
              )}
            </article>
          ))}
        </div>
      )}
    </section>
  )
}

function ApprovalRequest({
  message,
  provider,
  desktop,
  onResolve,
  disabled,
}: {
  message: Message
  provider: Conversation['provider']
  desktop: boolean
  onResolve: (
    decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel',
  ) => void
  disabled: boolean
}) {
  const choices =
    message.approvalChoices ??
    (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel'])
  return (
    <section
      className={`approval-request ${message.resolved ? 'is-resolved' : ''}`}
      aria-label="Approval request"
    >
      <div className="approval-topline">
        <div className="approval-icon">
          <ShieldCheck size={16} />
        </div>
        <div>
          <h2>{message.content}</h2>
          <p>
            {message.approvalReason ??
              (message.approvalKind === 'FileChange'
                ? `Review the requested file change before allowing ${provider} to continue.`
                : message.approvalKind === 'Permission'
                  ? 'Allowing grants this exact permission profile for this turn only.'
                  : `${provider} wants to run this command in the project folder.`)}
          </p>
        </div>
        <span className="approval-badge">
          {message.resolved ? 'Resolved' : 'Needs approval'}
        </span>
      </div>
      {message.approvalKind === 'Command' && (
        <code className="approval-command">
          <HighlightedCode
            code={message.detail ?? ''}
            language={commandLanguage(message.detail)}
          />
        </code>
      )}
      {message.approvalKind === 'FileChange' && message.approvalScope && (
        <p className="approval-scope">
          Requested write access: <code>{message.approvalScope}</code>
        </p>
      )}
      {message.approvalKind === 'Permission' && message.approvalScope && (
        <p className="approval-scope">
          Requested from: <code>{message.approvalScope}</code>
        </p>
      )}
      {message.approvalKind === 'Permission' && message.approvalPermissions && (
        <pre className="approval-permissions">
          {JSON.stringify(
            message.approvalPermissions,
            null,
            PERMISSION_JSON_INDENT_SPACES,
          )}
        </pre>
      )}
      {message.approvalChanges?.map((change) => (
        <section className="approval-change" key={change.path}>
          <strong>{change.path}</strong>
          <pre>{change.diff}</pre>
        </section>
      ))}
      {!message.resolved ? (
        choices.length > 0 ? (
          <div className="approval-actions">
            {choices.includes('Decline') && (
              <Button
                data-approval-choice="Decline"
                variant="ghost"
                size="sm"
                onClick={() => {
                  onResolve('Decline')
                }}
                disabled={disabled}
              >
                Decline
              </Button>
            )}
            <div>
              {choices.includes('Cancel') && (
                <Button
                  data-approval-choice="Cancel"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    onResolve('Cancel')
                  }}
                  disabled={disabled}
                >
                  Decline and stop turn
                </Button>
              )}
              {choices.includes('AcceptForSession') && (
                <Button
                  data-approval-choice="AcceptForSession"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    onResolve('AcceptForSession')
                  }}
                  disabled={disabled}
                >
                  {desktop ? 'Allow this session' : 'Allow session'}
                </Button>
              )}
              {choices.includes('Accept') && (
                <Button
                  data-approval-choice="Accept"
                  size="sm"
                  onClick={() => {
                    onResolve('Accept')
                  }}
                  disabled={disabled}
                >
                  <Check size={14} />
                  Allow once
                </Button>
              )}
            </div>
          </div>
        ) : (
          <div className="approval-resolved">
            {message.approvalKind === 'FileChange' &&
            (message.approvalChanges?.length ?? 0) === 0
              ? `Waiting for ${provider} to send the file diff. Approval is disabled until the requested change is visible.`
              : 'No supported decision is available for this request. Maitty has left it unanswered.'}
          </div>
        )
      ) : (
        <div className="approval-resolved">
          <CheckCheck size={14} />
          {desktop
            ? `Decision sent to ${provider}`
            : 'Decision recorded in local preview'}
        </div>
      )}
    </section>
  )
}

function StreamingRow({ provider }: { provider: Conversation['provider'] }) {
  return (
    <div className="streaming-row" role="status">
      <StatusMark status="working" />
      <span>{provider} is working</span>
      <span className="stream-caret" aria-hidden="true" />
    </div>
  )
}

function RecoveryNotice({
  conversation,
  desktop,
  disabled,
  pending,
  onResume,
}: {
  conversation: Conversation
  desktop: boolean
  disabled: boolean
  pending: boolean
  onResume: () => void
}) {
  const canResumeInChat = desktop && conversation.execution === 'Structured'
  return (
    <div className="recovery-notice" role="alert">
      <CircleHelp size={15} />
      <span>
        {canResumeInChat
          ? `${conversation.provider} stopped before the turn finished. Resume this exact session; Maitty will not replay the interrupted turn.`
          : conversation.execution === 'Native'
            ? 'This terminal session needs recovery. Reopen it from the terminal app.'
            : 'This conversation needs recovery. Reconnect to the daemon before continuing.'}
      </span>
      {canResumeInChat && (
        <Button
          variant="outline"
          size="sm"
          onClick={onResume}
          disabled={disabled}
        >
          {pending ? 'Resuming…' : 'Resume session'}
        </Button>
      )}
    </div>
  )
}

function EmptyConversation({ conversation }: { conversation: Conversation }) {
  return (
    <div className="empty-conversation">
      <h2>
        New {conversation.provider} session in {conversation.project}
      </h2>
      <p>
        <span className="conversation-branch">{conversation.branch}</span> ·{' '}
        {conversation.permission.split(' · ')[0]}. Send a message to start. It
        keeps running in maittyd if you close this window.
      </p>
    </div>
  )
}

function Composer({
  value,
  status,
  conversation,
  desktop,
  connected,
  cancelling,
  deliveryNotice,
  deliveryBlocked,
  onChange,
  onKeyDown,
  onSend,
  denyDecision,
  denyQueued,
  onDenyAndSend,
  onCancel,
  onRetry,
  onDiscard,
  onCheckDelivery,
}: {
  value: string
  status: SessionStatus
  conversation: Conversation
  desktop: boolean
  connected: boolean
  cancelling: boolean
  deliveryNotice: 'not-delivered' | 'unknown' | null
  deliveryBlocked: boolean
  onChange: (value: string) => void
  onKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void
  onSend: () => void
  denyDecision: 'Cancel' | 'Decline' | null
  denyQueued: boolean
  onDenyAndSend: () => void
  onCancel: () => void
  onRetry: () => void
  onDiscard: () => void
  onCheckDelivery: () => void
}) {
  const nativeOwner = conversation.execution === 'Native'
  const denying = denyDecision !== null
  const busy =
    status === 'working' ||
    (status === 'needs-you' && !denying) ||
    nativeOwner ||
    denyQueued
  const composerNote = denyQueued
    ? {
        tone: 'muted' as const,
        text: 'Request denied. Your message sends when this turn ends.',
      }
    : denying
      ? {
          tone: 'muted' as const,
          text: `Typing a message denies the request${denyDecision === 'Cancel' ? ' and stops this turn' : ''}, then sends your instructions instead.`,
        }
      : !connected
        ? {
            tone: 'muted' as const,
            text: 'Your draft is saved. Nothing is sent until you press Send after reconnecting.',
          }
        : status === 'recovery'
          ? {
              tone: 'danger' as const,
              text: 'Resume the session to send messages. Your draft is kept.',
            }
          : nativeOwner
            ? {
                tone: 'muted' as const,
                text: `Native ${conversation.provider} owns this conversation until you continue in chat.`,
              }
            : status === 'working'
              ? {
                  tone: 'muted' as const,
                  text: 'Working · your next message waits until this turn ends',
                }
              : null
  return (
    <div className="composer-wrap">
      {deliveryNotice && (
        <div className={`delivery-notice is-${deliveryNotice}`} role="alert">
          <CircleAlert size={15} />
          <span>
            {deliveryNotice === 'unknown'
              ? 'Delivery is unknown. Maitty will not retry this prompt. Check the native terminal before continuing.'
              : 'Message was not delivered. Your draft is still here and safe to retry.'}
          </span>
          {deliveryNotice === 'not-delivered' ? (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={onRetry}
                disabled={!connected || busy}
              >
                Retry
              </Button>
              <Button variant="ghost" size="sm" onClick={onDiscard}>
                Discard draft
              </Button>
            </>
          ) : (
            <Button
              variant="outline"
              size="sm"
              onClick={onCheckDelivery}
              disabled={!connected}
            >
              Check delivery
            </Button>
          )}
        </div>
      )}
      {composerNote && (
        <p
          className={`composer-note ${composerNote.tone === 'danger' ? 'is-danger' : ''}`}
          role="status"
        >
          {composerNote.text}
        </p>
      )}
      <div className="composer-box">
        <Textarea
          value={value}
          onChange={(event) => {
            onChange(event.target.value)
          }}
          onKeyDown={onKeyDown}
          placeholder={
            nativeOwner
              ? status === 'recovery'
                ? 'Recover this native session before chatting…'
                : 'Return to chat from the Terminal panel to continue…'
              : !connected
                ? 'Reconnect before sending. Your draft stays here.'
                : denying
                  ? 'Type instructions to deny this request and send them instead…'
                  : busy
                    ? 'Wait for this turn to finish…'
                    : deliveryBlocked
                      ? 'Check the native terminal before continuing.'
                      : `Ask ${conversation.provider} to work on this project…`
          }
          aria-label={`Message ${conversation.provider}`}
          disabled={busy || deliveryBlocked}
          rows={2}
        />
        <div className="composer-actions">
          <div className="composer-context">
            <span className="provider-mini">
              <span className="codex-glyph" aria-hidden="true">
                ●
              </span>
              {conversation.provider} · {conversation.model}
            </span>
            <span>{conversation.permission.split(' · ')[0]}</span>
          </div>
          <div className="composer-send">
            {((desktop && (status === 'working' || status === 'needs-you')) ||
              (!desktop && status === 'working')) && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    aria-label="Stop"
                    onClick={onCancel}
                    disabled={cancelling}
                  >
                    <Square size={12} fill="currentColor" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  {cancelling ? 'Stopping…' : 'Stop this turn'} ·{' '}
                  {shortcutModifier(navigator.userAgent)} .
                </TooltipContent>
              </Tooltip>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  size="icon-sm"
                  aria-label={denying ? 'Deny and send' : 'Send message'}
                  className={denying ? 'send-deny' : undefined}
                  onClick={denying ? onDenyAndSend : onSend}
                  disabled={
                    !value.trim() || busy || !connected || deliveryBlocked
                  }
                >
                  <ArrowUp />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {denying ? 'Deny the request and send' : 'Send'} · Enter
              </TooltipContent>
            </Tooltip>
          </div>
        </div>
      </div>
    </div>
  )
}

const PANEL_TABS: { id: PanelTab; label: string }[] = [
  { id: 'diff', label: 'Changes' },
  { id: 'files', label: 'Files' },
  { id: 'details', label: 'Context' },
]

function ContextPanel({
  tab,
  conversation,
  desktop,
  onTab,
  onClose,
}: {
  tab: PanelTab
  conversation: Conversation
  desktop: boolean
  onTab: (tab: PanelTab) => void
  onClose: () => void
}) {
  const reviewStat = useReviewStat(conversation)

  function handleTabKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
    const tabs = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
    )
    const currentIndex = tabs.findIndex(
      (item) => item === document.activeElement,
    )
    const selectedIndex = PANEL_TABS.findIndex((item) => item.id === tab)
    const targetIndex =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? tabs.length - 1
          : (Math.max(currentIndex, selectedIndex) +
              (event.key === 'ArrowRight' ? 1 : -1) +
              tabs.length) %
            tabs.length
    const target = tabs[targetIndex]
    if (!target) return
    event.preventDefault()
    const targetTab = PANEL_TABS[targetIndex]
    if (targetTab) onTab(targetTab.id)
    target.focus()
  }

  return (
    <aside
      className={`context-panel is-${tab}`}
      aria-label="Conversation details"
    >
      <div className="context-panel-heading">
        <div
          className="panel-tabs"
          role="tablist"
          aria-label="Conversation panels"
          onKeyDown={handleTabKeyDown}
        >
          {PANEL_TABS.map((item) => (
            <button
              key={item.id}
              className={tab === item.id ? 'active' : ''}
              type="button"
              role="tab"
              aria-selected={tab === item.id}
              tabIndex={tab === item.id ? 0 : -1}
              onClick={() => {
                onTab(item.id)
              }}
            >
              {item.label}
              {item.id === 'diff' && reviewStat && (
                <span className="panel-tab-count">{reviewStat.files}</span>
              )}
            </button>
          ))}
        </div>
        <Button
          className="panel-close"
          variant="ghost"
          size="sm"
          aria-label="Close details panel"
          onClick={onClose}
        >
          Close <kbd>esc</kbd>
        </Button>
      </div>
      {tab === 'files' ? (
        <div className="context-panel-fill">
          <FilesView conversation={conversation} desktop={desktop} />
        </div>
      ) : (
        <ScrollArea className="context-panel-scroll">
          {tab === 'details' && <DetailsView conversation={conversation} />}
          {tab === 'diff' && (
            <DiffView conversation={conversation} desktop={desktop} />
          )}
        </ScrollArea>
      )}
      {tab === 'diff' && (
        <p className="review-footer">
          Read-only. Ask the agent for edits, or open the file in your editor.
        </p>
      )}
    </aside>
  )
}

function DetailsView({ conversation }: { conversation: Conversation }) {
  return (
    <div className="details-view">
      <div className="detail-status-block">
        <div className="detail-status-head">
          <StatusMark status={conversation.status} />
          <strong>{statusText[conversation.status]}</strong>
          {conversation.updated && (
            <span>Updated {conversation.updated} ago</span>
          )}
        </div>
        <p>
          {conversation.status === 'unknown'
            ? `Last known status: ${conversation.lastKnownStatus ? statusText[conversation.lastKnownStatus] : 'Unavailable'}. Reconnect to confirm.`
            : conversation.status === 'needs-you'
              ? 'The agent is waiting for your decision.'
              : conversation.status === 'working'
                ? 'The agent is making progress on this task.'
                : 'The conversation is saved and ready to continue.'}
        </p>
      </div>
      <DetailSection title="Conversation">
        <DetailRow
          label="Provider"
          value={
            <span className="detail-agent">
              <AgentAvatar
                provider={conversation.provider}
                status={conversation.status}
                small
              />
              {conversation.provider}
            </span>
          }
        />
        <DetailRow label="Model" value={conversation.model} />
        <DetailRow
          label="Native session"
          value={
            conversation.nativePane === null
              ? 'Not linked'
              : `Pane ${String(conversation.nativePane)}`
          }
        />
      </DetailSection>
      <DetailSection title="Project">
        <DetailRow
          label="Folder"
          value={
            <span className="path-value" title={conversation.folder}>
              {conversation.folder}
            </span>
          }
        />
        <DetailRow
          label="Branch"
          value={
            <span className="detail-agent">
              <GitBranch size={13} />
              {conversation.branch}
            </span>
          }
        />
      </DetailSection>
      <DetailSection title="Permissions">
        <div className="permission-summary">
          <ShieldCheck size={15} />
          <span>{conversation.permission}</span>
        </div>
        <p className="permission-note">
          Maitty keeps the provider’s permission settings. Approval requests
          appear in this conversation.
        </p>
      </DetailSection>
      <DetailSection title="Context">
        <div className="context-meter-heading">
          <span>Conversation context</span>
          <span>
            {conversation.context === null
              ? 'Unavailable'
              : `${String(conversation.context)}%`}
          </span>
        </div>
        {conversation.context !== null && (
          <div className="context-meter">
            <span style={{ width: `${String(conversation.context)}%` }} />
          </div>
        )}
      </DetailSection>
      <div className="daemon-note">
        <span className="connection-dot" />
        <div>
          <strong>Owned by Maitty daemon</strong>
          <p>Closing the window will not stop this session.</p>
        </div>
      </div>
    </div>
  )
}

function DetailSection({
  title,
  children,
}: {
  title: string
  children: ReactNode
}) {
  return (
    <section className="detail-section">
      <h3>{title}</h3>
      {children}
    </section>
  )
}

function DetailRow({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <div>{value}</div>
    </div>
  )
}

function isPatchHeader(line: string): boolean {
  return (
    line.startsWith('@@') || line.startsWith('+++') || line.startsWith('---')
  )
}

function DiffView({
  conversation,
  desktop,
}: {
  conversation: Conversation
  desktop: boolean
}) {
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null)
  const diffQuery = useQuery({
    queryKey: ['conversation-diff', conversation.id, conversation.generation],
    queryFn: () => loadConversationDiff(conversation),
    enabled: desktop && conversation.isGitRepository !== false,
    retry: false,
    // A failed diff waits for the user's Retry instead of refetching on every mount.
    retryOnMount: false,
    staleTime: 1500,
    refetchInterval: (query) =>
      conversation.status === 'working'
        ? conversationDiffRefreshInterval(desktop, query.state.status)
        : false,
  })
  const files = useMemo(
    () =>
      desktop
        ? (diffQuery.data?.files ?? [])
        : previewDiff.map((file) => ({
            path: file.file,
            additions: file.added,
            deletions: file.removed,
            patch: file.lines.join('\n'),
            binary: false,
          })),
    [desktop, diffQuery.data?.files],
  )
  const selectedFileIndex = Math.max(
    0,
    files.findIndex((file) => file.path === selectedFilePath),
  )
  const selectedFile = files[selectedFileIndex]
  const selectRelativeFile = useCallback(
    (direction: -1 | 1) => {
      const currentIndex = Math.max(
        0,
        files.findIndex((file) => file.path === selectedFilePath),
      )
      const nextIndex = Math.max(
        0,
        Math.min(files.length - 1, currentIndex + direction),
      )
      const nextFile = files[nextIndex]
      if (nextFile) setSelectedFilePath(nextFile.path)
    },
    [files, selectedFilePath],
  )
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)
        return
      if (event.key !== '[' && event.key !== ']') return
      if (files.length <= 1) return
      const target = event.target instanceof HTMLElement ? event.target : null
      const isTextEntry =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        Boolean(target?.isContentEditable)
      if (isTextEntry || target?.closest('[role="dialog"], .terminal-screen'))
        return
      event.preventDefault()
      selectRelativeFile(event.key === ']' ? 1 : -1)
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [files.length, selectRelativeFile])

  if (desktop && conversation.isGitRepository === false)
    return (
      <div className="panel-empty">
        <GitBranch size={18} />
        <h2>Changes need Git</h2>
        <p>
          This folder isn’t a Git repository. Initialize Git here to view file
          changes.
        </p>
      </div>
    )
  if (desktop && diffQuery.isLoading)
    return (
      <div className="panel-empty" role="status">
        <LoaderCircle size={18} />
        <h2>Loading changes</h2>
      </div>
    )
  if (desktop && diffQuery.isError)
    return (
      <div className="panel-empty">
        <CircleAlert size={18} />
        <h2>Couldn?t load changes</h2>
        <p>
          Check that the project folder exists and is a Git repository, then
          retry.
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            void diffQuery.refetch()
          }}
        >
          Retry
        </Button>
      </div>
    )
  if (files.length === 0)
    return (
      <div className="panel-empty">
        <FileCode2 size={18} />
        <h2>No changes yet</h2>
        <p>Changes made in this conversation?s project will appear here.</p>
      </div>
    )

  const additions = desktop
    ? (diffQuery.data?.additions ?? 0)
    : files.reduce((total, file) => total + file.additions, 0)
  const deletions = desktop
    ? (diffQuery.data?.deletions ?? 0)
    : files.reduce((total, file) => total + file.deletions, 0)
  const totalFiles = desktop
    ? (diffQuery.data?.total_files ?? files.length)
    : files.length
  if (!selectedFile)
    return (
      <div className="panel-empty">
        <FileCode2 size={18} />
        <h2>No changes yet</h2>
        <p>Changes made in this conversation?s project will appear here.</p>
      </div>
    )
  return (
    <div className="diff-view">
      <div className="diff-summary">
        <strong>
          {totalFiles} {totalFiles === 1 ? 'file' : 'files'} changed
        </strong>
        <span>
          <i>+{additions}</i>{' '}
          <b>
            {'?'}
            {deletions}
          </b>
        </span>
      </div>
      {desktop && diffQuery.data?.truncated && (
        <p className="diff-truncated" role="status">
          Showing a partial diff. Some large or unsupported files were omitted.
        </p>
      )}
      <div className="diff-file-navigation">
        <span role="status" aria-live="polite" aria-atomic="true">
          File {selectedFileIndex + 1} of {files.length}: {selectedFile.path}
        </span>
        <div>
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label="Previous changed file"
            onClick={() => {
              selectRelativeFile(-1)
            }}
            disabled={selectedFileIndex === 0}
          >
            <ChevronDown className="diff-previous-icon" />
          </Button>
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label="Next changed file"
            onClick={() => {
              selectRelativeFile(1)
            }}
            disabled={selectedFileIndex === files.length - 1}
          >
            <ChevronDown className="diff-next-icon" />
          </Button>
        </div>
      </div>
      <div className="diff-file-list" role="group" aria-label="Changed files">
        {files.map((file, index) => (
          <button
            className={`diff-file-choice ${index === selectedFileIndex ? 'is-selected' : ''}`}
            type="button"
            key={file.path}
            aria-pressed={index === selectedFileIndex}
            aria-label={`Select changed file ${file.path}`}
            onClick={() => {
              setSelectedFilePath(file.path)
            }}
          >
            <FileCode2 size={13} />
            <span>{file.path}</span>
            <small>
              <i>+{file.additions}</i>{' '}
              <b>
                {'?'}
                {file.deletions}
              </b>
            </small>
          </button>
        ))}
      </div>
      <section className="diff-file">
        <div className="diff-file-head">
          <FileCode2 size={13} />
          <span>
            {selectedFile.path.split('/').at(-1) ?? selectedFile.path}
          </span>
          <small>
            <i>+{selectedFile.additions}</i>{' '}
            <b>
              {'?'}
              {selectedFile.deletions}
            </b>
          </small>
        </div>
        <div className="diff-file-path">{selectedFile.path}</div>
        {selectedFile.binary ? (
          <p className="diff-binary">Binary file changed</p>
        ) : (
          <pre>
            {selectedFile.patch.split('\n').map((line, index) => (
              <span
                className={
                  line.startsWith('+')
                    ? 'diff-added'
                    : line.startsWith('-')
                      ? 'diff-removed'
                      : 'diff-context'
                }
                key={`${selectedFile.path}:${String(index)}`}
              >
                {line.length > 0 && !isPatchHeader(line) ? (
                  <>
                    {line.slice(0, 1)}
                    <HighlightedLine
                      line={line.slice(1)}
                      path={selectedFile.path}
                    />
                  </>
                ) : (
                  line
                )}
              </span>
            ))}
          </pre>
        )}
      </section>
      {!desktop && (
        <Button
          variant="outline"
          className="open-editor-button"
          onClick={() =>
            toast.message('Open in editor is not connected in preview')
          }
        >
          <ArrowUpRight size={14} />
          Open in editor
        </Button>
      )}
    </div>
  )
}
function TerminalView({
  conversation,
  desktop,
  fontSize,
}: {
  conversation: Conversation
  desktop: boolean
  fontSize: number
}) {
  if (desktop)
    return <DaemonTerminalView conversation={conversation} desktop={desktop} fontSize={fontSize} />
  return (
    <div className="terminal-view">
      <div className="terminal-heading">
        <span className="terminal-live-dot" />
        Native session · pane {conversation.nativePane || 'pending'}
      </div>
      <pre className="terminal-output">
        <span className="terminal-dim">~/maitty · {conversation.branch}</span>
        {'\n'}
        <span className="terminal-green">$ cargo test branch_view_tests</span>
        {'\n'}running 8 tests{'\n'}
        <span className="terminal-ok">
          test branch_view::renders_status ... ok
        </span>
        {'\n'}test branch_view::opens_switcher ... ok{'\n'}test result: ok. 8
        passed{'\n'}
        <span className="terminal-prompt">❯</span>{' '}
        <span className="terminal-cursor" />
      </pre>
      <div className="terminal-footnote">
        <Activity size={13} />
        Preview terminal output · interactive PTY bridge not connected yet
      </div>
    </div>
  )
}

function AgentAvatar({
  provider,
  status,
  small = false,
}: {
  provider: Conversation['provider']
  status: SessionStatus
  small?: boolean
}) {
  const glyph = provider === 'Codex' ? '◆' : '✳'
  return (
    <span
      className={`agent-avatar ${provider === 'Claude' ? 'is-claude' : 'is-codex'} ${small ? 'is-small' : ''} status-${status}`}
      aria-label={provider}
    >
      {glyph}
    </span>
  )
}

function NewConversationDialog({
  open,
  project,
  onProjectChange,
  provider,
  onProviderChange,
  kind,
  onKindChange,
  permissions,
  onPermissionsChange,
  claudeAvailable,
  desktop,
  onChooseFolder,
  isChoosingFolder,
  onClose,
  onCreate,
  disabled,
  isCreating,
  recentProjects,
}: {
  open: boolean
  project: string
  onProjectChange: (value: string) => void
  provider: 'Codex' | 'Claude'
  onProviderChange: (provider: 'Codex' | 'Claude') => void
  kind: 'chat' | 'terminal'
  onKindChange: (kind: 'chat' | 'terminal') => void
  permissions: SessionPermissions
  onPermissionsChange: (permissions: SessionPermissions) => void
  claudeAvailable: boolean
  desktop: boolean
  onChooseFolder: () => void
  isChoosingFolder: boolean
  onClose: () => void
  onCreate: () => void
  disabled: boolean
  isCreating: boolean
  recentProjects: Conversation[]
}) {
  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="new-conversation-dialog">
        <DialogHeader>
          <DialogTitle>New conversation</DialogTitle>
          <DialogDescription>
            Choose a project and agent. The agent will use its saved provider
            configuration.
          </DialogDescription>
        </DialogHeader>
        <div className="dialog-field">
          <label htmlFor="project-folder">Project folder</label>
          <div className="project-folder-input">
            <input
              id="project-folder"
              value={project}
              onChange={(event) => {
                onProjectChange(event.target.value)
              }}
              placeholder="Home folder unless you choose one"
              autoFocus
            />
            <Button
              type="button"
              variant="outline"
              onClick={onChooseFolder}
              disabled={!desktop || disabled || isChoosingFolder}
              aria-label="Browse for project folder"
            >
              <FolderOpen size={14} />
              {isChoosingFolder ? 'Opening…' : 'Browse'}
            </Button>
          </div>
        </div>
        {recentProjects.length > 0 && (
          <div
            className="dialog-recent"
            role="group"
            aria-label="Recent projects"
          >
            <span className="dialog-caps">Recent</span>
            {recentProjects.map((item) => (
              <button
                key={item.folder}
                type="button"
                className={`dialog-recent-row ${project === item.folder ? 'is-selected' : ''}`}
                onClick={() => {
                  onProjectChange(item.folder)
                }}
              >
                <FolderOpen size={14} aria-hidden="true" />
                <strong>{item.project}</strong>
                <code>{item.folder}</code>
              </button>
            ))}
          </div>
        )}
        <div
          className="dialog-provider-options"
          role="group"
          aria-label="Agent"
        >
          {(['Codex', 'Claude'] as const).map((choice) => (
            <button
              key={choice}
              type="button"
              className={`dialog-provider ${provider === choice ? 'is-selected' : ''}`}
              disabled={choice === 'Claude' && !claudeAvailable}
              aria-pressed={provider === choice}
              onClick={() => {
                onProviderChange(choice)
              }}
            >
              <AgentAvatar provider={choice} status="idle" />
              <div>
                <strong>{choice}</strong>
                <span>
                  {choice === 'Claude'
                    ? claudeAvailable
                      ? 'Claude Code CLI and saved sign-in.'
                      : 'Claude Code CLI was not found.'
                    : 'Codex CLI and saved sign-in.'}
                </span>
              </div>
              {provider === choice && <Check size={15} />}
            </button>
          ))}
        </div>
        {desktop && (
          <div
            className="dialog-provider-options"
            role="group"
            aria-label="Session type"
          >
            {(
              [
                ['chat', 'Chat', 'Messages and tool output in this window.'],
                [
                  'terminal',
                  'Terminal',
                  'The agent in a terminal, also open in the terminal app.',
                ],
              ] as const
            ).map(([choice, name, note]) => (
              <button
                key={choice}
                type="button"
                className={`dialog-provider ${kind === choice ? 'is-selected' : ''}`}
                aria-pressed={kind === choice}
                onClick={() => {
                  onKindChange(choice)
                }}
              >
                <div>
                  <strong>{name}</strong>
                  <span>{note}</span>
                </div>
                {kind === choice && <Check size={15} />}
              </button>
            ))}
          </div>
        )}
        {kind === 'chat' && (
          <div
            className="dialog-permissions"
            role="group"
            aria-label="Permissions"
          >
            <span className="dialog-caps">Permissions</span>
            {(
              [
                ['read-only', 'Reads files and asks before anything else.'],
                [
                  'workspace-write',
                  'Edits files in the project and asks before the rest.',
                ],
                [
                  'full-access',
                  'Never asks and is not sandboxed. Use with care.',
                ],
              ] as const
            ).map(([value, help]) => (
              <button
                key={value}
                type="button"
                className={`dialog-permission ${permissions === value ? 'is-selected' : ''} ${value === 'full-access' ? 'is-danger' : ''}`}
                aria-pressed={permissions === value}
                onClick={() => {
                  onPermissionsChange(value)
                }}
              >
                <strong>{PERMISSION_LABELS[value]}</strong>
                <span>{help}</span>
              </button>
            ))}
          </div>
        )}
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={onCreate} disabled={disabled}>
            {isCreating ? (
              <LoaderCircle size={14} className="animate-spin" />
            ) : (
              <Plus size={14} />
            )}
            {isCreating
              ? 'Starting session…'
              : kind === 'terminal'
                ? 'Start terminal session'
                : 'Start conversation'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function CommandDialog({
  open,
  conversations,
  onClose,
  onSelect,
  onNew,
  onSettings,
  onKeyboard,
  onPanel,
  primaryModifier,
  onSelectInTerminal,
}: {
  open: boolean
  conversations: Conversation[]
  onClose: () => void
  onSelect: (id: string) => void
  onNew: () => void
  onSettings: () => void
  onKeyboard: () => void
  onPanel: (tab: PanelTab | 'terminal') => void
  primaryModifier: '⌘' | 'Ctrl'
  onSelectInTerminal: (id: string) => void
}) {
  const [query, setQuery] = useState('')
  const matchingConversations = conversations
    .filter((conversation) =>
      `${conversation.title} ${conversation.project} ${conversation.status}`
        .toLowerCase()
        .includes(query.trim().toLowerCase()),
    )
    .sort(
      (left, right) =>
        Number(right.status === 'needs-you' || right.status === 'recovery') -
        Number(left.status === 'needs-you' || left.status === 'recovery'),
    )
  const actionQuery = query.trim().toLowerCase()
  const actions = [
    {
      icon: <MessageSquarePlus />,
      title: 'New conversation',
      searchText: 'new conversation',
      onClick: onNew,
    },
    {
      icon: <FileCode2 />,
      title: 'Show changes',
      searchText: 'changes diff',
      onClick: () => {
        onPanel('diff')
      },
    },
    {
      icon: <PanelRight />,
      title: 'Show conversation details',
      searchText: 'details context',
      onClick: () => {
        onPanel('details')
      },
    },
    {
      icon: <TerminalSquare />,
      title: 'Show native terminal',
      searchText: 'terminal',
      onClick: () => {
        onPanel('terminal')
      },
    },
    {
      icon: <Settings2 />,
      title: 'Open settings',
      searchText: 'settings theme appearance notifications daemon providers',
      onClick: onSettings,
    },
    {
      icon: <Keyboard />,
      title: 'Keyboard shortcuts',
      searchText: 'keyboard shortcuts help keys',
      onClick: onKeyboard,
    },
  ].filter((action) => !actionQuery || action.searchText.includes(actionQuery))

  function moveFocus(event: KeyboardEvent<HTMLDivElement>) {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      const focusedItem = document.activeElement?.closest<HTMLButtonElement>(
        '.command-item[data-session-id]',
      )
      if (!focusedItem) return
      event.preventDefault()
      onSelectInTerminal(focusedItem.dataset.sessionId ?? '')
      return
    }
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
    const buttons = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>('.command-item'),
    )
    if (buttons.length === 0) return
    event.preventDefault()
    const currentIndex = buttons.findIndex(
      (button) => button === document.activeElement,
    )
    const step = event.key === 'ArrowDown' ? 1 : -1
    const nextIndex =
      currentIndex === -1
        ? step === 1
          ? 0
          : buttons.length - 1
        : (currentIndex + step + buttons.length) % buttons.length
    buttons[nextIndex]?.focus()
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="command-dialog" onKeyDown={moveFocus}>
        <DialogHeader>
          <DialogTitle className="sr-only">Commands</DialogTitle>
          <DialogDescription className="sr-only">
            Choose an action
          </DialogDescription>
        </DialogHeader>
        <div className="command-search">
          <Search size={16} />
          <input
            autoFocus
            aria-label="Search actions and sessions"
            placeholder="Search threads and actions"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value)
            }}
          />
          <kbd>ESC</kbd>
        </div>
        <div className="command-list">
          {matchingConversations.length > 0 && (
            <>
              <span className="command-group-title">Threads</span>
              {matchingConversations.map((conversation) => (
                <CommandItem
                  key={conversation.id}
                  sessionId={conversation.id}
                  icon={<StatusMark status={conversation.status} />}
                  title={conversation.title}
                  meta={`${conversation.project} · ${statusText[conversation.status]}`}
                  onClick={() => {
                    onSelect(conversation.id)
                  }}
                />
              ))}
            </>
          )}
          {actions.length > 0 && (
            <>
              <span className="command-group-title">Actions</span>
              {actions.map((action) => (
                <CommandItem
                  key={action.title}
                  icon={action.icon}
                  title={action.title}
                  onClick={() => {
                    action.onClick()
                    onClose()
                  }}
                />
              ))}
            </>
          )}
          {matchingConversations.length === 0 && actions.length === 0 && (
            <p className="command-empty">No matching threads or actions.</p>
          )}
        </div>
        <div className="command-footer">
          <Command size={13} />
          Navigate <kbd>↑</kbd>
          <kbd>↓</kbd>
          <span>Choose</span>
          <kbd>↵</kbd>
          <span>Terminal</span>
          <kbd>{primaryModifier}+↵</kbd>
          <span>Close</span>
          <kbd>Esc</kbd>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function KeyboardDialog({
  open,
  onClose,
  primaryModifier,
}: {
  open: boolean
  onClose: () => void
  primaryModifier: '⌘' | 'Ctrl'
}) {
  const shortcuts = [
    ['Open command switcher', primaryModifier, 'K'],
    ['Start a session in a folder', primaryModifier, 'O'],
    ['Move through switcher results', '↑ or ↓', ''],
    ['Open the focused switcher result', 'Enter', ''],
    ['Open the focused conversation in terminal', primaryModifier, 'Enter'],
    ['Close the command switcher', 'Esc', ''],
    ['Open settings', primaryModifier, ','],
    ['New conversation', primaryModifier, 'N'],
    ['Next conversation needing attention', `${primaryModifier}+Shift`, 'A'],
    ['Show or hide Changes', `${primaryModifier}+Shift`, 'D'],
    ['Previous changed file', '[', ''],
    ['Next changed file', ']', ''],
    ['Show or hide Context', primaryModifier, 'I'],
    ['Collapse or expand sidebar', primaryModifier, '\\'],
    ['Switch between chat and terminal', primaryModifier, '`'],
    ['Stop the current turn', primaryModifier, '.'],
    ['Previous conversation', 'Alt', '↑ or K'],
    ['Next conversation', 'Alt', '↓ or J'],
    ['Allow approval once', 'Y or A', ''],
    ['Decline approval', 'N', ''],
    ['Decline and stop turn', 'Shift', 'N'],
    ['Send message', 'Enter', ''],
    ['Add a new line', 'Shift', 'Enter'],
    ['Rename the focused conversation', 'F2', ''],
    ['Show this shortcut list', '?', ''],
  ]

  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="keyboard-dialog">
        <DialogHeader>
          <DialogTitle>Keyboard shortcuts</DialogTitle>
          <DialogDescription>
            Shortcuts available in Maitty Desktop.
          </DialogDescription>
        </DialogHeader>
        <div className="keyboard-shortcut-list">
          {shortcuts.map(([action, modifier, key]) => (
            <div className="keyboard-shortcut-row" key={action}>
              <span>{action}</span>
              <span>
                {modifier &&
                  modifier
                    .split('+')
                    .map((part) => <kbd key={part}>{part}</kbd>)}
                {key && <kbd>{key}</kbd>}
              </span>
            </div>
          ))}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function CommandItem({
  icon,
  title,
  shortcut,
  meta,
  sessionId,
  onClick,
}: {
  icon: ReactNode
  title: string
  shortcut?: string
  meta?: string
  sessionId?: string
  onClick: () => void
}) {
  return (
    <button
      className="command-item"
      type="button"
      data-session-id={sessionId}
      onClick={onClick}
    >
      {icon}
      <span>{title}</span>
      {meta && <small>{meta}</small>}
      {shortcut && <kbd>{shortcut}</kbd>}
    </button>
  )
}

export default App
