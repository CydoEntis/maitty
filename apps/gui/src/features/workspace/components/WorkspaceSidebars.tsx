import { Panel } from '@/components/ui/panel'
import { LOCAL_MACHINE } from '@/api/machines'
import { UpdateSlot } from '@/features/updates/UpdateSlot'
import { skipUpdate } from '@/features/updates/updateMemory'
import { Sidebar } from './Sidebar'
import { useWorkspace } from '../workspaceContext'

const noop = () => undefined

/** The sidebar: the docked column or the rail, and the full one in a drawer over a narrow window. */
export function WorkspaceSidebars() {
  const { sidebarSessions, sidebarMachineHeads, focusedId, online, canStart, renameInline, renameId, root, preferences, sidebarWidth, setLiveSidebarWidth, setPreferences, setPaletteOpen, newSession, newTerminal, setSettingsSection, setRenamingMachine, setUpdateAsk, showSessionById, setRenameId, saveName, openInSplit, copyPathOf, copyText, showChangesOf, markDone, putAway, bringBack, snooze, unsnooze, setGetFileId, endSession, newChatOpen, serviceFooterText, updateProgress, updater, runningVersion, skippedVersion, restartNow, setSkippedVersion, focusMode, sidebarMode, toggleSidebar, drawerShown, setDrawerOpen, setRecentOpen } = useWorkspace()
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

  // The sidebar, as the docked column or the 60px rail, or the full one inside the overlay of a narrow window.
  const renderSidebar = (mode: 'docked' | 'rail', afterPick: () => void, onToggle: () => void) => (
    <Sidebar
      sessions={sidebarSessions}
      machines={sidebarMachineHeads}
      focusedId={focusedId}
      mode={mode}
      online={online}
      canStart={canStart}
      renamingId={renameInline ? renameId : null}
      portalContainer={root}
      onToggleRail={onToggle}
      side={preferences.sidebarSide}
      order={preferences.sidebarOrder}
      grouping={preferences.sidebarGroup}
      onGrouping={(sidebarGroup) => { setPreferences((current) => ({ ...current, sidebarGroup })) }}
      onSearch={() => { afterPick(); setPaletteOpen(true) }}
      width={sidebarWidth}
      onResize={setLiveSidebarWidth}
      onResizeEnd={(width) => {
        setPreferences((current) => ({ ...current, sidebarWidth: width }))
        setLiveSidebarWidth(null)
      }}
      onNewAgentChat={(machine) => { afterPick(); newSession(machine) }}
      onNewShell={(machine) => { afterPick(); newTerminal(machine) }}
      onAddMachine={() => { afterPick(); setSettingsSection('machines') }}
      onMachineSettings={() => { afterPick(); setSettingsSection('machines') }}
      onRenameMachine={setRenamingMachine}
      onUpdateMachine={(name, action) => { afterPick(); setUpdateAsk({ name, action }) }}
      onShowSession={(id) => { afterPick(); showSessionById(id) }}
      onRenameSession={setRenameId}
      onSetName={saveName}
      onSplitSession={(id, direction) => { afterPick(); openInSplit(id, direction) }}
      onCopyPath={copyPathOf}
      onCopyText={copyText}
      onShowChanges={(id) => { afterPick(); showChangesOf(id) }}
      onMarkDone={markDone}
      onPutAway={putAway}
      onBringBack={bringBack}
      onSnooze={snooze}
      onUnsnooze={unsnooze}
      onGetFile={setGetFileId}
      onEndSession={endSession}
      onOpenSettings={() => { afterPick(); setSettingsSection('appearance') }}
      onRecentSessions={() => { afterPick(); setRecentOpen(true) }}
      updateSlot={updateSlot('footer')}
      railUpdateSlot={updateSlot('rail')}
      draft={newChatOpen ? { machine: LOCAL_MACHINE } : null}
      serviceText={serviceFooterText}
      updateProgress={updateProgress}
    />
  )

  return (
    <>
      {!focusMode && renderSidebar(sidebarMode, noop, toggleSidebar)}
            {!focusMode && drawerShown && (
              <Panel label="Sessions" onClose={() => { setDrawerOpen(false) }} scrimClassName="mx-drawer-scrim" className="mx-drawer">
                {renderSidebar('docked', () => { setDrawerOpen(false) }, () => { setDrawerOpen(false) })}
              </Panel>
            )}
    </>
  )
}
