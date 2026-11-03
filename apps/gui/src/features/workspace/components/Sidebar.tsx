import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import type { LeadMode, TrayStyle } from '@/features/settings/preferences'
import { displayName, groupInitials, STATUS_WORD, truncatePrompt, type SessionGroup, type SessionView } from '../model/session'
import { STATUS_LABEL } from '../model/statusText'
import { formatAgo, formatElapsed, MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { RailFolder } from './FolderTile'
import { SidebarResizer } from './SidebarResizer'
import { folderStyle } from './folderStyle'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { keepFocus } from './menuFocus'
import { SessionStack } from './SessionStack'
import { WorktreeMenu } from '@/features/worktrees/WorktreeMenu'

export type SidebarProps = {
  groups: SessionGroup[]
  focusedId: string | null
  tracks: ReadonlyMap<string, SessionTrack>
  now: number
  mode: 'docked' | 'rail'
  online: boolean
  sessionCount: number
  primary: string
  startFolderLabel: string
  canStart: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onToggleRail: () => void
  /** False in a window too narrow for the full sidebar: the rail has no expand button. */
  canExpand: boolean
  /** The tab (counting from 1) each session is open in; empty when only one tab is open. */
  tabOf: ReadonlyMap<string, number>
  /** The pane (counting from 1) each session has in its tab; absent for a session no tab shows. */
  paneOf: ReadonlyMap<string, number>
  /** How a project's other sessions sit under its main row, and which session leads it. */
  trayStyle: TrayStyle
  leadMode: LeadMode
  /** The docked sidebar's width in px, and how the user changes it by dragging its edge. */
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
  onRenameSession: (id: string) => void
  onOpenSettings: () => void
}

const HALF = 0.5

/** The space between a collapsed folder and its popover. */
const POPOVER_GAP_PX = 10
/** How close to the top or bottom of the window a popover's middle may sit before it is held back. */
const POPOVER_EDGE_PX = 90

const NEEDS_SUFFIX = (count: number) => `${String(count)} need${count > 1 ? '' : 's'} you`

export function Sidebar(props: SidebarProps) {
  const { mode, now, tracks } = props
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
  // A collapsed group opens for a moment when a session moves into it.
  const isOpen = (group: SessionGroup) => !collapsed.has(group.key) || group.sessions.some((session) => {
    const movedAt = tracks.get(session.id)?.movedAt
    return movedAt !== null && movedAt !== undefined && now - movedAt < MOVED_HIGHLIGHT_MS
  })
  const className = `mx-sidebar mx-card${mode === 'rail' ? ' is-rail' : ''}${resizing ? ' is-resizing' : ''}`
  return (
    <nav aria-label="Sessions by project" className={className}>
      {mode === 'rail' ? <Rail {...props} isOpen={isOpen} onToggleGroup={toggle} remembered={remembered} /> : <Expanded {...props} isOpen={isOpen} onToggleGroup={toggle} remembered={remembered} />}
      {mode === 'docked' && <SidebarResizer width={props.width} onResize={props.onResize} onResizeEnd={props.onResizeEnd} onDragging={setResizing} />}
    </nav>
  )
}

type GroupProps = SidebarProps & { isOpen: (group: SessionGroup) => boolean; onToggleGroup: (key: string) => void; remembered: ReadonlyMap<string, string> }

function DaemonFooterText({ online, sessionCount }: Pick<SidebarProps, 'online' | 'sessionCount'>) {
  return <>{online ? `maittyd · ${String(sessionCount)} session${sessionCount === 1 ? '' : 's'}` : 'Reconnecting to maittyd…'}</>
}

function Expanded(props: GroupProps) {
  const { groups, mode, primary, online, sessionCount, isOpen, onToggleGroup } = props
  return (
    <div className="mx-sidebar-expanded">
      <div className="mx-sidebar-head">
        <span className="mx-sidebar-title">Projects</span>
        <span className="mx-spacer" />
        <button type="button" className="mx-new-button" aria-label="New session" title={`New session in ${props.startFolderLabel} (${primary}T)`} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" size={15} /></button>
        {mode === 'docked' && (
          <button type="button" className="mx-icon-button mx-collapse" aria-label="Collapse sidebar" title={`Collapse sidebar (${primary}B)`} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        )}
      </div>
      <div className="mx-sidebar-list">
        {groups.map((group) => {
          const open = isOpen(group)
          return (
            <div key={group.key} className={`mx-group${open ? ' is-open' : ''}${group.containsFocused ? ' has-focused' : ''}`}>
              <ProjectMenu group={group} open={open} canStart={props.canStart} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} onNewSessionIn={props.onNewSessionIn} onNewWorktree={() => { props.onNewWorktree(group.key) }}>
              <button type="button" className="mx-group-head" aria-expanded={open} title={group.path} onClick={() => { onToggleGroup(group.key) }}>
                <Icon name="chevron" />
                <span className={`mx-folder-tile${group.containsFocused ? ' is-focused' : ''}`} style={folderStyle(group.key, group.colorId)}><Icon name="folder" /></span>
                <span className="mx-group-name">{group.name}</span>
                {group.branch && <span className="mx-group-branch">{group.branch}</span>}
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
                    now={props.now}
                    online={online}
                    tabOf={props.tabOf}
                    paneOf={props.paneOf}
                    portalContainer={props.portalContainer}
                    onShowSession={props.onShowSession}
                    onRenameSession={props.onRenameSession}
                    onEndSession={props.onEndSession}
                  />
                </div>
              )}
            </div>
          )
        })}
        {groups.length === 0 && <div className="mx-empty-note">No sessions. Projects show up here when a session cds into a git repo.</div>}
      </div>
      <div className="mx-sidebar-foot">
        <span className={`mx-daemon-dot${online ? '' : ' is-offline'}`} aria-hidden="true" />
        <span className={`mx-daemon-text${online ? '' : ' is-offline'}`} role="status"><DaemonFooterText online={online} sessionCount={sessionCount} /></span>
        <button type="button" className="mx-settings-button" title={`Settings (${primary},)`} onClick={props.onOpenSettings}><Icon name="gear" />Settings</button>
      </div>
    </div>
  )
}

