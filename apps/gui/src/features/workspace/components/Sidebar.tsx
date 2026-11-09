import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { machineTitle, UPDATE_WORDS, type MachineHead, type UpdateAction } from '../model/machines'
import { STATUS_WORD, type SessionView } from '../model/session'
import { isShell, machineGroups, projectGroups, rowTitle, type MachineGroup, type SidebarOrder } from '../model/sidebar'
import type { SubagentView } from '../model/subagents'
import { useTipWithKeys, useShortcutText } from '@/features/shortcuts/platform'
import { SidebarResizer } from './SidebarResizer'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { Collapse } from './Collapse'
import { useReducedMotion } from './motion'
import { useFlip } from './useFlip'
import { MenuPopup, type MenuEntry } from './MenuPopup'
import { SessionRow } from './SessionRow'
import { SubagentList, SubagentPeek } from './SubagentRows'

/** A subagent whose own chat a pane shows. */
export type OpenSubagent = { sessionId: string; subagentId: string }

export type SidebarProps = {
  /** Every session of every machine. Each machine lists its own, in `order`. */
  sessions: readonly SessionView[]
  /** The machine groups: this computer first, then the others. */
  machines: readonly MachineHead[]
  order: SidebarOrder
  focusedId: string | null
  openSubagent: OpenSubagent | null
  mode: 'docked' | 'rail'
  online: boolean
  canStart: boolean
  /** The session whose name is being typed in its row; the window starts it from a menu, F2 or a double-click. */
  renamingId: string | null
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  /** Collapses the docked sidebar to the rail, or expands the rail (over the window when it is narrow). */
  onToggleRail: () => void
  side: 'left' | 'right'
  /** The docked sidebar's width in px, and how the user changes it by dragging its edge. */
  width: number
  onResize: (width: number) => void
  onResizeEnd: (width: number) => void
  /** How many sessions the Inbox holds, and whether any of them needs you. */
  inboxCount: number
  inboxNeeds: boolean
  onOpenInbox: () => void
  onNewAgentChat: (machine: number) => void
  onNewShell: (machine: number) => void
  onAddMachine: () => void
  onMachineSettings: () => void
  /** Installs or updates the app on a machine that needs it; the window asks first. */
  onUpdateMachine: (name: string, action: UpdateAction) => void
  onShowSession: (id: string) => void
  onShowSubagent: (sessionId: string, subagentId: string) => void
  onRenameSession: (id: string | null) => void
  onSetName: (id: string, name: string) => void
  /** Shows a session beside the pane that has focus. */
  onSplitSession: (id: string, direction: 'right' | 'down') => void
  onSplitSubagent: (sessionId: string, subagentId: string) => void
  /** Opens a shell in an agent's folder, beside it. */
  onOpenTerminal: (id: string) => void
  onCopyPath: (id: string) => void
  onCopyText: (text: string, what: string) => void
  onShowChanges: (id: string) => void
  /** Asks for a file to pull from the machine of session `id`. */
  onGetFile: (id: string) => void
  onEndSession: (id: string) => void
  onOpenSettings: () => void
  /** The footer's left slot (the update button). The caller owns the updater. */
  updateSlot: ReactNode
  /** The update button in the rail's foot. */
  railUpdateSlot: ReactNode
  /** How far an install or update of each remote machine has got, by machine name, 0 to 100; absent when none runs. */
  updateProgress?: Readonly<Record<string, number>> | undefined
  /** What the footer says while the service is not connected; a red dot and this, in muted grey, never a banner. */
  serviceText?: string | undefined
  /** A chat that is being written but has no first message yet: listed on its machine so it is not lost. */
  draft?: DraftChat | null | undefined
}

export type DraftChat = { machine: number }

type OpenMenu = { title: string; at: { x: number; y: number }; items: MenuEntry[] }

