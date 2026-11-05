import { useState, type ReactNode } from 'react'
import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import type { LeadMode, TrayStyle } from '@/features/settings/preferences'
import { displayName, groupInitials, STATUS_WORD, type SessionGroup, type SessionView } from '../model/session'
import { MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { useRecent } from '@/lib/clock'
import { RailFolder } from './FolderTile'
import { SidebarResizer } from './SidebarResizer'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { SessionStack } from './SessionStack'
import { DisconnectedSince } from './times'
import { useSessionMenu, type SessionMenuActions } from './SessionMenu'
import { RailPopoverLayer } from './RailPopovers'
import { hoverHandlers, useRailHover, type RailHover } from './railHover'
import { LOCAL_MACHINE } from '@/api/machines'
import type { MachineHead } from '../model/machines'
import { WorktreeMenu } from '@/features/worktrees/WorktreeMenu'
import { useTipWithKeys } from '@/features/shortcuts/platform'

export type SidebarProps = {
  groups: SessionGroup[]
  /** Machine headers the folders sit under; empty while this computer is the only machine. */
  machineHeads: readonly MachineHead[]
  focusedId: string | null
  tracks: ReadonlyMap<string, SessionTrack>
  mode: 'docked' | 'rail'
  online: boolean
  sessionCount: number
  startFolderLabel: string
  canStart: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onToggleRail: () => void
  /** False in a window too narrow for the full sidebar: the rail has no expand button. */
  canExpand: boolean
  /** The tab (counting from 1) each session is open in; empty when only one tab is open. */
  tabOf: ReadonlyMap<string, number>
  tabPanes: ReadonlyMap<string, readonly SessionView[]>
  /** Sessions a tab shows. */
  openIds: ReadonlySet<string>
  /** The pane (counting from 1) each session has in its tab; absent for a session no tab shows. */
  paneOf: ReadonlyMap<string, number>
  /** How a project's other sessions sit under its main row, and which session leads it. */
  trayStyle: TrayStyle
  leadMode: LeadMode
  /** The docked sidebar's width in px, and how the user changes it by dragging its edge. */
  side: 'left' | 'right'
  width: number
  onResize: (width: number) => void
  onResizeEnd: (width: number) => void
  onNewSession: () => void
  /** A new shell in a project's folder. */
  onNewSessionIn: (folder: string) => void
  /** The New worktree dialog for a project, by its key. */
  onNewWorktree: (groupKey: string) => void
  onShowSession: (id: string) => void
  onEndSession: (id: string) => void
  /** Ends every session in a project; the app asks first. */
  onEndProject: (ids: string[], name: string) => void
  onSetName: (id: string, name: string) => void
  /** Opens the rename dialog, for a session with no room to rename it in place. */
  onRenameSession: (id: string) => void
  onPinSession: (id: string, pinned: boolean) => void
  onOpenSettings: () => void
  /** Opens the Usage and cost page; its icon sits beside Settings. */
  onOpenUsage: () => void
  /** The footer's left slot while the daemon is reachable: the version, or an update that needs you. */
  updateSlot: ReactNode
  /** The rail's download button, present only while an update exists. */
  railUpdateSlot: ReactNode
}


const NEEDS_SUFFIX = (count: number) => `${String(count)} need${count > 1 ? '' : 's'} you`

export function Sidebar(props: SidebarProps) {
  const { mode, tracks } = props
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())
  const [resizing, setResizing] = useState(false)
  // The session last picked in each project, so a project leads with it while you are in another one.
  const [remembered, setRemembered] = useState<ReadonlyMap<string, string>>(new Map())
  const focusedGroup = props.groups.find((group) => group.containsFocused)
  if (focusedGroup && props.focusedId && remembered.get(focusedGroup.key) !== props.focusedId) {
    setRemembered(new Map(remembered).set(focusedGroup.key, props.focusedId))
  }
  const toggle = (key: string) => {
    setCollapsed((current) => {
      const next = new Set(current)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }
  // A collapsed group opens for a moment when a session moves into it. The latest move decides
  // when that moment ends, so the sidebar re-renders once then instead of every second; a move
  // within the same window as the latest one shares its moment.
  const latestMove = Math.max(0, ...[...tracks.values()].map((track) => track.movedAt ?? 0))
  const anyRecentMove = useRecent(latestMove > 0 ? latestMove : null, MOVED_HIGHLIGHT_MS)
  const isOpen = (group: SessionGroup) => !collapsed.has(group.key) || (anyRecentMove && group.sessions.some((session) => {
    const movedAt = tracks.get(session.id)?.movedAt
    return movedAt !== null && movedAt !== undefined && movedAt >= latestMove - MOVED_HIGHLIGHT_MS
  }))
  const className = `mx-sidebar mx-card${mode === 'rail' ? ' is-rail' : ''}${resizing ? ' is-resizing' : ''}`
  return (
    <nav aria-label="Sessions by project" className={className}>
      {mode === 'rail' ? <Rail {...props} isOpen={isOpen} onToggleGroup={toggle} remembered={remembered} /> : <Expanded {...props} isOpen={isOpen} onToggleGroup={toggle} remembered={remembered} />}
      {mode === 'docked' && <SidebarResizer side={props.side} width={props.width} onResize={props.onResize} onResizeEnd={props.onResizeEnd} onDragging={setResizing} />}
    </nav>
  )
}

type GroupProps = SidebarProps & { isOpen: (group: SessionGroup) => boolean; onToggleGroup: (key: string) => void; remembered: ReadonlyMap<string, string> }

function Expanded(props: GroupProps) {
  const { groups, mode, online, isOpen, onToggleGroup } = props
  const tip = useTipWithKeys()
  const hover = useRailHover()
  const heads = props.machineHeads
  const renderGroup = (group: SessionGroup) => {
    const open = isOpen(group)
    return (
      <div key={group.key} className={`mx-group${open ? ' is-open' : ''}${group.containsFocused ? ' has-focused' : ''}`}>
        <ProjectMenu group={group} open={open} canStart={props.canStart} online={props.online} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} onNewSessionIn={props.onNewSessionIn} onNewWorktree={() => { props.onNewWorktree(group.key) }} onEnd={() => { props.onEndProject(group.sessions.map((session) => session.id), group.name) }}>
        <button type="button" className="mx-group-head" aria-expanded={open} {...hoverHandlers(hover, { kind: 'folder', key: group.key })} onClick={() => { hover.close(); onToggleGroup(group.key) }}>
          <Icon name="chevron" />
          <span className={`mx-folder-tile${group.containsFocused ? ' is-focused' : ''}`}><Icon name="folder" /></span>
          <span className="mx-group-name">{group.name}</span>
          <span className="mx-spacer" />
          {!open && online && group.workingCount > 0 && <Spinner />}
          {online && group.needsCount > 0 && <span className="mx-needs-text">{NEEDS_SUFFIX(group.needsCount)}</span>}
          <span className="mx-count">{group.sessions.length}</span>
        </button>
        </ProjectMenu>
        {open && (
          <div className="mx-group-rows">
            <span className="mx-group-guide" aria-hidden="true" />
            <SessionStack
              group={group}
              focusedId={props.focusedId}
              remembered={props.remembered.get(group.key)}
              trayStyle={props.trayStyle}
              leadMode={props.leadMode}
              tracks={props.tracks}
              online={online}
              paneOf={props.paneOf}
              portalContainer={props.portalContainer}
              hover={hover}
              onShowSession={props.onShowSession}
              onSetName={props.onSetName}
              onRenameSession={props.onRenameSession}
              onPinSession={props.onPinSession}
              onEndSession={props.onEndSession}
            />
          </div>
        )}
      </div>
    )
  }
  return (
    <div className="mx-sidebar-expanded">
      <RailPopoverLayer hover={hover} groups={groups} tabOf={props.tabOf} tabPanes={props.tabPanes} openIds={props.openIds} tracks={props.tracks} portalContainer={props.portalContainer} onShowSession={props.onShowSession} />
      <div className="mx-sidebar-head">
        <span className="mx-sidebar-title">Projects</span>
        <span className="mx-spacer" />
        <button type="button" className="mx-new-button" aria-label="New session" title={tip(`New session in ${props.startFolderLabel}`, 'new-session')} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" size={15} /></button>
        {mode === 'docked' && (
          <button type="button" className="mx-icon-button mx-collapse" aria-label="Collapse sidebar" title={tip('Collapse sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        )}
      </div>
      <div className="mx-sidebar-list">
        {heads.length === 0
          ? groups.map(renderGroup)
          : heads.map((head) => <MachineSection key={head.key} head={head} groups={groups.filter((group) => (group.machineName?.toLowerCase() ?? null) === (head.name?.toLowerCase() ?? null))} renderGroup={renderGroup} />)}
        {groups.length === 0 && <div className="mx-empty-note">No sessions. Projects show up here when a session cds into a git repo.</div>}
      </div>
      <div className="mx-sidebar-foot">
        {online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">Reconnecting to maittyd…</span></>}
        <button type="button" className="mx-icon-button mx-usage-button" aria-label="Usage and cost" title={tip('Usage and cost', 'usage')} onClick={props.onOpenUsage}><Icon name="chart" /></button>
        <button type="button" className="mx-settings-button" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" />Settings</button>
      </div>
    </div>
  )
}

/** One machine's header, with its state as a chip, and its folders under it. */
function MachineSection(props: { head: MachineHead; groups: SessionGroup[]; renderGroup: (group: SessionGroup) => ReactNode }) {
  const { head, groups, renderGroup } = props
  return (
    <section className={`mx-machine${head.down ? ' is-down' : ''}`} aria-label={head.label}>
      <div className="mx-machine-head">
        <span className="mx-machine-name">{head.label}</span>
        <span className="mx-spacer" />
        {head.chip && <span className={`mx-machine-chip is-${head.chip.tone}`}><span aria-hidden="true">{head.chip.glyph}</span> {head.chip.word}</span>}
      </div>
      {head.down && <p className="mx-machine-why"><DisconnectedSince at={head.downSince} /></p>}
      {head.why !== null && head.chip?.tone === 'bad' && <p className="mx-machine-why">{head.why}</p>}
      {groups.map(renderGroup)}
    </section>
  )
}

/** A collapsed session: its icon and state, and a popover beside it that says what is going on. */
function RailSessionButton(props: { session: SessionView; track: SessionTrack | undefined; selected: boolean; actions: SessionMenuActions; hover: RailHover }) {
  const { session, track, selected, hover, actions } = props
  const target = { kind: 'session', id: session.id } as const
  const menu = useSessionMenu(session, actions)
  const unread = Boolean(track?.unread) && !selected
  const words = `${displayName(session)}, ${session.process}, ${STATUS_WORD[session.status]}`
  return (
    <>
      <button
        type="button"
        className={`mx-rail-row${selected ? ' is-selected' : ''}`}
        aria-label={words}
        data-hovercard=""
        aria-current={selected ? 'page' : undefined}
        onMouseEnter={(event) => { hover.enter(target, event.currentTarget) }}
        onMouseLeave={hover.leave}
        onFocus={(event) => { hover.enter(target, event.currentTarget) }}
        onBlur={hover.leave}
        onContextMenu={(event) => { hover.close(); menu.onContextMenu(event) }}
        onClick={actions.onShow}
      >
        <AppIcon process={session.process} size={28} dimmed={session.status === 'unknown'} />
        <StatusBadge status={session.status} rail />
        {unread && <span className="mx-rail-unread" />}
      </button>
      {menu.element}
    </>
  )
}

/** A collapsed group: its folder, and a popover beside it that lists its sessions. */
function RailFolderButton(props: { group: SessionGroup; open: boolean; online: boolean; label: string; hover: RailHover; onToggle: () => void }) {
  const { group, open, online, hover } = props
  const target = { kind: 'folder', key: group.key } as const
  return (
    <button
      type="button"
      className={`mx-rail-label${group.containsFocused ? ' has-focused' : ''}`}
      aria-label={props.label}
      data-hovercard=""
      aria-expanded={open}
      onMouseEnter={(event) => { hover.enter(target, event.currentTarget) }}
      onMouseLeave={hover.leave}
      onFocus={(event) => { hover.enter(target, event.currentTarget) }}
      onBlur={hover.leave}
      onClick={props.onToggle}
    >
      <RailFolder initials={groupInitials(group)} focused={group.containsFocused} />
      {online && group.needsCount > 0 && <span className="mx-rail-needs" />}
      {!open && <span className="mx-rail-count">{group.sessions.length}</span>}
      {!open && online && group.workingCount > 0 && <span className="mx-rail-spin"><Spinner /></span>}
    </button>
  )
}

/** Right-click a project, in the sidebar or the rail: start a shell in it, fold it, copy its path. */
function ProjectMenu({ group, open, canStart, online, portalContainer, onToggle, onNewSessionIn, onNewWorktree, onEnd, children }: { group: SessionGroup; open: boolean; canStart: boolean; online: boolean; portalContainer: HTMLElement | null; onToggle: () => void; onNewSessionIn: (folder: string) => void; onNewWorktree: () => void; onEnd: () => void; children: ReactNode }) {
  const lead = group.sessions[0]
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>{children}</ContextMenuPrimitive.Trigger>
      <ContextMenuPrimitive.Portal container={portalContainer}>
        <ContextMenuPrimitive.Content className="mx-menu mx-project-menu" aria-label={`${group.name} project`}>
          <ContextMenuPrimitive.Label className="mx-menu-caps">{group.name}</ContextMenuPrimitive.Label>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!canStart || group.machineName !== null} onSelect={() => { onNewSessionIn(group.key) }}>New session here</ContextMenuPrimitive.Item>
          {lead && lead.machine === LOCAL_MACHINE && <WorktreeMenu term={lead.pane} portalContainer={portalContainer} canStart={canStart} onOpenFolder={onNewSessionIn} onNewWorktree={onNewWorktree} />}
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={onToggle}>{open ? 'Collapse' : 'Expand'}</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { void navigator.clipboard.writeText(group.path) }}>Copy path</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item is-danger" disabled={!online} onSelect={onEnd}>Close project tabs ({group.sessions.length})<span className="mx-menu-hint">ends {group.sessions.length === 1 ? '1 session' : `${String(group.sessions.length)} sessions`}</span></ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.Content>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Root>
  )
}

function Rail(props: GroupProps) {
  const { groups, focusedId, tracks, online, sessionCount, isOpen, onToggleGroup } = props
  const tip = useTipWithKeys()
  const hover = useRailHover()
  return (
    <div className="mx-rail">
      <RailPopoverLayer hover={hover} groups={groups} tabOf={props.tabOf} tabPanes={props.tabPanes} openIds={props.openIds} tracks={props.tracks} portalContainer={props.portalContainer} onShowSession={props.onShowSession} />
      <div className="mx-rail-top">
        {props.canExpand && <button type="button" className="mx-icon-button mx-rail-button" aria-label="Expand sidebar" title={tip('Expand sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="expand" /></button>}
        <button type="button" className="mx-icon-button mx-rail-button mx-rail-new" aria-label="New session" title={tip('New session', 'new-session')} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" /></button>
      </div>
      <div className="mx-rail-divider" />
      <div className="mx-rail-list mx-no-scrollbar">
        {groups.map((group) => {
          const open = isOpen(group)
          const label = `${group.name} · ${String(group.sessions.length)} session${group.sessions.length === 1 ? '' : 's'} · click to ${open ? 'collapse' : 'expand'}`
          return (
            <div key={group.key} className={`mx-rail-group${open ? ' is-open' : ''}${group.containsFocused ? ' has-focused' : ''}`}>
              <ProjectMenu group={group} open={open} canStart={props.canStart} online={props.online} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} onNewSessionIn={props.onNewSessionIn} onNewWorktree={() => { props.onNewWorktree(group.key) }} onEnd={() => { props.onEndProject(group.sessions.map((session) => session.id), group.name) }}>
                <RailFolderButton group={group} open={open} online={online} label={label} hover={hover} onToggle={() => { onToggleGroup(group.key) }} />
              </ProjectMenu>
              {open && (
                <div className="mx-rail-rows">
                  <div className="mx-rail-hairline" />
                  {group.sessions.map((session) => (
                    <RailSessionButton
                      key={session.id}
                      session={session}
                      track={tracks.get(session.id)}
                      selected={session.id === focusedId}
                      hover={hover}
                      actions={{
                        online,
                        portalContainer: props.portalContainer,
                        onShow: () => { props.onShowSession(session.id) },
                        onRename: () => { props.onRenameSession(session.id) },
                        onPin: (pinned) => { props.onPinSession(session.id, pinned) },
                        onEnd: () => { props.onEndSession(session.id) },
                      }}
                    />
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>
      <div className="mx-rail-bottom">
        {props.railUpdateSlot}
        <span className={`mx-daemon-dot${online ? '' : ' is-offline'}`} role="img" aria-label={online ? `maittyd · ${String(sessionCount)} sessions` : 'Reconnecting to maittyd…'} title={online ? `maittyd · ${String(sessionCount)} sessions` : 'Reconnecting to maittyd…'} />
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Usage and cost" title={tip('Usage and cost', 'usage')} onClick={props.onOpenUsage}><Icon name="chart" /></button>
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" /></button>
      </div>
    </div>
  )
}
