import { act, cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { DELIVERY_GRACE_MS, PENDING_APPROVAL_NOTE } from '../useChatComposer'
import { forgetChatFeeds, IDLE_POLL_MS } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
  cancelTurn: vi.fn<(conversation: { id: string; generation: number }) => Promise<void>>(),
  approveRequest: vi.fn<(conversation: { id: string; generation: number }, requestId: number | string, decision: string) => Promise<void>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  submitMessage: fake.submitMessage,
  cancelTurn: fake.cancelTurn,
  approveRequest: fake.approveRequest,
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
}))

const LINK = { conversation: FAKE_CONVERSATION, generation: 1 }
const TARGET = { id: String(FAKE_CONVERSATION), generation: 1 }

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  fake.submitMessage.mockReset()
  fake.cancelTurn.mockReset()
  fake.approveRequest.mockReset()
  fake.submitMessage.mockResolvedValue({})
  fake.cancelTurn.mockResolvedValue(undefined)
  fake.approveRequest.mockResolvedValue(undefined)
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

async function open(setup: (daemon: FakeDaemonHistory) => void = () => undefined, online = true) {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u', 'User', 'earlier'))
  setup(daemon)
  fake.current = daemon
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  render(<ChatSession link={LINK} agent="claude" online={online} />)
  await settle()
  return { daemon, user, box: screen.getByRole('textbox', { name: 'Message claude' }) }
}

describe('sending', () => {
  it('sends on Enter through the conversation with the next operation number, and clears the box', async () => {
    const { user, box } = await open()

    await user.type(box, 'run the tests{Enter}')

    expect(fake.submitMessage).toHaveBeenCalledTimes(1)
    expect(fake.submitMessage).toHaveBeenCalledWith(TARGET, 1, 'run the tests')
    expect(box).toHaveValue('')
  })

  it('adds a new line on Shift+Enter and sends nothing', async () => {
    const { user, box } = await open()

    await user.type(box, 'one{Shift>}{Enter}{/Shift}two')

    expect(box).toHaveValue('one\ntwo')
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('does not send an empty or blank message', async () => {
    const { user, box } = await open()

    await user.type(box, '   {Enter}')

    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('keeps the message when the send fails and retries it with the same operation number', async () => {
    const { user, box } = await open()
    fake.submitMessage.mockRejectedValueOnce(new Error('the daemon went away'))

    await user.type(box, 'please retry{Enter}')
    expect(await screen.findByRole('alert')).toHaveTextContent('the daemon went away')
    expect(box).toHaveValue('please retry')

    await user.type(box, '{Enter}')
    expect(fake.submitMessage).toHaveBeenNthCalledWith(1, TARGET, 1, 'please retry')
    expect(fake.submitMessage).toHaveBeenNthCalledWith(2, TARGET, 1, 'please retry')
    expect(box).toHaveValue('')
  })

  it('is off, with the reason, while the daemon is offline', async () => {
    const { box } = await open(() => undefined, false)

    expect(box).toBeDisabled()
    expect(box).toHaveAttribute('placeholder', 'The daemon is offline')
  })
})

describe('while a request waits for an answer', () => {
  const pending = [{ request_id: 9, kind: 'Command', turn_id: 't1', command: 'rm -rf build', choices: ['Accept', 'AcceptForSession', 'Decline'] }]

  it('turns the composer off and says to answer the request first', async () => {
    const { box } = await open((daemon) => { daemon.status = 'Blocked'; daemon.approvals = pending })

    expect(box).toBeDisabled()
    expect(box).toHaveAttribute('placeholder', PENDING_APPROVAL_NOTE)
    expect(screen.getByRole('group', { name: /Approval/ })).toHaveTextContent('rm -rf build')
  })

  it('answers through the daemon by request id, then shows the choice and turns the composer back on', async () => {
    const { daemon, user, box } = await open((setup) => { setup.status = 'Blocked'; setup.approvals = pending })

    await user.click(screen.getByRole('button', { name: 'This session (a)' }))

    await settle()
    expect(fake.approveRequest).toHaveBeenCalledWith(TARGET, 9, 'AcceptForSession')
    const card = screen.getByRole('group', { name: /Approval/ })
    expect(card).toHaveTextContent('You answered: This session')
    expect(within(card).queryByRole('button')).not.toBeInTheDocument()
    daemon.status = 'Working'
    daemon.approvals = []
    await settle(IDLE_POLL_MS)
    expect(box).not.toBeDisabled()
  })
})

describe('while the agent works', () => {
  it('queues a message instead of sending it, and says so', async () => {
    const { user, box } = await open((daemon) => { daemon.status = 'Working' })

    expect(box).toHaveAttribute('placeholder', 'Queue a message for when claude finishes')
    await user.type(box, 'and then lint it{Enter}')

    expect(fake.submitMessage).not.toHaveBeenCalled()
    const queued = screen.getByRole('list', { name: 'Queued messages' })
    expect(queued).toHaveTextContent('and then lint it')
    expect(queued).toHaveTextContent('Queued')
  })

  it('removes a queued message on request', async () => {
    const { user, box } = await open((daemon) => { daemon.status = 'Working' })
    await user.type(box, 'never mind{Enter}')

    await user.click(screen.getByRole('button', { name: /Remove/ }))

    expect(screen.queryByRole('list', { name: 'Queued messages' })).not.toBeInTheDocument()
  })

  it('delivers the queue in order, once each, after the turn ends', async () => {
    const { daemon, user, box } = await open((setup) => { setup.status = 'Working' })
    await user.type(box, 'first{Enter}')
    await user.type(box, 'second{Enter}')
    expect(fake.submitMessage).not.toHaveBeenCalled()

    daemon.status = 'Idle'
    await settle(IDLE_POLL_MS)
    expect(fake.submitMessage).toHaveBeenCalledTimes(1)
    expect(fake.submitMessage).toHaveBeenLastCalledWith(TARGET, 1, 'first')

    // The second waits out the grace period after the first, and does not go while the turn it started runs.
    daemon.status = 'Working'
    await settle(DELIVERY_GRACE_MS + IDLE_POLL_MS)
    expect(fake.submitMessage).toHaveBeenCalledTimes(1)
    daemon.status = 'Idle'
    await settle(IDLE_POLL_MS + DELIVERY_GRACE_MS)
    expect(fake.submitMessage).toHaveBeenCalledTimes(2)
    expect(fake.submitMessage).toHaveBeenLastCalledWith(TARGET, 2, 'second')
    expect(screen.queryByRole('list', { name: 'Queued messages' })).not.toBeInTheDocument()
  })

  it('interrupts with Stop and with Esc', async () => {
    const { user, box } = await open((daemon) => { daemon.status = 'Working' })

    await user.click(screen.getByRole('button', { name: 'Stop claude' }))
    expect(fake.cancelTurn).toHaveBeenCalledWith(TARGET)

    await user.click(box)
    await user.keyboard('{Escape}')
    expect(fake.cancelTurn).toHaveBeenCalledTimes(2)
  })

  it('does not treat Esc as an interrupt when nothing is running', async () => {
    const { user, box } = await open()

    await user.click(box)
    await user.keyboard('{Escape}')

    expect(fake.cancelTurn).not.toHaveBeenCalled()
  })
})
