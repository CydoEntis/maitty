import type { MouseEvent } from 'react'
import type { SubagentStatus, SubagentView } from '../model/subagents'
import { AppIcon, Icon, Spinner } from './glyphs'

const STATUS_WORDS: Record<SubagentStatus, string> = { working: 'working', needs: 'needs you', done: 'done', stopped: 'stopped' }

/** A subagent's tile and its corner mark: a spinner while it works, an amber square when it needs you, a check when done. */
function SubagentIcon({ process, status }: { process: string; status: SubagentStatus }) {
  return (
    <span className="mx-sub-icon">
      <AppIcon process={process} size={14} />
      {status === 'working' && <span className="mx-sub-mark is-working"><Spinner className="" /></span>}
      {status === 'needs' && <span className="mx-sub-mark is-needs" />}
      {status === 'done' && <span className="mx-sub-mark is-done"><Icon name="check" size={6} strokeWidth={4} /></span>}
    </span>
  )
}

/** Under a session that is not selected: an elbow line and one small icon per subagent, with no text. */
export function SubagentPeek({ process, subagents }: { process: string; subagents: readonly SubagentView[] }) {
  const said = subagents.map((subagent) => `${subagent.name} ${STATUS_WORDS[subagent.status]}`).join(', ')
  return (
    <div className="mx-sub-peek" role="img" aria-label={`Subagents: ${said}`}>
      <span className="mx-sub-elbow" aria-hidden="true" />
      <span className="mx-sub-peek-icons">
        {subagents.map((subagent) => <SubagentIcon key={subagent.id} process={process} status={subagent.status} />)}
      </span>
    </div>
  )
}

type ListProps = {
  process: string
  subagents: readonly SubagentView[]
  /** The subagent whose chat the pane shows, if one is. */
  openId: string | null
  onOpen: (id: string) => void
  onContextMenu: (subagent: SubagentView, event: MouseEvent) => void
}

/** Under the selected session: one row per subagent on a guide line. Clicking one opens its own chat. */
export function SubagentList({ process, subagents, openId, onOpen, onContextMenu }: ListProps) {
  return (
    <div className="mx-sub-list" role="group" aria-label="Subagents">
      {subagents.map((subagent) => (
        <button
          key={subagent.id}
          type="button"
          className={`mx-sub-row${subagent.id === openId ? ' is-open' : ''}`}
          aria-label={`${subagent.name}, ${STATUS_WORDS[subagent.status]}`}
          aria-current={subagent.id === openId ? 'page' : undefined}
          onClick={() => { onOpen(subagent.id) }}
          onContextMenu={(event) => { onContextMenu(subagent, event) }}
        >
          <span className="mx-sub-guide" aria-hidden="true" />
          <span className="mx-sub-bend" aria-hidden="true" />
          <SubagentIcon process={process} status={subagent.status} />
          <span className="mx-sub-name">{subagent.name}</span>
        </button>
      ))}
    </div>
  )
}
