import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { UpdateBanner } from './UpdateBanner'
import type { UpdaterState } from './useUpdater'

function show(state: UpdaterState, dismissed = false) {
  const actions = { onInstall: vi.fn(), onRestart: vi.fn(), onDismiss: vi.fn() }
  render(<UpdateBanner state={state} dismissed={dismissed} {...actions} />)
  return actions
}

describe('update banner', () => {
  it('offers a newer version with what changed, and installs only when asked', async () => {
    const actions = show({ status: 'available', version: '0.59.0', notes: '- New: a status bar\n- Fixed: lint' })
    expect(screen.getByText('Maitty 0.59.0')).toBeInTheDocument()
    expect(screen.getByText('New: a status bar')).toBeInTheDocument()
    expect(actions.onInstall).not.toHaveBeenCalled()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Update' }))
    expect(actions.onInstall).toHaveBeenCalledOnce()
  })

  it('puts the offer away for the run when you choose Later', async () => {
    const actions = show({ status: 'available', version: '0.59.0', notes: '' })
    await userEvent.setup().click(screen.getByRole('button', { name: 'Later' }))
    expect(actions.onDismiss).toHaveBeenCalledOnce()
  })

  it('stays hidden once dismissed', () => {
    show({ status: 'available', version: '0.59.0', notes: '' }, true)
    expect(screen.queryByRole('region', { name: 'Update available' })).not.toBeInTheDocument()
  })

  it('shows download progress, even after the offer was dismissed', () => {
    show({ status: 'downloading', version: '0.59.0', progress: 0.42 }, true)
    expect(screen.getByRole('status', { name: 'Updating' })).toHaveTextContent('Downloading Maitty 0.59.0… 42%')
  })

  it('says the window restart keeps sessions, and restarts only when clicked', async () => {
    const actions = show({ status: 'ready', version: '0.59.0' })
    expect(screen.getByText(/never ends your sessions/)).toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Restart now' }))
    expect(actions.onRestart).toHaveBeenCalledOnce()
  })

  it.each<UpdaterState>([{ status: 'idle' }, { status: 'checking' }, { status: 'current' }, { status: 'failed', message: 'offline' }])(
    'shows nothing while $status',
    (state) => {
      const { container } = render(<UpdateBanner state={state} dismissed={false} onInstall={vi.fn()} onRestart={vi.fn()} onDismiss={vi.fn()} />)
      expect(container).toBeEmptyDOMElement()
    },
  )
})
