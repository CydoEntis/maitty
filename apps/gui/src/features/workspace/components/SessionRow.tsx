import { useState } from 'react'
import { displayName, folderNameOf, MAX_SESSION_NAME_LENGTH, rowName, STATUS_WORD, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { InlineName } from './InlineName'
import { useSessionMenu } from './SessionMenu'
import { hoverHandlers, type RailHover } from './railHover'

export type SessionRowProps = {
  session: SessionView
  track: SessionTrack | undefined
  selected: boolean
  online: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  hover: RailHover
  onShow: () => void
  /** Clicking the folder badge puts the folder in the filter field. */
  onFilterFolder: (folder: string) => void
  onSetName: (name: string) => void
  onPin: (pinned: boolean) => void
  onSplit: (direction: 'right' | 'down') => void
  onCopyPath: () => void
  onEnd: () => void
}

/** The machine mark: a laptop for this computer, a server for another machine, dashed while that one is away. */
function MachineMark({ session }: { session: SessionView }) {
  const remote = session.machineName !== null
  const away = remote && session.status === 'unknown'
  return (
    <span className={`mx-row-machine${away ? ' is-away' : ''}`} title={remote ? `${session.machineName ?? ''}${away ? ' · reconnecting' : ''}` : 'This computer'}>
      <Icon name={remote ? 'server' : 'laptop'} size={13} />
    </span>
  )
}

/** One session in the flat list: its icon and status badge, name and diff counts, folder and machine. */
export function SessionRow(props: SessionRowProps) {
  const { session, track, selected, online } = props
  const unread = Boolean(track?.unread) && !selected
  const [editing, setEditing] = useState(false)
  const menu = useSessionMenu(session, { online, portalContainer: props.portalContainer, onSplit: props.onSplit, onCopyPath: props.onCopyPath, onRename: () => { setEditing(true) }, onPin: props.onPin, onEnd: props.onEnd })
  const folder = folderNameOf(session)
  const hasDiff = session.diffAdded + session.diffRemoved > 0
  const body = (
    <>
      <span className="mx-row-icon">
        <AppIcon process={session.process} size={24} dimmed={session.status === 'unknown'} />
        <StatusBadge status={session.status} />
      </span>
      <span className="mx-row-text">
        <span className="mx-row-line1">
          {editing
            ? <InlineName value={session.name ?? ''} placeholder={session.process} label={`Name for ${displayName(session)}`} maxLength={MAX_SESSION_NAME_LENGTH} onCommit={(name) => { setEditing(false); props.onSetName(name) }} onCancel={() => { setEditing(false) }} />
            : <span className="mx-row-title">{rowName(session)}</span>}
          {session.pinned && <span className="mx-row-pin" role="img" aria-label="Pinned" title="Pinned"><Icon name="pin" size={11} /></span>}
          {unread && <span className="mx-unread" role="img" aria-label="Unread" />}
          {hasDiff && <span className="mx-row-diff" title="Lines added and removed in this checkout"><span className="is-add">+{session.diffAdded}</span> <span className="is-del">−{session.diffRemoved}</span></span>}
        </span>
      </span>
    </>
  )
  return (
    <>
      <div className="mx-row-wrap" onContextMenu={(event) => { props.hover.close(); menu.onContextMenu(event) }}>
        {editing
          ? <div className="mx-row is-selected">{body}</div>
          : (
            <button
              type="button"
              className={`mx-row${selected ? ' is-selected' : ''}`}
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
        <button type="button" className="mx-row-folder" title={session.cwdLabel} aria-label={`Show only ${folder}`} onClick={() => { props.onFilterFolder(folder) }}>
          <Icon name="folder" size={11} />
          <span>{folder}</span>
        </button>
        <MachineMark session={session} />
      </div>
      {menu.element}
    </>
  )
}
