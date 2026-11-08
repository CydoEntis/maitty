import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SessionNotice } from './SessionNotice'

describe('session notice card', () => {
  it('shows the title, what it wants and when, and runs an action and goes away', async () => {
    const onDismiss = vi.fn()
    const onClick = vi.fn()
    render(<SessionNotice kind="needs" process="codex" status="needs" title="Review EmptyState" state="needs you" body="Wants to run $ git push" actions={[{ label: 'Allow', primary: true, onClick }, { label: 'Open session', onClick: vi.fn() }]} onDismiss={onDismiss} />)

    expect(screen.getByText('Review EmptyState')).toBeInTheDocument()
    expect(screen.getByText('needs you')).toBeInTheDocument()
    expect(screen.getByText('Wants to run $ git push')).toBeInTheDocument()
    expect(screen.getByText('just now')).toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Allow' }))
    expect(onClick).toHaveBeenCalledOnce()
    expect(onDismiss).toHaveBeenCalledOnce()
  })

  it('has a close that only closes the card', async () => {
    const onDismiss = vi.fn()
    const onClick = vi.fn()
    render(<SessionNotice kind="done" process="codex" status="done" title="Review EmptyState" state="finished" body="codex finished its work" actions={[{ label: 'Open session', onClick }]} countdownMs={7000} onDismiss={onDismiss} />)

    await userEvent.setup().click(screen.getByRole('button', { name: 'Dismiss notification' }))
    expect(onDismiss).toHaveBeenCalledOnce()
    expect(onClick).not.toHaveBeenCalled()
  })
})
