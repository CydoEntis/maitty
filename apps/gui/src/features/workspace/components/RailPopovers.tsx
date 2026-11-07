import type { CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import type { Fleet } from '@/api/fleets'
import { aboard, machineTitle } from '../model/fleets'
import type { MachineHead } from '../model/machines'
import { folderNameOf, rowName, STATUS_WORD, statusText, truncatePrompt, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import { Ago } from './times'
import { AppIcon, Spinner } from './glyphs'
import { FleetTile } from './FleetRow'
import { FLEET_HOVER_PREFIX, fleetHoverId, type RailHover } from './railHover'

const GAP_PX = 10
const CARD_WIDTH_PX = 300
/** Keeps a card for a row near the window's top or bottom inside the window. */
const EDGE_PX = 90
const HALF = 0.5

/** What the session is saying right now, in plain words: the exact request when it asks, else its state. */
function phraseOf(session: SessionView): string {
  if (session.status === 'needs' && session.asking) return truncatePrompt(session.asking)
  return statusText(session)
}

function SessionCard({ session, changedAt }: { session: SessionView; changedAt: number | null }) {
  return (
    <>
      <header className="mx-sc-head">
        <AppIcon process={session.process} size={22} />
        <span className="mx-sc-title">
          <span className="mx-sc-project">{folderNameOf(session)}</span>
          <strong>{rowName(session)}</strong>
        </span>
        <span className={`mx-sc-chip is-${session.status}`}>{session.status === 'working' && <Spinner />}{STATUS_WORD[session.status]}</span>
      </header>
      <div className="mx-sc-meta">
        <span className="mx-sc-path" dir="rtl"><bdi>{session.cwdLabel}</bdi></span>
        <span className="mx-sc-where">{session.machineName ?? 'This computer'}{changedAt !== null && <Ago at={changedAt} prefix=" · " />}</span>
      </div>
      <p className="mx-sc-phrase">{phraseOf(session)}</p>
    </>
  )
}

/** How many pending requests a fleet's card spells out before it says how many more. */
const MAX_ASKS = 3

/** A fleet at a glance: its machine, how many ships it holds and the exact request of each that needs you. */
function FleetCard({ fleet, head, ships }: { fleet: Fleet; head: MachineHead | undefined; ships: readonly SessionView[] }) {
  const asking = ships.filter((ship) => ship.status === 'needs')
  return (
    <>
      <header className="mx-sc-head">
        <FleetTile fleet={fleet} needs={0} size="row" />
        <span className="mx-sc-title">
          <span className="mx-sc-project">{head === undefined ? 'This computer' : machineTitle(head)}</span>
          <strong>{fleet.name}</strong>
        </span>
        {asking.length > 0 && <span className="mx-sc-chip is-needs">{String(asking.length)} need you</span>}
      </header>
      <div className="mx-sc-meta"><span>{ships.length} {ships.length === 1 ? 'ship' : 'ships'}</span></div>
      {asking.slice(0, MAX_ASKS).map((ship) => (
        <p key={ship.id} className="mx-sc-phrase"><strong>{rowName(ship)}</strong>{': '}{phraseOf(ship)}</p>
      ))}
      {asking.length > MAX_ASKS && <p className="mx-sc-phrase">and {asking.length - MAX_ASKS} more</p>}
    </>
  )
}

type LayerProps = {
  hover: RailHover
  fleets?: readonly Fleet[]
  machines?: readonly MachineHead[]
  sessions: readonly SessionView[]
  tracks: ReadonlyMap<string, SessionTrack>
  portalContainer: HTMLElement | null
}

/** The rail's hover card: the name, folder, machine and status of the session under the pointer. */
export function RailPopoverLayer({ hover, fleets = [], machines = [], sessions, tracks, portalContainer }: LayerProps) {
  const { shown } = hover
  if (!shown || !portalContainer) return null
  const fleet = shown.id.startsWith(FLEET_HOVER_PREFIX) ? fleets.find((candidate) => fleetHoverId(candidate.key) === shown.id) : undefined
  const session = fleet ? undefined : sessions.find((candidate) => candidate.id === shown.id)
  if (!fleet && !session) return null
  const centre = shown.anchor.top + shown.anchor.height * HALF
  const top = Math.min(Math.max(centre, EDGE_PX), window.innerHeight - EDGE_PX)
  // With the sidebar on the right, the card opens toward the middle of the window.
  const onRight = document.querySelector('.mx-app')?.getAttribute('data-side') === 'right'
  const left = onRight ? shown.anchor.left - GAP_PX - CARD_WIDTH_PX : shown.anchor.right + GAP_PX
  const style = { top, left, '--arrow-shift': `${String(centre - top)}px` } as CSSProperties
  return createPortal(
    <div role="tooltip" className={`mx-railpop is-session${onRight ? ' is-left' : ''}`} style={style} onMouseEnter={hover.hold} onMouseLeave={hover.leave}>
      {fleet
        ? <FleetCard fleet={fleet} head={machines.find((candidate) => candidate.machine === fleet.machine)} ships={aboard(sessions, fleets, fleet.key)} />
        : session && <SessionCard session={session} changedAt={tracks.get(session.id)?.changedAt ?? null} />}
    </div>,
    portalContainer,
  )
}
