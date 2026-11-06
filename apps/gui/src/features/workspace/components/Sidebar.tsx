import { useState, type ReactNode } from 'react'
import { ContextMenu as ContextMenuPrimitive, DropdownMenu } from 'radix-ui'
import type { TrayStyle } from '@/features/settings/preferences'
import { displayName, groupInitials, STATUS_WORD, type SessionGroup, type SessionView } from '../model/session'
import { MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { useRecent } from '@/lib/clock'
import { RailFolder } from './FolderTile'
import { SidebarResizer } from './SidebarResizer'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { SessionStack } from './SessionStack'
import { AgentMenu } from './AgentMenu'
import type { ChatAgent } from '../model/chatView'
import { DisconnectedSince } from './times'
import { useSessionMenu, type SessionMenuActions } from './SessionMenu'
import { RailPopoverLayer } from './RailPopovers'
import { hoverHandlers, useRailHover, type RailHover } from './railHover'
import { LOCAL_MACHINE } from '@/api/machines'
import type { MachineHead } from '../model/machines'
import { isChatSession } from '../model/chatSession'
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
  /** The docked sidebar's width in px, and how the user changes it by dragging its edge. */
  side: 'left' | 'right'
  width: number
  onResize: (width: number) => void
  onResizeEnd: (width: number) => void
  onNewSession: () => void
  /** A new chat with this agent, whatever the default is. */
  onNewSessionWith: (agent: ChatAgent) => void
  /** The agent the + button starts. */
  defaultAgent: ChatAgent
  /** A new shell in a project's folder. */
  onNewSessionIn: (folder: string) => void
  /** A plain shell, in the start folder or in a project's folder. */
  onNewTerminal: () => void
  /** A shell in a project's folder, on the machine the project is on (null: this computer). */
  onNewTerminalIn: (folder: string, machineName: string | null) => void
  /** "New session here" on a project of another machine: chats run on this computer only. */
  onLocalOnly: (machineName: string) => void
  /** A new shell on another machine; it asks for a folder there. */
  onNewSessionOn: (machine: number, name: string) => void
  /** The New worktree dialog for a project, by its key. */
  onNewWorktree: (groupKey: string) => void
  /** Shows the project's repository in the Inspect panel, on the session that lives in it. */
  onInspectProject: (groupKey: string) => void
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
  // Which machine's sessions the list shows; every machine until one is picked in the header's menu.
  const [machineFilter, setMachineFilter] = useState<number | null>(null)
  const shownHeads = machineFilter === null ? heads : heads.filter((head) => head.machine === machineFilter)
  const renderGroup = (group: SessionGroup) => {
    const open = isOpen(group)
    return (
      <div key={group.key} className={`mx-group${open ? ' is-open' : ''}${group.containsFocused ? ' has-focused' : ''}`}>
        <ProjectMenu group={group} open={open} canStart={props.canStart} online={props.online} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} onNewSessionIn={props.onNewSessionIn} onNewTerminalIn={props.onNewTerminalIn} onLocalOnly={props.onLocalOnly} onNewWorktree={() => { props.onNewWorktree(group.key) }} onInspect={() => { props.onInspectProject(group.key) }} onEnd={() => { props.onEndProject(group.sessions.map((session) => session.id), group.name) }}>
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
        {heads.length === 0
          ? <span className="mx-sidebar-title">Projects</span>
          : <MachineSwitcher heads={heads} filter={machineFilter} onFilter={setMachineFilter} portalContainer={props.portalContainer} />}
        <span className="mx-spacer" />
        <button type="button" className="mx-new-button" aria-label="New terminal" title={tip('New terminal: a plain shell, terminal view only', 'new-terminal')} disabled={!props.canStart} onClick={props.onNewTerminal}><Icon name="prompt" size={15} /></button>
        <button type="button" className="mx-new-button" aria-label="New session" title={tip(`New session in ${props.startFolderLabel}`, 'new-session')} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" size={15} /></button>
        <AgentMenu disabled={!props.canStart} defaultAgent={props.defaultAgent} onPick={props.onNewSessionWith} />
        {mode === 'docked' && (
          <button type="button" className="mx-icon-button mx-collapse" aria-label="Collapse sidebar" title={tip('Collapse sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        )}
      </div>
      <div className="mx-sidebar-list">
        {heads.length === 0
          ? groups.map(renderGroup)
          : shownHeads.map((head) => <MachineSection key={head.key} head={head} online={props.online} onNewSessionOn={props.onNewSessionOn} groups={groups.filter((group) => (group.machineName?.toLowerCase() ?? null) === (head.name?.toLowerCase() ?? null))} renderGroup={renderGroup} />)}
        {groups.length === 0 && <div className="mx-empty-note">No sessions. Projects show up here when a session cds into a git repo.</div>}
      </div>
      <div className="mx-sidebar-foot">
        {online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">Reconnecting to maittyd…</span></>}
        <button type="button" className="mx-settings-button" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" />Settings</button>
      </div>
    </div>
  )
}

/** One machine's header, with its state as a chip, and its folders under it. */
function MachineSection(props: { head: MachineHead; online: boolean; onNewSessionOn: (machine: number, name: string) => void; groups: SessionGroup[]; renderGroup: (group: SessionGroup) => ReactNode }) {
  const { head, groups, renderGroup, onNewSessionOn } = props
  const target = head.name
  const up = props.online && head.chip?.tone === 'ok'
  // What the + says when it cannot be used: the machine's own state, in words.
  const why = target === null ? '' : `${target} is ${head.chip?.word ?? 'not connected'} · can’t open a terminal`
  return (
    <section className={`mx-machine${head.down ? ' is-down' : ''}`} aria-label={head.label}>
      <div className="mx-machine-head">
        <span className="mx-machine-tile" aria-hidden="true">{(head.label.at(0) ?? '?').toUpperCase()}</span>
        <span className="mx-machine-name">{head.label}</span>
        <span className="mx-spacer" />
        {head.chip && <span className={`mx-machine-chip is-${head.chip.tone}`}><span aria-hidden="true">{head.chip.glyph}</span> {head.chip.word}</span>}
        {target !== null && (
          <button type="button" className="mx-machine-new" aria-label={`New terminal on ${target}`} title={up ? `New terminal on ${target}` : why} aria-disabled={!up} disabled={!up} onClick={() => { onNewSessionOn(head.machine, target) }}><Icon name="plus" size={13} /></button>
        )}
      </div>
      {head.down && <p className="mx-machine-why"><DisconnectedSince at={head.downSince} /></p>}
      {head.why !== null && head.chip?.tone === 'bad' && <p className="mx-machine-why">{head.why}</p>}
      {target !== null && up && groups.length === 0 && <p className="mx-machine-why">No sessions. Use + to open a terminal here.</p>}
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
function ProjectMenu({ group, open, canStart, online, portalContainer, onToggle, onNewSessionIn, onNewTerminalIn, onLocalOnly, onNewWorktree, onInspect, onEnd, children }: { group: SessionGroup; open: boolean; canStart: boolean; online: boolean; portalContainer: HTMLElement | null; onToggle: () => void; onNewSessionIn: (folder: string) => void; onNewTerminalIn: (folder: string, machineName: string | null) => void; onLocalOnly: (machineName: string) => void; onNewWorktree: () => void; onInspect: () => void; onEnd: () => void; children: ReactNode }) {
  // A worktree is made from a terminal's checkout; a chat session has no terminal to ask.
  const lead = group.sessions.find((session) => !isChatSession(session.conversation))
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>{children}</ContextMenuPrimitive.Trigger>
      <ContextMenuPrimitive.Portal container={portalContainer}>
        <ContextMenuPrimitive.Content className="mx-menu mx-project-menu" aria-label={`${group.name} project`}>
          <ContextMenuPrimitive.Label className="mx-menu-caps">{group.name}</ContextMenuPrimitive.Label>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!canStart} onSelect={() => { if (group.machineName === null) onNewSessionIn(group.key); else onLocalOnly(group.machineName) }}>New session here{group.machineName !== null && <span className="mx-menu-hint"> · local only</span>}</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!canStart} onSelect={() => { onNewTerminalIn(group.machineName === null ? group.key : group.path, group.machineName) }}>New terminal here</ContextMenuPrimitive.Item>
          {lead && lead.machine === LOCAL_MACHINE && <WorktreeMenu term={lead.pane} portalContainer={portalContainer} canStart={canStart} onOpenFolder={onNewSessionIn} onNewWorktree={onNewWorktree} />}
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={onToggle}>{open ? 'Collapse' : 'Expand'}</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={lead === undefined || lead.machine !== LOCAL_MACHINE} onSelect={onInspect}>Inspect project</ContextMenuPrimitive.Item>
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
              <ProjectMenu group={group} open={open} canStart={props.canStart} online={props.online} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} onNewSessionIn={props.onNewSessionIn} onNewTerminalIn={props.onNewTerminalIn} onLocalOnly={props.onLocalOnly} onNewWorktree={() => { props.onNewWorktree(group.key) }} onInspect={() => { props.onInspectProject(group.key) }} onEnd={() => { props.onEndProject(group.sessions.map((session) => session.id), group.name) }}>
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
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" /></button>
      </div>
    </div>
  )
}

/** The sidebar header's machine menu (v19 §8): which machine's sessions the list shows. An amber dot says some machine is not up. */
function MachineSwitcher({ heads, filter, onFilter, portalContainer }: { heads: readonly MachineHead[]; filter: number | null; onFilter: (machine: number | null) => void; portalContainer: HTMLElement | null }) {
  const picked = filter === null ? undefined : heads.find((head) => head.machine === filter)
  const anyDown = heads.some((head) => head.down)
  const nameOf = (head: MachineHead) => head.name ?? 'This computer'
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger className="mx-machine-switcher" aria-label={`Show sessions on: ${picked === undefined ? 'all machines' : nameOf(picked)}`}>
        <span className="mx-sidebar-title">{picked === undefined ? 'All machines' : nameOf(picked)}</span>
        <Icon name="chevron" />
        {anyDown && <span className="mx-machine-switcher-dot" role="img" aria-label="A machine is not connected" />}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal container={portalContainer}>
        <DropdownMenu.Content className="mx-menu mx-machine-menu" sideOffset={6} align="start">
          <DropdownMenu.Label className="mx-menu-caps">Show sessions on</DropdownMenu.Label>
          <DropdownMenu.Item className="mx-menu-item" onSelect={() => { onFilter(null) }}>
            All machines{filter === null && <span className="mx-menu-hint">✓</span>}
          </DropdownMenu.Item>
          {heads.map((head) => (
            <DropdownMenu.Item key={head.key} className="mx-menu-item" onSelect={() => { onFilter(head.machine) }}>
              {nameOf(head)}
              <span className="mx-menu-hint">{head.chip?.word ?? 'local'}{filter === head.machine ? ' ✓' : ''}</span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}
