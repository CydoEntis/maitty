import { describe, expect, it } from 'vitest'
import type { WorkflowRun, WorkflowStepState } from '@/data/preview'
import { toWorkflowView } from '@/features/workspace/model/workflows'
import { columnNames, sittingStep, workBoard } from './board'

const run = (id: string, steps: Array<[string, WorkflowStepState]>, key: string | null = null) => toWorkflowView({
  workflow: 'kitchen',
  run: id,
  ticket: key === null ? null : { key, title: null, link: null },
  steps: steps.map(([name, state]) => ({ name, state, note: null, helper: null })),
  reportedAt: 1,
} satisfies WorkflowRun)

const names = (board: ReturnType<typeof workBoard>) => board.columns.map((column) => [column.name, column.tickets.map((ticket) => ticket.run.label)])

describe('the Work board', () => {
  it('has one column per reported step name, in the order the runs give them', () => {
    const a = run('1', [['intake', 'passed'], ['build', 'working'], ['review', 'waiting']])
    const b = run('2', [['intake', 'passed'], ['spec', 'working'], ['build', 'waiting'], ['ship', 'waiting']])
    expect(columnNames([a, b])).toEqual(['intake', 'spec', 'build', 'ship', 'review'])
  })

  it('places a ticket in its first step that has not passed, skipped steps included as passed', () => {
    expect(sittingStep(run('1', [['intake', 'passed'], ['spec', 'skipped'], ['build', 'sent_back'], ['review', 'failed']]))?.name).toBe('build')
    expect(sittingStep(run('2', [['intake', 'needs_you'], ['build', 'waiting']]))?.name).toBe('intake')
    expect(sittingStep(run('3', [['intake', 'passed'], ['build', 'failed']]))?.name).toBe('build')
    expect(sittingStep(run('4', []))).toBeNull()
  })

  it('folds finished runs away, keeps their columns, and shows them in their last step when asked', () => {
    const going = run('1', [['intake', 'passed'], ['build', 'working'], ['review', 'waiting']], 'MAITTY-42')
    const finished = run('2', [['intake', 'passed'], ['build', 'passed'], ['review', 'passed'], ['approve', 'passed']], 'MAITTY-40')
    const failed = run('3', [['intake', 'passed'], ['build', 'failed']], 'MAITTY-41')
    const folded = workBoard([going, finished, failed], false)
    expect(names(folded)).toEqual([['intake', []], ['build', ['MAITTY-42']], ['review', []], ['approve', []]])
    expect(folded).toMatchObject({ doneCount: 2, goingCount: 1, needsYou: false })
    expect(names(workBoard([going, finished, failed], true))).toEqual([['intake', []], ['build', ['MAITTY-42', 'MAITTY-41']], ['review', []], ['approve', ['MAITTY-40']]])
  })

  it('says when a run going needs the person, and names a run with no ticket by its workflow', () => {
    const board = workBoard([run('007', [['spec', 'needs_you']])], false)
    expect(board.needsYou).toBe(true)
    expect(names(board)).toEqual([['spec', ['kitchen 007']]])
  })
})
