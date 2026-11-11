import type { MouseEvent } from 'react'
import { MAX_SESSION_NAME_LENGTH, STATUS_WORD, type SessionView } from '../model/session'
import { isShell, projectColor, projectOf, rowTitle, rowWord } from '../model/sidebar'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { beginPointerDrag } from './dragSession'
import { InlineName } from './InlineName'
import { Ago } from './times'

type Props = {
  session: SessionView
  selected: boolean
  /** Inside a project group the project is in the heading, so the row leaves it out. */
  showProject: boolean
  /** The other machine the session runs on, when no heading above the row says so. */
  machine?: string | null
  renaming: boolean
  onShow: () => void
  onRename: () => void
  onSetName: (name: string) => void
  onCancelRename: () => void
  onContextMenu: (event: MouseEvent) => void
}

/**
 * The line over a row's name, saying where the session is: the project's colour and folder at the left, and at the
 * right the branch in the theme's colour, or the tree and the branch in the worktree's when the session runs in one.
 * Nothing when there is none of those.
 */
function PlaceLine({ session, showProject, machine }: { session: SessionView; showProject: boolean; machine: string | null }) {
  const project = projectOf(session)
  if (!showProject && machine === null && !session.branch) return null
  return (
    <span className="mx-row-sub">
      {showProject && (
        <span className="mx-row-project">
          <span className="mx-row-square" style={{ background: projectColor(project) }} aria-hidden="true" />
          <span className="mx-row-project-name">{project}</span>
        </span>
      )}
      {machine !== null && <span className="mx-row-machine">· {machine}</span>}
      {session.branch && (
        <span className={`mx-row-branch${session.worktree ? ' is-worktree' : ''}`} title={session.worktree ? `Worktree on branch ${session.branch}` : `Branch ${session.branch}`}>
          <Icon name={session.worktree ? 'worktree' : 'branch'} size={10} />
          <span className="mx-row-branch-name">{session.branch}</span>
        </span>
      )}
    </span>
  )
}

/** One session in the sidebar: its app tile and status badge, where it is over its name (the folder at the left, the branch or worktree at the right), and beside the name a word when it stopped, else how long it has been in its state. */
export function SessionRow({ session, selected, showProject, machine = null, renaming, onShow, onRename, onSetName, onCancelRename, onContextMenu }: Props) {
  const title = rowTitle(session)
  const active = !isShell(session) && session.status !== 'idle'
  const word = rowWord(session)
  const className = `mx-row${selected ? ' is-selected' : ''}${active ? ' is-active' : ''}${isShell(session) ? ' is-shell' : ''}`
  const icon = (
    <span className="mx-row-icon">
      <AppIcon process={session.process} size={20} dimmed={session.status === 'unknown'} />
      <StatusBadge status={session.status} />
    </span>
  )
  if (renaming) {
    return (
      <div className={className} onContextMenu={onContextMenu}>
        {icon}
        <span className="mx-row-text">
          <PlaceLine session={session} showProject={showProject} machine={machine} />
          <InlineName value={session.name ?? ''} placeholder={title} label="Session name" maxLength={MAX_SESSION_NAME_LENGTH} onCommit={onSetName} onCancel={onCancelRename} />
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
        <PlaceLine session={session} showProject={showProject} machine={machine} />
        <span className="mx-row-title">
          <span className="mx-row-name">{title}</span>
          {word !== '' && <span className="mx-row-word">{word}</span>}
          {/* How long the agent has been in this state; a shell has no state worth timing. */}
          {word === '' && !isShell(session) && session.statusSince !== null && <span className="mx-row-when"><Ago at={session.statusSince} /></span>}
        </span>
      </span>
    </button>
  )
}
