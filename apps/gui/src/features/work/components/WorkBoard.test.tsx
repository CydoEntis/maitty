import { describe, expect, it, vi } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { WorkflowRun } from '@/data/preview'
import { toWorkflowView } from '@/features/workspace/model/workflows'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { WorkBoard } from './WorkBoard'

const report = (run: string, key: string | null, steps: WorkflowRun['steps']): WorkflowRun => ({
  workflow: 'kitchen', run, ticket: key === null ? null : { key, title: `Title of ${key}`, link: `https://plane.example/${key}` }, steps, reportedAt: 1,
})
const step = (name: string, state: WorkflowRun['steps'][number]['state'], helper: string | null = null, note: string | null = null) => ({ name, state, note, helper })

const runs = [
  report('001', 'MAITTY-42', [step('intake', 'passed'), step('build', 'working', 'MAITTY-42 builder', '3 of 5 files'), step('review', 'waiting')]),
  report('002', 'MAITTY-43', [step('intake', 'needs_you', null, 'approve the spec'), step('build', 'waiting'), step('review', 'waiting')]),
  report('003', 'MAITTY-40', [step('intake', 'passed'), step('build', 'passed'), step('review', 'passed')]),
].map(toWorkflowView)

const builder: SubagentView = { id: 'agent-1', name: 'Builder', kind: 'general-purpose', status: 'working', task: 'MAITTY-42 builder', latest: 'Editing src/export.ts', result: null, startedAt: null, updatedAt: 1 }

describe('WorkBoard', () => {
  it('puts each run in the column of its first step not passed, and folds the finished ones', async () => {
    const user = userEvent.setup()
    render(<WorkBoard runs={runs} helpers={[]} onOpenHelper={vi.fn()} />)
    const column = (name: string) => within(screen.getByRole('region', { name }))
    expect(column('intake').getByRole('button', { name: /^MAITTY-43, Title of MAITTY-43: intake, Needs you/ })).toBeInTheDocument()
    expect(column('build').getByRole('button', { name: /^MAITTY-42/ })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^MAITTY-40/ })).toBeNull()
    expect(screen.getByText('2 runs going')).toBeInTheDocument()
    expect(screen.getByText('needs you')).toBeInTheDocument()

    expect(screen.getByText('1 run done')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Show' }))
    expect(column('review').getByRole('button', { name: /^MAITTY-40/ })).toBeInTheDocument()
  })

  it('opens a run in a drawer with its steps, and a helper label in a read-only card', async () => {
    const user = userEvent.setup()
    const onOpenHelper = vi.fn()
    render(<WorkBoard runs={runs} helpers={[builder]} onOpenHelper={onOpenHelper} />)
    await user.click(screen.getByRole('button', { name: /^MAITTY-42/ }))
    const drawer = within(screen.getByRole('dialog', { name: 'Run MAITTY-42' }))
    expect(drawer.getByRole('button', { name: 'Open ticket' })).toBeInTheDocument()
    const steps = drawer.getAllByRole('listitem')
    expect(steps.map((row) => row.textContent)).toEqual([
      expect.stringContaining('intakePassed'),
      expect.stringContaining('buildWorking3 of 5 files'),
      expect.stringContaining('reviewWaiting'),
    ])

    await user.click(drawer.getByRole('button', { name: 'Helper MAITTY-42 builder' }))
    expect(await screen.findByText('Editing src/export.ts')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Open its chat' }))
    expect(onOpenHelper).toHaveBeenCalledWith('agent-1')
  })

  it('closes the drawer with Escape and with its close button', async () => {
    const user = userEvent.setup()
    render(<WorkBoard runs={runs} helpers={[]} onOpenHelper={vi.fn()} />)
    await user.click(screen.getByRole('button', { name: /^MAITTY-43/ }))
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog')).toBeNull()
    await user.click(screen.getByRole('button', { name: /^MAITTY-43/ }))
    await user.click(screen.getByRole('button', { name: 'Close run' }))
    expect(screen.queryByRole('dialog')).toBeNull()
  })
})
