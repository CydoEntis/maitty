import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { machineTitle, UPDATE_WORDS, type MachineHead, type UpdateAction } from '../model/machines'
import { STATUS_WORD, type SessionView } from '../model/session'
import { inProject, isShell, machineGroups, projectChoices, projectGroups, projectSections, rowTitle, SIDEBAR_GROUPINGS, statusSections, type MachineGroup, type SidebarGrouping, type SidebarOrder } from '../model/sidebar'
import type { SubagentView } from '../model/subagents'
import { useTipWithKeys, useShortcutText } from '@/features/shortcuts/platform'
import { SidebarResizer } from './SidebarResizer'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { Collapse } from './Collapse'
import { useReducedMotion } from './motion'
import { useFlip } from './useFlip'
import { useStatusFlash } from './useStatusFlash'
import { MenuPopup, type MenuEntry } from './MenuPopup'
import { SessionRow } from './SessionRow'
import { RowHoverCard } from './RowHoverCard'
import { HOVER_CARD_DELAY_MS, type Box } from '../model/hoverCard'
import { SubagentList, SubagentPeek } from './SubagentRows'

/** A subagent whose own chat a pane shows. */
export type OpenSubagent = { sessionId: string; subagentId: string }

export type SidebarProps = {
  /** Every session of every machine. Each machine lists its own, in `order`. */
  sessions: readonly SessionView[]
  /** The machine groups: this computer first, then the others. */
  machines: readonly MachineHead[]
  order: SidebarOrder
  /** What the headings are, and how the user picks another. */
  grouping: SidebarGrouping
  onGrouping: (grouping: SidebarGrouping) => void
  /** Opens the search over every session, project and action. */
  onSearch: () => void
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
  onNewAgentChat: (machine: number) => void
  onNewShell: (machine: number) => void
  onAddMachine: () => void
  onMachineSettings: () => void
  /** Asks for a name of the user's own for another machine (by its real name). */
  onRenameMachine: (name: string) => void
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
  onCopyPath: (id: string) => void
  onCopyText: (text: string, what: string) => void
  onShowChanges: (id: string) => void
  onMarkDone: (id: string) => void
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

/** The gap between a button and the menu that opens under it. */
const MENU_GAP_PX = 4

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

/** Which headings are folded and which machines are grouped by project. */
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
  const list = useRef<HTMLDivElement>(null)
  const [menu, setMenu] = useState<OpenMenu | null>(null)
  // The card beside a row the pointer rests on. A click, a menu or leaving the row puts it away.
  const [hover, setHover] = useState<{ id: string; row: Box } | null>(null)
  const hoverTimer = useRef<number | null>(null)
  const endHover = () => {
    if (hoverTimer.current !== null) window.clearTimeout(hoverTimer.current)
    hoverTimer.current = null
    setHover(null)
  }
  const beginHover = (id: string, element: HTMLElement) => {
    endHover()
    hoverTimer.current = window.setTimeout(() => {
      const box = element.getBoundingClientRect()
      setHover({ id, row: { left: box.left, right: box.right, top: box.top } })
    }, HOVER_CARD_DELAY_MS)
  }
  useEffect(() => () => { if (hoverTimer.current !== null) window.clearTimeout(hoverTimer.current) }, [])
  const hovered = hover === null || menu !== null ? undefined : props.sessions.find((session) => session.id === hover.id)
  const [pickedProject, setPickedProject] = useState<string | null>(null)
  const views = useMachineViews()
  const reduced = useReducedMotion()
  useFlip(list, reduced)
  useStatusFlash(list, props.sessions, reduced)
  const projects = projectChoices(props.sessions)
  // A project whose last session closed is no longer a choice, so the list goes back to all of them.
  const project = projects.some((choice) => choice.name === pickedProject) ? pickedProject : null
  const sessions = inProject(props.sessions, project)
  const draft = props.draft ?? null
  const groups = machineGroups(sessions, props.machines, draft !== null, props.order)
  const groupingLabel = SIDEBAR_GROUPINGS.find(([grouping]) => grouping === props.grouping)?.[1] ?? ''
  const localMachine = props.machines[0]?.machine ?? 0
  const picked = (on: boolean, otherwise = '') => (on ? '✓' : otherwise)
  const projectMenu = (): MenuEntry[] => [
    { label: 'All projects', hint: picked(project === null, String(props.sessions.length)), run: () => { setPickedProject(null) } },
    ...(projects.length > 0 ? ['separator' as const] : []),
    ...projects.map((choice) => ({ label: choice.name, hint: picked(project === choice.name, String(choice.count)), run: () => { setPickedProject(choice.name) } })),
  ]
  const groupingMenu = (): MenuEntry[] => SIDEBAR_GROUPINGS.map(([grouping, label]) => ({ label, hint: picked(props.grouping === grouping), run: () => { props.onGrouping(grouping) } }))
  const startMenu = (): MenuEntry[] => [
    { label: 'New agent chat', hint: shortcutText('new-agent-chat'), disabled: !props.canStart, run: () => { props.onNewAgentChat(localMachine) } },
    { label: 'New shell', hint: shortcutText('new-terminal'), disabled: !props.canStart, run: () => { props.onNewShell(localMachine) } },
    'separator',
    { label: 'Add a remote machine…', run: props.onAddMachine },
  ]
  /** A menu under the button that opens it, not at the pointer, so it opens the same from the keyboard. */
  const openUnder = (event: MouseEvent<HTMLElement>, title: string, items: MenuEntry[]) => {
    const box = event.currentTarget.getBoundingClientRect()
    setMenu({ title, at: { x: box.left, y: box.bottom + MENU_GAP_PX }, items })
  }
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
      ...(agent ? [{ label: 'Show changes', run: () => { props.onShowChanges(session.id) } }] : []),
      'separator',
      { label: 'Rename…', hint: 'F2', run: () => { props.onRenameSession(session.id) } },
      { label: 'Copy folder path', run: () => { props.onCopyPath(session.id) } },
      ...(session.machineName === null ? [] : [{ label: 'Get a file…', disabled: !props.online, run: () => { props.onGetFile(session.id) } }]),
      // A finished agent waits under Finished until you look; this files it under Idle without opening it.
      ...(session.status === 'done' ? [{ label: 'Mark as done', disabled: !props.online, run: () => { props.onMarkDone(session.id) } }] : []),
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
      ...(name === null ? [] : [{ label: 'Rename…', run: () => { props.onRenameMachine(name) } }]),
      { label: 'Machine settings…', run: props.onMachineSettings },
    ]
  }

  const renderRow = (session: SessionView, showProject: boolean, showMachine = false) => {
    const selected = session.id === props.focusedId
    const subagents = session.subagents
    return (
      <div key={session.id} className="mx-row-wrap" role="listitem" data-flip={session.id} onMouseEnter={(event) => { beginHover(session.id, event.currentTarget) }} onMouseLeave={endHover} onMouseDown={endHover}>
        <SessionRow
          session={session}
          selected={selected}
          showProject={showProject}
          machine={showMachine ? session.machineName : null}
          renaming={props.renamingId === session.id}
          onShow={() => { props.onShowSession(session.id) }}
          onRename={() => { props.onRenameSession(session.id) }}
          onSetName={(name) => { props.onSetName(session.id, name) }}
          onCancelRename={() => { props.onRenameSession(null) }}
          onContextMenu={(event) => { openMenu(event, rowTitle(session), sessionMenu(session)) }}
        />
        {/* A sibling of the row, not inside it (a button cannot hold a button): it shows on hover and focus. */}
        <button type="button" className="mx-row-close" aria-label={`${isShell(session) ? 'Close' : 'Stop'} ${rowTitle(session)}`} title={isShell(session) ? 'Close shell' : 'Stop session'} disabled={!props.online} onClick={() => { props.onEndSession(session.id) }}><Icon name="close" size={12} /></button>
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
      <div className="mx-sidebar-top">
        <div className="mx-top-line">
          <button type="button" className="mx-top-button is-new" title={tip('New chat', 'new-agent-chat')} disabled={!props.canStart} onClick={() => { props.onNewAgentChat(localMachine) }}>
            <Icon name="plus" size={14} /><span className="mx-top-label">New chat</span>
          </button>
          <button type="button" className="mx-head-button" aria-label="More to start" title="New shell, or add a remote machine" aria-haspopup="menu" onClick={(event) => { openUnder(event, 'Start', startMenu()) }}><Icon name="chevronDown" size={13} /></button>
          <button type="button" className="mx-head-button" aria-label="Collapse sidebar" title={tip('Collapse sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        </div>
        <button type="button" className="mx-top-button" title={tip('Search', 'palette')} onClick={props.onSearch}>
          <Icon name="search" size={14} /><span className="mx-top-label">Search</span><span className="mx-top-keys mono" aria-hidden="true">{shortcutText('palette')}</span>
        </button>
      </div>
      <div className="mx-sidebar-filters">
        <button type="button" className="mx-filter-button is-project" aria-label={`Show sessions from: ${project ?? 'All projects'}`} aria-haspopup="menu" onClick={(event) => { openUnder(event, 'Show sessions from', projectMenu()) }}>
          <Icon name="folder" size={13} /><span className="mx-filter-label">{project ?? 'All projects'}</span><Icon name="chevronDown" size={10} />
        </button>
        <button type="button" className="mx-filter-button" aria-label={`Group by: ${groupingLabel}`} title="Group by" aria-haspopup="menu" onClick={(event) => { openUnder(event, 'Group by', groupingMenu()) }}>
          <Icon name="layout" size={13} />{groupingLabel}<Icon name="chevronDown" size={10} />
        </button>
      </div>
      <div ref={list} className="mx-sidebar-list" onScroll={endHover}>
        {props.grouping !== 'machine' && (
          <section aria-label="Agents" className="mx-sections">
            {draft !== null && <DraftRow />}
            {props.grouping === 'status' && statusSections(sessions).map((section) => (
              <ListSection key={section.key} id={`status:${section.key}`} tone={section.key} name={section.name} hint={section.hint} count={section.sessions.length} label={section.name} open={views.isOpen(`status:${section.key}`)} onToggle={() => { views.toggle(`status:${section.key}`) }}>
                {section.sessions.map((session) => renderRow(session, true, true))}
              </ListSection>
            ))}
            {props.grouping === 'project' && projectSections(sessions, props.machines).map((section) => (
              <ListSection key={section.key} id={section.key} name={section.name} color={section.color} machine={section.head.name === null ? null : section.head} needs={section.needs} count={section.sessions.length} label={section.head.name === null ? section.name : `${section.name} on ${machineTitle(section.head)}`} open={views.isOpen(section.key)} onToggle={() => { views.toggle(section.key) }}>
                {section.sessions.map((session) => renderRow(session, false))}
              </ListSection>
            ))}
            {props.sessions.length === 0 && draft === null && <div className="mx-empty-note">No agents running. New chat starts one.</div>}
            <MachineNotes heads={props.machines} progress={props.updateProgress} onUpdate={props.onUpdateMachine} />
          </section>
        )}
        {props.grouping === 'machine' && (
        <section aria-label="Agents" className="mx-machines">
          {groups.map((group) => (
            <MachineSection
              key={group.head.key}
              group={group}
              open={views.isOpen(group.head.key) || draft?.machine === group.head.machine}
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
          {props.sessions.length === 0 && draft === null && <div className="mx-empty-note">No agents running. New chat starts one.</div>}
        </section>
        )}
      </div>
      <div className="mx-sidebar-foot">
        {props.online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">{props.serviceText ?? 'Reconnecting to maittyd…'}</span></>}
        <button type="button" className="mx-head-button mx-settings-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" size={15} /></button>
      </div>
      {hovered && hover && <RowHoverCard session={hovered} row={hover.row} portalContainer={props.portalContainer} />}
      {menu && <MenuPopup key={`${String(menu.at.x)},${String(menu.at.y)}`} title={menu.title} at={menu.at} items={menu.items} portalContainer={props.portalContainer} onClose={() => { setMenu(null) }} />}
    </div>
  )
}

type ListSectionProps = {
  id: string
  name: string
  /** What the list is called for a screen reader: the heading, with its machine when it has one. */
  label: string
  count: number
  open: boolean
  onToggle: () => void
  children: ReactNode
  /** A status section: its colour, and the few words that say what the status means. */
  tone?: string
  hint?: string
  /** A project section: its colour square, the other machine it is on, and how many of its sessions need you. */
  color?: string
  machine?: MachineHead | null
  needs?: number
}

/** One heading of the status or project list, with the rows under it; the heading folds them away. */
function ListSection({ id, name, label, count, open, onToggle, children, tone, hint, color, machine = null, needs = 0 }: ListSectionProps) {
  return (
    <div className="mx-section" data-section={id}>
      <button type="button" className={`mx-section-head${tone === undefined ? '' : ` is-${tone}`}`} data-flip={`head:${id}`} aria-expanded={open} onClick={onToggle}>
        {color !== undefined && <span className="mx-project-square" style={{ background: color }} aria-hidden="true" />}
        <span className="mx-section-name">{name}</span>
        {machine !== null && <span className="mx-section-machine">· {machineTitle(machine)}</span>}
        {machine !== null && <span className={`mx-machine-dot${machine.down ? ' is-offline' : ''}`} role="img" aria-label={machine.down ? `${machineTitle(machine)} is ${machine.chip?.word ?? 'not reachable'}` : `${machineTitle(machine)} is connected`} />}
        <span className="mx-section-count mono">{count}</span>
        {needs > 0 && <span className="mx-project-needs" aria-label={`${String(needs)} need you`}>{needs}</span>}
        {hint !== undefined && <span className="mx-section-hint">{hint}</span>}
        <span className="mx-spacer" />
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
      </button>
      <Collapse open={open}>
        <div className="mx-rows" role="list" aria-label={label}>{children}</div>
      </Collapse>
    </div>
  )
}

/**
 * Machines that need the app, when the list is not grouped by machine: there is no machine heading
 * to carry the Update button then, and a machine that cannot run anything must still say so.
 */
function MachineNotes({ heads, progress, onUpdate }: { heads: readonly MachineHead[]; progress: Readonly<Record<string, number>> | undefined; onUpdate: (name: string, action: UpdateAction) => void }) {
  const waiting = heads.flatMap((head) => (head.name !== null && (head.updateAction !== null || progress?.[head.name] !== undefined) ? [{ head, name: head.name }] : []))
  if (waiting.length === 0) return null
  return (
    <div className="mx-machine-notes" role="group" aria-label="Machines that need Maitty">
      {waiting.map(({ head, name }) => {
        const done = progress?.[name]
        return (
          <div key={head.key} className="mx-machine-note">
            <span className="mx-machine-icon" aria-hidden="true"><Icon name="server" size={13} /></span>
            <span className="mx-machine-name">{machineTitle(head)}</span>
            {done !== undefined
              ? <span className="mx-machine-pct mono">{String(done)}%</span>
              : head.updateAction !== null && <UpdateButton name={name} action={head.updateAction} onUpdate={onUpdate} />}
          </div>
        )
      })}
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
      <div className="mx-machine-head" data-flip={`machine:${head.key}`}>
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
        <button type="button" className="mx-machine-add" aria-label={`New agent chat on ${title}`} title={head.down ? `Cannot start a chat on ${title}: ${head.why ?? head.chip?.word ?? 'not reachable'}` : `New agent chat on ${title} (right-click for more)`} disabled={!canStart || head.down} onClick={onAdd} onContextMenu={onAddMenu}><Icon name="plus" size={13} /></button>
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
 * placeholders with a softly pulsing icon square, last in its machine. The real row takes this place when the chat starts.
 */
function DraftRow() {
  return (
    <div className="mx-row-wrap mx-draft-wrap" role="status" aria-label="New chat, waiting for the first message" data-flip="draft">
      <div className="mx-row mx-draft">
        <span className="mx-row-icon"><span className="mx-draft-tile" /></span>
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
        <button type="button" className="mx-head-button is-rail" aria-label="Search" title={tip('Search', 'palette')} onClick={props.onSearch}><Icon name="search" size={15} /></button>
      </div>
      <div className="mx-rail-divider" />
      <div className="mx-rail-list mx-no-scrollbar" role="list" aria-label="Sessions">
        {machineGroups(props.sessions, props.machines, false, props.order).flatMap((group) => group.sessions).map((session) => (
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
