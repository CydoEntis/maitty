import { useEffect, useRef, useState, type ReactNode } from 'react'
import type { Fleet } from '@/api/fleets'
import { aboard, emptyShipsNote, fleetKeyOf, fleetsOfMachine, isDaemonFleet, isFreeFleetName, machineTitle, needsCount, priorityGroups, type FleetColor, shipsOf, visibleFleets } from '../model/fleets'
import type { PaneView } from '../model/chatView'
import type { MachineHead } from '../model/machines'
import { matchesFilter, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import { SidebarResizer } from './SidebarResizer'
import { Icon } from './glyphs'
import { Collapse, SectionHead } from './Collapse'
import { FleetRow, FleetTile } from './FleetRow'
import { MAX_FLEET_NAME_LENGTH } from './fleetStyle'
import { InlineName } from './InlineName'
import { ShipRow } from './ShipRow'
import { RailPopoverLayer } from './RailPopovers'
import { fleetHoverId, hoverHandlers, useRailHover } from './railHover'
import { useTipWithKeys } from '@/features/shortcuts/platform'

export type SidebarProps = {
  /** Every ship of every machine, in creation order. The filter narrows what shows; it never reorders. */
  sessions: readonly SessionView[]
  /** Every fleet of every machine, in the daemon's order. */
  fleets: readonly Fleet[]
  /** The machine headings: this computer first, then the others. */
  machines: readonly MachineHead[]
  selectedFleet: string | null
  focusedId: string | null
  tracks: ReadonlyMap<string, SessionTrack>
  mode: 'docked' | 'rail'
  online: boolean
  canStart: boolean
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
  /** The header's three starters: a chat in the current fleet, a shell there, and the Add machine page. */
  onNewAgentChat: () => void
  onNewShell: () => void
  onAddMachine: () => void
  onSelectFleet: (key: string) => void
  /** Makes a fleet on a machine; the sidebar waits for it so a failed name stays in the field. */
  onAddFleet: (machine: number, name: string) => void
  onRenameFleet: (fleet: Fleet, name: string) => void
  onRecolorFleet: (fleet: Fleet, color: FleetColor) => void
  /** Moves a ship to a fleet (the drag target and the ship menu). The caller refuses a move across machines. */
  onMoveShip: (sessionId: string, fleet: Fleet) => void
  /** The view a ship shows, for the terminal or chat mark on its row. */
  viewOf: (session: SessionView) => PaneView
  onShowSession: (id: string) => void
  onEndSession: (id: string) => void
  onSetName: (id: string, name: string) => void
  /** Opens the rename dialog, for the rail, which has no room to rename a session in place. */
  onRenameSession: (id: string) => void
  onPinSession: (id: string, pinned: boolean) => void
  /** Shows a session beside the pane that has focus. */
  onSplitSession: (id: string, direction: 'right' | 'down') => void
  onCopyPath: (id: string) => void
  onOpenSettings: () => void
  /** The footer's left slot (version, update). The caller owns the updater. */
  updateSlot: ReactNode
  /** The update button in the rail's foot. */
  railUpdateSlot: ReactNode
}

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
    <button type="button" className={`mx-inbox-button${rail ? ' mx-icon-button mx-rail-button' : ''}${needs ? ' is-needs' : ''}`} aria-label={label} title={tip('Inbox', 'inbox')} onClick={onClick}>
      <Icon name="inbox" size={15} />
      {count > 0 && <span className={`mx-inbox-count${needs ? ' is-needs' : ''}`}>{count}</span>}
    </button>
  )
}

