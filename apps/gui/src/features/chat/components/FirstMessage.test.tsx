import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { carryDraft, entrustFirstMessage, forgetFirstMessages, heldFirstMessage, holdFirstMessage } from '../firstMessage'
import { PASTE_END, PASTE_START } from '../useChatComposer'
import { ACTIVE_POLL_MS, IDLE_POLL_MS, forgetChatFeeds } from '../useChatFeed'
import { chatFindOf } from '../chatSearch'
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

function open(status: 'Idle' | 'Working' | 'None') {
  const daemon = new FakeDaemonHistory()
  daemon.status = status
  fake.current = daemon
  const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
  render(<ChatSession link={LINK} agent="claude" online typeInto={typeInto} sessionId="s1" />)
  return { daemon, typeInto }
}

describe('a message typed while the agent was starting', () => {
  it('goes out once the chat can send, exactly once, and is no longer held', async () => {
    holdFirstMessage('s1', 'hello there')
    const { typeInto } = open('Idle')
    await settle()
    await settle(ACTIVE_POLL_MS)

    expect(typeInto).toHaveBeenCalledExactlyOnceWith(`${PASTE_START}hello there${PASTE_END}\r`)
    expect(heldFirstMessage('s1')).toBeNull()
  })

  it('waits, shown as your message with the agent at work on it, while the agent is busy, and goes when it is ready', async () => {
    holdFirstMessage('s1', 'hello there')
    const { daemon, typeInto } = open('Working')
    await settle()

    expect(typeInto).not.toHaveBeenCalled()
    // The same bubble and working line the starting chat showed, never the empty-chat note.
    expect(screen.getByLabelText('Sending')).toHaveTextContent('hello there')
    expect(screen.queryByText(/No messages yet/)).not.toBeInTheDocument()

    daemon.status = 'Idle'
    await settle(ACTIVE_POLL_MS + 1)
    expect(typeInto).toHaveBeenCalledExactlyOnceWith(`${PASTE_START}hello there${PASTE_END}\r`)
  })

  it('keeps the held message on screen while the history is first read, with no skeleton in between', () => {
    holdFirstMessage('s1', 'hello there')
    open('Working')

    // Before anything has been read: the frame the person sees when the real chat takes over from the starting one.
    expect(screen.getByLabelText('Sending')).toHaveTextContent('hello there')
    expect(screen.queryByRole('status', { name: 'Reading the conversation' })).not.toBeInTheDocument()
  })

  it('brings a half-typed message over into the box', async () => {
    carryDraft('s1', 'half a thou')
    open('Idle')
    await settle()

    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveValue('half a thou')
  })
})

describe('a first message the daemon holds', () => {
  it('is typed in by the daemon and not by the window, and its stand-in goes once the history has it', async () => {
    holdFirstMessage('s1', 'hello there')
    entrustFirstMessage('s1', 'hello there', () => Promise.resolve())
    const { daemon, typeInto } = open('Idle')
    await settle()
    await settle(ACTIVE_POLL_MS)

    expect(typeInto).not.toHaveBeenCalled()
    expect(screen.getByLabelText('Sending')).toHaveTextContent('hello there')

    daemon.upsert(daemon.message('t1', 'u1', 'User', 'hello there'))
    await settle(IDLE_POLL_MS + 1)

    expect(heldFirstMessage('s1')).toBeNull()
    expect(typeInto).not.toHaveBeenCalled()
  })

  it('is sent by the window when the daemon does not take it', async () => {
    holdFirstMessage('s1', 'hello there')
    entrustFirstMessage('s1', 'hello there', () => Promise.reject(new Error('no pane 1')))
    const { typeInto } = open('Idle')
    await settle()
    await settle(ACTIVE_POLL_MS)

    expect(typeInto).toHaveBeenCalledExactlyOnceWith(`${PASTE_START}hello there${PASTE_END}\r`)
    expect(heldFirstMessage('s1')).toBeNull()
  })

  it('is sent by the window for a session the daemon cannot type into', async () => {
    holdFirstMessage('s1', 'hello there')
    entrustFirstMessage('s1', 'hello there', () => { throw new Error('Only a terminal session holds a message for its agent') })
    const { typeInto } = open('Idle')
    await settle()
    await settle(ACTIVE_POLL_MS)

    expect(typeInto).toHaveBeenCalledOnce()
  })
})

describe('a chat whose agent has not said what it is doing yet', () => {
  it('takes a message at once, holds it, and types it in when the agent is ready', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    const { daemon, typeInto } = open('None')
    await settle()
    const box = screen.getByRole('textbox', { name: 'Message claude' })
    expect(box).not.toBeDisabled()

    await user.type(box, 'start on the parser{Enter}')
    expect(typeInto).not.toHaveBeenCalled()
    expect(screen.getByRole('list', { name: 'Queued messages' })).toHaveTextContent('start on the parser')

    daemon.status = 'Idle'
    await settle(IDLE_POLL_MS + 1)
    await settle(10)
    expect(typeInto).toHaveBeenCalledExactlyOnceWith(`${PASTE_START}start on the parser${PASTE_END}\r`)
  })

  it('puts the cursor in the message box when nothing else has it', async () => {
    open('Idle')
    await settle()

    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveFocus()
  })

  it('leaves the cursor where it is when something else has it', async () => {
    const other = document.createElement('input')
    document.body.append(other)
    other.focus()
    open('Idle')
    await settle()

    expect(other).toHaveFocus()
    other.remove()
  })
})

describe('finding text in the chat a session shows', () => {
  it('marks the row that holds the text, and unmarks it when the search is cleared', async () => {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message('t1', 'u1', 'User', 'look at the parser'))
    daemon.upsert(daemon.message('t1', 'a1', 'Assistant', 'It drops the last row.'))
    fake.current = daemon
    render(<ChatSession link={LINK} agent="claude" online sessionId="s1" />)
    await settle()
    const find = chatFindOf('s1')

    let hit = false
    act(() => { hit = find?.find('PARSER', 'next') ?? false })
    expect(hit).toBe(true)
    expect(document.querySelector('[data-found="true"]')).toHaveTextContent('look at the parser')

    act(() => { hit = find?.find('lexer', 'next') ?? true })
    expect(hit).toBe(false)

    act(() => { find?.clear() })
    expect(document.querySelector('[data-found="true"]')).toBeNull()
  })
})

describe('a message written while a terminal agent works', () => {
  it('is held in the queue, and typed in once the agent is ready, exactly once', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    const { daemon, typeInto } = open('Working')
    await settle()

    await user.type(screen.getByRole('textbox', { name: 'Message claude' }), 'and then add tests')
    await user.click(screen.getByRole('button', { name: 'Queue message' }))
    expect(typeInto).not.toHaveBeenCalled()
    expect(screen.getByRole('list', { name: 'Queued messages' })).toHaveTextContent('and then add tests')

    daemon.status = 'Idle'
    await settle(ACTIVE_POLL_MS + 1)
    await settle(10)
    expect(typeInto).toHaveBeenCalledExactlyOnceWith(`${PASTE_START}and then add tests${PASTE_END}\r`)
    expect(screen.queryByRole('list', { name: 'Queued messages' })).not.toBeInTheDocument()
  })
})
