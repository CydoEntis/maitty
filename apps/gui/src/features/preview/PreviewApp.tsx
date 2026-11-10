import { installUndoRedo } from '@/features/editing/undoRedo'
import { type KeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { CircleAlert, FolderOpen, LoaderCircle, MessageSquarePlus } from 'lucide-react'
import { toast } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { TitleBar } from '@/components/TitleBar'
import { TerminalPanes } from '@/features/terminal/TerminalPanes'
import { WorkingSinceContext } from '@/lib/working-since'
import { Toaster } from '@/components/ui/sonner'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { TooltipProvider } from '@/components/ui/tooltip'
import { answerUserInput as answerDaemonInput, approveRequest, cancelTurn as cancelDaemonTurn, classifySubmissionFailure, createConversation as createDaemonConversation, defaultSessionFolder, isDesktop, loadClaudeExecutablePath, loadCodexExecutablePath, loadConversations, loadDaemonRestartStatus, loadMachines, loadMessages, resumeConversation as resumeDaemonConversation, type SessionPermissions, startShellSession, startTerminalSession, submitMessage } from '@/api/daemon'
import { chooseProjectFolder } from '@/api/folderPicker'
import { type Conversation, type Message, previewConversations, previewMessages, type SessionStatus } from '@/data/preview'
import { markConversationsDisconnected } from '@/data/connection'
import { readSidebarPreferences } from '@/data/sidebarPreferences'
import { readConversationDrafts } from '@/data/conversationDrafts'
import { readThemePreference } from '@/data/themePreference'
import { readSettingsPreferences } from '@/features/settings/preferences'
import { readSelectedConversation } from '@/data/selectedConversation'
import { closeConversationTab, openConversationTab, readOpenConversationTabs, reconcileConversationTabs } from '@/data/openConversationTabs'
import { PERMISSION_LABELS, readConversationPermissions, rememberConversationPermissions } from '@/data/conversationPermissions'
import { shortcutModifier } from '@/lib/shortcut-modifier'
import { errorMessage } from '@/lib/error-message'
import { daemonConnectionError, daemonProtocolMismatch } from '@/lib/daemon-connection'
import { focusMainContent, HISTORY_REFRESH_INTERVAL_MS, type PanelTab, PREVIEW_APPROVAL_DELAY_MS, PREVIEW_RESPONSE_DELAY_MS, RECENT_PROJECT_LIMIT, type SendFailure, SESSION_REFRESH_INTERVAL_MS, sessionGroup } from './shared'
import { ConversationSidebar } from './Sidebar'
import { NewConversationDialog } from './NewConversationDialog'
import { ConversationTabStrip } from './TabStrip'
import { ConversationHeader, EmptyConversation, FilesChangedButton, MessageRow, StreamingRow } from './Messages'
import { ContextPanel, TerminalView } from './ContextPanel'
import { ApprovalDock, RecoveryNotice } from './Approvals'
import { ActivityGroup } from './Activity'
import { Composer } from './Composer'
import { CommandDialog } from './CommandDialog'
import { KeyboardDialog } from './KeyboardDialog'
import { usePreviewEffects } from './usePreviewEffects'
import { usePreviewKeys } from './usePreviewKeys'
import { useConversationActions } from './useConversationActions'
import { PreviewSettingsDialog } from './PreviewSettingsDialog'
import { ServiceRestartButton, ServiceRestartDialog } from './ServiceRestart'

export function PreviewApp() {
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
  useEffect(() => installUndoRedo(document, navigator.userAgent), [])
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
  usePreviewEffects({ desktop, sessionsQuery, sidebarPreferences, setSidebarPreferences, queryClient, drafts, selectedConversationId, openTabIds, daemonDisconnected, previousConversationStatuses, activeConversations, setStatusAnnouncement, selected, selectConversation, settingsPreferences, setActivePanel, theme, setApprovalCollapsed, setIsCompactSidebar, setSidebarDrawerOpen })
  usePreviewKeys({ setCommandDialogOpen, openTabIds, selected, selectConversation, closeConversation, setSettingsDialogOpen, openNewSession, activePanel, setActivePanel, sidebarDrawerOpen, setSidebarDrawerOpen, isCompactSidebar, setSidebarCollapsed, setKeyboardDialogOpen, pendingApproval, daemonDisconnected, desktop, toggleTerminal, cancelTurn, activeConversations, chooseFolder })
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
