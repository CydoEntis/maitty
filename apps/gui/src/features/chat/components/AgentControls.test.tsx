import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { forgetFirstMessages } from '../firstMessage'
import { forgetChatFeeds, IDLE_POLL_MS } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({ current: null as null | { read: (cursor: never) => Promise<unknown> } }))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
  loadTerminalScreen: () => Promise.resolve('> hi\n\n  dev · Sonnet 5.5\n  ⏵⏵ accept edits on (shift+tab to cycle)\n'),
  loadAgentModels: () => Promise.resolve([{ id: 'claude-opus-5-5', label: 'Opus 5.5', blurb: 'The most capable' }, { id: 'claude-sonnet-5-5', label: 'Sonnet 5.5', blurb: 'Fast' }]),
}))

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  forgetFirstMessages()
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

function open(agent: 'claude' | 'codex', status: 'Idle' | 'Working' = 'Idle') {
  const daemon = new FakeDaemonHistory()
  daemon.status = status
  daemon.meta = { mode: 'auto', model: 'claude-sonnet-5-5' }
  fake.current = daemon
  const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
  render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }} agent={agent} online typeInto={typeInto} sessionId="s1" />)
  return typeInto
}

describe('changing the mode, the model and the reasoning of a chat that follows the terminal', () => {
  it('moves the agent to the mode picked from the dropdown, with its own Shift+Tab', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    const typeInto = open('claude')
    await settle(IDLE_POLL_MS)

    await user.click(screen.getByRole('button', { name: /^Permission mode: .* Pick another/ }))
    await user.click(await screen.findByRole('menuitemradio', { name: /Accept edits/ }))
    await settle(1000)
    // The fake screen already shows "accept edits", so no key is needed; the chip shows it.
    expect(typeInto).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: /^Permission mode: Accept edits/ })).toBeInTheDocument()
  })

  it('sends /model and /effort when a model and a reasoning level are picked', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    const typeInto = open('claude')
    await settle(IDLE_POLL_MS)

    await user.click(screen.getByRole('button', { name: /^Agent, model and reasoning/ }))
    await user.click(await screen.findByRole('radio', { name: /Opus 5.5/ }))
    expect(typeInto).toHaveBeenLastCalledWith('/model claude-opus-5-5\r')
    // The chip shows the picked model at once.
    expect(screen.getByRole('button', { name: /^Agent, model and reasoning/ })).toHaveTextContent('Opus')
    await user.click(screen.getByRole('radio', { name: 'High' }))
    expect(typeInto).toHaveBeenLastCalledWith('/effort high\r')
  })

  it('stays display-only for another agent, and while the agent is working', async () => {
    open('codex')
    await settle(IDLE_POLL_MS)
    expect(screen.queryByRole('button', { name: /^Permission mode/ })).not.toBeInTheDocument()
    cleanup()
    open('claude', 'Working')
    await settle(IDLE_POLL_MS)
    expect(screen.queryByRole('button', { name: /^Permission mode/ })).not.toBeInTheDocument()
  })
})
