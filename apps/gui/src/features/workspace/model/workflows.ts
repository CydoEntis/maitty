import type { WorkflowRun, WorkflowStepState } from '@/data/preview'

/** One run of a workflow as the sidebar and the run view show it (ADR 0010). Display only. */
export type WorkflowView = {
  /** The workflow and its run id: what names the run among the session's. */
  key: string
  /** The ticket's key, else the workflow and run id ("kitchen 007"). */
  label: string
  title: string | null
  link: string | null
  steps: WorkflowRun['steps']
  /** The step the run is on: the first that needs the person, else the first in hand, else the first still to do. */
  current: WorkflowRun['steps'][number] | null
  /** Some step is not over yet: waiting, working, sent back or needing the person. */
  active: boolean
  needsYou: boolean
}

/** States after which a step has nothing more to do. */
const OVER: ReadonlySet<WorkflowStepState> = new Set(['passed', 'failed', 'skipped'])

/** The order in which a step is the one a run is on. */
const CURRENT_FIRST: readonly WorkflowStepState[] = ['needs_you', 'working', 'sent_back', 'waiting']

export function toWorkflowView(run: WorkflowRun): WorkflowView {
  const current = CURRENT_FIRST.map((state) => run.steps.find((step) => step.state === state)).find((step) => step !== undefined)
    ?? run.steps.findLast((step) => step.state === 'failed')
    ?? null
  return {
    key: `${run.workflow}:${run.run}`,
    label: run.ticket?.key ?? `${run.workflow} ${run.run}`,
    title: run.ticket?.title ?? null,
    link: run.ticket?.link ?? null,
    steps: run.steps,
    current,
    active: run.steps.some((step) => !OVER.has(step.state) && step.state !== 'unknown'),
    needsYou: run.steps.some((step) => step.state === 'needs_you'),
  }
}

/** The words for a step's state, for its mark's label and screen readers. */
export const STEP_WORDS: Record<WorkflowStepState, string> = {
  waiting: 'Waiting',
  working: 'Working',
  passed: 'Passed',
  failed: 'Failed',
  sent_back: 'Sent back',
  needs_you: 'Needs you',
  skipped: 'Skipped',
  unknown: 'Unknown',
}
