import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { UpdateSlot } from './UpdateSlot'
import type { UpdaterState } from './useUpdater'

function setup(state: UpdaterState, extra: { skipped?: string | null; variant?: 'footer' | 'rail' } = {}) {
  const actions = { onInstall: vi.fn(), onRestartNow: vi.fn(), onCheck: vi.fn(), onSkip: vi.fn() }
  render(<UpdateSlot variant={extra.variant ?? 'footer'} state={state} version="0.60.2" skipped={extra.skipped ?? null} portalContainer={null} {...actions} />)
  return actions
}

describe('update slot', () => {
  it('shows the version when there is nothing to do, and nothing in the rail', () => {
    setup({ status: 'current' })
    expect(screen.getByText(/^Version/)).toHaveTextContent('Version 0.60.2')
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })

  it('offers the rail no button without an update', () => {
    setup({ status: 'current' }, { variant: 'rail' })
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })

  it('offers an available update in a pill, with notes, and downloads on Download', async () => {
    const user = userEvent.setup()
    const actions = setup({ status: 'available', version: '0.61.0', notes: '- Faster start\n- Fewer flashes' })

    await user.click(screen.getByRole('button', { name: 'Update to 0.61.0' }))

    const card = screen.getByRole('dialog', { name: 'Update' })
    expect(card).toHaveTextContent('Maitty 0.61.0 is available')
    expect(card).toHaveTextContent('You have 0.60.2')
    expect(card).toHaveTextContent('Faster start')
    await user.click(screen.getByRole('button', { name: 'Download' }))
    expect(actions.onInstall).toHaveBeenCalledTimes(1)
  })

  it('remembers a skipped version and shows the plain version again', async () => {
    const user = userEvent.setup()
    const actions = setup({ status: 'available', version: '0.61.0', notes: '' })
    await user.click(screen.getByRole('button', { name: 'Update to 0.61.0' }))

    await user.click(screen.getByRole('button', { name: 'Skip this version' }))

    expect(actions.onSkip).toHaveBeenCalledWith('0.61.0')
  })

  it('shows no pill for a skipped version', () => {
    setup({ status: 'available', version: '0.61.0', notes: '' }, { skipped: '0.61.0' })
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
    expect(screen.getByText(/^Version/)).toBeInTheDocument()
  })

  it('shows the percent, the size and the time left while downloading, and closes the card with its X', async () => {
    const user = userEvent.setup()
    const MB = 1024 * 1024
    setup({ status: 'downloading', version: '0.61.0', progress: 0.42, transfer: { received: 42 * MB, total: 100 * MB, bytesPerSecond: MB / 6 } })

    await user.click(screen.getByRole('button', { name: '42%' }))
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '42')
    expect(screen.getByText('42.0 MB of 100 MB')).toBeInTheDocument()
    expect(screen.getByText('42% · about 6 min left')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Hide' })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Close' }))

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('restarts from the ready card and says sessions keep running', async () => {
    const user = userEvent.setup()
    const actions = setup({ status: 'ready', version: '0.61.0' })

    await user.click(screen.getByRole('button', { name: 'Restart to update' }))
    expect(screen.getByRole('dialog', { name: 'Update' })).toHaveTextContent('Your sessions keep running in maittyd')
    await user.click(screen.getByRole('button', { name: 'Restart now' }))

    expect(actions.onRestartNow).toHaveBeenCalledTimes(1)
  })

  it('shows the real error on failure and retries', async () => {
    const user = userEvent.setup()
    const actions = setup({ status: 'failed', message: 'signature mismatch', during: 'update' })

    await user.click(screen.getByRole('button', { name: 'Update failed' }))
    expect(screen.getByRole('dialog', { name: 'Update' })).toHaveTextContent('signature mismatch')
    await user.click(screen.getByRole('button', { name: 'Retry' }))

    expect(actions.onCheck).toHaveBeenCalledTimes(1)
  })

  it('shows no pill for a failed check, which you did not start', () => {
    setup({ status: 'failed', message: 'offline', during: 'check' })

    expect(screen.queryByRole('button')).not.toBeInTheDocument()
    expect(screen.getByText(/^Version/)).toBeInTheDocument()
  })

  it('closes the card with Escape', async () => {
    const user = userEvent.setup()
    setup({ status: 'ready', version: '0.61.0' })
    await user.click(screen.getByRole('button', { name: 'Restart to update' }))

    await user.keyboard('{Escape}')

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})
