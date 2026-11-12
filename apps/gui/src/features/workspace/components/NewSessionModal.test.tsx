import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { NewSessionModal, type NewSessionRequest } from './NewSessionModal'

const PROJECTS = [
  { folder: '/code/api', name: 'api', color: 'var(--proj-1)' },
  { folder: '/code/web', name: 'web', color: 'var(--proj-2)' },
]

function open(more: Partial<Parameters<typeof NewSessionModal>[0]> = {}) {
  const onStart = vi.fn<(request: NewSessionRequest) => Promise<void>>(() => Promise.resolve())
  const onCancel = vi.fn()
  render(<NewSessionModal machine={0} machineName={null} projects={PROJECTS} defaultAgent="claude" browse={null} onStart={onStart} onCancel={onCancel} portalContainer={null} {...more} />)
  return { onStart, onCancel, user: userEvent.setup(), dialog: screen.getByRole('dialog', { name: /^New session/ }) }
}

describe('the New session dialog', () => {
  it('starts the default agent in the first project with its first message', async () => {
    const { onStart, user, dialog } = open()
    await user.type(within(dialog).getByRole('textbox', { name: 'First message' }), 'Add draft routes')
    await user.click(within(dialog).getByRole('button', { name: /^Start agent/ }))
    expect(onStart).toHaveBeenCalledWith({ machine: 0, machineName: 'this computer', folder: '/code/api', startAs: 'claude', message: 'Add draft routes' })
  })

  it('starts the agent and project picked, with Ctrl+Enter', async () => {
    const { onStart, user, dialog } = open()
    await user.click(within(dialog).getByRole('radio', { name: /Codex/ }))
    await user.click(within(dialog).getByRole('radio', { name: 'web' }))
    await user.keyboard('{Control>}{Enter}{/Control}')
    expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ folder: '/code/web', startAs: 'codex' }))
  })

  it('opens a plain shell, which takes no agent, place or message', async () => {
    const { onStart, user, dialog } = open()
    await user.click(within(dialog).getByRole('radio', { name: 'Shell' }))
    expect(within(dialog).queryByRole('radiogroup', { name: 'Agent' })).not.toBeInTheDocument()
    expect(within(dialog).queryByRole('textbox', { name: 'First message' })).not.toBeInTheDocument()
    await user.click(within(dialog).getByRole('button', { name: /^Open shell/ }))
    expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ startAs: 'terminal', message: '' }))
  })

  it('says it opens a terminal and starts the agent in it, with no worktree choice to make', async () => {
    const { user, dialog } = open({ machine: 2, machineName: 'devbox' })
    expect(within(dialog).getByText(/Opens a terminal on devbox and starts Claude Code in it/)).toBeInTheDocument()
    expect(within(dialog).queryByRole('radio', { name: 'New worktree' })).not.toBeInTheDocument()
    expect(within(dialog).queryByRole('radio', { name: 'Current branch' })).not.toBeInTheDocument()
    await user.click(within(dialog).getByRole('radio', { name: /Codex/ }))
    expect(within(dialog).getByText(/starts Codex in it/)).toBeInTheDocument()
  })

  it('offers another folder through the folder picker, on this computer only', async () => {
    const browse = vi.fn(() => Promise.resolve('/elsewhere/tool'))
    const { onStart, user, dialog } = open({ browse })
    await user.click(within(dialog).getByRole('button', { name: /Other folder/ }))
    await waitFor(() => { expect(within(dialog).getByRole('radio', { name: 'tool' })).toHaveAttribute('aria-checked', 'true') })
    await user.keyboard('{Control>}{Enter}{/Control}')
    expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ folder: '/elsewhere/tool' }))
  })

  it('names the machine it starts on when it is not this computer', () => {
    open({ machine: 2, machineName: 'devbox' })
    expect(screen.getByRole('dialog', { name: 'New session on devbox' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Other folder/ })).not.toBeInTheDocument()
  })

  it('cancels with its button', async () => {
    const { onCancel, user, dialog } = open()
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalled()
  })
})
