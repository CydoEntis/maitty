import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { SubagentActivity } from '@/data/preview'
import { toSubagentView } from '@/features/workspace/model/subagents'
import { RunningSubagents } from './RunningSubagents'

afterEach(cleanup)

const child = (id: string, task: string, state: SubagentActivity['state'], latest: string | null = null) =>
  toSubagentView({ id, name: 'Explore', state, task, startedAt: null, updatedAt: 0, latest, result: null })

describe('the running subagents bar', () => {
  it('appears while one runs: the count, the first one’s task and activity, and how many more', () => {
    render(<RunningSubagents subagents={[child('a', 'Find call sites', 'Working', 'Grep refundable'), child('b', 'Check limits', 'Working'), child('c', 'Old one', 'Completed')]} onOpen={null} />)

    const bar = screen.getByRole('region', { name: '2 subagents running' })
    expect(bar).toHaveTextContent('Find call sites · Grep refundable')
    expect(within(bar).getByText('+1 more')).toBeInTheDocument()
    expect(bar).not.toHaveTextContent('Old one')
  })

  it('clears itself once none runs', () => {
    const { rerender } = render(<RunningSubagents subagents={[child('a', 'Find call sites', 'Working')]} onOpen={null} />)
    expect(screen.getByRole('region', { name: '1 subagent running' })).toBeInTheDocument()

    rerender(<RunningSubagents subagents={[child('a', 'Find call sites', 'Completed'), child('b', 'Gone', 'Cancelled')]} onOpen={null} />)
    expect(screen.queryByRole('region')).toBeNull()
  })

  it('opens to a row each, and a row unfolds what it was asked and what it ran, with a way to its chat', async () => {
    const user = userEvent.setup()
    const onOpen = vi.fn()
    render(<RunningSubagents subagents={[child('a', 'Find call sites', 'Working', 'Grep refundable'), child('b', 'Check limits', 'AwaitingApproval')]} onOpen={onOpen} />)

    await user.click(screen.getByRole('button', { name: 'Show' }))
    expect(screen.getByRole('button', { name: 'Hide' })).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText('Waiting for an answer in the terminal')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /Find call sites/ }))
    expect(screen.getByText('Asked to')).toBeInTheDocument()
    expect(screen.getByText('Ran')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Open its chat' }))
    expect(onOpen).toHaveBeenCalledWith('a')
  })
})
