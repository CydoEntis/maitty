import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { NO_EVENTS_HINT_AFTER_MS } from '@/features/workspace/model/chatView'
import { forgetFirstMessages, heldFirstMessage, holdStartChoice, takeCarriedDraft } from '../firstMessage'
import { StartingChat } from './StartingChat'

const attach = vi.hoisted(() => ({ attachLocalFile: vi.fn(), chooseFilesToAttach: vi.fn() }))
vi.mock('@/api/attachments', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/attachments')>()), ...attach }))
vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  loadAgentModels: () => Promise.resolve([]),
}))

beforeEach(() => { forgetFirstMessages() })
afterEach(() => { cleanup(); vi.useRealTimers() })

const noop = () => undefined
const open = () => render(<StartingChat agent="codex" sessionId="s1" onBackToTerminal={noop} />)

describe('the chat of a session that is still starting', () => {
  it('is there at once, with a message box for the agent that was chosen', () => {
    open()
    expect(screen.getByText(/Type below: it is sent as soon as codex is ready/)).toBeInTheDocument()
    expect(screen.queryByText('Starting codex…')).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message codex' })).toBeEnabled()
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
  })

  it('holds a file attached now with the message, and shows it as a chip, not as the block that names it', async () => {
    attach.chooseFilesToAttach.mockResolvedValue(['/home/me/errors.log'])
    attach.attachLocalFile.mockResolvedValue({ path: '/data/attachments/attach-1-errors.log', name: 'errors.log', size: 120, kind: 'file' })
    const user = userEvent.setup()
    open()
    await user.click(screen.getByRole('button', { name: 'Attach files' }))
    expect(await screen.findByText('errors.log')).toBeInTheDocument()
    await user.type(screen.getByRole('textbox', { name: 'Message codex' }), 'why does this fail?{Enter}')

    expect(heldFirstMessage('s1')).toContain('/data/attachments/attach-1-errors.log')
    expect(screen.getByLabelText('Sending')).toHaveTextContent('why does this fail?')
    expect(screen.getByLabelText('Sending')).not.toHaveTextContent('Attached files (kept by Maitty')
    expect(screen.getByLabelText('Sending')).toHaveTextContent('errors.log')
  })

  it('holds a message sent now and shows it in the conversation, where it stays when the chat takes over', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('textbox', { name: 'Message codex' }), 'fix the build{Enter}')

    expect(heldFirstMessage('s1')).toBe('fix the build')
    expect(screen.queryByRole('list', { name: 'Queued messages' })).not.toBeInTheDocument()
    expect(screen.getByLabelText('Sending')).toHaveTextContent('fix the build')
    expect(screen.getByText('codex is starting…')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message codex' })).toHaveValue('')
  })

  it('carries a half-typed message over to the chat that replaces it', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('textbox', { name: 'Message codex' }), 'half a thou')

    expect(takeCarriedDraft('s1')).toBe('half a thou')
  })

  it('draws no chips for an agent whose model and mode cannot be changed from the box', () => {
    open()
    expect(screen.queryByRole('button', { name: /^Agent, model and reasoning/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Permission mode/ })).not.toBeInTheDocument()
  })

  it('draws the agent name where its model will be said, so that is never added later', () => {
    open()
    expect(document.querySelector('.mx-model-button.is-static')).toHaveTextContent('Codex')
  })

  it('says the agent is waiting in the terminal at once when the daemon says it needs you, without blaming the hooks', () => {
    render(<StartingChat agent="claude" sessionId="s1" paneStatus="needs" onBackToTerminal={noop} />)
    expect(screen.getByRole('note')).toHaveTextContent(/waiting for you in the terminal.*trust this folder/)
    expect(screen.queryByRole('button', { name: /^Connect/ })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Open terminal' })).toBeInTheDocument()
  })

  it('shows a question on the agent’s screen with its choices, and answers it by moving the cursor and pressing Enter', async () => {
    const user = userEvent.setup()
    const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    const asking = 'Quick safety check: do you trust this folder?\n❯ 1. No, exit\n2. Yes, I trust this folder'
    render(<StartingChat agent="claude" sessionId="s1" paneStatus="needs" asking={asking} typeInto={typeInto} onBackToTerminal={noop} />)
    expect(screen.getByText(/do you trust this folder/)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Yes, I trust this folder' }))
    expect(typeInto.mock.calls.map(([keys]) => keys)).toEqual([String.fromCharCode(27) + '[B', String.fromCharCode(13)])
  })

  it('floats the hooks hint over the conversation, so it moves nothing when it comes or goes', () => {
    vi.useFakeTimers()
    open()
    act(() => { vi.advanceTimersByTime(NO_EVENTS_HINT_AFTER_MS) })
    expect(screen.getByRole('note').closest('.mx-chat')).toHaveClass('is-starting')
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

describe('the chips of a Claude chat that is still starting', () => {
  const openClaude = () => render(<StartingChat agent="claude" sessionId="s1" onBackToTerminal={noop} />)

  it('draws the model, effort and mode that were picked, greyed out, where the real chat will have them', () => {
    holdStartChoice('s1', { model: 'opus', reasoning: 'high', mode: 'plan' })
    openClaude()

    const model = screen.getByRole('button', { name: /^Agent, model and reasoning: Claude · Opus · High/ })
    expect(model).toHaveAttribute('aria-disabled', 'true')
    expect(model).toHaveAttribute('title', expect.stringMatching(/starting/))
    expect(screen.getByRole('button', { name: /^Permission mode: Plan/ })).toHaveAttribute('aria-disabled', 'true')
  })

  it('opens no picker while it is locked', async () => {
    const user = userEvent.setup()
    openClaude()
    await user.click(screen.getByRole('button', { name: /^Agent, model and reasoning/ }))

    expect(screen.queryByRole('radio')).not.toBeInTheDocument()
  })

  it('replaces the send arrow with a working indicator once a message is held, and keeps that message', async () => {
    const user = userEvent.setup()
    openClaude()
    expect(screen.getByRole('button', { name: 'Send message' })).toBeInTheDocument()
    await user.type(screen.getByRole('textbox', { name: 'Message claude' }), 'hello{Enter}')

    expect(screen.getByRole('button', { name: 'claude is working' })).toBeDisabled()
    expect(screen.queryByRole('button', { name: 'Send message' })).not.toBeInTheDocument()
    await user.type(screen.getByRole('textbox', { name: 'Message claude' }), 'and another{Enter}')
    expect(heldFirstMessage('s1')).toBe('hello')
  })
})
