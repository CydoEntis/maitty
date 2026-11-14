import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Counter } from '@/components/ui/progress'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { STEP_WORDS, type WorkflowView } from '@/features/workspace/model/workflows'
import { workBoard, type BoardTicket } from '../model/board'
import { RunDrawer } from './RunDrawer'
import { WorkStepMark } from './WorkStepMark'

/** A run's card in the column of the step it sits in: that step's mark, the key, the title, the run's progress. */
function TicketCard({ ticket, open, onOpen }: { ticket: BoardTicket; open: boolean; onOpen: () => void }) {
  const { run, step } = ticket
  const passed = run.steps.filter((one) => one.state === 'passed' || one.state === 'skipped').length
  return (
    <li>
      <button
        type="button"
        className={`mx-board-card is-${step.state}${run.active ? '' : ' is-over'}${open ? ' is-open' : ''}`}
        aria-label={`${run.label}${run.title === null ? '' : `, ${run.title}`}: ${step.name}, ${STEP_WORDS[step.state]}`}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={onOpen}
      >
        <span className="mx-board-card-head">
          <WorkStepMark state={step.state} />
          <span className="mx-board-card-key">{run.label}</span>
        </span>
        {run.title !== null && <span className="mx-board-card-title">{run.title}</span>}
        <span className="mx-board-segments" role="img" aria-label={`${String(passed)} of ${String(run.steps.length)} steps passed`}>
          {run.steps.map((one) => <span key={one.name} className={`mx-board-segment is-${one.state}`} />)}
        </span>
        {(step.note !== null || step.helper !== null) && (
          <span className="mx-board-card-foot">
            {step.note !== null && <span className="mx-board-card-note">{step.note}</span>}
            {step.helper !== null && <span className="mx-board-card-helper">{step.helper}</span>}
          </span>
        )}
      </button>
    </li>
  )
}

type Props = {
  runs: readonly WorkflowView[]
  /** The session's subagents, which a step's helper label may name. */
  helpers: readonly SubagentView[]
  onOpenHelper: (id: string) => void
  /** The run whose drawer starts open, as the gallery draws it. */
  startOpen?: string
}

/**
 * A session's workflow runs as a board (ADR 0010): one equal column per step name the workflows reported, in order,
 * each run in the step it sits in. Finished runs fold into one line until shown. A run opens in a drawer.
 */
export function WorkBoard({ runs, helpers, onOpenHelper, startOpen }: Props) {
  const [showDone, setShowDone] = useState(false)
  const [openKey, setOpenKey] = useState<string | null>(startOpen ?? null)
  const [root, setRoot] = useState<HTMLDivElement | null>(null)
  const board = workBoard(runs, showDone)
  const opened = runs.find((run) => run.key === openKey)
  const going = `${String(board.goingCount)} ${board.goingCount === 1 ? 'run' : 'runs'} going`
  return (
    <div ref={setRoot} className="mx-board" role="region" aria-label="Work">
      <div className="mx-board-bar">
        <strong>{going}</strong>
        {board.needsYou && <span className="mx-board-needs">needs you</span>}
        <span className="mx-spacer" />
        {board.doneCount > 0 && (
          <span className="mx-board-fold">
            <Counter>{board.doneCount} {board.doneCount === 1 ? 'run' : 'runs'} done</Counter>
            <Button variant="ghost" size="sm" aria-expanded={showDone} onClick={() => { setShowDone(!showDone) }}>{showDone ? 'Hide' : 'Show'}</Button>
          </span>
        )}
      </div>
      {board.columns.length === 0 ? (
        <p className="mx-board-empty">No steps reported yet.</p>
      ) : (
        <div className="mx-board-columns" style={{ ['--work-columns' as string]: board.columns.length }}>
          {board.columns.map((column) => (
            <section key={column.name} className="mx-board-column" aria-label={column.name}>
              <header className="mx-board-column-head">
                <span className="mx-board-column-name">{column.name}</span>
                <Counter>{column.tickets.length}</Counter>
              </header>
              <ul className="mx-board-tickets">
                {column.tickets.map((ticket) => <TicketCard key={ticket.run.key} ticket={ticket} open={ticket.run.key === openKey} onOpen={() => { setOpenKey(ticket.run.key) }} />)}
              </ul>
            </section>
          ))}
        </div>
      )}
      <RunDrawer run={opened ?? null} helpers={helpers} onOpenHelper={onOpenHelper} onClose={() => { setOpenKey(null) }} container={root} />
    </div>
  )
}
