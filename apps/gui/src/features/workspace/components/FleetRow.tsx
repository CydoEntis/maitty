import { useState } from 'react'
import type { Fleet } from '@/api/fleets'
import { iconStack, initialsOf, needsCount } from '../model/fleets'
import type { SessionView } from '../model/session'
import { AppIcon } from './glyphs'
import { fleetStyle, MAX_FLEET_NAME_LENGTH } from './fleetStyle'
import { InlineName } from './InlineName'

/** A fleet's tile: its initials in its colour, with the number of its ships that need you. */
export function FleetTile({ fleet, needs, size = 'row' }: { fleet: Fleet; needs: number; size?: 'row' | 'rail' }) {
  return (
    <span className={`mx-fleet-tile is-${size}`} style={fleetStyle(fleet)} aria-hidden="true">
      {initialsOf(fleet.name)}
      {needs > 0 && <span className="mx-fleet-badge">{needs}</span>}
    </span>
  )
}

type Props = {
  fleet: Fleet
  ships: readonly SessionView[]
  selected: boolean
  /** The machine is away: the row is dimmed but still opens what was last seen. */
  dimmed: boolean
  onSelect: () => void
  onRename: (name: string) => void
}

/** One fleet in the FLEETS section. A double-click, or F2, renames it where it stands. */
export function FleetRow({ fleet, ships, selected, dimmed, onSelect, onRename }: Props) {
  const [editing, setEditing] = useState(false)
  const needs = needsCount(ships)
  const stack = iconStack(ships)
  const words = `${fleet.name}, ${String(ships.length)} ${ships.length === 1 ? 'ship' : 'ships'}${needs > 0 ? `, ${String(needs)} need you` : ''}`
  const body = (
    <>
      <FleetTile fleet={fleet} needs={needs} />
      {editing
        ? <InlineName value={fleet.name} placeholder="Fleet name" label={`Name for ${fleet.name}`} maxLength={MAX_FLEET_NAME_LENGTH} onCommit={(name) => { setEditing(false); if (name !== '' && name !== fleet.name) onRename(name) }} onCancel={() => { setEditing(false) }} />
        : <span className="mx-fleet-name">{fleet.name}</span>}
      <span className="mx-fleet-stack" aria-hidden="true">
        {stack.processes.map((process, index) => <span key={`${process}-${String(index)}`} className="mx-fleet-stack-icon"><AppIcon process={process} size={18} /></span>)}
        {stack.more > 0 && <span className="mx-fleet-more">+{stack.more}</span>}
      </span>
    </>
  )
  if (editing) return <div className={`mx-fleet is-editing${selected ? ' is-selected' : ''}`} style={fleetStyle(fleet)}>{body}</div>
  return (
    <button
      type="button"
      className={`mx-fleet${selected ? ' is-selected' : ''}${dimmed ? ' is-dimmed' : ''}`}
      style={fleetStyle(fleet)}
      aria-label={words}
      aria-current={selected ? 'true' : undefined}
      title="Double-click to rename"
      onClick={onSelect}
      onDoubleClick={() => { setEditing(true) }}
      onKeyDown={(event) => { if (event.key === 'F2') { event.preventDefault(); setEditing(true) } }}
    >
      {body}
    </button>
  )
}
