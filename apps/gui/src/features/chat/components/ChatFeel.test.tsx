import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { forgetFirstMessages } from '../firstMessage'
import { ACTIVE_POLL_MS, forgetChatFeeds, IDLE_POLL_MS } from '../useChatFeed'
import type { SessionStatus } from '@/features/workspace/model/session'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({ current: null as null | { read: (cursor: never) => Promise<unknown> } }))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
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

const LINK = { conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

function open(status: 'Idle' | 'Working' = 'Idle') {
  const daemon = new FakeDaemonHistory()
  daemon.status = status
  daemon.upsert(daemon.message('t1', 'u1', 'User', 'earlier'))
  fake.current = daemon
  const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  const element = (paneStatus?: SessionStatus) => <ChatSession link={LINK} agent="claude" online typeInto={typeInto} {...(paneStatus === undefined ? {} : { paneStatus })} />
  const view = render(element())
  return { daemon, typeInto, user, setPaneStatus: (paneStatus: SessionStatus) => { view.rerender(element(paneStatus)) } }
}

describe('how the chat feels while it waits', () => {
  it('shows the shape of a conversation while the history is first read, then the history', async () => {
    open()
    expect(screen.getByRole('status', { name: 'Reading the conversation' })).toBeInTheDocument()

    await settle()

    expect(screen.queryByRole('status', { name: 'Reading the conversation' })).not.toBeInTheDocument()
    expect(screen.getByText('earlier')).toBeInTheDocument()
  })

  it('draws a message at once when it is sent and says the agent is thinking, until the history has it', async () => {
    const { daemon, typeInto, user } = open()
    await settle()

    await user.type(screen.getByRole('textbox', { name: 'Message claude' }), 'hello there{Enter}')
    await settle()

    expect(typeInto).toHaveBeenCalledTimes(1)
    expect(screen.getByLabelText('Sending')).toHaveTextContent('hello there')
    expect(screen.getByText(/claude is thinking/)).toBeInTheDocument()

    // The agent writes its file: the real message replaces the drawn one, and there is only one.
    daemon.upsert(daemon.message('t1', 'u2', 'User', 'hello there'))
    await settle(IDLE_POLL_MS)

    expect(screen.queryByLabelText('Sending')).not.toBeInTheDocument()
    expect(screen.getAllByText('hello there')).toHaveLength(1)
  })

  it('stops saying the agent is thinking after a few seconds, when its own status does not, while the message stays', async () => {
    const { user } = open()
    await settle()
    await user.type(screen.getByRole('textbox', { name: 'Message claude' }), 'hello there{Enter}')
    await settle()
    expect(screen.getByText(/claude is thinking/)).toBeInTheDocument()

    await settle(4_100)

    expect(screen.queryByText(/claude is thinking/)).not.toBeInTheDocument()
    expect(screen.getByLabelText('Sending')).toHaveTextContent('hello there')
  })

  it('reads again as a turn ends, so the last words are not left to the slow idle poll', async () => {
    const { daemon } = open('Working')
    await settle()
    expect(screen.getByText(/claude is thinking/)).toBeInTheDocument()

    // The turn ends; the agent's last message is written a moment after the status changes.
    daemon.status = 'Idle'
    await settle(ACTIVE_POLL_MS + 1)
    daemon.upsert(daemon.message('t1', 'a1', 'Assistant', 'the answer'))
    await settle(700)

    expect(screen.getByText('the answer')).toBeInTheDocument()
  })

  it('shows the agent working as soon as the pane says so, before the conversation has been read again', async () => {
    const { daemon, setPaneStatus } = open('Idle')
    await settle()
    expect(screen.queryByText(/claude is thinking/)).not.toBeInTheDocument()

    // A turn started in the terminal: the pane's status is live, the chat's own read is still "idle".
    setPaneStatus('working')
    await settle()

    expect(screen.getByText(/claude is thinking/)).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveAttribute('placeholder', 'Queue a message for when claude finishes')

    // And the chat read the conversation at that moment, not at its next idle poll.
    daemon.upsert(daemon.message('t1', 'u3', 'User', 'asked in the terminal'))
    setPaneStatus('idle')
    await settle(700)
    expect(screen.getByText('asked in the terminal')).toBeInTheDocument()
    expect(screen.queryByText(/claude is thinking/)).not.toBeInTheDocument()
  })

  it('says under the message box how the agent is set up, from its own file', async () => {
    const { daemon } = open()
    daemon.meta = { mode: 'auto', model: 'claude-sonnet-5-5' }
    await settle(IDLE_POLL_MS)

    expect(screen.getByText('Auto')).toBeInTheDocument()
    expect(screen.getByText('Claude Sonnet 5.5')).toBeInTheDocument()
  })

  it('marks a mode that asks about nothing, and shows nothing when the file said nothing', async () => {
    const { daemon } = open()
    await settle()
    expect(screen.getByTitle('Not reported by the agent yet')).toHaveTextContent('Permissions')

    daemon.meta = { mode: 'bypassPermissions', model: null }
    await settle(IDLE_POLL_MS)

    const chip = screen.getByText('Bypass permissions')
    expect(chip.className).toContain('danger')
  })

  it('drops a message the history never showed, rather than leaving it standing as sent', async () => {
    const { user } = open()
    await settle()
    await user.type(screen.getByRole('textbox', { name: 'Message claude' }), 'lost words{Enter}')
    await settle()
    expect(screen.getByLabelText('Sending')).toBeInTheDocument()

    await settle(21_000)

    expect(screen.queryByLabelText('Sending')).not.toBeInTheDocument()
  })
})
