import { useEffect, useState } from 'react'
import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import { displayName, statusPhrase, STATUS_WORD, type SessionView } from '../model/session'
import { formatAgo, formatElapsed, MOVED_HIGHLIGHT_MS, type SessionTrack } from '../model/tracking'
import { AppIcon, Icon, Spinner, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { keepFocus } from './menuFocus'
import { useEffectClass } from './motion'

const END_ARM_MS = 3000

export type SessionRowProps = {
  session: SessionView
  track: SessionTrack | undefined
  now: number
  selected: boolean
  /** The tab (counting from 1) the session is open in; undefined when only one tab is open. */
  tabNumber: number | undefined
  online: boolean
  /** This row just took over as its project's main row. */
  swapped: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onShow: () => void
  onRename: () => void
  onEnd: () => void
}

/** A project's main row: the session's icon and state, what runs in it, and its menu. */
export function SessionRow(props: SessionRowProps) {
  const { session, track, now, selected, online } = props
  const [armed, setArmed] = useState(false)
  useEffect(() => {
    if (!armed) return
    const timer = window.setTimeout(() => { setArmed(false) }, END_ARM_MS)
    return () => { window.clearTimeout(timer) }
  }, [armed])
  const unread = Boolean(track?.unread) && !selected
  const moved = track?.movedAt !== null && track?.movedAt !== undefined && now - track.movedAt < MOVED_HIGHLIGHT_MS
  const status = session.status
  const meta = status === 'working'
    ? (track?.workingSince ? formatElapsed(now - track.workingSince) : '')
    : status === 'needs' || !track?.changedAt ? '' : formatAgo(now - track.changedAt)
  const loud = selected || unread || status === 'needs'
  const effect = useEffectClass(status, selected)
  const rowClass = ['mx-row', selected && 'is-selected', loud && 'is-loud', props.swapped && 'is-swapped', moved ? 'is-moved' : effect].filter(Boolean).join(' ')
  const line2Class = status === 'needs' ? 'is-needs' : status === 'error' ? 'is-error' : status === 'working' ? 'is-working' : ''
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>
        <div className="mx-row-wrap">
          <button type="button" className={rowClass} draggable onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData(SESSION_DRAG_TYPE, session.id) }} aria-label={`${displayName(session)}, ${session.process}, ${STATUS_WORD[status]}`} aria-current={selected ? 'page' : undefined} onClick={props.onShow}>
            <span className="mx-row-icon">
              <AppIcon process={session.process} size={24} dimmed={status === 'unknown'} />
              <StatusBadge status={status} />
            </span>
            <span className="mx-row-text">
              <span className="mx-row-line1">
                <span className="mx-row-title">{displayName(session)}</span>
                {unread && <span className="mx-unread" aria-label="Unread" />}
                {status === 'working' && <Spinner />}
                {meta && <span className={`mx-row-meta${status === 'working' ? ' is-working' : ''}`}>{meta}</span>}
                {props.tabNumber !== undefined && <span className="mx-row-tab" title={`Open in tab ${String(props.tabNumber)}`}>{props.tabNumber}</span>}
              </span>
              <span className={`mx-row-line2 ${line2Class}`}><span className="mx-proc">{session.process}</span>{statusPhrase(session)}</span>
            </span>
          </button>
          <button
            type="button"
            className={`mx-icon-button mx-row-end${armed ? ' is-armed' : ''}`}
            aria-label={armed ? `End ${displayName(session)}: click again to confirm` : `Close session ${displayName(session)}`}
            title={armed ? 'Click again to end the session' : 'Close session · ends its shell'}
            disabled={!online}
            onClick={() => {
              if (!armed) {
                setArmed(true)
                return
              }
              setArmed(false)
              props.onEnd()
            }}
          >
            <Icon name="close" size={11} strokeWidth={2.25} />
          </button>
        </div>
      </ContextMenuPrimitive.Trigger>
      <ContextMenuPrimitive.Portal container={props.portalContainer}>
        <ContextMenuPrimitive.Content className="mx-menu" onCloseAutoFocus={keepFocus}>
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={props.onShow}>Show in pane</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={props.onRename}>Rename…</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item is-danger" disabled={!online} onSelect={props.onEnd}>Close session</ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.Content>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Root>
  )
}
