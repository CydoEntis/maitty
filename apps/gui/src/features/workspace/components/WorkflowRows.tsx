import { useState } from 'react'
import type { WorkflowStepState } from '@/data/preview'
import { STEP_WORDS, type WorkflowView } from '../model/workflows'
import { Spinner } from './glyphs'

/** A workflow step's mark: its own shape for each state, so it reads without colour too. */
const STEP_GLYPH: Record<Exclude<WorkflowStepState, 'working'>, string> = {
  waiting: '·',
  passed: '✓',
  failed: '✕',
  sent_back: '↺',
  needs_you: '●',
  skipped: '–',
  unknown: '?',
}

export function StepMark({ state }: { state: WorkflowStepState }) {
  return (
    <span className={`mx-step-mark is-${state}`} role="img" aria-label={STEP_WORDS[state]}>
      {state === 'working' ? <Spinner className="" /> : STEP_GLYPH[state]}
    </span>
  )
}

type ListProps = {
  runs: readonly WorkflowView[]
  openKey: string | null
  onOpen: (key: string) => void
}

/**
 * Under the selected session: one row per workflow run still going, with the step it is on; the
 * finished ones fold into a line that lists them. Clicking a run opens its steps.
 */
export function WorkflowList({ runs, openKey, onOpen }: ListProps) {
  const [showDone, setShowDone] = useState(false)
  const active = runs.filter((run) => run.active)
  const done = runs.filter((run) => !run.active)
  const shown = showDone ? [...active, ...done] : active
  if (runs.length === 0) return null
  return (
    <div className="mx-sub-list" role="group" aria-label="Workflow runs">
      {shown.map((run) => (
        <button
          key={run.key}
          type="button"
          className={`mx-sub-row mx-run-row${run.key === openKey ? ' is-open' : ''}`}
          aria-label={`${run.label}${run.current === null ? '' : `, ${run.current.name}: ${STEP_WORDS[run.current.state]}`}`}
          aria-current={run.key === openKey ? 'page' : undefined}
          onClick={() => { onOpen(run.key) }}
        >
          <span className="mx-sub-guide" aria-hidden="true" />
          <span className="mx-sub-bend" aria-hidden="true" />
          <StepMark state={run.current?.state ?? 'passed'} />
          <span className="mx-sub-name">{run.label}</span>
          {run.current !== null && <span className="mx-run-step">{run.current.name}</span>}
        </button>
      ))}
      {done.length > 0 && (
        <button type="button" className="mx-sub-row mx-run-fold" aria-expanded={showDone} onClick={() => { setShowDone((now) => !now) }}>
          <span className="mx-sub-guide" aria-hidden="true" />
          <span className="mx-sub-bend" aria-hidden="true" />
          <span className="mx-sub-name">{done.length} {done.length === 1 ? 'run' : 'runs'} done · {showDone ? 'hide' : 'show'}</span>
        </button>
      )}
    </div>
  )
}
