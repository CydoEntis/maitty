import { useState } from 'react'
import { displayName, iconSaysProcess, MAX_SESSION_NAME_LENGTH, statusPhrase, statusText, STATUS_WORD, type SessionView } from '../model/session'
import { formatAgo, MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { InlineName } from './InlineName'
import { useSessionMenu } from './SessionMenu'
import { hoverHandlers, type RailHover } from './railHover'
import { useEffectClass } from './motion'

export type SessionRowProps = {
  session: SessionView
  track: SessionTrack | undefined
  now: number
  selected: boolean
  online: boolean
  /** This row just took over as its project's main row. */
  swapped: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  /** Opens the session's popover beside the row. */
  hover: RailHover
  onShow: () => void
  onSetName: (name: string) => void
  onPin: (pinned: boolean) => void
  onArchive: () => void
  onEnd: () => void
}

/** A project's main row: the session's icon and state, what runs in it, and its menu. */
export function SessionRow(props: SessionRowProps) {
  const { session, track, now, selected, online } = props
  const unread = Boolean(track?.unread) && !selected
  const moved = track?.movedAt !== null && track?.movedAt !== undefined && now - track.movedAt < MOVED_HIGHLIGHT_MS
  const status = session.status
  // How long ago it changed. A working row shows its spinner instead: a run time only counts from when this window saw it start.
  const meta = status === 'working' || status === 'needs' || !track?.changedAt ? '' : formatAgo(now - track.changedAt)
  const loud = selected || unread || status === 'needs'
  const effect = useEffectClass(status, selected)
  const [editing, setEditing] = useState(false)
  const menu = useSessionMenu(session, { online, portalContainer: props.portalContainer, onShow: props.onShow, onRename: () => { setEditing(true) }, onPin: props.onPin, onArchive: props.onArchive, onEnd: props.onEnd })
  const rowClass = ['mx-row', selected && 'is-selected', loud && 'is-loud', props.swapped && 'is-swapped', moved ? 'is-moved' : effect].filter(Boolean).join(' ')
  // The icon already names the program, and a working row has its spinner: say only what they do not.
  const line2 = iconSaysProcess(session.process) ? (status === 'working' ? '' : statusText(session)) : <><span className="mx-proc">{session.process}</span>{statusPhrase(session)}</>
  const line2Class = status === 'needs' ? 'is-needs' : status === 'error' ? 'is-error' : status === 'working' ? 'is-working' : ''
  const body = (
    <>
          <span className="mx-row-icon">
            <AppIcon process={session.process} size={24} dimmed={status === 'unknown'} />
            <StatusBadge status={status} />
          </span>
          <span className="mx-row-text">
            <span className="mx-row-line1">
              {editing
                ? <InlineName value={session.name ?? ''} placeholder={session.title} label={`Name for ${displayName(session)}`} maxLength={MAX_SESSION_NAME_LENGTH} onCommit={(name) => { setEditing(false); props.onSetName(name) }} onCancel={() => { setEditing(false) }} />
                : <span className="mx-row-title">{displayName(session)}</span>}
              {session.pinned && <span className="mx-row-pin" role="img" aria-label="Pinned" title="Pinned to the top"><Icon name="pin" size={11} /></span>}
              {unread && <span className="mx-unread" aria-label="Unread" />}
              {status === 'working' && <Spinner />}
              {meta && <span className={`mx-row-meta${status === 'working' ? ' is-working' : ''}`}>{meta}</span>}
            </span>
            {line2 && <span className={`mx-row-line2 ${line2Class}`}>{line2}</span>}
          </span>
    </>
  )
  return (
    <>
      <div className="mx-row-wrap" onContextMenu={(event) => { props.hover.close(); menu.onContextMenu(event) }}>
        {editing ? <div className={rowClass}>{body}</div> : <button type="button" className={rowClass} {...hoverHandlers(props.hover, { kind: 'session', id: session.id })} draggable onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData(SESSION_DRAG_TYPE, session.id) }} aria-label={`${displayName(session)}, ${session.process}, ${STATUS_WORD[status]}`} aria-current={selected ? 'page' : undefined} onClick={props.onShow}>{body}</button>}
        <button
          type="button"
          className="mx-icon-button mx-row-end"
          aria-label={`Close session ${displayName(session)}`}
          title="Close session · ends its shell"
          disabled={!online}
          onClick={props.onEnd}
        >
          <Icon name="close" size={11} strokeWidth={2.25} />
        </button>
      </div>
      {menu.element}
    </>
  )
}