/** The three starters, in the order the design gives them. `rail` stacks them as the rail's buttons. */
function StartButtons({ props, rail = false }: { props: SidebarProps; rail?: boolean }) {
  const tip = useTipWithKeys()
  const cls = rail ? 'mx-icon-button mx-rail-button mx-rail-new' : 'mx-new-button'
  return (
    <>
      <button type="button" className={cls} aria-label="New agent chat" title={tip('New agent chat', 'new-agent-chat')} disabled={!props.canStart} onClick={props.onNewAgentChat}><Icon name="chatPlus" size={15} /></button>
      <button type="button" className={cls} aria-label="New shell" title={tip('New shell', 'new-terminal')} disabled={!props.canStart} onClick={props.onNewShell}><Icon name="terminal" size={15} /></button>
      <button type="button" className={cls} aria-label="Add a remote machine" title="Add a remote machine" onClick={props.onAddMachine}><Icon name="serverPlus" size={15} /></button>
    </>
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

/** Which sections are folded. A key is in the set while its section is closed. */
function useFolds() {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set())
  const toggle = (key: string) => { setFolded((current) => { const next = new Set(current); if (!next.delete(key)) next.add(key); return next }) }
  return { isOpen: (key: string) => !folded.has(key), toggle }
}

type ShipsView = 'fleets' | 'priority'

function Expanded(props: SidebarProps) {
  const { sessions, fleets, machines, online } = props
  const tip = useTipWithKeys()
  const hover = useRailHover()
  const input = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ShipsView>('fleets')
  const [adding, setAdding] = useState<number | null>(null)
  const folds = useFolds()
  useSlashFocus(input)
  const selected = fleets.find((fleet) => fleet.key === props.selectedFleet)
  const listed = visibleFleets(fleets, sessions, query)
  const fleetShips = selected ? aboard(sessions, fleets, selected.key) : []
  const matching = fleetShips.filter((ship) => matchesFilter(ship, query))
  const { agents, shells } = shipsOf(matching)
  const priority = priorityGroups(sessions.filter((ship) => matchesFilter(ship, query)))
  const priorityTotal = priority.reduce((sum, group) => sum + group.ships.length, 0)
  const otherMatch = listed.find((fleet) => fleet.key !== selected?.key)?.name ?? null
  const shipCount = view === 'fleets' ? matching.length : priorityTotal
  const fleetNameOf = (ship: SessionView) => fleets.find((fleet) => fleet.key === fleetKeyOf(ship.machine, ship.fleet, fleets))?.name

  // The other fleets of the ship's own machine; a ship cannot leave its machine, and `Main` is no daemon fleet.
  const moveTargetsOf = (ship: SessionView) => {
    const own = fleetKeyOf(ship.machine, ship.fleet, fleets)
    return fleets.filter((fleet) => fleet.machine === ship.machine && fleet.key !== own && isDaemonFleet(fleet))
  }

  const row = (session: SessionView, fleetName?: string) => (
    <ShipRow
      key={session.id}
      session={session}
      track={props.tracks.get(session.id)}
      selected={session.id === props.focusedId}
      online={online}
      view={props.viewOf(session)}
      {...(fleetName === undefined ? {} : { fleetName })}
      portalContainer={props.portalContainer}
      hover={hover}
      moveTargets={moveTargetsOf(session)}
      onMove={(fleet) => { props.onMoveShip(session.id, fleet) }}
      onShow={() => { props.onShowSession(session.id) }}
      onSetName={(name) => { props.onSetName(session.id, name) }}
      onPin={(pinned) => { props.onPinSession(session.id, pinned) }}
      onSplit={(direction) => { props.onSplitSession(session.id, direction) }}
      onCopyPath={() => { props.onCopyPath(session.id) }}
      onEnd={() => { props.onEndSession(session.id) }}
    />
  )

  return (
    <div className="mx-sidebar-expanded">
      <RailPopoverLayer hover={hover} fleets={fleets} machines={machines} sessions={sessions} tracks={props.tracks} portalContainer={props.portalContainer} />
      <div className="mx-sidebar-head">
        <button type="button" className="mx-icon-button mx-collapse" aria-label="Collapse sidebar" title={tip('Collapse sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        <span className="mx-spacer" />
        <StartButtons props={props} />
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
        <section aria-label="Fleets" className="mx-group">
          <SectionHead label="FLEETS" open={folds.isOpen('fleets')} onToggle={() => { folds.toggle('fleets') }} />
          <Collapse open={folds.isOpen('fleets')}>
            {machines.map((head) => {
              const own = fleetsOfMachine(listed, head.machine)
              if (query.trim() !== '' && own.length === 0) return null
              const key = `machine-${head.key}`
              const away = head.down
              return (
                <div key={head.key} className={`mx-machine${away ? ' is-away' : ''}`}>
                  <SectionHead
                    label={machineTitle(head)}
                    icon={<Icon name={head.name === null ? 'laptop' : 'server'} size={13} />}
                    open={folds.isOpen(key)}
                    onToggle={() => { folds.toggle(key) }}
                    className="is-machine"
                  >
                    <span className={`mx-machine-dot${away ? ' is-offline' : ''}`} role="img" aria-label={away ? `${machineTitle(head)} is ${head.chip?.word ?? 'away'}` : `${machineTitle(head)} is connected`} title={head.why ?? head.chip?.word ?? 'connected'} />
                    <button type="button" className="mx-icon-button mx-machine-add" aria-label={`Add a fleet on ${machineTitle(head)}`} title={`Add a fleet on ${machineTitle(head)}`} disabled={away || !props.canStart} onClick={() => { if (!folds.isOpen(key)) folds.toggle(key); setAdding(head.machine) }}><Icon name="plus" size={13} /></button>
                  </SectionHead>
                  <Collapse open={folds.isOpen(key)}>
                    <div className="mx-fleet-list">
                      {own.map((fleet) => (
                        <FleetRow
                          key={fleet.key}
                          fleet={fleet}
                          ships={aboard(sessions, fleets, fleet.key)}
                          selected={fleet.key === selected?.key}
                          dimmed={away}
                          portalContainer={props.portalContainer}
                          editable={isDaemonFleet(fleet)}
                          onSelect={() => { props.onSelectFleet(fleet.key) }}
                          onRename={(name) => { props.onRenameFleet(fleet, name) }}
                          onRecolor={(color) => { props.onRecolorFleet(fleet, color) }}
                          onDropShip={(sessionId) => { props.onMoveShip(sessionId, fleet) }}
                        />
                      ))}
                      {adding === head.machine && (
                        <div className="mx-fleet is-editing" role="group" aria-label={`New fleet on ${machineTitle(head)}`}>
                          <span className="mx-fleet-tile is-row is-new" aria-hidden="true"><Icon name="plus" size={12} /></span>
                          <InlineName
                            value=""
                            placeholder="Fleet name"
                            label={`Name for the new fleet on ${machineTitle(head)}`}
                            maxLength={MAX_FLEET_NAME_LENGTH}
                            onCommit={(name) => { setAdding(null); if (isFreeFleetName(fleets, head.machine, name)) props.onAddFleet(head.machine, name) }}
                            onCancel={() => { setAdding(null) }}
                          />
                        </div>
                      )}
                      {own.length === 0 && adding !== head.machine && <div className="mx-empty-note">No fleets here yet. Use + to add one.</div>}
                    </div>
                  </Collapse>
                </div>
              )
            })}
            {query.trim() !== '' && listed.length === 0 && <div className="mx-empty-note">No fleet matches “{query.trim()}”.</div>}
          </Collapse>
        </section>

        <section aria-label="Ships" className="mx-group">
          <SectionHead label="SHIPS" count={shipCount} open={folds.isOpen('ships')} onToggle={() => { folds.toggle('ships') }}>
            <span className="mx-view-switch" role="radiogroup" aria-label="Ships view">
              <button type="button" role="radio" aria-checked={view === 'fleets'} aria-label="Selected fleet" title="Ships of the selected fleet" className={view === 'fleets' ? 'is-on' : ''} onClick={() => { setView('fleets') }}><Icon name="fleets" /></button>
              <button type="button" role="radio" aria-checked={view === 'priority'} aria-label="Priority" title="Every agent, most urgent first" className={view === 'priority' ? 'is-on' : ''} onClick={() => { setView('priority') }}><Icon name="priority" /></button>
            </span>
          </SectionHead>
          <Collapse open={folds.isOpen('ships')}>
            {view === 'fleets' && (
              <>
                {agents.length > 0 && (
                  <div className="mx-ship-group">
                    <SectionHead label="Agents" count={agents.length} open={folds.isOpen('agents')} onToggle={() => { folds.toggle('agents') }} className="is-sub" />
                    <Collapse open={folds.isOpen('agents')}><div className="mx-ship-list" role="list" aria-label="Agents">{agents.map((ship) => row(ship))}</div></Collapse>
                  </div>
                )}
                {shells.length > 0 && (
                  <div className="mx-ship-group">
                    <SectionHead label="Shells" count={shells.length} open={folds.isOpen('shells')} onToggle={() => { folds.toggle('shells') }} className="is-sub" />
                    <Collapse open={folds.isOpen('shells')}><div className="mx-ship-list" role="list" aria-label="Shells">{shells.map((ship) => row(ship))}</div></Collapse>
                  </div>
                )}
                {matching.length === 0 && (
                  <div className="mx-empty-note">{selected ? emptyShipsNote({ fleetName: selected.name, query, hasShips: fleetShips.length > 0, otherMatch }) : 'Nothing aboard yet. Use + to start a session.'}</div>
                )}
              </>
            )}
            {view === 'priority' && (
              <>
                {priority.map((group) => (
                  <div key={group.id} className="mx-ship-group">
                    <SectionHead label={group.title} count={group.ships.length} open={folds.isOpen(`p-${group.id}`)} onToggle={() => { folds.toggle(`p-${group.id}`) }} className="is-sub" />
                    <Collapse open={folds.isOpen(`p-${group.id}`)}><div className="mx-ship-list" role="list" aria-label={group.title}>{group.ships.map((ship) => row(ship, fleetNameOf(ship)))}</div></Collapse>
                  </div>
                ))}
                {priorityTotal === 0 && <div className="mx-empty-note">{query.trim() === '' ? 'No agents are running. Use the chat button above to start one.' : `No agent matches “${query.trim()}”.`}</div>}
              </>
            )}
          </Collapse>
        </section>
      </div>
      <div className="mx-sidebar-foot">
        {online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">Reconnecting to maittyd…</span></>}
        <button type="button" className="mx-icon-button mx-settings-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" size={15} /></button>
      </div>
    </div>
  )
}

/** The words a rail tile says: the fleet, its machine and how many ships need you. */
function railWords(fleet: Fleet, head: MachineHead | undefined, needs: number): string {
  const where = head === undefined ? '' : ` on ${machineTitle(head)}`
  return `${fleet.name}${where}${needs > 0 ? `, ${String(needs)} need you` : ''}`
}

function Rail(props: SidebarProps) {
  const { sessions, fleets, machines, online } = props
  const tip = useTipWithKeys()
  const label = online ? `maittyd · ${String(sessions.length)} sessions` : 'Reconnecting to maittyd…'
  const hover = useRailHover()
  return (
    <div className="mx-rail">
      <RailPopoverLayer hover={hover} fleets={fleets} machines={machines} sessions={sessions} tracks={props.tracks} portalContainer={props.portalContainer} />
      <div className="mx-rail-top">
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Expand sidebar" title={tip('Expand sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="expand" /></button>
        <StartButtons props={props} rail />
        <InboxButton rail count={props.inboxCount} needs={props.inboxNeeds} onClick={props.onOpenInbox} />
      </div>
      <div className="mx-rail-divider" />
      <div className="mx-rail-list mx-no-scrollbar" role="group" aria-label="Fleets">
        {fleets.map((fleet) => {
          const head = machines.find((candidate) => candidate.machine === fleet.machine)
          const ships = aboard(sessions, fleets, fleet.key)
          const needs = needsCount(ships)
          const words = railWords(fleet, head, needs)
          return (
            <button
              key={fleet.key}
              type="button"
              className={`mx-rail-fleet${fleet.key === props.selectedFleet ? ' is-selected' : ''}${head?.down ? ' is-dimmed' : ''}`}
              aria-label={words}
              aria-current={fleet.key === props.selectedFleet ? 'true' : undefined}
              {...hoverHandlers(hover, fleetHoverId(fleet.key))}
              onClick={() => { hover.close(); props.onSelectFleet(fleet.key) }}
            >
              <FleetTile fleet={fleet} needs={needs} size="rail" />
            </button>
          )
        })}
      </div>
      <div className="mx-rail-bottom">
        {props.railUpdateSlot}
        <span className={`mx-daemon-dot${online ? '' : ' is-offline'}`} role="img" aria-label={label} title={label} />
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" /></button>
      </div>
    </div>
  )
}