/** A collapsed session: its icon and state, and a popover beside it that says what is going on. */
function RailSessionButton(props: { session: SessionView; track: SessionTrack | undefined; now: number; selected: boolean; tabNumber: number | undefined; online: boolean; portalContainer: HTMLElement | null; onShow: () => void; onRename: () => void; onEnd: () => void }) {
  const { session, track, now, selected, portalContainer } = props
  const [anchor, setAnchor] = useState<DOMRect | null>(null)
  const show = (element: HTMLElement) => { setAnchor(element.getBoundingClientRect()) }
  const unread = Boolean(track?.unread) && !selected
  const words = `${displayName(session)}, ${session.process}, ${STATUS_WORD[session.status]}`
  return (
    <>
      <ContextMenuPrimitive.Root onOpenChange={(open) => { if (open) setAnchor(null) }}>
        <ContextMenuPrimitive.Trigger asChild>
          <button
            type="button"
            className={`mx-rail-row${selected ? ' is-selected' : ''}`}
            aria-label={words}
            aria-current={selected ? 'page' : undefined}
            onMouseEnter={(event) => { show(event.currentTarget) }}
            onMouseLeave={() => { setAnchor(null) }}
            onFocus={(event) => { show(event.currentTarget) }}
            onBlur={() => { setAnchor(null) }}
            onClick={props.onShow}
          >
            <AppIcon process={session.process} size={28} dimmed={session.status === 'unknown'} />
            <StatusBadge status={session.status} rail />
            {unread && <span className="mx-rail-unread" />}
          </button>
        </ContextMenuPrimitive.Trigger>
        <ContextMenuPrimitive.Portal container={portalContainer}>
          <ContextMenuPrimitive.Content className="mx-menu" onCloseAutoFocus={keepFocus}>
            <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={props.onShow}>Show in pane</ContextMenuPrimitive.Item>
            <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={props.onRename}>Rename…</ContextMenuPrimitive.Item>
            <ContextMenuPrimitive.Separator className="mx-menu-sep" />
            <ContextMenuPrimitive.Item className="mx-menu-item is-danger" disabled={!props.online} onSelect={props.onEnd}>Close session<span className="mx-menu-hint">ends it</span></ContextMenuPrimitive.Item>
          </ContextMenuPrimitive.Content>
        </ContextMenuPrimitive.Portal>
      </ContextMenuPrimitive.Root>
      {anchor && portalContainer && createPortal(<SessionPopover session={session} track={track} now={now} tabNumber={props.tabNumber} anchor={anchor} />, portalContainer)}
    </>
  )
}

