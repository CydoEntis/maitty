import { Fragment } from 'react'
import { toast } from '@/lib/toast'
import { openDaemonLog } from '@/api/daemon'
import { LOCAL_MACHINE } from '@/api/machines'
import { TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN } from '@/features/settings/preferences'
import { daemonConnectionError } from '@/lib/daemon-connection'
import { errorMessage } from '@/lib/error-message'
import { SplitHandle } from './SplitHandle'
import { StatusBar } from './StatusBar'
import { FindBar } from '@/features/find/FindBar'
import { RestoreBanner } from './RestoreBanner'
import { chatFindOf } from '@/features/chat/chatSearch'
import { chooseProjectFolder } from '@/api/folderPicker'
import { machineProjects } from '../model/newSession'
import { NewChatPage } from './NewChatPage'
import { EmptyState } from './states'
import { ServiceStatus } from './ServiceStatus'
import { paneColumns, setColumnWidths, setPaneHeights } from '../model/layout'
import { displayName } from '../model/session'
import { resolveStartFolder } from '../model/startFolder'
import { WorkspacePane } from './WorkspacePane'
import { useWorkspace } from '../workspaceContext'

/** The middle of the window: the new chat page, find, the service's state, the panes of each mounted tab and the status bar. */
export function WorkspaceMain() {
  const { serviceFooterText, newChatOpen, sidebarMachineHeads, setNewChatOpen, setNewSessionMachine, sessions, focusedSession, preferences, home, startNewChat, setSettingsSection, findOpen, online, heads, focusedId, terminalHandles, showSessionById, setFindOpen, dismissedRestores, resumeInterrupted, setDismissedRestores, failures, mismatch, retryAt, offlineDetail, sessionsQuery, restartOffer, appBehindOffer, refetchSessions, tab, mountedTabs, layout, narrow, setLayout, closingPanes, startFolderLabel, canStart, newSession, focusedMachineNumber, focusMode, focusMachine, theme, shortcutChord, root, setThemeMode, pickTheme, openAppearance, setStyle, setTextSize, resetTextSize, openShortcuts } = useWorkspace()
  return (
    <div className="mx-main-wrap">
              <main className="mx-main" aria-label="Terminals">
                {newChatOpen && (
                  <NewChatPage
                    machines={sidebarMachineHeads.filter((head) => head.machine === LOCAL_MACHINE || head.chip?.tone === 'ok').map((head) => ({ machine: head.machine, label: head.label, local: head.machine === LOCAL_MACHINE }))}
                    onOtherMachine={(machine) => { setNewChatOpen(false); setNewSessionMachine(machine) }}
                    projects={machineProjects(sessions, LOCAL_MACHINE, focusedSession, resolveStartFolder(preferences.startFolder, home))}
                    defaultAgent={preferences.defaultAgent}
                    bypassAllowed={preferences.yoloClaude}
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
                    findInPane={(query, direction) => {
                      if (!focusedId) return false
                      // A pane that shows its chat is searched as the chat; its terminal is under it and out of sight.
                      const chat = chatFindOf(focusedId)
                      return chat ? chat.find(query, direction) : terminalHandles.current.get(focusedId)?.find(query, direction) ?? false
                    }}
                    clearPane={() => {
                      if (!focusedId) return
                      chatFindOf(focusedId)?.clear()
                      terminalHandles.current.get(focusedId)?.clearFind()
                    }}
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
                <ServiceStatus online={online} failures={failures} mismatch={Boolean(mismatch)} retryAt={retryAt} detail={offlineDetail ?? (sessionsQuery.isError ? daemonConnectionError(sessionsQuery.error) : null)} restart={restartOffer} appBehind={appBehindOffer} sessions={sessions.length} onRetry={() => { void refetchSessions() }} onOpenLog={() => { openDaemonLog().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not open the log')) }) }} />
                {tab ? (
                  <div className="mx-pane-stack">
                  {mountedTabs.map((tab) => {
                  const kept = tab.id !== layout.activeId
                  return (
                  <div key={tab.id} className={`mx-panes${tab.panes.length > 1 && tab.max === null ? ' is-split' : ''}${kept ? ' is-kept' : ''}`} aria-hidden={kept ? true : undefined} inert={kept}>
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
                              <WorkspacePane shown={tab} pane={pane} kept={kept} />
                            </div>
                          </Fragment>
                        ))}
                      </div>
                      </Fragment>
                    ))}
                  </div>
                  )
                  })}
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
                    offlineText={mismatch ? serviceFooterText : undefined}
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
  )
}
