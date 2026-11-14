import type { WorkflowStepState } from '@/data/preview'
import type { WorkflowView } from '@/features/workspace/model/workflows'

type Step = WorkflowView['steps'][number]

/** A run on the board: the step it sits in, and that step's place among the run's own. */
export type BoardTicket = { run: WorkflowView; step: Step }

/** One column: a step name as the workflows reported it, and the runs sitting in it. */
export type BoardColumn = { name: string; tickets: BoardTicket[] }

export type Board = {
  columns: BoardColumn[]
  /** Runs with every step over, folded away unless shown. */
  doneCount: number
  /** Runs with something still to do. */
  goingCount: number
  /** Some run going has a step that needs the person. */
  needsYou: boolean
}

/** States a run has moved past: the step is behind it and the ticket sits further on. */
const PASSED_BY: ReadonlySet<WorkflowStepState> = new Set(['passed', 'skipped'])

/**
 * The step names of every run, in the order the runs give them. A name one run has and another
 * lacks goes in after the step before it, so two workflows with different steps still read in order.
 */
export function columnNames(runs: readonly WorkflowView[]): string[] {
  const names: string[] = []
  for (const run of runs) {
    let previous = -1
    for (const step of run.steps) {
      let at = names.indexOf(step.name)
      if (at < 0) {
        at = previous + 1
        names.splice(at, 0, step.name)
      }
      previous = at
    }
  }
  return names
}

/** The step a run sits in: its first that has not passed, else its last (a run that finished). */
export function sittingStep(run: WorkflowView): Step | null {
  return run.steps.find((step) => !PASSED_BY.has(step.state)) ?? run.steps.at(-1) ?? null
}

/**
 * The Work board of a session's workflow runs (ADR 0010): one column per reported step name, each run
 * in the step it sits in. Finished runs stay out of the columns unless `showDone`, but always name the
 * columns, so showing them never reorders the board. A run that reported no steps has nowhere to sit.
 */
export function workBoard(runs: readonly WorkflowView[], showDone: boolean): Board {
  const columns: BoardColumn[] = columnNames(runs).map((name) => ({ name, tickets: [] }))
  for (const run of runs) {
    if (!run.active && !showDone) continue
    const step = sittingStep(run)
    if (step === null) continue
    columns.find((column) => column.name === step.name)?.tickets.push({ run, step })
  }
  const going = runs.filter((run) => run.active)
  return { columns, doneCount: runs.length - going.length, goingCount: going.length, needsYou: going.some((run) => run.needsYou) }
}
