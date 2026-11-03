import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SessionNotice } from './SessionNotice'

describe('session notice card', () => {
  it('shows the app, the title and the body, and runs an action and goes away', async () => {
    const onDismiss = vi.fn()
    const onClick = vi.fn()
    render(<SessionNotice kind="needs" color="#3ad" title="Needs you · maitty" body="codex in (root): $ git push" actions={[{ label: 'Allow once', primary: true, onClick }, { label: 'Open', onClick: vi.fn() }]} onDismiss={onDismiss} />)

    expect(screen.getByText('MAITTY')).toBeInTheDocument()
    expect(screen.getByText('Needs you · maitty')).toBeInTheDocument()
    expect(screen.getByText('codex in (root): $ git push')).toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Allow once' }))
    expect(onClick).toHaveBeenCalledOnce()
    expect(onDismiss).toHaveBeenCalledOnce()
  })

  it('always has a Dismiss that only closes the card', async () => {
    const onDismiss = vi.fn()
    const onClick = vi.fn()
    render(<SessionNotice kind="done" color="#3ad" title="Finished · maitty" body="codex in (root) finished" actions={[{ label: 'Open', onClick }]} countdownMs={7000} onDismiss={onDismiss} />)

    await userEvent.setup().click(screen.getByRole('button', { name: 'Dismiss' }))
    expect(onDismiss).toHaveBeenCalledOnce()
    expect(onClick).not.toHaveBeenCalled()
  })
})
