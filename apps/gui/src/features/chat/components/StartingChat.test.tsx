import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { NO_EVENTS_HINT_AFTER_MS } from '@/features/workspace/model/chatView'
import { forgetFirstMessages, heldFirstMessage, takeCarriedDraft } from '../firstMessage'
import { StartingChat } from './StartingChat'

beforeEach(() => { forgetFirstMessages() })
afterEach(() => { cleanup(); vi.useRealTimers() })

const noop = () => undefined
const open = () => render(<StartingChat agent="codex" sessionId="s1" onBackToTerminal={noop} />)

describe('the chat of a session that is still starting', () => {
  it('is there at once, with a message box for the agent that was chosen', () => {
    open()
    expect(screen.getByText('Starting codex…')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message codex' })).toBeEnabled()
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
  })

  it('holds a message sent now, shows it as waiting, and lets it be taken back', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('textbox', { name: 'Message codex' }), 'fix the build{Enter}')

    expect(heldFirstMessage('s1')).toBe('fix the build')
    expect(screen.getByRole('list', { name: 'Queued messages' })).toHaveTextContent('fix the build')
    expect(screen.getByRole('textbox', { name: 'Message codex' })).toHaveValue('')

    await user.click(screen.getByRole('button', { name: /Remove/ }))
    expect(heldFirstMessage('s1')).toBeNull()
  })

  it('carries a half-typed message over to the chat that replaces it', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('textbox', { name: 'Message codex' }), 'half a thou')

    expect(takeCarriedDraft('s1')).toBe('half a thou')
  })

  it('suggests the hooks only when it is still empty after a while', () => {
    vi.useFakeTimers()
    open()
    act(() => { vi.advanceTimersByTime(NO_EVENTS_HINT_AFTER_MS - 1) })
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
    act(() => { vi.advanceTimersByTime(1) })
    expect(screen.getByRole('note')).toHaveTextContent('maitty integrate codex')
  })
})