/** What a session is doing, in words: the agent, its state and what that means, where it is and for how long. */
function SessionPopover({ session, track, now, tabNumber, anchor }: { session: SessionView; track: SessionTrack | undefined; now: number; tabNumber: number | undefined; anchor: DOMRect }) {
  const status = session.status
  const since = status === 'working' && track?.workingSince ? `for ${formatElapsed(now - track.workingSince)}` : track?.changedAt && status !== 'unknown' ? `${formatAgo(now - track.changedAt)} ago` : ''
  // Kept inside the window: a popover for a row near the top or bottom slides in rather than being cut off.
  const top = Math.min(Math.max(anchor.top + anchor.height * HALF, POPOVER_EDGE_PX), window.innerHeight - POPOVER_EDGE_PX)
  return (
    <div role="tooltip" className="mx-rail-popover is-session" style={{ top, left: anchor.right + POPOVER_GAP_PX }}>
      <div className="mx-pop-head">
        <AppIcon process={session.process} size={22} dimmed={status === 'unknown'} />
        <strong>{session.process}</strong>
        <span className={`mx-pop-state is-${status}`}>{STATUS_LABEL[status]}{since ? ` · ${since}` : ''}</span>
      </div>
      {session.name && <div className="mx-pop-name">{session.name}</div>}
      <div className="mx-pop-path">{session.cwdLabel}</div>
      {tabNumber !== undefined && <div className="mx-pop-tab">Open in tab {tabNumber}</div>}
      {session.repoName && <div className="mx-pop-repo"><Icon name="branch" />{session.repoName}{session.branch ? ` · ${session.branch}` : ''}</div>}
      {session.lastPrompt && <p className="mx-pop-prompt"><span className="mx-pop-label">Last prompt</span>{truncatePrompt(session.lastPrompt)}</p>}
    </div>
  )
}

/** A collapsed group: its folder, and a popover beside it that says what it is. */
function RailFolderButton(props: { group: SessionGroup; open: boolean; online: boolean; label: string; portalContainer: HTMLElement | null; onToggle: () => void }) {
  const { group, open, online, portalContainer } = props
  const [anchor, setAnchor] = useState<DOMRect | null>(null)
  const show = (element: HTMLElement) => { setAnchor(element.getBoundingClientRect()) }
  return (
    <>
      <button
        type="button"
        className={`mx-rail-label${group.containsFocused ? ' has-focused' : ''}`}
        aria-label={props.label}
        aria-expanded={open}
        onMouseEnter={(event) => { show(event.currentTarget) }}
        onMouseLeave={() => { setAnchor(null) }}
        onFocus={(event) => { show(event.currentTarget) }}
        onBlur={() => { setAnchor(null) }}
        onClick={props.onToggle}
      >
        <RailFolder colorKey={group.key} colorId={group.colorId} initials={groupInitials(group)} />
        {online && group.needsCount > 0 && <span className="mx-rail-needs" />}
      </button>
      {anchor && portalContainer && createPortal(<FolderPopover group={group} anchor={anchor} />, portalContainer)}
    </>
  )
}

