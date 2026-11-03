import { useId } from 'react'
import type { SessionStatus } from '../model/session'
import { STATUS_HELP, STATUS_LABEL } from '../model/statusText'
import { Icon, PauseBars, Spinner } from './glyphs'
import { ElapsedSince } from './times'

function Glyph({ status }: { status: SessionStatus }) {
  switch (status) {
    case 'working':
      return <Spinner className="" />
    case 'needs':
      return <PauseBars />
    case 'done':
      return <Icon name="check" />
    case 'error':
      return <Icon name="bang" />
    case 'unknown':
      return <span className="mx-status-dashed" />
    case 'idle':
      return <span className="mx-status-dot" />
  }
}

/** A session's state in the pane's bottom-right corner, with a popover that says what it means. */
export function PaneStatus({ status, workingSince }: { status: SessionStatus; workingSince: number | null }) {
  const popoverId = useId()
  const label = STATUS_LABEL[status]
  return (
    <span className="mx-pane-status-wrap">
      <button key={status} type="button" className={`mx-pane-status is-${status}`} aria-label={`Status: ${label}`} aria-describedby={popoverId}>
        <span className="mx-pane-status-icon" aria-hidden="true"><Glyph status={status} /></span>
        <span>{label}{status === 'working' && workingSince !== null && <ElapsedSince since={workingSince} prefix=" · " />}</span>
      </button>
      <span role="tooltip" id={popoverId} className="mx-status-pop"><strong>{label}</strong>{STATUS_HELP[status]}</span>
    </span>
  )
}
