import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { SubagentChat } from './SubagentChat'

const readHelper = vi.fn()
vi.mock('@/api/daemon', () => ({ readHelper: (...args: unknown[]) => readHelper(...args) as unknown }))

const subagent = (over: Partial<SubagentView> = {}): SubagentView => ({
  id: 'a1b2',
  name: 'count files',
  kind: 'general-purpose',
  status: 'done',
  task: 'count the files',
  latest: null,
  result: 'There are 2.',
  startedAt: null,
  updatedAt: 0,
  ...over,
})

const message = (item: string, role: 'User' | 'Assistant', text: string) => ({
  source: { conversation: 0, turn: 't1', item },
  content: { Message: { role, text } },
  state: 'Completed',
})

beforeEach(() => {
  readHelper.mockReset()
})

describe("a subagent's own chat", () => {
  it('shows the steps from its own record for a pane on this computer', async () => {
    readHelper.mockResolvedValue({ term: 4, helper: 'a1b2', entries: [message('u', 'User', 'count the files'), message('a', 'Assistant', 'Ran ls: a.txt and b.txt')], start_missing: true })
    render(<SubagentChat subagent={subagent()} agent="claude" term={4} />)
    expect(await screen.findByText('Ran ls: a.txt and b.txt')).toBeInTheDocument()
    expect(screen.getByText('Its earlier steps are not shown.')).toBeInTheDocument()
    expect(readHelper).toHaveBeenCalledWith(4, 'a1b2')
  })

  it('keeps what the runtime reported when the record cannot be read, and asks nothing for another machine', async () => {
    readHelper.mockRejectedValue(new Error('the service is older'))
    const { unmount } = render(<SubagentChat subagent={subagent()} agent="codex" term={4} />)
    expect(await screen.findByText('There are 2.')).toBeInTheDocument()
    unmount()
    readHelper.mockClear()
    render(<SubagentChat subagent={subagent()} agent="codex" term={null} />)
    expect(screen.getByText('count the files')).toBeInTheDocument()
    expect(readHelper).not.toHaveBeenCalled()
  })
})
