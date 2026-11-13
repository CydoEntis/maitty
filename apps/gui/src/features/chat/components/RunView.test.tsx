import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { toWorkflowView } from '@/features/workspace/model/workflows'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { RunView } from './RunView'

const builder: SubagentView = { id: 'a1', name: 'MAITTY-42 builder', kind: 'builder', status: 'working', task: 'MAITTY-42 builder', latest: null, result: null, startedAt: null, updatedAt: 0 }

describe("a workflow run's steps", () => {
  it('shows each step with its state and note, marks the current one, and opens the helper doing it', async () => {
    const user = userEvent.setup()
    const onOpenHelper = vi.fn()
    const run = toWorkflowView({
      workflow: 'kitchen',
      run: '001',
      ticket: { key: 'MAITTY-42', title: 'Add export', link: null },
      steps: [
        { name: 'checks', state: 'failed', note: '2 tests failed', helper: null },
        { name: 'build', state: 'sent_back', note: null, helper: 'MAITTY-42 builder' },
        { name: 'review', state: 'waiting', note: null, helper: 'MAITTY-42 reviewer' },
      ],
      reportedAt: 1,
    })
    render(<RunView run={run} helpers={[builder]} onOpenHelper={onOpenHelper} />)
    expect(screen.getByRole('img', { name: 'Failed' })).toHaveTextContent('✕')
    expect(screen.getByText('2 tests failed')).toBeInTheDocument()
    expect(screen.getByText('build').closest('li')).toHaveAttribute('aria-current', 'step')
    await user.click(screen.getByRole('button', { name: 'MAITTY-42 builder' }))
    expect(onOpenHelper).toHaveBeenCalledWith('a1')
    expect(screen.queryByRole('button', { name: 'MAITTY-42 reviewer' })).toBeNull()
    expect(screen.getByText('MAITTY-42 reviewer')).toBeInTheDocument()
  })
})
