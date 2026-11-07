import { useState } from 'react'
import type { Fleet } from '@/api/fleets'
import { iconStack, initialsOf, needsCount, type FleetColor } from '../model/fleets'
import type { SessionView } from '../model/session'
import { AppIcon, Icon } from './glyphs'
import { fleetStyle, MAX_FLEET_NAME_LENGTH } from './fleetStyle'
import { SESSION_DRAG_TYPE } from './dragSession'
import { useFleetMenu } from './FleetMenu'
import { InlineName } from './InlineName'

/** A fleet's tile: a sailboat (the rail: its initials) in its colour, with the number of its ships that need you. */
export function FleetTile({ fleet, needs, size = 'row' }: { fleet: Fleet; needs: number; size?: 'row' | 'rail' }) {
  return (
    <span className={`mx-fleet-tile is-${size}`} style={fleetStyle(fleet)} aria-hidden="true">
      {size === 'rail' ? initialsOf(fleet.name) : <Icon name="fleets" size={13} />}
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
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  /** False for the `Main` group this window adds: no command can name it, so it has no menu and takes no drops. */
  editable: boolean
  onSelect: () => void
  onRename: (name: string) => void
  onRecolor: (color: FleetColor) => void
  /** A ship was dropped on the row. */
  onDropShip: (sessionId: string) => void
}

/** One fleet in the FLEETS section. A double-click, or F2, renames it where it stands; a right-click recolours it; a ship dropped on it moves there. */
export function FleetRow({ fleet, ships, selected, dimmed, portalContainer, editable, onSelect, onRename, onRecolor, onDropShip }: Props) {
  const [editing, setEditing] = useState(false)
  const [over, setOver] = useState(false)
  const menu = useFleetMenu(fleet, editable ? { portalContainer, onRename: () => { setEditing(true) }, onRecolor } : null)
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
    <>
      <button
        type="button"
        className={`mx-fleet${selected ? ' is-selected' : ''}${dimmed ? ' is-dimmed' : ''}${over ? ' is-drop' : ''}`}
        style={fleetStyle(fleet)}
        aria-label={words}
        aria-current={selected ? 'true' : undefined}
        title={editable ? 'Double-click to rename. Right-click for colour. Drop a ship here to move it.' : undefined}
        onClick={onSelect}
        onDoubleClick={() => { setEditing(true) }}
        onContextMenu={menu.onContextMenu}
        onKeyDown={(event) => { if (event.key === 'F2') { event.preventDefault(); setEditing(true) } }}
        onDragOver={(event) => {
          if (!editable || !event.dataTransfer.types.includes(SESSION_DRAG_TYPE)) return
          event.preventDefault()
          event.dataTransfer.dropEffect = 'move'
          setOver(true)
        }}
        onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(false) }}
        onDrop={(event) => {
          setOver(false)
          const id = event.dataTransfer.getData(SESSION_DRAG_TYPE)
          if (!id) return
          event.preventDefault()
          onDropShip(id)
        }}
      >
        {body}
      </button>
      {menu.element}
    </>
  )
}
