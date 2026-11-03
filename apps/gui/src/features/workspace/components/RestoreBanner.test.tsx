import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { SessionView } from '../model/session'
import { RestoreBanner } from './RestoreBanner'

const session = { id: 't7', name: 'api', title: 'maitty', interrupted: { agent: 'codex', resume: 'codex resume --last' } } as unknown as SessionView

describe('RestoreBanner', () => {
  it('shows nothing when no agent was interrupted', () => {
    const { container } = render(<RestoreBanner sessions={[]} online onResume={vi.fn()} onDismiss={vi.fn()} />)
    expect(container.firstChild).toBeNull()
  })

  it('names the agent and the command, and resumes nothing until you click', () => {
    const onResume = vi.fn()
    render(<RestoreBanner sessions={[session]} online onResume={onResume} onDismiss={vi.fn()} />)
    expect(screen.getByText('codex resume --last')).toBeTruthy()
    expect(onResume).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Resume' }))
    expect(onResume).toHaveBeenCalledWith(session)
  })

  it('can be dismissed, and cannot resume while the daemon is unreachable', () => {
    const onDismiss = vi.fn()
    render(<RestoreBanner sessions={[session]} online={false} onResume={vi.fn()} onDismiss={onDismiss} />)
    expect(screen.getByRole('button', { name: 'Resume' }).hasAttribute('disabled')).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }))
    expect(onDismiss).toHaveBeenCalledWith('t7')
  })
})
