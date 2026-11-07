import { useState } from 'react'
import type { Fleet } from '@/api/fleets'
import { displayName, MAX_SESSION_NAME_LENGTH, rowName, STATUS_WORD, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import type { PaneView } from '../model/chatView'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { InlineName } from './InlineName'
import { useSessionMenu } from './SessionMenu'
import { hoverHandlers, type RailHover } from './railHover'

export type ShipRowProps = {
  session: SessionView
  track: SessionTrack | undefined
  selected: boolean
  online: boolean
  /** The view the ship shows, for the terminal or chat mark at the row's right. */
  view: PaneView
  /** The Priority view names the ship's fleet on its row. */
  fleetName?: string
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  hover: RailHover
  onShow: () => void
  onSetName: (name: string) => void
  onPin: (pinned: boolean) => void
  onSplit: (direction: 'right' | 'down') => void
  onCopyPath: () => void
  onEnd: () => void
  /** The other fleets on the ship's machine it can move to, and the move. */
  moveTargets: readonly Fleet[]
  onMove: (fleet: Fleet) => void
}

/** The line under a ship's name: an agent's branch, a shell's folder (never a branch), or in Priority the fleet. */
function SubLine({ session, agent, fleetName }: { session: SessionView; agent: boolean; fleetName: string | undefined }) {
  if (fleetName !== undefined) return <span className="mx-ship-sub is-fleet">{fleetName}{session.branch ? <> · <span className="mx-ship-branch">{session.branch}</span></> : null}</span>
  if (agent) {
    return session.branch
      ? <span className="mx-ship-sub is-branch"><Icon name="branch" size={10} /><span className="mx-ship-branch">{session.branch}</span></span>
      : <span className="mx-ship-sub">{session.cwdLabel}</span>
  }
  return <span className="mx-ship-sub is-folder"><Icon name="folder" /><span>{session.cwdLabel}</span></span>
}

/** One ship in the SHIPS section: its app tile and status badge, name and a line of where it is. */
export function ShipRow(props: ShipRowProps) {
  const { session, track, selected, online } = props
  const agent = session.process !== 'shell'
  const unread = Boolean(track?.unread) && !selected
  const [editing, setEditing] = useState(false)
  const menu = useSessionMenu(session, { online, portalContainer: props.portalContainer, onSplit: props.onSplit, onCopyPath: props.onCopyPath, onRename: () => { setEditing(true) }, onPin: props.onPin, onEnd: props.onEnd, moveTargets: props.moveTargets, onMove: props.onMove })
  const body = (
    <>
      <span className="mx-ship-icon">
        <AppIcon process={session.process} size={20} dimmed={session.status === 'unknown'} />
        <StatusBadge status={session.status} />
      </span>
      <span className="mx-ship-text">
        <span className="mx-ship-line1">
          {editing
            ? <InlineName value={session.name ?? ''} placeholder={session.process} label={`Name for ${displayName(session)}`} maxLength={MAX_SESSION_NAME_LENGTH} onCommit={(name) => { setEditing(false); props.onSetName(name) }} onCancel={() => { setEditing(false) }} />
            : <span className="mx-ship-title">{rowName(session)}</span>}
          {session.pinned && <span className="mx-row-pin" role="img" aria-label="Pinned" title="Pinned"><Icon name="pin" size={11} /></span>}
          {unread && <span className="mx-unread" role="img" aria-label="Unread" />}
        </span>
        <SubLine session={session} agent={agent} fleetName={props.fleetName} />
      </span>
      {agent && (
        <span className="mx-ship-side" aria-hidden="true">
          <Icon name={props.view === 'chat' ? 'chat' : 'terminal'} size={13} />
          {session.worktree && <small>worktree</small>}
        </span>
      )}
    </>
  )
  return (
    <>
      <div className="mx-ship-wrap" onContextMenu={(event) => { props.hover.close(); menu.onContextMenu(event) }}>
        {editing
          ? <div className="mx-ship is-selected">{body}</div>
          : (
            <button
              type="button"
              className={`mx-ship${selected ? ' is-selected' : ''}${agent ? '' : ' is-shell'}${session.status === 'idle' ? ' is-idle' : ''}`}
              {...hoverHandlers(props.hover, session.id)}
              draggable
              onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData(SESSION_DRAG_TYPE, session.id) }}
              aria-label={`${displayName(session)}, ${session.process}, ${STATUS_WORD[session.status]}`}
              aria-current={selected ? 'page' : undefined}
              onClick={props.onShow}
              onKeyDown={(event) => { if (event.key === 'F2') { event.preventDefault(); setEditing(true) } }}
            >
              {body}
            </button>
          )}
      </div>
      {menu.element}
    </>
  )
}
