import type { WorkflowView } from '@/features/workspace/model/workflows'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { StepMark } from '@/features/workspace/components/WorkflowRows'
import { STEP_WORDS, namesHelper } from '@/features/workspace/model/workflows'

/** A run a pane shows, the session's subagents its steps may name, and how one is opened. */
export type OpenRun = { view: WorkflowView; helpers: readonly SubagentView[]; onOpenHelper: (id: string) => void }

/** The bar over a workflow run's steps: which run, and the way back to its session. */
export function RunBar({ run, onBack }: { run: WorkflowView; onBack: () => void }) {
  return (
    <div className="mx-sub-bar" role="status">
      <span className="mx-sub-bar-dim">Run</span>
      <strong>{run.label}</strong>
      {run.title !== null && <span className="mx-sub-bar-dim">· {run.title}</span>}
      <span className="mx-spacer" />
      <button type="button" className="mx-sub-back" onClick={onBack}>Back to main session</button>
    </div>
  )
}

/**
 * A workflow run's steps as the workflow reported them (ADR 0010): each with its mark, note and the
 * helper doing it. A helper that is one of the session's subagents opens its chat.
 */
export function RunView({ run, helpers, onOpenHelper }: { run: WorkflowView; helpers: readonly SubagentView[]; onOpenHelper: (id: string) => void }) {
  return (
    <div className="mx-chat-scroll" role="log" aria-label={`Run ${run.label}`}>
      <div className="mx-chat-column">
        <ol className="mx-run-steps" aria-label="Steps">
          {run.steps.map((step) => {
            const label = step.helper
            const helper = label === null ? undefined : helpers.find((child) => namesHelper(label, child))
            return (
              <li key={step.name} className={`mx-run-step-row is-${step.state}`} aria-current={step === run.current ? 'step' : undefined}>
                <StepMark state={step.state} />
                <span className="mx-run-step-name">{step.name}</span>
                <span className="sr-only">{STEP_WORDS[step.state]}</span>
                {step.note !== null && <span className="mx-run-step-note">{step.note}</span>}
                <span className="mx-spacer" />
                {step.helper !== null && (helper === undefined
                  ? <span className="mx-run-helper">{step.helper}</span>
                  : <button type="button" className="mx-run-helper is-link" onClick={() => { onOpenHelper(helper.id) }}>{step.helper}</button>)}
              </li>
            )
          })}
        </ol>
        {run.link !== null && <p className="mx-sub-note">Ticket: {run.link}</p>}
      </div>
    </div>
  )
}
