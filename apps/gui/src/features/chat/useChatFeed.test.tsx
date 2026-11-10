import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from './fakeDaemonHistory'
import { ACTIVE_POLL_MS, forgetChatFeeds, NEWS_READ_GAP_MS, TOLD_ACTIVE_POLL_MS, useChatFeed } from './useChatFeed'

const fake = vi.hoisted(() => ({
  history: null as null | { read: (cursor: never) => Promise<unknown> },
  told: new Set<(conversation: number) => void>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.history === null) throw new Error('no fake daemon')
    return fake.history.read(cursor as never)
  },
}))

vi.mock('@/api/conversationNews', () => ({
  listenConversationChanged: (onChange: (conversation: number) => void) => {
    fake.told.add(onChange)
    return Promise.resolve(() => { fake.told.delete(onChange) })
  },
}))

const daemonSays = (conversation: number) => { for (const listener of fake.told) listener(conversation) }

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

function working(): FakeDaemonHistory {
  const history = new FakeDaemonHistory()
  history.upsert(history.message('t1', 'u1', 'User', 'fix the parser'))
  history.status = 'Working'
  fake.history = history
  return history
}

const open = () => renderHook(() => useChatFeed({ conversation: FAKE_CONVERSATION, generation: 1, provider: 'codex' }))

beforeEach(() => {
  vi.useFakeTimers()
  forgetChatFeeds()
  fake.told.clear()
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.history = null
})

describe('a chat the daemon tells of changes', () => {
  it('reads when the daemon says its conversation changed, without waiting for the next poll', async () => {
    const history = working()
    open()
    await settle()
    const before = history.reads.length

    daemonSays(FAKE_CONVERSATION)
    await settle(NEWS_READ_GAP_MS)

    expect(history.reads.length).toBe(before + 1)
  })

  it('does not read for another conversation', async () => {
    const history = working()
    open()
    await settle()
    const before = history.reads.length

    daemonSays(FAKE_CONVERSATION + 1)
    await settle(NEWS_READ_GAP_MS)

    expect(history.reads.length).toBe(before)
  })

  it('reads once for a burst of changes', async () => {
    const history = working()
    open()
    await settle()
    const before = history.reads.length

    for (let change = 0; change < 20; change += 1) daemonSays(FAKE_CONVERSATION)
    await settle(NEWS_READ_GAP_MS)

    expect(history.reads.length).toBe(before + 1)
  })

  it('asks on its own less often once the daemon has told it of a change', async () => {
    const history = working()
    open()
    await settle()
    daemonSays(FAKE_CONVERSATION)
    await settle(NEWS_READ_GAP_MS)
    const before = history.reads.length

    await settle(ACTIVE_POLL_MS * 2)
    expect(history.reads.length).toBe(before)

    await settle(TOLD_ACTIVE_POLL_MS)
    expect(history.reads.length).toBe(before + 1)
  })

  it('keeps asking as before while the daemon has said nothing', async () => {
    const history = working()
    open()
    await settle()
    const before = history.reads.length

    await settle(ACTIVE_POLL_MS)

    expect(history.reads.length).toBe(before + 1)
  })

  it('stops listening when the chat closes', async () => {
    working()
    const chat = open()
    await settle()
    expect(fake.told.size).toBe(1)

    chat.unmount()
    await settle()

    expect(fake.told.size).toBe(0)
  })
})
