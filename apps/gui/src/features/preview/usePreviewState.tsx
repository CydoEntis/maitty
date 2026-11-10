import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from '@/lib/toast'
import { cancelTurn as cancelDaemonTurn, isDesktop, loadClaudeExecutablePath, loadCodexExecutablePath, loadConversations, loadDaemonRestartStatus, loadMachines, loadMessages, type SessionPermissions } from '@/api/daemon'
import { chooseProjectFolder } from '@/api/folderPicker'
import { type Conversation, type Message, previewConversations, previewMessages, type SessionStatus } from '@/data/preview'
import { markConversationsDisconnected } from '@/data/connection'
import { readSidebarPreferences } from '@/data/sidebarPreferences'
import { readConversationDrafts } from '@/data/conversationDrafts'
import { readThemePreference } from '@/data/themePreference'
import { readSettingsPreferences } from '@/features/settings/preferences'
import { readSelectedConversation } from '@/data/selectedConversation'
import { closeConversationTab, openConversationTab, readOpenConversationTabs, reconcileConversationTabs } from '@/data/openConversationTabs'
import { PERMISSION_LABELS, readConversationPermissions } from '@/data/conversationPermissions'
import { shortcutModifier } from '@/lib/shortcut-modifier'
import { daemonProtocolMismatch } from '@/lib/daemon-connection'
import { HISTORY_REFRESH_INTERVAL_MS, type PanelTab, type SendFailure, SESSION_REFRESH_INTERVAL_MS, sessionGroup } from './shared'
import { usePreviewEffects } from './usePreviewEffects'
import { usePreviewKeys } from './usePreviewKeys'
import { useConversationActions } from './useConversationActions'
import { PreviewSettingsDialog } from './PreviewSettingsDialog'
import { ServiceRestartButton, ServiceRestartDialog } from './ServiceRestart'
import { useConversationStarts } from './useConversationStarts'
import { useConversationSending } from './useConversationSending'

