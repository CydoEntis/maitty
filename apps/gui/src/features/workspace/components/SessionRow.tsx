import { useState } from 'react'
import { displayName, MAX_SESSION_NAME_LENGTH, statusText, STATUS_WORD, type SessionView } from '../model/session'
import { MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { useRecent } from '@/lib/clock'
import { Ago } from './times'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { InlineName } from './InlineName'
import { useSessionMenu } from './SessionMenu'
import { hoverHandlers, type RailHover } from './railHover'
import { useStatusEffect } from './motion'
import { StatusEffect } from './StatusEffect'
import { runsAgent, useArmed } from './useArmed'

export type SessionRowProps = {
  session: SessionView
  track: SessionTrack | undefined
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
  onEnd: () => void
}

/** A project's main row: the session's icon and state, what runs in it, and its menu. */
export function SessionRow(props: SessionRowProps) {
  const { session, track, selected, online } = props
  const unread = Boolean(track?.unread) && !selected
  const moved = useRecent(track?.movedAt ?? null, MOVED_HIGHLIGHT_MS)
  const status = session.status
  // How long ago it changed. A working row shows its spinner instead: a run time only counts from when this window saw it start.
  const changedAt = status === 'working' || status === 'needs' ? null : track?.changedAt ?? null
  const loud = selected || unread || status === 'needs'
  const effect = useStatusEffect(status, selected)
  const closing = useArmed(runsAgent(session.process))
  const [editing, setEditing] = useState(false)
  const menu = useSessionMenu(session, { online, portalContainer: props.portalContainer, onShow: props.onShow, onRename: () => { setEditing(true) }, onPin: props.onPin, onEnd: props.onEnd })
  const rowClass = ['mx-row', selected && 'is-selected', loud && 'is-loud', props.swapped && 'is-swapped', moved ? 'is-moved' : effect.className].filter(Boolean).join(' ')
  // The first line names the program, a working row has its spinner and an idle one is the default: say only what they do not.
  const line2 = status === 'working' || status === 'idle' ? '' : statusText(session)
  const line2Class = status === 'needs' ? 'is-needs' : status === 'error' ? 'is-error' : status === 'working' ? 'is-working' : ''
  const body = (
    <>
          {!moved && <StatusEffect effect={effect} />}
          <span className="mx-row-icon">
            <AppIcon process={session.process} size={24} dimmed={status === 'unknown'} />
            <StatusBadge status={status} />
          </span>
          <span className="mx-row-text">
            <span className="mx-row-line1">
              {editing
                ? <InlineName value={session.name ?? ''} placeholder={session.title} label={`Name for ${displayName(session)}`} maxLength={MAX_SESSION_NAME_LENGTH} onCommit={(name) => { setEditing(false); props.onSetName(name) }} onCancel={() => { setEditing(false) }} />
                : <span className="mx-row-title"><span className="mx-row-proc">{session.process}</span> · {displayName(session)}</span>}
              {session.pinned && <span className="mx-row-pin" role="img" aria-label="Pinned" title="Pinned to the top"><Icon name="pin" size={11} /></span>}
              {unread && <span className="mx-unread" aria-label="Unread" />}
              {status === 'working' && <Spinner />}
              {changedAt !== null && <span className="mx-row-meta"><Ago at={changedAt} /></span>}
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
          className={`mx-icon-button mx-row-end${closing.armed ? ' is-armed' : ''}`}
          aria-label={closing.armed ? `Click again to close and stop ${session.process}` : `Close session ${displayName(session)}`}
          title={closing.armed ? `Click again to close and stop ${session.process}` : 'Close session · ends its shell'}
          disabled={!online}
          onBlur={closing.disarm}
          onClick={() => { if (closing.press()) props.onEnd() }}
        >
          <Icon name="close" size={11} strokeWidth={2.25} />
        </button>
      </div>
      {menu.element}
    </>
  )
}
