import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { NewAgentDialog, type NewAgentRequest } from './NewAgentDialog'
import type { AgentProject } from './newAgent'

const PROJECTS: AgentProject[] = [
  { key: '/home/dev/code/app', name: 'app', path: '/home/dev/code/app' },
  { key: '/home/dev/code/api', name: 'api', path: '/home/dev/code/api' },
]

function open(overrides: { initialKey?: string | null; onStart?: (request: NewAgentRequest) => Promise<void>; onCancel?: () => void } = {}) {
  const onStart = overrides.onStart ?? vi.fn(() => Promise.resolve())
  const onCancel = overrides.onCancel ?? vi.fn()
  render(<NewAgentDialog projects={PROJECTS} initialKey={overrides.initialKey ?? null} home="/home/dev" container={null} onStart={onStart} onCancel={onCancel} />)
  return { onStart, onCancel, user: userEvent.setup() }
}

const branchField = () => screen.getByRole('textbox', { name: 'Branch' })
const promptField = () => screen.getByRole('textbox', { name: 'First prompt' })
const startButton = () => screen.getByRole('button', { name: /^Start/ })

describe('the New agent on a branch dialog', () => {
  it('opens on the focused project, with claude chosen and Start waiting for a branch', () => {
    open({ initialKey: '/home/dev/code/api' })
    expect(screen.getByRole('radio', { name: 'api' })).toBeChecked()
    expect(screen.getByRole('radio', { name: 'Claude' })).toBeChecked()
    expect(startButton()).toBeDisabled()
  })

  it('offers a chip for each project and the claude / codex choice', () => {
    open()
    expect(screen.getAllByRole('radio').map((chip) => chip.textContent)).toEqual(['app', 'api', 'Claude', 'Codex'])
  })

  it('fills the branch from the prompt and shows where the worktree goes', async () => {
    const { user } = open()
    await user.type(promptField(), 'Fix the login bug')
    expect(branchField()).toHaveValue('agent/fix-login-bug')
    expect(screen.getByText('~/code/app-worktrees/agent-fix-login-bug')).toBeInTheDocument()
  })

  it('stops following the prompt once the branch is edited', async () => {
    const { user } = open()
    await user.type(promptField(), 'Fix the login bug')
    await user.clear(branchField())
    await user.type(branchField(), 'my/branch')
    await user.type(promptField(), ' and more words here')
    expect(branchField()).toHaveValue('my/branch')
  })

  it('marks a branch that will not do, and keeps Start off until it is fixed', async () => {
    const { user } = open()
    await user.type(promptField(), 'Fix login')
    await user.clear(branchField())
    await user.type(branchField(), 'bad name')
    expect(branchField()).toHaveAttribute('aria-invalid', 'true')
    expect(screen.getByRole('alert')).toHaveTextContent(/letters, digits/i)
    expect(startButton()).toBeDisabled()
  })

  it('refuses a prompt that starts like an option', async () => {
    const { user } = open()
    await user.type(promptField(), '--yolo')
    await user.type(branchField(), 'agent/x')
    expect(screen.getByRole('alert')).toHaveTextContent(/dash/)
    expect(startButton()).toBeDisabled()
  })

  it('starts with Ctrl+Enter, sending the project, agent, trimmed prompt and branch', async () => {
    const { user, onStart } = open({ initialKey: '/home/dev/code/api' })
    await user.click(screen.getByRole('radio', { name: 'Codex' }))
    await user.type(promptField(), '  Fix the login bug  ')
    fireEvent.keyDown(promptField(), { key: 'Enter', ctrlKey: true })
    await waitFor(() => { expect(onStart).toHaveBeenCalledTimes(1) })
    expect(onStart).toHaveBeenCalledWith({ project: PROJECTS[1], agent: 'codex', prompt: 'Fix the login bug', branch: 'agent/fix-login-bug' })
  })

  it('does not start with Ctrl+Enter while the branch is missing or invalid', () => {
    const { onStart } = open()
    fireEvent.keyDown(promptField(), { key: 'Enter', ctrlKey: true })
    expect(onStart).not.toHaveBeenCalled()
  })

  it('starts from the Start button too, with no prompt when a branch is typed', async () => {
    const { user, onStart } = open()
    await user.type(branchField(), 'agent/scratch')
    await user.click(startButton())
    await waitFor(() => { expect(onStart).toHaveBeenCalledWith({ project: PROJECTS[0], agent: 'claude', prompt: '', branch: 'agent/scratch' }) })
  })

  it('shows what went wrong and stays open for another try', async () => {
    const onStart = vi.fn(() => Promise.reject(new Error('agent/x already exists')))
    const { user } = open({ onStart })
    await user.type(branchField(), 'agent/x')
    await user.click(startButton())
    expect(await screen.findByText('agent/x already exists')).toBeInTheDocument()
    expect(startButton()).toBeEnabled()
  })

  it('cancels with the Cancel button', async () => {
    const { user, onCancel } = open()
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalled()
  })
})
