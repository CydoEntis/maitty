import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { machineTitle, UPDATE_WORDS, type MachineHead, type UpdateAction } from '../model/machines'
import { STATUS_WORD, type SessionView } from '../model/session'
import { isShell, machineGroups, projectGroups, rowTitle, type MachineGroup } from '../model/sidebar'
import type { SubagentView } from '../model/subagents'
import { useTipWithKeys, useShortcutText } from '@/features/shortcuts/platform'
import { SidebarResizer } from './SidebarResizer'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { Collapse } from './Collapse'
import { MenuPopup, type MenuEntry } from './MenuPopup'
import { SessionRow } from './SessionRow'
import { SubagentList, SubagentPeek } from './SubagentRows'

/** A subagent whose own chat a pane shows. */
export type OpenSubagent = { sessionId: string; subagentId: string }

export type SidebarProps = {
  /** Every session of every machine. Each machine lists its own, most urgent first. */
  sessions: readonly SessionView[]
  /** The machine groups: this computer first, then the others. */
  machines: readonly MachineHead[]
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
}

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
  const [query, setQuery] = useState('')
  const [menu, setMenu] = useState<OpenMenu | null>(null)
  const views = useMachineViews()
  useSlashFocus(input)
  const groups = machineGroups(props.sessions, props.machines, query)
  const filtering = query.trim() !== ''
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
      { label: agent ? 'Stop session' : 'Close shell', danger: true, ...(agent ? { armedLabel: `Stop ${session.process}? Click again` } : {}), disabled: !props.online, run: () => { props.onEndSession(session.id) } },
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
      <div key={session.id} className="mx-row-wrap" role="listitem">
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
      <div className="mx-sidebar-list">
        <section aria-label="Agents" className="mx-machines">
          {groups.map((group) => (
            <MachineSection
              key={group.head.key}
              group={group}
              open={filtering || views.isOpen(group.head.key)}
              byProject={views.isByProject(group.head.key)}
              canStart={props.canStart}
              onToggle={() => { views.toggle(group.head.key) }}
              onToggleByProject={() => { views.toggleByProject(group.head.key) }}
              onAdd={(event) => { openMenu(event, `New on ${machineTitle(group.head)}`, addMenu(group.head)) }}
              onUpdate={props.onUpdateMachine}
              onContextMenu={(event) => { openMenu(event, machineTitle(group.head), machineMenu(group.head)) }}
              renderRow={renderRow}
            />
          ))}
          {filtering && groups.length === 0 && <div className="mx-empty-note">No agent matches “{query.trim()}”.</div>}
          {!filtering && props.sessions.length === 0 && <div className="mx-empty-note">No agents running. Use the buttons above to start one.</div>}
        </section>
      </div>
      <div className="mx-sidebar-foot">
        {props.online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">Reconnecting to maittyd…</span></>}
        <button type="button" className="mx-head-button mx-settings-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" size={15} /></button>
      </div>
      {menu && <MenuPopup title={menu.title} at={menu.at} items={menu.items} portalContainer={props.portalContainer} onClose={() => { setMenu(null) }} />}
    </div>
  )
}

type SectionProps = {
  group: MachineGroup
  open: boolean
  byProject: boolean
  canStart: boolean
  onToggle: () => void
  onToggleByProject: () => void
  onAdd: (event: MouseEvent) => void
  onUpdate: (name: string, action: UpdateAction) => void
  onContextMenu: (event: MouseEvent) => void
  renderRow: (session: SessionView, showProject: boolean) => ReactNode
}

/** One machine: its heading (name, count, group-by-project, its line's dot, +) and its sessions. */
function MachineSection({ group, open, byProject, canStart, onToggle, onToggleByProject, onAdd, onUpdate, onContextMenu, renderRow }: SectionProps) {
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
        <button type="button" className={`mx-machine-group${byProject ? ' is-on' : ''}`} aria-pressed={byProject} aria-label={byProject ? 'Show one list, most urgent first' : 'Group by project'} title={byProject ? 'Show one list, most urgent first' : 'Group by project'} onClick={onToggleByProject}><Icon name="folder" /></button>
        <span className={`mx-machine-dot${head.down ? ' is-offline' : ''}`} role="img" aria-label={head.down ? `${title} is ${head.chip?.word ?? 'not reachable'}` : `${title} is connected`} title={head.why ?? (head.down ? head.chip?.word ?? 'not reachable' : `${title} is connected`)} />
        <button type="button" className="mx-machine-add" aria-haspopup="menu" aria-label={`New on ${title}`} title={`New on ${title}`} disabled={!canStart || head.down} onClick={onAdd}><Icon name="plus" size={13} /></button>
      </div>
      {head.name !== null && head.updateAction !== null && <UpdateNote name={head.name} action={head.updateAction} onUpdate={onUpdate} />}
      <Collapse open={open}>
        {byProject ? (
          <div className="mx-projects">
            {projects.map((project) => <ProjectSection key={project.name} name={project.name} color={project.color} needs={project.needs} sessions={project.sessions} renderRow={renderRow} />)}
          </div>
        ) : (
          <div className="mx-rows" role="list" aria-label={`Sessions on ${title}`}>{sessions.map((session) => renderRow(session, true))}</div>
        )}
      </Collapse>
    </div>
  )
}

/** What a machine without the app, or with another version of it, needs, and the button that does it. */
function UpdateNote({ name, action, onUpdate }: { name: string; action: UpdateAction; onUpdate: (name: string, action: UpdateAction) => void }) {
  const words = UPDATE_WORDS[action]
  return (
    <div className="mx-machine-note">
      <span className="mx-machine-state"><span aria-hidden="true">▲ </span>{words.state}</span>
      <button type="button" className="mx-machine-fix-button" aria-label={`${words.verb} Maitty on ${name}`} title={words.need(name)} onClick={() => { onUpdate(name, action) }}>{words.verb}</button>
    </div>
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
        {machineGroups(props.sessions, props.machines, '').flatMap((group) => group.sessions).map((session) => (
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
