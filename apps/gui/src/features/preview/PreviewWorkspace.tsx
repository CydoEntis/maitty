import { CircleAlert } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { TitleBar } from '@/components/TitleBar'
import { TerminalPanes } from '@/features/terminal/TerminalPanes'
import { WorkingSinceContext } from '@/lib/working-since'
import { Toaster } from '@/components/ui/sonner'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { TooltipProvider } from '@/components/ui/tooltip'
import type { Message } from '@/data/preview'
import { focusMainContent, RECENT_PROJECT_LIMIT } from './shared'
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
import { usePreview } from './previewContext'

/** The screen with a conversation open: the sidebar, its tabs, the feed, the composer and each dialog. */
export function PreviewWorkspace() {
  const { workingSince, statusAnnouncement, shownPanel, isCompactSidebar, sidebarDrawerOpen, setSidebarDrawerOpen, filteredConversations, selected, filter, sidebarPreferences, sidebarCollapsed, daemonDisconnected, setFilter, openNewSession, togglePin, setArchived, renameConversation, requestEndSession, selectInCurrentView, setSidebarCollapsed, setNewProject, startingSessions, desktop, conversations, setSettingsDialogOpen, primaryModifier, openTabs, closeConversation, showTerminal, setActivePanel, toggleTerminal, activeConversations, terminalSwitchNotice, sessionsQuery, daemonRestartAction, settingsPreferences, pendingApproval, recoveryPendingId, resumeSelectedConversation, messages, approvalCollapsed, setApprovalCollapsed, resolveApproval, answerInput, draft, historyQuery, deliveryNotice, checkSubmissionDelivery, sendMessage, setDrafts, clearSendFailure, updateDraft, handleComposerKeyDown, denyDecision, denyQueuedFor, denyAndSend, cancelTurn, newDialogOpen, newProject, newProvider, setNewProvider, newKind, setNewKind, newPermissions, setNewPermissions, claudeExecutableQuery, chooseFolder, isChoosingFolder, setNewDialogOpen, createConversation, conversationCreationPending, commandDialogOpen, setCommandDialogOpen, selectConversation, setKeyboardDialogOpen, openConversationInTerminal, keyboardDialogOpen, settingsDialog, endTarget, setEndTarget, confirmEndSession, daemonRestartDialog, theme } = usePreview()
  if (!selected) return null
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
                      variant="soft"
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
              <Button variant="danger" onClick={confirmEndSession}>
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