/** Right-click a project, in the sidebar or the rail: start a shell in it, fold it, copy its path. */
function ProjectMenu({ group, open, canStart, portalContainer, onToggle, onNewSessionIn, onNewWorktree, children }: { group: SessionGroup; open: boolean; canStart: boolean; portalContainer: HTMLElement | null; onToggle: () => void; onNewSessionIn: (folder: string) => void; onNewWorktree: () => void; children: ReactNode }) {
  const lead = group.sessions[0]
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>{children}</ContextMenuPrimitive.Trigger>
      <ContextMenuPrimitive.Portal container={portalContainer}>
        <ContextMenuPrimitive.Content className="mx-menu mx-project-menu" aria-label={`${group.name} project`}>
          <ContextMenuPrimitive.Label className="mx-menu-caps">{group.name}</ContextMenuPrimitive.Label>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!canStart} onSelect={() => { onNewSessionIn(group.key) }}>New session here</ContextMenuPrimitive.Item>
          {lead && <WorktreeMenu term={lead.pane} portalContainer={portalContainer} canStart={canStart} onOpenFolder={onNewSessionIn} onNewWorktree={onNewWorktree} />}
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={onToggle}>{open ? 'Collapse' : 'Expand'}</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { void navigator.clipboard.writeText(group.key) }}>Copy path</ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.Content>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Root>
  )
}

function FolderPopover({ group, anchor }: { group: SessionGroup; anchor: DOMRect }) {
  return (
    <div role="tooltip" className="mx-rail-popover" style={{ top: anchor.top + anchor.height * HALF, left: anchor.right + POPOVER_GAP_PX }}>
      <div className="mx-pop-head">
        <span className="mx-folder-tile" style={folderStyle(group.key, group.colorId)}><Icon name="folder" /></span>
        <strong>{group.name}</strong>
        {group.branch && <span className="mx-group-branch">{group.branch}</span>}
      </div>
      <div className="mx-pop-path">{group.path}</div>
      <ul className="mx-pop-sessions">
        {group.sessions.map((session) => (
          <li key={session.id}>
            <span className="mx-pop-title">{session.title}</span>
            <span className="mx-pop-status">{session.process} · {STATUS_WORD[session.status]}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}

function Rail(props: GroupProps) {
  const { groups, focusedId, tracks, primary, online, sessionCount, isOpen, onToggleGroup } = props
  return (
    <div className="mx-rail">
      <div className="mx-rail-top">
        {props.canExpand && <button type="button" className="mx-icon-button mx-rail-button" aria-label="Expand sidebar" title={`Expand sidebar (${primary}B)`} onClick={props.onToggleRail}><Icon name="expand" /></button>}
        <button type="button" className="mx-icon-button mx-rail-button mx-rail-new" aria-label="New session" title={`New session (${primary}T)`} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" /></button>
      </div>
      <div className="mx-rail-divider" />
      <div className="mx-rail-list mx-no-scrollbar">
        {groups.map((group) => {
          const open = isOpen(group)
          const label = `${group.name} · ${String(group.sessions.length)} session${group.sessions.length === 1 ? '' : 's'} · click to ${open ? 'collapse' : 'expand'}`
          return (
            <div key={group.key} className="mx-rail-group" style={folderStyle(group.key, group.colorId)}>
              <ProjectMenu group={group} open={open} canStart={props.canStart} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} onNewSessionIn={props.onNewSessionIn} onNewWorktree={() => { props.onNewWorktree(group.key) }}>
                <RailFolderButton group={group} open={open} online={online} label={label} portalContainer={props.portalContainer} onToggle={() => { onToggleGroup(group.key) }} />
              </ProjectMenu>
              {open && (
                <div className="mx-rail-rows">
                  {group.sessions.map((session) => (
                    <RailSessionButton
                      key={session.id}
                      session={session}
                      track={tracks.get(session.id)}
                      now={props.now}
                      selected={session.id === focusedId}
                      tabNumber={props.tabOf.get(session.id)}
                      online={online}
                      portalContainer={props.portalContainer}
                      onShow={() => { props.onShowSession(session.id) }}
                      onRename={() => { props.onRenameSession(session.id) }}
                      onEnd={() => { props.onEndSession(session.id) }}
                    />
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>
      <div className="mx-rail-bottom">
        <span className={`mx-daemon-dot${online ? '' : ' is-offline'}`} role="img" aria-label={online ? `maittyd · ${String(sessionCount)} sessions` : 'Reconnecting to maittyd…'} title={online ? `maittyd · ${String(sessionCount)} sessions` : 'Reconnecting to maittyd…'} />
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Settings" title={`Settings (${primary},)`} onClick={props.onOpenSettings}><Icon name="gear" /></button>
      </div>
    </div>
  )
}
