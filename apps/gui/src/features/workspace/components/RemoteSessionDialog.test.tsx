import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { RemoteSessionDialog } from './RemoteSessionDialog'

describe('a new session on another machine', () => {
  it('asks for a folder there, starting at ~, and starts in the one typed', async () => {
    const user = userEvent.setup()
    const onStart = vi.fn().mockResolvedValue(undefined)
    render(<RemoteSessionDialog machineName="build" onStart={onStart} onCancel={vi.fn()} />)
    const folder = screen.getByRole('textbox', { name: 'Folder on build' })
    expect(folder).toHaveValue('~')
    await user.clear(folder)
    await user.type(folder, '/srv/app{Enter}')
    expect(onStart).toHaveBeenCalledWith('/srv/app')
  })

  it('shows why a machine could not start it and stays open', async () => {
    const user = userEvent.setup()
    const onStart = vi.fn().mockRejectedValue(new Error('build is not connected'))
    render(<RemoteSessionDialog machineName="build" onStart={onStart} onCancel={vi.fn()} />)
    await user.click(screen.getByRole('button', { name: 'Start' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('build is not connected')
    expect(screen.getByRole('dialog', { name: 'New session on build' })).toBeInTheDocument()
  })

  it('starts nothing without a folder, and Esc cancels', async () => {
    const user = userEvent.setup()
    const onStart = vi.fn()
    const onCancel = vi.fn()
    render(<RemoteSessionDialog machineName="build" onStart={onStart} onCancel={onCancel} />)
    await user.clear(screen.getByRole('textbox', { name: 'Folder on build' }))
    expect(screen.getByRole('button', { name: 'Start' })).toBeDisabled()
    await user.keyboard('{Escape}')
    expect(onCancel).toHaveBeenCalled()
    expect(onStart).not.toHaveBeenCalled()
  })
})
