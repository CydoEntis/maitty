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
  Check,
  CheckCheck,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  CircleCheck,
  CircleHelp,
  Command,
  Circle,
  CircleDashed,
  FileCode2,
  FolderOpen,
  Archive,
  ArchiveRestore,
  GitBranch,
  Keyboard,
  LoaderCircle,
  MessageSquarePlus,
  MoreHorizontal,
  PanelRight,
  PanelLeft,
  Pin,
  Pencil,
  Plus,
  Diamond,
  RefreshCw,
  Search,
  Send,
  Settings2,
  ShieldCheck,
  Square,
  TerminalSquare,
  X,
} from 'lucide-react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Toaster } from '@/components/ui/sonner'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import { Textarea } from '@/components/ui/textarea'
import { TerminalView as DaemonTerminalView } from './TerminalView'
import { TooltipProvider } from '@/components/ui/tooltip'
import {
  answerUserInput as answerDaemonInput,
  approveRequest,
  cancelTurn as cancelDaemonTurn,
  classifySubmissionFailure,
  createConversation as createDaemonConversation,
  handoffToChat,
  handoffToNative,
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
import { shortcutModifier } from '@/lib/shortcut-modifier'
import { errorMessage } from '@/lib/error-message'
import './App.css'

function focusMainContent(event: MouseEvent<HTMLAnchorElement>) {
  event.preventDefault()
  document.getElementById('conversation-main')?.focus()
}

const PREVIEW_RESPONSE_DELAY_MS = 1500
const PREVIEW_APPROVAL_DELAY_MS = 1700
const PERMISSION_JSON_INDENT_SPACES = 2
const SESSION_REFRESH_INTERVAL_MS = 1500
const HISTORY_REFRESH_INTERVAL_MS = 750
const APPROVAL_COMPACT_HEIGHT_PX = 760
const COMPACT_SIDEBAR_MAX_WIDTH_PX = 899

type PanelTab = 'details' | 'diff'
type SendFailure = { kind: 'not-delivered' | 'unknown'; operation: number }

const statusText: Record<SessionStatus, string> = {
  working: 'Working',
  'needs-you': 'Needs you',
  done: 'Done',
  idle: 'Idle',
  recovery: 'Recovery needed',
  unknown: 'Disconnected',
}

function daemonProtocolMismatch(
  error: unknown,
): { daemonProtocol: number; guiProtocol: number } | null {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : ''
  const versionMismatch =
    /protocol v(\d+), (?:this one v|while this GUI needs v)(\d+)/.exec(message)
  if (!versionMismatch) return null
  const daemonProtocol = Number(versionMismatch[1])
  const guiProtocol = Number(versionMismatch[2])
  return Number.isSafeInteger(daemonProtocol) &&
    Number.isSafeInteger(guiProtocol)
    ? { daemonProtocol, guiProtocol }
    : null
}

function daemonConnectionError(error: unknown): string {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : ''
  const mismatch = daemonProtocolMismatch(error)
  if (mismatch) {
    return `The Maitty service uses protocol v${String(mismatch.daemonProtocol)}, but this GUI needs v${String(mismatch.guiProtocol)}. This GUI will leave the service running while it owns sessions.`
  }
  return message
    ? `Could not connect to the local Maitty service: ${message}`
    : 'Could not connect to the local Maitty service. Check that Maitty is installed, then retry.'
}

function App() {
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
  const conversations = useMemo(() => {
    const current = desktop ? (sessionsQuery.data ?? []) : previewSessions
    const withNames = current.map((conversation) => ({
      ...conversation,
      title:
        sidebarPreferences.renamedTitles[conversation.id] ?? conversation.title,
    }))
    return daemonDisconnected
      ? markConversationsDisconnected(withNames)
      : withNames
  }, [
    desktop,
    daemonDisconnected,
    sessionsQuery.data,
    previewSessions,
    sidebarPreferences.renamedTitles,
  ])
  const [selectedId, setSelectedId] = useState(
    () => readSelectedConversation() ?? (desktop ? '' : 'codex-43'),
  )
  const [savedOpenTabIds, setSavedOpenTabIds] = useState(
    readOpenConversationTabs,
  )
  const [messagesById, setMessagesById] = useState<Record<string, Message[]>>(
    () => structuredClone(previewMessages),
  )
  const [filter, setFilter] = useState('')
  const [drafts, setDrafts] = useState<Record<string, string>>(
    readConversationDrafts,
  )
  const [activePanel, setActivePanel] = useState<PanelTab | null>('diff')
  const [terminalConversationId, setTerminalConversationId] = useState<
    string | null
  >(null)
  const [sidebarDrawerOpen, setSidebarDrawerOpen] = useState(false)
  const [settingsPreferences, setSettingsPreferences] = useState(
    readSettingsPreferences,
  )
  const [sidebarCollapsed, setSidebarCollapsed] = useState(
    settingsPreferences.compactSidebar,
  )
  const [isCompactSidebar, setIsCompactSidebar] = useState(false)
  const [newDialogOpen, setNewDialogOpen] = useState(false)
  const [commandDialogOpen, setCommandDialogOpen] = useState(false)
  const [keyboardDialogOpen, setKeyboardDialogOpen] = useState(false)
  const [settingsDialogOpen, setSettingsDialogOpen] = useState(false)
  const [newProject, setNewProject] = useState('')
  const [newProvider, setNewProvider] = useState<'Codex' | 'Claude'>('Codex')
  const [conversationCreationPending, setConversationCreationPending] =
    useState(false)
  const [isChoosingFolder, setIsChoosingFolder] = useState(false)
  const [theme, setTheme] = useState(readThemePreference)
  const [approvalCollapsed, setApprovalCollapsed] = useState(false)
  const [handoffPending, setHandoffPending] = useState(false)
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
  const chooseFolder = useCallback(async () => {
    if (!desktop || isChoosingFolder) return
    setIsChoosingFolder(true)
    try {
      const folder = await chooseProjectFolder()
      if (folder) setNewProject(folder)
    } catch {
      toast.error('Could not open the folder picker')
    } finally {
      setIsChoosingFolder(false)
    }
  }, [desktop, isChoosingFolder])

  const activeConversations = useMemo(
    () =>
      conversations.filter(
        (conversation) =>
          !sidebarPreferences.archivedIds.includes(conversation.id),
      ),
    [conversations, sidebarPreferences.archivedIds],
  )
  const selected =
    conversations.find((conversation) => conversation.id === selectedId) ??
    activeConversations[0] ??
    conversations[0]
  const selectedConversationId = selected?.id
  const availableConversationIds = useMemo(
    () => activeConversations.map((conversation) => conversation.id),
    [activeConversations],
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
        setSelectedId(result.selectedId)
      }
    },
    [openTabIds, selectedConversationId],
  )
  const showTerminal = Boolean(
    selected && terminalConversationId === selected.id,
  )
  const historyQuery = useQuery({
    queryKey: ['conversation-history', selected?.id],
    queryFn: () => loadMessages(selected?.id ?? ''),
    enabled: desktop && Boolean(selected),
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
  const nativeHistoryComplete =
    !historyQuery.isError && historyQuery.data?.completeness === 'Complete'
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
          toast.message(`Cancellation requested. Waiting for ${selected.provider} to stop.`)
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
    if (!desktop || !selected.execution) {
      setTerminalConversationId(showTerminal ? null : selected.id)
      return
    }
    if (daemonDisconnected) {
      toast.message('Reconnect to Maitty before opening the native terminal')
      return
    }
    if (handoffPending) {
      toast.message('Wait for the current conversation handoff to finish')
      return
    }

    if (
      selected.execution === 'Structured' &&
      selected.status !== 'idle' &&
      selected.status !== 'done'
    ) {
      toast.message(
        'Finish the current turn before opening the native terminal',
      )
      return
    }
    if (
      selected.execution === 'Native' &&
      showTerminal &&
      selected.status !== 'idle' &&
      selected.status !== 'done'
    ) {
      toast.message('Finish the native turn before returning to chat')
      return
    }

    if (selected.execution === 'Native' && !showTerminal) {
      setTerminalConversationId(selected.id)
      return
    }

    const returningToChat = selected.execution === 'Native' && showTerminal
    setHandoffPending(true)
    const handoff =
      selected.execution === 'Structured'
        ? handoffToNative(selected)
        : returningToChat
          ? handoffToChat(selected)
          : Promise.resolve()
    void handoff
      .then(async () => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['conversations'] }),
          queryClient.invalidateQueries({
            queryKey: ['conversation-history', selected.id],
          }),
        ])
        setTerminalConversationId(returningToChat ? null : selected.id)
        toast.success(
          returningToChat
            ? 'Conversation returned to chat'
            : selected.execution === 'Structured'
              ? 'Native terminal opened'
              : 'Terminal opened',
        )
      })
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error
            ? error.message
            : 'Could not switch conversation view',
        )
      })
      .finally(() => {
        setHandoffPending(false)
      })
  }, [
    daemonDisconnected,
    desktop,
    handoffPending,
    queryClient,
    selected,
    showTerminal,
  ])
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
      if (daemonDisconnected) {
        toast.message('Reconnect to Maitty before opening the native terminal')
        return
      }
      if (handoffPending) {
        toast.message('Wait for the current conversation handoff to finish')
        return
      }
      if (conversation.status !== 'idle' && conversation.status !== 'done') {
        toast.message(
          'Finish the current turn before opening the native terminal',
        )
        return
      }
      setHandoffPending(true)
      void handoffToNative(conversation)
        .then(async () => {
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['conversations'] }),
            queryClient.invalidateQueries({
              queryKey: ['conversation-history', conversation.id],
            }),
          ])
          setTerminalConversationId(conversation.id)
          toast.success('Native terminal opened')
        })
        .catch((error: unknown) => {
          toast.error(
            error instanceof Error
              ? error.message
              : 'Could not open native terminal',
          )
        })
        .finally(() => {
          setHandoffPending(false)
        })
    },
    [
      activeConversations,
      daemonDisconnected,
      desktop,
      handoffPending,
      queryClient,
      selectConversation,
    ],
  )
  const retryNativeHandoff = useCallback(() => {
    if (
      !selected ||
      selected.execution !== 'Native' ||
      selected.status !== 'recovery'
    )
      return
    if (daemonDisconnected) {
      toast.message('Reconnect to Maitty before returning to chat')
      return
    }
    if (handoffPending) {
      toast.message('Wait for the current conversation handoff to finish')
      return
    }

    setHandoffPending(true)
    void handoffToChat(selected)
      .then(async () => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['conversations'] }),
          queryClient.invalidateQueries({
            queryKey: ['conversation-history', selected.id],
          }),
        ])
        setTerminalConversationId(null)
        toast.success('Conversation returned to chat')
      })
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error
            ? error.message
            : 'Could not return conversation to chat',
        )
      })
      .finally(() => {
        setHandoffPending(false)
      })
  }, [daemonDisconnected, handoffPending, queryClient, selected])
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
      const openConversation = {
        label: 'Open',
        onClick: () => {
          selectConversation(notification.conversationId)
        },
      }
      if (notification.kind === 'attention') {
        if (!settingsPreferences.notifyNeedsYou) continue
        toast.warning(`${notification.title} needs your attention`, {
          action: openConversation,
        })
      } else {
        if (!settingsPreferences.notifyFinished) continue
        toast.success(`${notification.title} is done`, {
          action: openConversation,
        })
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
    document.documentElement.classList.toggle('dark', theme === 'night')
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
  }, [
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
        setNewDialogOpen(true)
        if (desktop) void chooseFolder()
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
        setNewDialogOpen(true)
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

  function setArchived(conversationId: string, archived: boolean) {
    const conversation = conversations.find(
      (item) => item.id === conversationId,
    )
    if (archived && conversation?.status === 'working') {
      toast.message('Wait for this conversation to finish before archiving it')
      return
    }
    setSidebarPreferences((current) => ({
      ...current,
      archivedIds: archived
        ? [...new Set([...current.archivedIds, conversationId])]
        : current.archivedIds.filter((id) => id !== conversationId),
    }))
  }

  function renameConversation(conversationId: string, title: string) {
    const cleanTitle = title.trim()
    if (!cleanTitle) {
      toast.error('Conversation name cannot be empty')
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
        toast.message('Test notification', {
          description: 'Maitty notifications are working.',
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

  if (!selected)
    return (
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
        <header className="window-bar">
          <div className="brand-lockup">
            <div className="brand-mark" aria-hidden="true">
              <span />
            </div>
            <span className="brand-name">maitty</span>
          </div>
          <div className="window-location">Local workspace</div>
          <div className="window-actions">
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Open settings"
              onClick={() => {
                setSettingsDialogOpen(true)
              }}
            >
              <Settings2 />
            </Button>
            <span className="preview-label">
              {sessionsQuery.isLoading
                ? 'CONNECTING'
                : sessionsQuery.isError
                  ? 'DAEMON DISCONNECTED'
                  : 'DAEMON CONNECTED'}
            </span>
          </div>
        </header>
        <div className="workspace-shell empty-shell">
          <ConversationSidebar
            conversations={[]}
            archivedConversations={[]}
            selectedId=""
            filter={filter}
            pinnedIds={sidebarPreferences.pinnedIds}
            compact={isCompactSidebar}
            desktopCollapsed={sidebarCollapsed}
            disconnected={daemonDisconnected}
            drawerOpen={sidebarDrawerOpen}
            onFilter={setFilter}
            onNew={() => {
              setNewDialogOpen(true)
            }}
            onTogglePin={togglePin}
            onArchive={setArchived}
            onRename={renameConversation}
            onSelect={selectConversation}
            onToggleDrawer={() => {
              setSidebarDrawerOpen((open) => !open)
            }}
            onToggleCollapse={() => {
              setSidebarCollapsed((collapsed) => !collapsed)
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
                  : 'Your conversations stay running in Maitty, even when you close this window.'}
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
                  setNewDialogOpen(true)
                  if (desktop) void chooseFolder()
                }}
              >
                <FolderOpen size={14} />
                Open a project folder
                <span className="shortcut-hint">{primaryModifier} O</span>
              </Button>
            )}
          </main>
        </div>
        <footer className="status-bar">
          <div
            role="status"
            aria-live="polite"
            aria-label={
              sessionsQuery.isLoading
                ? 'Connecting to Maitty daemon'
                : sessionsQuery.isError
                  ? 'Daemon disconnected'
                  : 'Daemon connected'
            }
          >
            <span className="connection-dot" />
            {sessionsQuery.isLoading
              ? 'Connecting to Maitty daemon'
              : sessionsQuery.isError
                ? 'Daemon disconnected'
                : 'Daemon connected'}
          </div>
          <div>Sessions remain owned by the daemon</div>
        </footer>
        <NewConversationDialog
          open={newDialogOpen}
          project={newProject}
          onProjectChange={setNewProject}
          provider={newProvider}
          onProviderChange={setNewProvider}
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
        />
        {settingsDialog}
        {daemonRestartDialog}
        <Toaster
          position={settingsPreferences.toastPosition}
          theme={theme === 'night' ? 'dark' : 'light'}
        />
      </div>
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
          toast.success('Message sent')
        })
        .catch(async () => {
          let outcome: ReturnType<typeof classifySubmissionFailure> = 'unknown'
          try {
            const latest = await queryClient.query({
              queryKey: ['conversation-history', conversationId],
              queryFn: () => loadMessages(conversationId),
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
      toast.success('Preview response complete')
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
      const latest = await loadMessages(selected.id)
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
        toast.success('Session resumed. The interrupted turn was not replayed.')
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
          toast.success(
            decision === 'Decline' || decision === 'Cancel'
              ? 'Request declined'
              : 'Command approved',
          )
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
    if (allow) toast.success('Approval recorded in preview')
    else toast.message('Request declined in preview')
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

  function createConversation() {
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    const project = newProject.trim()
    if (!project) {
      toast.error('Choose a project folder first')
      return
    }
    if (desktop) {
      conversationCreationPendingRef.current = true
      setConversationCreationPending(true)
      void createDaemonConversation(project, newProvider)
        .then(async ({ id }) => {
          setNewDialogOpen(false)
          selectConversation(String(id))
          await queryClient.invalidateQueries({ queryKey: ['conversations'] })
          toast.success(`${newProvider} conversation started`)
        })
        .catch((error: unknown) => {
          toast.error(
            errorMessage(error, `Could not start ${newProvider} conversation`),
          )
        })
        .finally(() => {
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

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      sendMessage()
    }
  }

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
          Skip to conversation
        </a>
        <header className="window-bar">
          <div className="brand-lockup">
            <div className="brand-mark" aria-hidden="true">
              <span />
            </div>
            <span className="brand-name">maitty</span>
          </div>
          <div className="window-location">
            <span>Workspace</span>
            <ChevronRight size={13} />
            <span>{selected.project}</span>
          </div>
          <div className="window-actions">
            {!desktop && import.meta.env.DEV && (
              <span className="preview-label">LOCAL PREVIEW · SAMPLE DATA</span>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Open settings"
              onClick={() => {
                setSettingsDialogOpen(true)
              }}
            >
              <Settings2 />
            </Button>
          </div>
        </header>

        <div className={`workspace-shell ${activePanel ? 'has-panel' : ''}`}>
          <ConversationSidebar
            conversations={filteredConversations}
            archivedConversations={conversations.filter((conversation) =>
              sidebarPreferences.archivedIds.includes(conversation.id),
            )}
            selectedId={selected.id}
            filter={filter}
            pinnedIds={sidebarPreferences.pinnedIds}
            compact={isCompactSidebar}
            desktopCollapsed={sidebarCollapsed}
            disconnected={daemonDisconnected}
            drawerOpen={sidebarDrawerOpen}
            onFilter={setFilter}
            onNew={() => {
              setNewDialogOpen(true)
            }}
            onTogglePin={togglePin}
            onArchive={setArchived}
            onRename={renameConversation}
            onSelect={(id) => {
              selectConversation(id)
              setSidebarDrawerOpen(false)
            }}
            onToggleDrawer={() => {
              setSidebarDrawerOpen((open) => !open)
            }}
            onToggleCollapse={() => {
              setSidebarCollapsed((collapsed) => !collapsed)
            }}
            primaryModifier={primaryModifier}
          />

          <main
            className="conversation-workspace"
            id="conversation-main"
            tabIndex={-1}
          >
            <ConversationTabStrip
              tabs={openTabs}
              selectedId={selected.id}
              onSelect={selectConversation}
              onClose={closeConversation}
              onNew={() => {
                setNewDialogOpen(true)
              }}
              conversation={selected}
              panel={activePanel}
              showTerminal={showTerminal}
              onPanel={setActivePanel}
              onToggleTerminal={toggleTerminal}
              handoffPending={handoffPending}
              terminalDisabled={
                daemonDisconnected ||
                (selected.execution === 'Structured' &&
                  selected.status !== 'idle' &&
                  selected.status !== 'done') ||
                (selected.execution === 'Native' &&
                  showTerminal &&
                  selected.status !== 'idle' &&
                  selected.status !== 'done')
              }
            />

            {daemonDisconnected && (
              <div className="connection-warning" role="alert">
                <CircleAlert size={15} />
                <span>
                  Daemon disconnected. Showing the last known status; reconnect
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
            {selected.execution === 'Native' && historyQuery.isError && (
              <div className="history-warning" role="alert">
                <CircleAlert size={15} />
                <span>
                  Could not load the full conversation history. Maitty will
                  retry the exact {selected.provider} session when you return to chat.
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    void historyQuery.refetch()
                  }}
                >
                  Retry history
                </Button>
              </div>
            )}
            {selected.execution === 'Native' &&
              !historyQuery.isError &&
              !nativeHistoryComplete && (
                <div className="history-warning" role="status">
                  <CircleAlert size={15} />
                  <span>
                    {!historyQuery.data ||
                    historyQuery.data.completeness === 'Loading'
                      ? 'Loading the full conversation history. Maitty will verify it before chat resumes.'
                      : `Conversation history is incomplete. Maitty will reconcile the exact ${selected.provider} session before chat resumes.`}
                  </span>
                </div>
              )}
            {showTerminal ? (
              <div className="terminal-workspace">
                <TerminalView conversation={selected} desktop={desktop} />
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
                      handoffPending ||
                      recoveryPendingId === selected.id
                    }
                    pending={
                      handoffPending || recoveryPendingId === selected.id
                    }
                    onResume={resumeSelectedConversation}
                    onOpenTerminal={toggleTerminal}
                    onRetryNativeHandoff={retryNativeHandoff}
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
                      <EmptyConversation project={selected.project} />
                    ) : (
                      messages.map((message) => (
                        <MessageRow
                          key={message.id}
                          message={message}
                          provider={selected.provider}
                        />
                      ))
                    )}
                    {selected.status === 'working' && (
                      <StreamingRow provider={selected.provider} />
                    )}
                  </div>
                </ScrollArea>

                {pendingApproval && (
                  <ApprovalDock
                    key={`${pendingApproval.id}:${selected.id}`}
                    message={pendingApproval}
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
                    setDrafts((current) => ({ ...current, [selected.id]: '' }))
                    clearSendFailure(selected.id)
                  }}
                  onChange={updateDraft}
                  onKeyDown={handleComposerKeyDown}
                  onSend={sendMessage}
                  onCancel={cancelTurn}
                />
              </>
            )}
          </main>

          {activePanel && (
            <ContextPanel
              tab={activePanel}
              conversation={selected}
              desktop={desktop}
              onTab={setActivePanel}
              onClose={() => {
                setActivePanel(null)
              }}
            />
          )}
        </div>

        <footer className="status-bar">
          <div
            role="status"
            aria-live="polite"
            aria-label={
              desktop
                ? daemonDisconnected
                  ? 'Daemon disconnected'
                  : 'Connected to Maitty daemon'
                : 'Local preview · daemon not connected'
            }
          >
            <span className="connection-dot" />
            {desktop
              ? daemonDisconnected
                ? 'Daemon disconnected'
                : 'Connected to Maitty daemon'
              : 'Local preview · daemon not connected'}
          </div>
          <div>
            <Keyboard size={13} />
            <span>
              <kbd>{primaryModifier}</kbd>
              <kbd>K</kbd> commands
            </span>
            <span className="status-divider" />
            <span>All sessions stay alive when you close Maitty</span>
          </div>
        </footer>
      </div>

      <NewConversationDialog
        open={newDialogOpen}
        project={newProject}
        onProjectChange={setNewProject}
        provider={newProvider}
        onProviderChange={setNewProvider}
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
          setNewDialogOpen(true)
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
      {daemonRestartDialog}
      <Toaster
        position={settingsPreferences.toastPosition}
        theme={theme === 'night' ? 'dark' : 'light'}
      />
    </TooltipProvider>
  )
}

type SidebarProps = {
  conversations: Conversation[]
  archivedConversations: Conversation[]
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
  primaryModifier: '⌘' | 'Ctrl'
  onToggleDrawer: () => void
  onToggleCollapse: () => void
}

function ConversationSidebar({
  conversations,
  archivedConversations,
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
  onToggleDrawer,
  onToggleCollapse,
  primaryModifier,
}: SidebarProps) {
  const [archivedOpen, setArchivedOpen] = useState(false)
  const projects = Array.from(
    new Set(conversations.map((conversation) => conversation.project)),
  )
  const rowActions = {
    pinnedIds,
    selectedId,
    onSelect,
    onTogglePin,
    onArchive,
    onRename,
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
        <div>
          <span className="sidebar-title">Threads</span>
          <span className="session-total">{conversations.length}</span>
        </div>
      </div>
      <Button
        className="new-session-button"
        variant="secondary"
        aria-label="New conversation"
        onClick={onNew}
        disabled={disconnected}
      >
        <Plus size={15} />
        <span className="new-session-label">New conversation</span>
        <span className="shortcut-hint">{primaryModifier} N</span>
      </Button>
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
        <kbd>{primaryModifier} K</kbd>
      </label>
      <ScrollArea className="session-list-scroll">
        {projects.map((project) => (
          <ProjectSection
            key={project}
            project={project}
            conversations={conversations.filter(
              (conversation) => conversation.project === project,
            )}
            disconnected={disconnected}
            {...rowActions}
          />
        ))}
        {conversations.length === 0 && (
          <div className="no-sessions">No sessions match “{filter}”</div>
        )}
        {archivedConversations.length > 0 && (
          <section className="session-group archived-group">
            <button
              className="archived-toggle"
              type="button"
              aria-expanded={archivedOpen}
              onClick={() => {
                setArchivedOpen((open) => !open)
              }}
            >
              <Archive size={13} />
              <span>Archived {archivedConversations.length}</span>
              <ChevronDown
                className={archivedOpen ? 'is-open' : ''}
                size={13}
              />
            </button>
            {archivedOpen && (
              <div className="session-rows">
                {archivedConversations.map((conversation) => (
                  <SessionRow
                    key={conversation.id}
                    conversation={conversation}
                    selected={conversation.id === selectedId}
                    isPinned={pinnedIds.includes(conversation.id)}
                    isArchived
                    {...rowActions}
                  />
                ))}
              </div>
            )}
          </section>
        )}
      </ScrollArea>
      <div className="sidebar-bottom">
        <Separator />
        <div className="sidebar-connection" role="status">
          <span
            className={`connection-dot ${disconnected ? '' : 'is-connected'}`}
          />
          {disconnected ? 'Daemon disconnected' : 'Connected to Maitty'}
        </div>
        <div className="workspace-picker">
          <span className="workspace-avatar">M</span>
          <span className="workspace-copy">
            <strong>My workspace</strong>
            <small>Local machine</small>
          </span>
          <ChevronDown size={14} />
        </div>
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
}

function ProjectSection({
  project,
  conversations,
  disconnected,
  ...actions
}: {
  project: string
  conversations: Conversation[]
  disconnected: boolean
} & SessionRowActions) {
  const [settledOpen, setSettledOpen] = useState(false)
  const active = [...conversations]
    .filter(
      (conversation) =>
        conversation.status !== 'done' && conversation.status !== 'idle',
    )
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
  const settled = conversations.filter(
    (conversation) =>
      conversation.status === 'done' || conversation.status === 'idle',
  )

  return (
    <section className="project-section" aria-label={`${project} project`}>
      <div className="project-heading">
        <FolderOpen size={14} />
        <span>{project}</span>
        <span className="project-count">{conversations.length}</span>
      </div>
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
      {settled.length > 0 && (
        <>
          <button
            className="settled-toggle"
            type="button"
            aria-expanded={settledOpen}
            onClick={() => {
              setSettledOpen((open) => !open)
            }}
          >
            <span>Settled {settled.length}</span>
            <ChevronDown className={settledOpen ? 'is-open' : ''} size={13} />
          </button>
          {settledOpen && (
            <div className="session-rows">
              {settled.map((conversation) => (
                <SessionRow
                  key={conversation.id}
                  conversation={conversation}
                  selected={conversation.id === actions.selectedId}
                  isPinned={actions.pinnedIds.includes(conversation.id)}
                  {...actions}
                />
              ))}
            </div>
          )}
        </>
      )}
      {disconnected && (
        <span className="project-offline">Last known statuses</span>
      )}
    </section>
  )
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
}: {
  conversation: Conversation
  selected: boolean
  isPinned: boolean
  isArchived?: boolean
} & Pick<
  SessionRowActions,
  'onSelect' | 'onTogglePin' | 'onArchive' | 'onRename'
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
          <AgentAvatar
            provider={conversation.provider}
            status={conversation.status}
            small
          />
          <span className="session-row-copy">
            <span className="session-row-top">
              <span className="session-row-title">{conversation.title}</span>
              <span className="session-updated">{conversation.updated}</span>
            </span>
            <span className="session-row-bottom">
              <span>{conversation.project}</span>
              <StatusMark status={conversation.status} />
            </span>
          </span>
        </button>
      )}
      {!isRenaming && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              className="session-row-menu"
              variant="ghost"
              size="icon-xs"
              aria-label={`Menu for ${conversation.title}`}
            >
              <MoreHorizontal />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" side="right">
            <DropdownMenuItem
              onSelect={() => {
                onTogglePin(conversation.id)
              }}
            >
              <Pin size={14} />
              {isPinned ? 'Unpin conversation' : 'Pin conversation'}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={beginRename}>
              <Pencil size={14} />
              Rename
            </DropdownMenuItem>
            <DropdownMenuItem
              disabled={!isArchived && conversation.status === 'working'}
              onSelect={() => {
                onArchive(conversation.id, !isArchived)
              }}
            >
              {isArchived ? (
                <ArchiveRestore size={14} />
              ) : (
                <Archive size={14} />
              )}
              {isArchived ? 'Unarchive conversation' : 'Archive conversation'}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  )
}

function ConversationTabStrip({
  tabs,
  selectedId,
  onSelect,
  onClose,
  onNew,
  conversation,
  panel,
  showTerminal,
  onPanel,
  onToggleTerminal,
  handoffPending,
  terminalDisabled,
}: {
  tabs: Conversation[]
  selectedId: string
  onSelect: (id: string) => void
  onClose: (id: string) => void
  onNew: () => void
  conversation: Conversation
  panel: PanelTab | null
  showTerminal: boolean
  onPanel: (tab: PanelTab | null) => void
  onToggleTerminal: () => void
  handoffPending: boolean
  terminalDisabled: boolean
}) {
  return (
    <div className="conversation-header">
      <h1 className="sr-only">{conversation.title}</h1>
      <div
        className="conversation-tabs"
        role="tablist"
        aria-label="Open conversations"
      >
        {tabs.map((tab) => (
          <div
            className={`conversation-tab ${tab.id === selectedId ? 'is-active' : ''}`}
            key={tab.id}
          >
            <button
              type="button"
              role="tab"
              aria-selected={tab.id === selectedId}
              aria-controls="conversation-main"
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
          </div>
        ))}
        <Button
          className="conversation-tab-new"
          variant="ghost"
          size="icon-sm"
          aria-label="Add conversation tab"
          onClick={onNew}
        >
          <Plus size={15} />
        </Button>
      </div>
      <div className="header-toolbar">
        <Button
          variant={panel === 'diff' ? 'secondary' : 'outline'}
          size="sm"
          onClick={() => {
            onPanel(panel === 'diff' ? null : 'diff')
          }}
        >
          <FileCode2 size={14} />
          Diff{conversation.hasDiff && <span className="diff-indicator" />}
        </Button>
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
            <MessageSquarePlus size={14} />
            Chat
          </Button>
          <Button
            variant={showTerminal ? 'secondary' : 'ghost'}
            size="sm"
            disabled={showTerminal || handoffPending || terminalDisabled}
            onClick={onToggleTerminal}
            aria-pressed={showTerminal}
            aria-label={
              conversation.execution === 'Structured'
                ? 'Open in native terminal'
                : 'Show terminal'
            }
          >
            <TerminalSquare size={14} />
            {handoffPending ? 'Switching…' : 'Terminal'}
          </Button>
        </div>
        <Button
          variant={panel === 'details' ? 'secondary' : 'outline'}
          size="icon-sm"
          aria-label="Toggle details"
          onClick={() => {
            onPanel(panel === 'details' ? null : 'details')
          }}
        >
          <PanelRight />
        </Button>
      </div>
    </div>
  )
}

function MessageRow({
  message,
  provider,
}: {
  message: Message
  provider: Conversation['provider']
}) {
  if (message.role === 'activity') return <ActivityRow message={message} />
  if (message.role === 'approval') return <ApprovalMarker message={message} />
  if (message.role === 'user')
    return (
      <div className="message-row user-message">
        <div className="message-avatar user-avatar">Y</div>
        <div className="message-body">
          <div className="message-byline">
            <strong>You</strong>
            <time>{message.time}</time>
          </div>
          <div className="user-bubble">{message.content}</div>
        </div>
      </div>
    )
  return (
    <div className="message-row assistant-message">
      <AgentAvatar provider={provider} status="working" />
      <div className="message-body">
        <div className="message-byline">
          <strong>{provider}</strong>
          <span className="message-model">
            {provider === 'Codex' ? 'GPT-5 Codex' : 'Claude Code'}
          </span>
          <time>{message.time}</time>
        </div>
        <div className="assistant-copy">{message.content}</div>
      </div>
    </div>
  )
}

function ApprovalMarker({ message }: { message: Message }) {
  const resolved = message.resolved
  const text = resolved
    ? 'Request answered'
    : message.approvalKind === 'UserInput'
      ? 'Waiting for your answer'
      : `Waiting for approval · ${message.detail || message.content}`
  return (
    <div
      className={`approval-marker ${resolved ? 'is-resolved' : ''}`}
      role="status"
    >
      {resolved ? <Check size={13} /> : <ShieldCheck size={13} />}
      <span>{text}</span>
      {!resolved && (
        <button
          type="button"
          onClick={() => {
            document
              .querySelector<HTMLElement>('.approval-dock')
              ?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
          }}
        >
          Answer below ↓
        </button>
      )}
    </div>
  )
}

function ApprovalDock({
  message,
  desktop,
  disabled,
  collapsed,
  onToggle,
  onResolve,
  onUserInput,
}: {
  message: Message
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
            questions={message.userQuestions ?? []}
            onSubmit={onUserInput}
            disabled={disabled}
          />
        ) : (
          <ApprovalRequest
            message={message}
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
  questions,
  onSubmit,
  disabled,
}: {
  message: Message
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
      aria-label="Codex asks for input"
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
            Answers are sent only to this Codex request.
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

function ActivityRow({ message }: { message: Message }) {
  const [expanded, setExpanded] = useState(false)
  const activityState = message.activityState ?? 'Completed'
  const stateIcon =
    activityState === 'Running' ? (
      <LoaderCircle size={12} />
    ) : activityState === 'Failed' ? (
      <CircleAlert size={12} />
    ) : activityState === 'Interrupted' ? (
      <Circle size={10} />
    ) : (
      <Check size={13} />
    )
  return (
    <div
      className={`activity-row activity-${activityState.toLowerCase()} ${expanded ? 'is-expanded' : ''}`}
    >
      <div className="activity-marker" aria-label={activityState}>
        {stateIcon}
      </div>
      <button
        className="activity-trigger"
        type="button"
        onClick={() => {
          setExpanded(!expanded)
        }}
        aria-expanded={expanded}
      >
        <span className="activity-label">{message.content}</span>
        {activityState !== 'Completed' && (
          <span className="activity-state">{activityState}</span>
        )}
        <span className="activity-detail">{message.detail}</span>
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
      </button>
      {expanded && (
        <pre className="activity-expanded">
          {message.detail ?? 'No additional details.'}
        </pre>
      )}
    </div>
  )
}

function ApprovalRequest({
  message,
  desktop,
  onResolve,
  disabled,
}: {
  message: Message
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
                ? 'Review the requested file change before allowing Codex to continue.'
                : message.approvalKind === 'Permission'
                  ? 'Allowing grants this exact permission profile for this turn only.'
                  : 'Codex wants to run this command in the project folder.')}
          </p>
        </div>
        <span className="approval-badge">
          {message.resolved ? 'Resolved' : 'Needs approval'}
        </span>
      </div>
      {message.approvalKind === 'Command' && (
        <code className="approval-command">{message.detail}</code>
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
              ? 'Waiting for Codex to send the file diff. Approval is disabled until the requested change is visible.'
              : 'No supported decision is available for this request. Maitty has left it unanswered.'}
          </div>
        )
      ) : (
        <div className="approval-resolved">
          <CheckCheck size={14} />
          {desktop
            ? 'Decision sent to Codex'
            : 'Decision recorded in local preview'}
        </div>
      )}
    </section>
  )
}

function StreamingRow({ provider }: { provider: Conversation['provider'] }) {
  return (
    <div className="streaming-row" role="status">
      <span className="streaming-orb">
        <LoaderCircle size={14} />
      </span>
      <span>{provider} is working</span>
      <span className="streaming-dots">
        <i />
        <i />
        <i />
      </span>
    </div>
  )
}

function RecoveryNotice({
  conversation,
  desktop,
  disabled,
  pending,
  onResume,
  onOpenTerminal,
  onRetryNativeHandoff,
}: {
  conversation: Conversation
  desktop: boolean
  disabled: boolean
  pending: boolean
  onResume: () => void
  onOpenTerminal: () => void
  onRetryNativeHandoff: () => void
}) {
  const canResumeInChat = desktop && conversation.execution === 'Structured'
  const canOpenNativeTerminal =
    desktop &&
    conversation.execution === 'Native' &&
    conversation.nativePane !== null
  const canRetryNativeHandoff =
    desktop &&
    conversation.execution === 'Native' &&
    conversation.status === 'recovery' &&
    conversation.nativePane !== null
  return (
    <div className="recovery-notice" role="alert">
      <CircleHelp size={15} />
      <span>
        {canResumeInChat
          ? `${conversation.provider} stopped before the turn finished. Resume this exact session; Maitty will not replay the interrupted turn.`
          : conversation.execution === 'Native'
            ? canOpenNativeTerminal
              ? 'Native process cleanup was not confirmed. Inspect the terminal; Maitty will confirm cleanup again before chat resumes.'
              : 'This native conversation needs recovery in the TUI before it can return to chat.'
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
      {canRetryNativeHandoff && (
        <Button
          variant="outline"
          size="sm"
          onClick={onRetryNativeHandoff}
          disabled={disabled}
        >
          {pending ? 'Checking cleanup…' : 'Retry return to chat'}
        </Button>
      )}
      {canOpenNativeTerminal && (
        <Button
          variant="outline"
          size="sm"
          onClick={onOpenTerminal}
          disabled={disabled}
        >
          Open terminal
        </Button>
      )}
    </div>
  )
}

function EmptyConversation({ project }: { project: string }) {
  return (
    <div className="empty-conversation">
      <div className="empty-mark">
        <MessageSquarePlus size={20} />
      </div>
      <h2>Start with a clear task</h2>
      <p>
        This conversation will run in <strong>{project}</strong>. Its work stays
        with that project.
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
  onCancel: () => void
  onRetry: () => void
  onDiscard: () => void
  onCheckDelivery: () => void
}) {
  const nativeOwner = conversation.execution === 'Native'
  const busy = status === 'working' || status === 'needs-you' || nativeOwner
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
      <div className="composer-box">
        <Textarea
          value={value}
          onChange={(event) => {
            onChange(event.target.value)
          }}
          onKeyDown={onKeyDown}
          placeholder={
            nativeOwner
              ? 'Return to chat from the Terminal panel to continue…'
              : !connected
                ? 'Reconnect before sending. Your draft stays here.'
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
              <span className="codex-glyph">◆</span>
              {conversation.provider}
            </span>
            <span className="composer-separator" />
            <span>{conversation.permission.split(' · ')[0]}</span>
            <ChevronDown size={12} />
          </div>
          <div className="composer-send">
            {((desktop && (status === 'working' || status === 'needs-you')) ||
              (!desktop && status === 'working')) && (
              <Button
                variant="outline"
                size="sm"
                onClick={onCancel}
                disabled={cancelling}
              >
                <Square size={12} fill="currentColor" />
                {cancelling ? 'Stopping' : 'Stop'}
              </Button>
            )}
            <span className="send-hint">
              <kbd>↵</kbd> to send
            </span>
            <Button
              size="icon-sm"
              aria-label="Send message"
              onClick={onSend}
              disabled={!value.trim() || busy || !connected || deliveryBlocked}
            >
              <Send />
            </Button>
          </div>
        </div>
      </div>
      <p className="composer-footnote">
        {nativeOwner
          ? `Native ${conversation.provider} owns this conversation until you continue in chat.`
          : !connected
            ? 'Daemon disconnected. Reconnect to send this draft.'
            : 'Agent actions follow the permissions shown above.'}
      </p>
    </div>
  )
}

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
  function handleTabKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
    const tabs = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
    )
    const currentIndex = tabs.findIndex(
      (item) => item === document.activeElement,
    )
    const selectedIndex = tab === 'diff' ? 0 : 1
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
    onTab(targetIndex === 0 ? 'diff' : 'details')
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
          <button
            className={tab === 'diff' ? 'active' : ''}
            type="button"
            role="tab"
            aria-selected={tab === 'diff'}
            tabIndex={tab === 'diff' ? 0 : -1}
            onClick={() => {
              onTab('diff')
            }}
          >
            Changes
          </button>
          <button
            className={tab === 'details' ? 'active' : ''}
            type="button"
            role="tab"
            aria-selected={tab === 'details'}
            tabIndex={tab === 'details' ? 0 : -1}
            onClick={() => {
              onTab('details')
            }}
          >
            Context
          </button>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close details panel"
          onClick={onClose}
        >
          <X />
        </Button>
      </div>
      <ScrollArea className="context-panel-scroll">
        {tab === 'details' && <DetailsView conversation={conversation} />}
        {tab === 'diff' && (
          <DiffView conversation={conversation} desktop={desktop} />
        )}
      </ScrollArea>
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
                {line}
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
}: {
  conversation: Conversation
  desktop: boolean
}) {
  if (desktop)
    return <DaemonTerminalView conversation={conversation} desktop={desktop} />
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

function StatusMark({ status }: { status: SessionStatus }) {
  const Icon =
    status === 'working'
      ? LoaderCircle
      : status === 'needs-you'
        ? Diamond
        : status === 'recovery'
          ? RefreshCw
          : status === 'done'
            ? CircleCheck
            : status === 'unknown'
              ? CircleDashed
              : Circle
  return (
    <Icon
      className={`status-mark status-${status}`}
      size={13}
      aria-hidden="true"
    />
  )
}

function NewConversationDialog({
  open,
  project,
  onProjectChange,
  provider,
  onProviderChange,
  claudeAvailable,
  desktop,
  onChooseFolder,
  isChoosingFolder,
  onClose,
  onCreate,
  disabled,
  isCreating,
}: {
  open: boolean
  project: string
  onProjectChange: (value: string) => void
  provider: 'Codex' | 'Claude'
  onProviderChange: (provider: 'Codex' | 'Claude') => void
  claudeAvailable: boolean
  desktop: boolean
  onChooseFolder: () => void
  isChoosingFolder: boolean
  onClose: () => void
  onCreate: () => void
  disabled: boolean
  isCreating: boolean
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
              placeholder="Choose or enter a folder path"
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
            {isCreating ? 'Starting conversation…' : 'Start conversation'}
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
    ['Open a project folder', primaryModifier, 'O'],
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
