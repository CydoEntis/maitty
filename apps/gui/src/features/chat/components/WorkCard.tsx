import { useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import { commandMeta, isWaitingStep, relativePath, tookText } from '../stepInfo'
import { stackCards, stackLines, stepsBeyondStack, type StackStep } from '../workStack'
import { workDurationMs, workStatus, workSummary, type WorkStep } from '../workSummary'
import { ChangePill, FileBadge, StepGlyph } from './cardParts'
import { StepView } from './StepCards'

const SUFFIX = { running: '', completed: '', failed: ' · a step failed', interrupted: ', then stopped' } as const
const TIME_TONE = { running: 'is-working', completed: 'is-dim', failed: 'is-danger', interrupted: 'is-dim' } as const

function cardMeta(step: StackStep) {
  if (step.status === 'running') return <ElapsedSince since={step.timing.startedAtMs ?? step.observed.firstSeenAtMs} />
  if (step.kind === 'command') return commandMeta(step)
  if (step.status === 'failed') return 'failed'
  if (step.status === 'interrupted') return 'stopped'
  if (step.diff.status !== 'available') return null
  const { added, removed } = step.diff
  return [added > 0 ? `+${String(added)}` : null, removed > 0 ? `−${String(removed)}` : null].filter((part) => part !== null).join(' ')
}

/** One card of the stack. Only the front one shows its content; the ones behind show their edges. */
function StackCard({ step, depth, root }: { step: StackStep; depth: number; root: string | null }) {
  return (
    <span className={`mx-stack-card is-depth-${String(depth)} is-${step.status}`}>
      <span className="mx-stack-head">
        <StepGlyph status={step.status} />
        {step.kind === 'command'
          ? <span className="mx-stack-command"><span className="mx-step-prompt">$</span><span>{step.command.split('\n')[0]}</span></span>
          : <span className="mx-stack-file">{step.file === null ? 'a file' : <FileBadge path={relativePath(step.file, root)} />}<ChangePill change={step.change} /></span>}
        <span className="mx-spacer" />
        <span className={`mx-step-meta ${TIME_TONE[step.status]}`}>{cardMeta(step)}</span>
      </span>
      <span className="mx-stack-body">
        {stackLines(step).map((line, index) => (
          <span key={index} className={`mx-stack-line is-${line.tone}`}><span className="mx-stack-mark">{line.mark}</span><span>{line.text}</span></span>
        ))}
      </span>
    </span>
  )
}

type WorkCardProps = {
  steps: readonly WorkStep[]
  root: string | null
  openDiff: ((file: string) => void) | null
  /** The id of the step a request is waiting on, when it is one of these. */
  waitingStep: string | null
}

/**
 * A run of steps as one line saying what it did, over a stack of its latest commands and edits. A click on
 * either opens every step on a rail. One that failed starts open, so a failure is never folded away.
 */
export function WorkCard({ steps, root, openDiff, waitingStep }: WorkCardProps) {
  const status = workStatus(steps)
  const [chosen, setChosen] = useState<boolean | null>(null)
  const open = chosen ?? status === 'failed'
  const toggle = () => { setChosen(!open) }
  const took = workDurationMs(steps)
  const cards = stackCards(steps)
  const beyond = stepsBeyondStack(steps)
  const first = steps[0]
  return (
    <div className="mx-work">
      <button type="button" className="mx-work-head" aria-expanded={open} title={open ? 'Hide the steps' : 'Show the steps'} onClick={toggle}>
        <StepGlyph status={status} />
        <span className="mx-work-text">{workSummary(steps)}{SUFFIX[status]}</span>
        {beyond > 0 && <span className="mx-work-more">+{beyond} more</span>}
        <span className="mx-spacer" />
        {status === 'running' && first !== undefined
          ? <span className={`mx-step-meta ${TIME_TONE.running}`}><ElapsedSince since={first.timing.startedAtMs ?? first.observed.firstSeenAtMs} /></span>
          : took !== null && <span className={`mx-step-meta ${TIME_TONE[status]}`}>{tookText(took)}</span>}
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
      </button>
      {!open && cards.length > 0 && (
        // The stack repeats what the row above opens, so it is a second place to click and not a second stop for the keyboard.
        <button type="button" tabIndex={-1} aria-hidden="true" className={`mx-stack has-${String(cards.length)}`} onClick={toggle}>
          {cards.map((step, depth) => <StackCard key={step.id} step={step} depth={depth} root={root} />)}
        </button>
      )}
      {open && (
        <div className="mx-work-steps">
          {steps.map((step) => (
            <div key={step.id} className={step.status === 'failed' ? 'mx-work-failed' : undefined}>
              <StepView event={step} root={root} openDiff={openDiff} waiting={isWaitingStep(waitingStep, step.id)} />
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
