import { FolderOpen, LoaderCircle, MessageSquarePlus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { TitleBar } from '@/components/TitleBar'
import { Toaster } from '@/components/ui/sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { daemonConnectionError } from '@/lib/daemon-connection'
import { focusMainContent } from './shared'
import { ConversationSidebar } from './Sidebar'
import { NewConversationDialog } from './NewConversationDialog'
import { usePreview } from './previewContext'

/** The screen with no conversation open: the sidebar and what to start. */
export function PreviewEmpty() {
  const { statusAnnouncement, filter, sidebarPreferences, isCompactSidebar, sidebarCollapsed, daemonDisconnected, sidebarDrawerOpen, setFilter, openNewSession, togglePin, setArchived, renameConversation, requestEndSession, selectConversation, setSidebarDrawerOpen, setSidebarCollapsed, startingSessions, sessionsQuery, setSettingsDialogOpen, primaryModifier, codexExecutableQuery, desktop, claudeExecutableQuery, daemonRestartAction, mismatch, daemonRestartStatus, newDialogOpen, newProject, setNewProject, newProvider, setNewProvider, newKind, setNewKind, newPermissions, setNewPermissions, chooseFolder, isChoosingFolder, setNewDialogOpen, createConversation, conversationCreationPending, settingsDialog, daemonRestartDialog, settingsPreferences, theme } = usePreview()
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
}
