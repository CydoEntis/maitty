import { act, cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { carryDraft, forgetFirstMessages, heldFirstMessage, holdFirstMessage } from '../firstMessage'
import { PASTE_END, PASTE_START } from '../useChatComposer'
import { ACTIVE_POLL_MS, forgetChatFeeds } from '../useChatFeed'
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

function open(status: 'Idle' | 'Working') {
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

  it('waits, shown as waiting, while the agent is busy, and goes when it is ready', async () => {
    holdFirstMessage('s1', 'hello there')
    const { daemon, typeInto } = open('Working')
    await settle()

    expect(typeInto).not.toHaveBeenCalled()
    expect(screen.getByRole('list', { name: 'Queued messages' })).toHaveTextContent('hello there')

    daemon.status = 'Idle'
    await settle(ACTIVE_POLL_MS + 1)
    expect(typeInto).toHaveBeenCalledExactlyOnceWith(`${PASTE_START}hello there${PASTE_END}\r`)
  })

  it('brings a half-typed message over into the box', async () => {
    carryDraft('s1', 'half a thou')
    open('Idle')
    await settle()

    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveValue('half a thou')
  })
})
