import { act, cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { READ_ONLY_NOTE } from '../useChatComposer'
import { forgetChatFeeds } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
  cancelTurn: vi.fn<(conversation: { id: string; generation: number }) => Promise<void>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  submitMessage: fake.submitMessage,
  cancelTurn: fake.cancelTurn,
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
}))

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  fake.submitMessage.mockReset()
  fake.cancelTurn.mockReset()
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

describe('a chat that follows an agent started in the terminal (ADR 0052)', () => {
  it('shows the history, turns the composer off with the reason, and offers no Stop while the agent works', async () => {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message('t1', 'u', 'User', 'earlier'))
    daemon.status = 'Working'
    fake.current = daemon

    render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }} agent="claude" online />)
    await act(async () => { await vi.advanceTimersByTimeAsync(0) })

    expect(screen.getByText('earlier')).toBeInTheDocument()
    const box = screen.getByRole('textbox', { name: 'Message claude' })
    expect(box).toBeDisabled()
    expect(box).toHaveAttribute('placeholder', READ_ONLY_NOTE)
    expect(screen.queryByRole('button', { name: 'Stop claude' })).not.toBeInTheDocument()
    expect(fake.submitMessage).not.toHaveBeenCalled()
    expect(fake.cancelTurn).not.toHaveBeenCalled()
  })
})
