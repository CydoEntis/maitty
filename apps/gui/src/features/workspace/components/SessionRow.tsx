import { displayName, statusPhrase, STATUS_WORD, type SessionView } from '../model/session'
import { formatAgo, MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { useSessionMenu } from './SessionMenu'
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
  const menu = useSessionMenu(session, { online, portalContainer: props.portalContainer, onShow: props.onShow, onSetName: props.onSetName, onPin: props.onPin, onArchive: props.onArchive, onEnd: props.onEnd })
  const rowClass = ['mx-row', selected && 'is-selected', loud && 'is-loud', props.swapped && 'is-swapped', moved ? 'is-moved' : effect].filter(Boolean).join(' ')
  const line2Class = status === 'needs' ? 'is-needs' : status === 'error' ? 'is-error' : status === 'working' ? 'is-working' : ''
  return (
    <>
      <div className="mx-row-wrap" onContextMenu={menu.onContextMenu}>
        <button type="button" className={rowClass} draggable onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData(SESSION_DRAG_TYPE, session.id) }} aria-label={`${displayName(session)}, ${session.process}, ${STATUS_WORD[status]}`} aria-current={selected ? 'page' : undefined} onClick={props.onShow}>
          <span className="mx-row-icon">
            <AppIcon process={session.process} size={24} dimmed={status === 'unknown'} />
            <StatusBadge status={status} />
          </span>
          <span className="mx-row-text">
            <span className="mx-row-line1">
              <span className="mx-row-title">{displayName(session)}</span>
              {session.pinned && <span className="mx-row-pin" role="img" aria-label="Pinned" title="Pinned to the top"><Icon name="pin" size={11} /></span>}
              {unread && <span className="mx-unread" aria-label="Unread" />}
              {status === 'working' && <Spinner />}
              {meta && <span className={`mx-row-meta${status === 'working' ? ' is-working' : ''}`}>{meta}</span>}
            </span>
            <span className={`mx-row-line2 ${line2Class}`}><span className="mx-proc">{session.process}</span>{statusPhrase(session)}</span>
          </span>
        </button>
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
