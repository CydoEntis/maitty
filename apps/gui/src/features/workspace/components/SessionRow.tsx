import type { MouseEvent } from 'react'
import { MAX_SESSION_NAME_LENGTH, STATUS_WORD, type SessionView } from '../model/session'
import { isShell, projectColor, projectOf, rowTitle, rowWord } from '../model/sidebar'
import { AppIcon, StatusBadge } from './glyphs'
import { beginPointerDrag } from './dragSession'
import { InlineName } from './InlineName'

type Props = {
  session: SessionView
  selected: boolean
  /** Inside a project group the project is in the heading, so the row leaves it out. */
  showProject: boolean
  renaming: boolean
  onShow: () => void
  onRename: () => void
  onSetName: (name: string) => void
  onCancelRename: () => void
  onContextMenu: (event: MouseEvent) => void
}

/** The line under a row's name: the project's colour and name, then the branch in mono. */
function SubLine({ session, showProject }: { session: SessionView; showProject: boolean }) {
  const project = projectOf(session)
  return (
    <span className="mx-row-sub">
      {showProject && (
        <span className="mx-row-project">
          <span className="mx-row-square" style={{ background: projectColor(project) }} aria-hidden="true" />
          {project}
        </span>
      )}
      {showProject && session.branch && <span aria-hidden="true">·</span>}
      {session.branch && <span className="mx-row-branch">{session.branch}</span>}
    </span>
  )
}

/** One session in the sidebar: its app tile and status badge, name, project and branch, and a word when it stopped. */
export function SessionRow({ session, selected, showProject, renaming, onShow, onRename, onSetName, onCancelRename, onContextMenu }: Props) {
  const title = rowTitle(session)
  const active = !isShell(session) && session.status !== 'idle'
  const word = rowWord(session)
  const className = `mx-row${selected ? ' is-selected' : ''}${active ? ' is-active' : ''}${isShell(session) ? ' is-shell' : ''}`
  const icon = (
    <span className="mx-row-icon">
      <AppIcon process={session.process} size={20} dimmed={session.status === 'unknown'} />
      {/* A stopped agent or an exited shell says so in words at the row's right instead. */}
      {session.status !== 'error' && <StatusBadge status={session.status} />}
    </span>
  )
  if (renaming) {
    return (
      <div className={className} onContextMenu={onContextMenu}>
        {icon}
        <span className="mx-row-text">
          <InlineName value={session.name ?? ''} placeholder={title} label="Session name" maxLength={MAX_SESSION_NAME_LENGTH} onCommit={onSetName} onCancel={onCancelRename} />
          <SubLine session={session} showProject={showProject} />
        </span>
      </div>
    )
  }
  return (
    <button
      type="button"
      className={className}
      onPointerDown={(event) => { beginPointerDrag(event, { kind: 'session', id: session.id, label: title }) }}
      aria-label={`${title}, ${session.process}, ${STATUS_WORD[session.status]}`}
      aria-current={selected ? 'page' : undefined}
      onClick={onShow}
      onDoubleClick={(event) => { event.preventDefault(); onRename() }}
      onKeyDown={(event) => { if (event.key === 'F2') { event.preventDefault(); onRename() } }}
      onContextMenu={onContextMenu}
    >
      {icon}
      <span className="mx-row-text">
        <span className="mx-row-name">{title}</span>
        <SubLine session={session} showProject={showProject} />
      </span>
      {word !== '' && <span className="mx-row-word">{word}</span>}
    </button>
  )
}
