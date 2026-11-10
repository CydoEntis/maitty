import { listSshHosts, machineActivity } from '@/api/machines'
import { DesktopSettings } from '@/features/settings/components/DesktopSettings'
import { PerfOverlay } from './PerfOverlay'
import { CommandPalette } from './CommandPalette'
import { InspectPanel } from '@/features/inspect/InspectPanel'
import { AgentTerminals } from './AgentTerminals'
import { HoverTips } from './HoverTips'
import { ShortcutOverlay } from '@/features/shortcuts/ShortcutOverlay'
import { WelcomeDialog } from '@/features/welcome/WelcomeDialog'
import { markWelcomed } from '@/features/welcome/welcomed'
import { UpdatedNotice } from '@/features/updates/UpdatedNotice'
import { indexRemoteFiles, listRemoteFolder } from '@/api/fileGets'
import { updaterSummary } from '@/features/updates/useUpdater'
import { NewWorktreeDialog } from '@/features/worktrees/NewWorktreeDialog'
import { NewSessionHost } from './NewSessionHost'
import { GetFileDialog } from './GetFileDialog'
import { MachineUpdateDialog } from './MachineUpdateDialog'
import { ConfirmCloseDialog } from './ConfirmCloseDialog'
import { RenameDialog, RenameMachineDialog } from './RenameDialog'
import { withNickname, writeMachineNicknames } from '@/data/machineNicknames'
import { Toast } from './states'
import type { SessionView } from '../model/session'
import { isShell } from '../model/sidebar'
import { agentName, sendAgentMessage, suggestedCommitMessage } from '../model/agentMessage'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { folderExists, resolveStartFolder } from '../model/startFolder'
import { useWorkspace } from '../workspaceContext'

/** What opens over or beside the panes: Inspect, the palette, Settings and each dialog. */
export function WorkspaceDialogs() {
  const { mismatch, inspectOpen, focusMode, inspectTerm, ownsTerminals, focusedSession, projectOf, diffStat, inspectView, lookAt, setInspectView, setInspectOpen, flash, shortcutsOpen, platform, root, setShortcutsOpen, welcomeOpen, preferences, setPreferences, setWelcomeOpen, newSession, focusedMachineNumber, paletteOpen, paletteItems, setPaletteOpen, focusedId, sessionMap, needsElsewhere, setFocusMode, focusModeChord, worktreeFrom, createWorktree, setWorktreeFrom, newSessionMachine, heads, sessions, home, startFromModal, setNewSessionMachine, getFileId, setGetFileId, getFile, updateAsk, setUpdateAsk, updateNamed, closeAsk, setCloseAsk, renamingMachine, machineNicknames, setMachineNicknames, setRenamingMachine, renameId, renameInline, saveName, setRenameId, settingsSection, setSettingsSection, systemReducedMotion, systemDark, previewThemeSoon, showTestNotice, updater, restartNow, setPreviewTheme, online, runningVersion, machinesKnown, retryNamed, removeNamed, grantNamed, copyCommand, addNamed, cancelAdding, usageInfo, chooseStartFolder, toastMessage, updatedTo, setUpdatedTo, restarting } = useWorkspace()
  // An agent always has a Terminal tab. Pinned under the chat, its terminals are in the pane, and the tab offers to bring them back.
  const hasTerminals = focusedSession !== undefined && ownsTerminals(focusedSession)
  return (
    <>
      {inspectOpen && !focusMode && focusedSession && (inspectTerm !== null || hasTerminals) && (
              <InspectPanel
                key={focusedSession.id}
                term={inspectTerm}
                terminals={!hasTerminals ? null : preferences.terminalDock === 'panel' || isShell(focusedSession)
                  ? <AgentTerminals agent={focusedSession} dock="panel" focused={inspectView === 'terminal'} canPin={!isShell(focusedSession)} />
                  : (
                    <div className="mx-terms-empty">
                      <strong>Terminals are pinned under the chat</strong>
                      <span>They show at the bottom of this agent's pane.</span>
                      <button type="button" className="mx-terms-start" onClick={() => { setPreferences((current) => ({ ...current, terminalDock: 'panel' })) }}>Show them here</button>
                    </div>
                  )}
                changedFiles={diffStat.data?.files ?? 0}
                {...(focusedSession.branch ? { branch: focusedSession.branch } : {})}
                view={inspectView}
                agentWorking={focusedSession.status === 'working'}
                timeline={focusedSession.timeline}
                lookAt={lookAt}
                {...(isShell(focusedSession) ? {} : { comments: { agent: agentName(focusedSession.process), onSend: (message: string) => sendAgentMessage(focusedSession, message) } })}
                {...(focusedSession.machineName === null && focusedSession.repoRoot !== null ? { changeActions: { folder: focusedSession.repoRoot, suggested: suggestedCommitMessage(focusedSession) } } : {})}
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
            {renamingMachine !== null && (
              <RenameMachineDialog
                machine={renamingMachine}
                nickname={machineNicknames[renamingMachine.toLowerCase()] ?? ''}
                onSave={(nickname) => {
                  const next = withNickname(machineNicknames, renamingMachine, nickname)
                  setMachineNicknames(next)
                  writeMachineNicknames(next)
                  setRenamingMachine(null)
                }}
                onCancel={() => { setRenamingMachine(null) }}
              />
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
            {/* Not while the service still runs the older version: the update is not finished, and its card says so. */}
            {updatedTo && !mismatch && <UpdatedNotice update={updatedTo} onDone={() => { setUpdatedTo(null) }} />}
            {preferences.perfOverlay && <PerfOverlay />}
            {restarting && (
              <div className="mx-restarting" role="status">
                <strong>Restarting Maitty</strong>
                <span>Reopening your layout. Your sessions never stopped.</span>
              </div>
            )}
    </>
  )
}