export function Sidebar(props: SidebarProps) {
  const [resizing, setResizing] = useState(false)
  const className = `mx-sidebar mx-card${props.mode === 'rail' ? ' is-rail' : ''}${resizing ? ' is-resizing' : ''}`
  return (
    <nav aria-label="Sessions" className={className}>
      {props.mode === 'rail' ? <Rail {...props} /> : <Expanded {...props} />}
      {props.mode === 'docked' && <SidebarResizer side={props.side} width={props.width} onResize={props.onResize} onResizeEnd={props.onResizeEnd} onDragging={setResizing} />}
    </nav>
  )
}

/** The Inbox button with its count; amber while anything needs you. */
function InboxButton({ count, needs, onClick, rail = false }: { count: number; needs: boolean; onClick: () => void; rail?: boolean }) {
  const tip = useTipWithKeys()
  const label = `Inbox, ${String(count)}${needs ? ', something needs you' : ''}`
  return (
    <button type="button" className={`mx-head-button mx-inbox-button${rail ? ' is-rail' : ''}`} aria-label={label} title={tip('Inbox', 'inbox')} onClick={onClick}>
      <Icon name="inbox" size={15} />
      {count > 0 && <span className={`mx-inbox-count${needs ? ' is-needs' : ''}`}>{count}</span>}
    </button>
  )
}

/** `/` focuses the filter from anywhere outside a pane or another field. */
function useSlashFocus(input: React.RefObject<HTMLInputElement | null>) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== '/' || event.ctrlKey || event.metaKey || event.altKey) return
      const target = event.target
      if (target instanceof Element && target.closest('input, textarea, select, [contenteditable="true"], [data-pane], .xterm')) return
      event.preventDefault()
      input.current?.focus()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => { window.removeEventListener('keydown', onKeyDown) }
  }, [input])
}

/** Which machines are folded and which are grouped by project. */
function useMachineViews() {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set())
  const [byProject, setByProject] = useState<ReadonlySet<string>>(new Set())
  const flip = (set: ReadonlySet<string>, key: string) => { const next = new Set(set); if (!next.delete(key)) next.add(key); return next }
  return {
    isOpen: (key: string) => !folded.has(key),
    toggle: (key: string) => { setFolded((current) => flip(current, key)) },
    isByProject: (key: string) => byProject.has(key),
    toggleByProject: (key: string) => { setByProject((current) => flip(current, key)) },
  }
}

