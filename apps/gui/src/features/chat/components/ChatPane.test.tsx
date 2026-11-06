import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ChatPane } from './ChatPane'

vi.mock('./ChatSession', () => ({
  ChatSession: ({ hasTerminal }: { hasTerminal?: boolean }) => <p>chat session, terminal: {String(hasTerminal)}</p>,
}))

afterEach(cleanup)

const shell = { kind: 'shell' } as const
const noop = () => undefined

describe('the chat card of a shell pane', () => {
  it('keeps Start claude here, and also offers a Claude chat and a Codex chat', async () => {
    const onStartAgent = vi.fn()
    const onStartChat = vi.fn()
    render(<ChatPane availability={shell} process="shell" online onBackToTerminal={noop} onStartAgent={onStartAgent} onStartChat={onStartChat} />)
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'Start claude here' }))
    expect(onStartAgent).toHaveBeenCalledWith('claude')
    expect(onStartChat).not.toHaveBeenCalled()

    await user.click(screen.getByRole('button', { name: 'Start a Claude chat' }))
    expect(onStartChat).toHaveBeenCalledWith('claude', false)
    await user.click(screen.getByRole('button', { name: 'Start a Codex chat' }))
    expect(onStartChat).toHaveBeenCalledWith('codex', false)
    // The full-access switch is labelled, warns when on, and goes with the start.
    await user.click(screen.getByRole('checkbox', { name: 'Full access (skip permission prompts)' }))
    expect(screen.getByRole('note')).toHaveTextContent(/No sandbox and no approval prompts/)
    await user.click(screen.getByRole('button', { name: 'Start a Claude chat' }))
    expect(onStartChat).toHaveBeenLastCalledWith('claude', true)
    // Starting a chat does not type into the shell.
    expect(onStartAgent).toHaveBeenCalledTimes(1)
  })

  it('offers neither start while the daemon is offline', () => {
    render(<ChatPane availability={shell} process="shell" online={false} onBackToTerminal={noop} onStartAgent={noop} onStartChat={noop} />)
    for (const name of ['Start claude here', 'Start a Claude chat', 'Start a Codex chat']) {
      expect(screen.getByRole('button', { name })).toBeDisabled()
    }
  })

  it('does not offer a chat start where the pane gives no way to make one', () => {
    render(<ChatPane availability={shell} process="shell" online onBackToTerminal={noop} onStartAgent={noop} />)
    expect(screen.queryByRole('button', { name: /chat$/ })).not.toBeInTheDocument()
  })
})

describe('the chat of a ready session', () => {
  const ready = { kind: 'ready', agent: 'claude', link: { conversation: 44, generation: 1 } } as const

  it('tells the chat when the session has no terminal', () => {
    render(<ChatPane availability={ready} process="claude" online hasTerminal={false} onBackToTerminal={noop} onStartAgent={noop} />)
    expect(screen.getByText('chat session, terminal: false')).toBeInTheDocument()
  })

  it('assumes a terminal unless told otherwise', () => {
    render(<ChatPane availability={ready} process="claude" online onBackToTerminal={noop} onStartAgent={noop} />)
    expect(screen.getByText('chat session, terminal: true')).toBeInTheDocument()
  })
})

describe('the chat of a session whose agent has not reported in yet', () => {
  const starting = { kind: 'no-events', agent: 'claude' } as const

  it('is the chat with its message box at once, not a card that says there is nothing', () => {
    render(<ChatPane availability={starting} process="claude" online sessionId="s1" onBackToTerminal={noop} onStartAgent={noop} />)
    expect(screen.getByText('Starting claude…')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message claude' })).toBeEnabled()
    expect(screen.queryByText('Chat view is for agent sessions')).not.toBeInTheDocument()
  })

  it('keeps the old card for a pane that gave the chat no session to hold a message for', () => {
    render(<ChatPane availability={starting} process="claude" online onBackToTerminal={noop} onStartAgent={noop} />)
    expect(screen.getByText('Chat view is for agent sessions')).toBeInTheDocument()
  })
})
