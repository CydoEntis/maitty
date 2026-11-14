import { useState } from 'react'
import { Card, CardHead } from '@/components/ui/card'
import { Icon } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import { isWaitingStep, tookText } from '../stepInfo'
import { workDurationMs, workStatus, workSummary, type WorkStep } from '../workSummary'
import { StepGlyph } from './cardParts'
import { StepView } from './StepCards'

/** How many of a folded run's latest steps show under its summary. */
export const LATEST_STEPS = 3

const SUFFIX = { running: '', completed: '', failed: ' · a step failed', interrupted: ', then stopped' } as const
const TIME_TONE = { running: 'is-working', completed: 'is-dim', failed: 'is-danger', interrupted: 'is-dim' } as const

type WorkCardProps = {
  steps: readonly WorkStep[]
  root: string | null
  openDiff: ((file: string) => void) | null
  /** The id of the step a request is waiting on, when it is one of these. */
  waitingStep: string | null
}

/**
 * A run of steps as a card: one line saying what it did, over its latest three steps and how many more there are.
 * The head opens every step. One that failed, or that waits on the person for a step the fold hides, starts open,
 * so neither is ever folded away.
 */
export function WorkCard({ steps, root, openDiff, waitingStep }: WorkCardProps) {
  const status = workStatus(steps)
  const latest = steps.slice(-LATEST_STEPS)
  const hiddenWaiting = steps.slice(0, -LATEST_STEPS).some((step) => isWaitingStep(waitingStep, step.id))
  const [chosen, setChosen] = useState<boolean | null>(null)
  const open = chosen ?? (status === 'failed' || hiddenWaiting)
  const toggle = () => { setChosen(!open) }
  const took = workDurationMs(steps)
  const first = steps[0]
  const beyond = steps.length - latest.length
  const shown = open ? steps : latest
  return (
    <Card className="mx-work">
      <CardHead className="mx-work-cardhead">
        <button type="button" className="mx-work-head" aria-expanded={open} title={open ? 'Show only the latest steps' : 'Show every step'} onClick={toggle}>
          <StepGlyph status={status} />
          <span className="mx-work-text">{workSummary(steps)}{SUFFIX[status]}</span>
          {status === 'running' && first !== undefined
            ? <span className={`mx-step-meta ${TIME_TONE.running}`}><ElapsedSince since={first.timing.startedAtMs ?? first.observed.firstSeenAtMs} /></span>
            : took !== null && <span className={`mx-step-meta ${TIME_TONE[status]}`}>{tookText(took)}</span>}
          <span className="mx-spacer" />
          <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
        </button>
      </CardHead>
      <div className="mx-work-steps">
        {shown.map((step) => (
          <div key={step.id} className={step.status === 'failed' ? 'mx-work-failed' : undefined}>
            <StepView event={step} root={root} openDiff={openDiff} waiting={isWaitingStep(waitingStep, step.id)} />
          </div>
        ))}
        {!open && beyond > 0 && <button type="button" className="mx-work-more" onClick={toggle}>+{beyond} more</button>}
      </div>
    </Card>
  )
}
