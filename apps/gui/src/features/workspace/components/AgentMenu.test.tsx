import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { AgentMenu } from './AgentMenu'

afterEach(cleanup)

describe('the agent menu next to the new-session button', () => {
  it('offers a Claude chat and a Codex chat and marks the default', async () => {
    const user = userEvent.setup()
    render(<AgentMenu disabled={false} defaultAgent="codex" onPick={vi.fn()} />)

    await user.click(screen.getByRole('button', { name: 'Choose an agent for a new session' }))

    expect(screen.getByRole('menuitem', { name: 'New Claude chat' })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: 'New Codex chat (default)' })).toBeInTheDocument()
  })

  it('starts the chosen agent, whatever the default is', async () => {
    const user = userEvent.setup()
    const onPick = vi.fn()
    render(<AgentMenu disabled={false} defaultAgent="claude" onPick={onPick} />)

    await user.click(screen.getByRole('button', { name: 'Choose an agent for a new session' }))
    await user.click(screen.getByRole('menuitem', { name: 'New Codex chat' }))

    expect(onPick).toHaveBeenCalledExactlyOnceWith('codex')
  })

  it('is off while nothing can be started', () => {
    render(<AgentMenu disabled defaultAgent="claude" onPick={vi.fn()} />)
    expect(screen.getByRole('button', { name: 'Choose an agent for a new session' })).toBeDisabled()
  })
})
