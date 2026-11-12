import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

const api = vi.hoisted(() => ({ switchBranch: vi.fn(() => Promise.resolve()) }))
vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  switchBranch: api.switchBranch,
  folderBranches: () => Promise.resolve({ current: 'task-1', branches: [{ name: 'task-1', when: 5 }, { name: 'main', when: 4 }, { name: 'dev', when: 3 }], worktrees: [{ path: '/w/dev', branch: 'dev', main: false, when: 3 }] }),
}))

import { BranchMenu } from './BranchMenu'

function menu(onNewWorktree?: () => void, blocked?: string) {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <BranchMenu spec={{ folder: '/w/task', ...(onNewWorktree === undefined ? {} : { onNewWorktree }), ...(blocked === undefined ? {} : { newWorktreeBlocked: blocked }) }} branch="task-1" worktree={false} />
    </QueryClientProvider>,
  )
}

describe('the tray branch menu', () => {
  it('lists the branches with the current one ticked, and switches to another', async () => {
    menu()
    await userEvent.click(screen.getByRole('button', { name: /This chat works on/ }))
    expect(await screen.findByText('Switching changes where the agent\'s next edits go. Work already done stays on its branch.')).toBeInTheDocument()
    await userEvent.click(await screen.findByRole('radio', { name: /main/ }))
    expect(api.switchBranch).toHaveBeenCalledWith('/w/task', 'main', false)
  })

  it('labels a branch that has a worktree, and offers a new worktree', async () => {
    const onNewWorktree = vi.fn()
    menu(onNewWorktree)
    await userEvent.click(screen.getByRole('button', { name: /This chat works on/ }))
    expect(await screen.findByText('worktree')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Move to a new worktree/ }))
    expect(onNewWorktree).toHaveBeenCalled()
  })

  it('keeps the move row off, with the reason, while the move is not possible', async () => {
    menu(vi.fn(), 'Wait until the agent is idle; moving restarts it')
    await userEvent.click(screen.getByRole('button', { name: /This chat works on/ }))
    const row = await screen.findByRole('button', { name: /Move to a new worktree/ })
    expect(row).toBeDisabled()
    expect(row).toHaveAttribute('title', 'Wait until the agent is idle; moving restarts it')
  })
})