/** The preview app's state and what each action does, put together once. `PreviewApp` hands it to its screens through `usePreview`. */
export function usePreviewState() {
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
  const machinesQuery = useQuery({
    queryKey: ['machines'],
    queryFn: loadMachines,
    enabled: desktop,
    retry: false,
    staleTime: Infinity,
  })
  const machinesError = machinesQuery.data?.error ?? null
  useEffect(() => {
    if (machinesError) {
      toast.error(`Machines are not available: ${machinesError}`, {
        id: 'machines-config',
        duration: Infinity,
      })
    }
  }, [machinesError])
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
    // The desktop app is a terminal multiplexer: chat sessions are not shown.
    const current = desktop
      ? (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal')
      : previewSessions
    const withNames = current.map((conversation) => {
      const started = startedWith[conversation.id]
      return {
        ...conversation,
        // Terminal sessions group by repository; the browser preview keeps its own.
        ...(desktop
          ? { project: sessionGroup(conversation) }
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
      (readSettingsPreferences().restoreLayout
        ? readSelectedConversation()
        : null) ?? (desktop ? '' : 'codex-43'),
  )
  const [savedOpenTabIds, setSavedOpenTabIds] = useState(() =>
    readSettingsPreferences().restoreLayout ? readOpenConversationTabs() : [],
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
  // The desktop app shows only terminal sessions. The browser preview, which has no
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
  const deliveryNotice: 'unknown' | 'not-delivered' | null =
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
  usePreviewEffects({ desktop, sessionsQuery, sidebarPreferences, setSidebarPreferences, queryClient, drafts, selectedConversationId, openTabIds, daemonDisconnected, previousConversationStatuses, activeConversations, setStatusAnnouncement, selected, selectConversation, settingsPreferences, setActivePanel, theme, setApprovalCollapsed, setIsCompactSidebar, setSidebarDrawerOpen })
  const { openNewSession, createConversation } = useConversationStarts({ desktop, setNewDialogOpen, daemonDisconnected, conversationCreationPendingRef, setConversationCreationPending, conversations, settingsPreferences, queryClient, selectConversation, newProject, newKind, newProvider, setStartingSessions, newPermissions, setStartedWith, setConversations, setMessagesById, setActivePanel })
  usePreviewKeys({ setCommandDialogOpen, openTabIds, selected, selectConversation, closeConversation, setSettingsDialogOpen, openNewSession, activePanel, setActivePanel, sidebarDrawerOpen, setSidebarDrawerOpen, isCompactSidebar, setSidebarCollapsed, setKeyboardDialogOpen, pendingApproval, daemonDisconnected, desktop, toggleTerminal, cancelTurn, activeConversations })
  const { updateSettingsPreferences, togglePin, setArchived, renameConversation, requestEndSession, confirmEndSession } = useConversationActions({ setSidebarPreferences, settingsPreferences, setSidebarCollapsed, setSettingsPreferences, desktop, queryClient, conversations, openTabIds, closeConversation, setEndTarget, endTarget })


  const settingsDialog = (
    <PreviewSettingsDialog settingsDialogOpen={settingsDialogOpen} setSettingsDialogOpen={setSettingsDialogOpen} theme={theme} setTheme={setTheme} settingsPreferences={settingsPreferences} updateSettingsPreferences={updateSettingsPreferences} desktop={desktop} daemonDisconnected={daemonDisconnected} sessionsQuery={sessionsQuery} codexExecutableQuery={codexExecutableQuery} claudeExecutableQuery={claudeExecutableQuery} primaryModifier={primaryModifier} />
  )
  const daemonRestartAction = mismatch && (
    <ServiceRestartButton daemonRestartStatusQuery={daemonRestartStatusQuery} daemonRestartPending={daemonRestartPending} setDaemonRestartDialogOpen={setDaemonRestartDialogOpen} />
  )
  const daemonRestartStatus = daemonRestartStatusQuery.data
  const daemonRestartDialog = mismatch && (
    <ServiceRestartDialog daemonRestartDialogOpen={daemonRestartDialogOpen} daemonRestartPending={daemonRestartPending} setDaemonRestartDialogOpen={setDaemonRestartDialogOpen} daemonRestartStatus={daemonRestartStatus} daemonRestartStatusQuery={daemonRestartStatusQuery} mismatch={mismatch} setDaemonRestartPending={setDaemonRestartPending} sessionsQuery={sessionsQuery} />
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

  const { sendMessage, resumeSelectedConversation, resolveApproval, answerInput, checkSubmissionDelivery, clearSendFailure, updateDraft, handleComposerKeyDown, denyAndSend } = useConversationSending({ setDrafts, setSendFailures, setMessagesById, setDenyQueuedFor, daemonDisconnected, deliveryNotice, draft, desktop, historyQuery, queryClient, setStatus, selected, sendFailure, setRecoveryPendingId, selectConversation, pendingApproval, denyDecision })
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

  return { workingSince, statusAnnouncement, shownPanel, isCompactSidebar, sidebarDrawerOpen, setSidebarDrawerOpen, filteredConversations, selected, filter, sidebarPreferences, sidebarCollapsed, daemonDisconnected, setFilter, openNewSession, togglePin, setArchived, renameConversation, requestEndSession, selectInCurrentView, setSidebarCollapsed, setNewProject, startingSessions, desktop, conversations, setSettingsDialogOpen, primaryModifier, openTabs, closeConversation, showTerminal, setActivePanel, toggleTerminal, activeConversations, terminalSwitchNotice, sessionsQuery, daemonRestartAction, settingsPreferences, pendingApproval, recoveryPendingId, resumeSelectedConversation, messages, approvalCollapsed, setApprovalCollapsed, resolveApproval, answerInput, draft, historyQuery, deliveryNotice, checkSubmissionDelivery, sendMessage, setDrafts, clearSendFailure, updateDraft, handleComposerKeyDown, denyDecision, denyQueuedFor, denyAndSend, cancelTurn, newDialogOpen, newProject, newProvider, setNewProvider, newKind, setNewKind, newPermissions, setNewPermissions, claudeExecutableQuery, chooseFolder, isChoosingFolder, setNewDialogOpen, createConversation, conversationCreationPending, commandDialogOpen, setCommandDialogOpen, selectConversation, setKeyboardDialogOpen, openConversationInTerminal, keyboardDialogOpen, settingsDialog, endTarget, setEndTarget, confirmEndSession, daemonRestartDialog, theme, codexExecutableQuery, mismatch, daemonRestartStatus }
}