function Expanded(props: SidebarProps) {
  const tip = useTipWithKeys()
  const shortcutText = useShortcutText()
  const input = useRef<HTMLInputElement>(null)
  const list = useRef<HTMLDivElement>(null)
  const [query, setQuery] = useState('')
  const [menu, setMenu] = useState<OpenMenu | null>(null)
  const views = useMachineViews()
  useFlip(list, useReducedMotion())
  useSlashFocus(input)
  const filtering = query.trim() !== ''
  const draft = filtering ? null : props.draft ?? null
  const groups = machineGroups(props.sessions, props.machines, query, draft !== null, props.order)
  const openMenu = (event: MouseEvent, title: string, items: MenuEntry[]) => {
    event.preventDefault()
    event.stopPropagation()
    setMenu({ title, at: { x: event.clientX, y: event.clientY }, items })
  }

  // The heading's +: what can start on that machine, as a short list under the button.
  const addMenu = (head: MachineHead): MenuEntry[] => [
    { label: 'New agent chat', hint: shortcutText('new-agent-chat'), disabled: !props.canStart || head.down, run: () => { props.onNewAgentChat(head.machine) } },
    { label: 'New shell', hint: shortcutText('new-terminal'), disabled: !props.canStart || head.down, run: () => { props.onNewShell(head.machine) } },
    'separator',
    { label: 'Add a remote machine…', run: props.onAddMachine },
  ]

  const sessionMenu = (session: SessionView): MenuEntry[] => {
    const agent = !isShell(session)
    const other = session.id !== props.focusedId
    return [
      { label: 'Open', run: () => { props.onShowSession(session.id) } },
      ...(other ? [{ label: 'Open in split right', run: () => { props.onSplitSession(session.id, 'right') } }, { label: 'Open in split down', run: () => { props.onSplitSession(session.id, 'down') } }] : []),
      ...(agent ? [{ label: 'Open terminal here', run: () => { props.onOpenTerminal(session.id) }, disabled: !props.canStart }] : []),
      'separator',
      { label: 'Rename…', hint: 'F2', run: () => { props.onRenameSession(session.id) } },
      { label: 'Copy folder path', run: () => { props.onCopyPath(session.id) } },
      ...(session.machineName === null ? [] : [{ label: 'Get a file…', disabled: !props.online, run: () => { props.onGetFile(session.id) } }]),
      ...(agent ? [{ label: 'Show changes', run: () => { props.onShowChanges(session.id) } }] : []),
      'separator',
      // Stopping an agent asks a second click; a plain shell has nothing running to lose.
      { label: agent ? 'Stop session' : 'Close shell', danger: true, disabled: !props.online, run: () => { props.onEndSession(session.id) } },
    ]
  }

  const subagentMenu = (session: SessionView, subagent: SubagentView): MenuEntry[] => [
    { label: 'Open', run: () => { props.onShowSubagent(session.id, subagent.id) } },
    { label: 'Open in split right', run: () => { props.onSplitSubagent(session.id, subagent.id) } },
    'separator',
    { label: 'Copy summary', disabled: (subagent.result ?? subagent.latest ?? subagent.task) === null, run: () => { props.onCopyText(subagent.result ?? subagent.latest ?? subagent.task ?? '', 'Summary') } },
    { label: 'Stop subagent', danger: true, disabled: true, why: `${session.process} stops its own subagents; stop the session to stop them all` },
  ]

  const machineMenu = (head: MachineHead): MenuEntry[] => {
    const open = views.isOpen(head.key)
    const grouped = views.isByProject(head.key)
    const name = head.name
    const action = head.updateAction
    return [
      ...(name !== null && action !== null ? [{ label: `${UPDATE_WORDS[action].verb} Maitty`, run: () => { props.onUpdateMachine(name, action) } }, 'separator' as const] : []),
      { label: 'New agent chat here', hint: shortcutText('new-agent-chat'), disabled: !props.canStart || head.down, run: () => { props.onNewAgentChat(head.machine) } },
      { label: 'New shell here', disabled: !props.canStart || head.down, run: () => { props.onNewShell(head.machine) } },
      'separator',
      { label: open ? 'Collapse' : 'Expand', run: () => { views.toggle(head.key) } },
      { label: grouped ? 'Show as one list' : 'Group by project', run: () => { views.toggleByProject(head.key) } },
      'separator',
      { label: 'Machine settings…', run: props.onMachineSettings },
    ]
  }

  const renderRow = (session: SessionView, showProject: boolean) => {
    const selected = session.id === props.focusedId
    const subagents = session.subagents
    return (
      <div key={session.id} className="mx-row-wrap" role="listitem" data-flip={session.id}>
        <SessionRow
          session={session}
          selected={selected}
          showProject={showProject}
          renaming={props.renamingId === session.id}
          onShow={() => { props.onShowSession(session.id) }}
          onRename={() => { props.onRenameSession(session.id) }}
          onSetName={(name) => { props.onSetName(session.id, name) }}
          onCancelRename={() => { props.onRenameSession(null) }}
          onContextMenu={(event) => { openMenu(event, rowTitle(session), sessionMenu(session)) }}
        />
        {subagents.length > 0 && !selected && <SubagentPeek process={session.process} subagents={subagents} />}
        {subagents.length > 0 && selected && (
          <SubagentList
            process={session.process}
            subagents={subagents}
            openId={props.openSubagent?.sessionId === session.id ? props.openSubagent.subagentId : null}
            onOpen={(id) => { props.onShowSubagent(session.id, id) }}
            onContextMenu={(subagent, event) => { openMenu(event, subagent.name, subagentMenu(session, subagent)) }}
          />
        )}
      </div>
    )
  }

  return (
    <div className="mx-sidebar-expanded">
      <div className="mx-sidebar-head">
        <button type="button" className="mx-head-button is-small" aria-label="Collapse sidebar" title={tip('Collapse sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        <span className="mx-spacer" />
        <button type="button" className="mx-head-button" aria-label="New agent chat" title={tip('New agent chat', 'new-agent-chat')} disabled={!props.canStart} onClick={() => { props.onNewAgentChat(props.machines[0]?.machine ?? 0) }}><Icon name="chatPlus" size={15} /></button>
        <button type="button" className="mx-head-button" aria-label="New shell" title={tip('New shell', 'new-terminal')} disabled={!props.canStart} onClick={() => { props.onNewShell(props.machines[0]?.machine ?? 0) }}><Icon name="terminal" size={15} /></button>
        <button type="button" className="mx-head-button" aria-label="Add a remote machine" title="Add a remote machine" onClick={props.onAddMachine}><Icon name="serverPlus" size={15} /></button>
        <InboxButton count={props.inboxCount} needs={props.inboxNeeds} onClick={props.onOpenInbox} />
      </div>
      <div className="mx-filter">
        <Icon name="search" size={13} />
        <input
          ref={input}
          type="text"
          value={query}
          placeholder="Filter sessions"
          aria-label="Filter sessions"
          spellCheck={false}
          onChange={(event) => { setQuery(event.target.value) }}
          onKeyDown={(event) => {
            if (event.key !== 'Escape') return
            // Esc empties the field and gives the keyboard back; it must not reach a pane behind it.
            event.preventDefault()
            event.stopPropagation()
            setQuery('')
            event.currentTarget.blur()
          }}
        />
        {query === '' ? <kbd className="mx-filter-hint" aria-hidden="true">/</kbd> : <button type="button" className="mx-filter-clear" aria-label="Clear filter" onClick={() => { setQuery(''); input.current?.focus() }}><Icon name="close" size={11} /></button>}
      </div>
      <div ref={list} className="mx-sidebar-list">
        <section aria-label="Agents" className="mx-machines">
          {groups.map((group) => (
            <MachineSection
              key={group.head.key}
              group={group}
              open={filtering || views.isOpen(group.head.key) || draft?.machine === group.head.machine}
              draft={draft?.machine === group.head.machine}
              progress={group.head.name === null ? undefined : props.updateProgress?.[group.head.name]}
              byProject={views.isByProject(group.head.key)}
              canStart={props.canStart}
              onToggle={() => { views.toggle(group.head.key) }}
              onToggleByProject={() => { views.toggleByProject(group.head.key) }}
              onAdd={() => { props.onNewAgentChat(group.head.machine) }}
              onAddMenu={(event) => { openMenu(event, `New on ${machineTitle(group.head)}`, addMenu(group.head)) }}
              onUpdate={props.onUpdateMachine}
              onContextMenu={(event) => { openMenu(event, machineTitle(group.head), machineMenu(group.head)) }}
              renderRow={renderRow}
            />
          ))}
          {filtering && groups.length === 0 && <div className="mx-empty-note">No agent matches “{query.trim()}”.</div>}
          {!filtering && props.sessions.length === 0 && draft === null && <div className="mx-empty-note">No agents running. Use the buttons above to start one.</div>}
        </section>
      </div>
      <div className="mx-sidebar-foot">
        {props.online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">{props.serviceText ?? 'Reconnecting to maittyd…'}</span></>}
        <button type="button" className="mx-head-button mx-settings-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" size={15} /></button>
      </div>
      {menu && <MenuPopup title={menu.title} at={menu.at} items={menu.items} portalContainer={props.portalContainer} onClose={() => { setMenu(null) }} />}
    </div>
  )
}

type SectionProps = {
  /** A draft chat is listed first in this machine. */
  draft: boolean
  /** How far this machine's update has got, while one runs. */
  progress: number | undefined
  group: MachineGroup
  open: boolean
  byProject: boolean
  canStart: boolean
  onToggle: () => void
  onToggleByProject: () => void
  /** Plain click on +: an agent chat on this machine. */
  onAdd: () => void
  /** Right-click on +: the full menu of what can start here. */
  onAddMenu: (event: MouseEvent) => void
  onUpdate: (name: string, action: UpdateAction) => void
  onContextMenu: (event: MouseEvent) => void
  renderRow: (session: SessionView, showProject: boolean) => ReactNode
}

/** One machine: its heading (name, count, group-by-project, its line's dot, +) and its sessions. */
function MachineSection({ draft, progress, group, open, byProject, canStart, onToggle, onToggleByProject, onAdd, onAddMenu, onUpdate, onContextMenu, renderRow }: SectionProps) {
  const { head, sessions } = group
  const title = machineTitle(head)
  const projects = byProject ? projectGroups(sessions) : []
  return (
    <div className={`mx-machine${head.down ? ' is-away' : ''}`}>
      <div className="mx-machine-head">
        <button type="button" className="mx-machine-toggle" aria-expanded={open} onClick={onToggle} onContextMenu={onContextMenu}>
          <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
          <span className="mx-machine-icon" aria-hidden="true"><Icon name={head.name === null ? 'laptop' : 'server'} size={13} /></span>
          <span className="mx-machine-name">{title}</span>
          <span className="mx-machine-count">{sessions.length}</span>
        </button>
        {progress !== undefined
          ? <span className="mx-machine-pct mono">{String(progress)}%</span>
          : head.name !== null && head.updateAction !== null && <UpdateButton name={head.name} action={head.updateAction} onUpdate={onUpdate} />}
        <button type="button" className={`mx-machine-group${byProject ? ' is-on' : ''}`} aria-pressed={byProject} aria-label={byProject ? 'Show one list, most urgent first' : 'Group by project'} title={byProject ? 'Show one list, most urgent first' : 'Group by project'} onClick={onToggleByProject}><Icon name="folder" /></button>
        <span className={`mx-machine-dot${head.updateAction !== null ? ' is-warn' : head.down ? ' is-offline' : ''}`} role="img" aria-label={head.down ? `${title} is ${head.chip?.word ?? 'not reachable'}` : `${title} is connected`} title={head.why ?? (head.down ? head.chip?.word ?? 'not reachable' : `${title} is connected`)} />
        <button type="button" className="mx-machine-add" aria-label={`New agent chat on ${title}`} title={`New agent chat on ${title} (right-click for more)`} disabled={!canStart || head.down} onClick={onAdd} onContextMenu={onAddMenu}><Icon name="plus" size={13} /></button>
      </div>
      {progress !== undefined && head.name !== null && (
        <span className="mx-machine-progress" role="progressbar" aria-label={`Updating ${head.name}`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress}><span style={{ width: `${String(progress)}%` }} /></span>
      )}
      <Collapse open={open}>
        {byProject ? (
          <div className="mx-projects">
            {projects.map((project) => <ProjectSection key={project.name} name={project.name} color={project.color} needs={project.needs} sessions={project.sessions} renderRow={renderRow} />)}
          </div>
        ) : (
          <div className="mx-rows" role="list" aria-label={`Sessions on ${title}`}>{sessions.map((session) => renderRow(session, true))}</div>
        )}
        {/* Last, where the new chat will be (sessions keep creation order), so it fills in without anything moving. */}
        {draft && <DraftRow />}
      </Collapse>
    </div>
  )
}

/**
 * The chat that has not been sent yet: a copy of a session row (its icon, its name line and its project line) drawn as
 * placeholders with three dots bouncing in the icon, last in its machine. The real row takes this place when the chat starts.
 */
function DraftRow() {
  return (
    <div className="mx-row-wrap mx-draft-wrap" role="status" aria-label="New chat, waiting for the first message" data-flip="draft">
      <div className="mx-row mx-draft">
        <span className="mx-row-icon"><span className="mx-draft-tile"><span className="mx-draft-dots" aria-hidden="true"><i /><i /><i /></span></span></span>
        <span className="mx-row-text" aria-hidden="true">
          <span className="mx-draft-line is-name" />
          <span className="mx-draft-line is-sub" />
        </span>
      </div>
    </div>
  )
}

/** What a machine without the app, or with another version of it, needs: an amber text button in its header, no row of its own. */
function UpdateButton({ name, action, onUpdate }: { name: string; action: UpdateAction; onUpdate: (name: string, action: UpdateAction) => void }) {
  const words = UPDATE_WORDS[action]
  return (
    <button type="button" className="mx-machine-update" aria-label={`${words.verb} Maitty on ${name}`} title={words.need(name)} onClick={() => { onUpdate(name, action) }}>
      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 19V5" /><path d="m5 12 7-7 7 7" /></svg>
      {words.verb}
    </button>
  )
}

function ProjectSection({ name, color, needs, sessions, renderRow }: { name: string; color: string; needs: number; sessions: readonly SessionView[]; renderRow: (session: SessionView, showProject: boolean) => ReactNode }) {
  const [open, setOpen] = useState(true)
  return (
    <div className="mx-project">
      <button type="button" className="mx-project-head" aria-expanded={open} onClick={() => { setOpen(!open) }}>
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
        <span className="mx-project-square" style={{ background: color }} aria-hidden="true" />
        <span className="mx-project-name">{name}</span>
        {needs > 0 && <span className="mx-project-needs" aria-label={`${String(needs)} need you`}>{needs}</span>}
        <span className="mx-project-count">{sessions.length}</span>
      </button>
      <Collapse open={open}>
        <div className="mx-rows" role="list" aria-label={name}>{sessions.map((session) => renderRow(session, false))}</div>
      </Collapse>
    </div>
  )
}

/** The 60px rail of a narrow window: the starters, then one tile per session with its status badge. */
function Rail(props: SidebarProps) {
  const tip = useTipWithKeys()
  const label = props.online ? `maittyd · ${String(props.sessions.length)} sessions` : 'Reconnecting to maittyd…'
  return (
    <div className="mx-rail">
      <div className="mx-rail-top">
        <button type="button" className="mx-head-button is-rail" aria-label="Expand sidebar" title={tip('Expand sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="expand" /></button>
        <button type="button" className="mx-head-button is-rail" aria-label="New agent chat" title={tip('New agent chat', 'new-agent-chat')} disabled={!props.canStart} onClick={() => { props.onNewAgentChat(props.machines[0]?.machine ?? 0) }}><Icon name="chatPlus" size={15} /></button>
        <button type="button" className="mx-head-button is-rail" aria-label="New shell" title={tip('New shell', 'new-terminal')} disabled={!props.canStart} onClick={() => { props.onNewShell(props.machines[0]?.machine ?? 0) }}><Icon name="terminal" size={15} /></button>
        <InboxButton rail count={props.inboxCount} needs={props.inboxNeeds} onClick={props.onOpenInbox} />
      </div>
      <div className="mx-rail-divider" />
      <div className="mx-rail-list mx-no-scrollbar" role="list" aria-label="Sessions">
        {machineGroups(props.sessions, props.machines, '', false, props.order).flatMap((group) => group.sessions).map((session) => (
          <button
            key={session.id}
            type="button"
            role="listitem"
            className={`mx-rail-session${session.id === props.focusedId ? ' is-selected' : ''}`}
            aria-label={`${rowTitle(session)}, ${STATUS_WORD[session.status]}`}
            aria-current={session.id === props.focusedId ? 'page' : undefined}
            title={rowTitle(session)}
            onClick={() => { props.onShowSession(session.id) }}
          >
            <span className="mx-row-icon"><AppIcon process={session.process} size={24} dimmed={session.status === 'unknown'} /><StatusBadge status={session.status} /></span>
          </button>
        ))}
      </div>
      <div className="mx-rail-bottom">
        {props.railUpdateSlot}
        <span className={`mx-daemon-dot${props.online ? '' : ' is-offline'}`} role="img" aria-label={label} title={label} />
        <button type="button" className="mx-head-button is-rail" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" /></button>
      </div>
    </div>
  )
}
