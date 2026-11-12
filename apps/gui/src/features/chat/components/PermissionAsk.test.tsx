import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { PermissionAsk } from '@/data/preview'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { ANSWER_KEYS } from '../permissionAsk'
import { PENDING_APPROVAL_NOTE, TYPING_ASKING_NOTE } from '../useChatComposer'
import { forgetChatFeeds } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
}))

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
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

const LINK = { conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }
const WRITE_ASK: PermissionAsk = { tool: 'Write', detail: 'src/notes.txt', answerable: true }

async function open(status: 'Blocked' | 'Idle', ask: PermissionAsk | null) {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u', 'User', 'earlier'))
  daemon.status = status
  fake.current = daemon
  const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  render(<ChatSession link={LINK} agent="claude" online typeInto={typeInto} ask={ask} />)
  await act(async () => { await vi.advanceTimersByTimeAsync(0) })
  return { typeInto, user }
}

describe('the chat of an agent stopped on a permission prompt', () => {
  it('says what the agent wants to do, and approves with the key Claude Code takes', async () => {
    const { typeInto, user } = await open('Blocked', WRITE_ASK)

    expect(screen.getByText('Wants to use Write')).toBeInTheDocument()
    expect(screen.getByText('src/notes.txt')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /^Yes, once/ }))

    expect(typeInto).toHaveBeenCalledExactlyOnceWith(ANSWER_KEYS.approve)
    expect(ANSWER_KEYS.approve).toBe('1')
  })

  it('refuses with Esc', async () => {
    const { typeInto, user } = await open('Blocked', WRITE_ASK)

    await user.click(screen.getByRole('button', { name: /^No/ }))

    expect(typeInto).toHaveBeenCalledExactlyOnceWith('\x1b')
  })

  it('points the composer at the card, which holds the answer', async () => {
    await open('Blocked', WRITE_ASK)

    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveAttribute('placeholder', PENDING_APPROVAL_NOTE)
  })

  it('offers no allow button for a prompt whose answer key is not known, only the keys to answer it in the terminal', async () => {
    const { typeInto } = await open('Blocked', { ...WRITE_ASK, answerable: false })

    expect(screen.getByText('Wants to use Write')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Yes, once/ })).not.toBeInTheDocument()
    expect(screen.getByRole('group', { name: "Keys for the agent's terminal" })).toBeInTheDocument()
    expect(typeInto).not.toHaveBeenCalled()
  })

  it('shows a question with its choices as a question, with no allow button that would pick for you', async () => {
    const { typeInto } = await open('Blocked', { tool: 'AskUserQuestion', detail: 'Which database? 1 SQLite · 2 Postgres', answerable: false })

    expect(screen.getByText('claude has a question for you')).toBeInTheDocument()
    expect(screen.getByText('Which database? 1 SQLite · 2 Postgres')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Yes, once/ })).not.toBeInTheDocument()
    expect(screen.getByRole('group', { name: "Keys for the agent's terminal" })).toBeInTheDocument()
    expect(typeInto).not.toHaveBeenCalled()
  })

  it('offers one button per choice of a question, and picks it with its number', async () => {
    const { typeInto, user } = await open('Blocked', { tool: 'AskUserQuestion', detail: 'Which database?', answerable: false, choices: ['SQLite', 'Postgres'] })

    expect(screen.getByText('Which database?')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Yes, once/ })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /Postgres/ }))

    expect(typeInto).toHaveBeenCalledExactlyOnceWith('2')
    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveAttribute('placeholder', PENDING_APPROVAL_NOTE)
  })

  it('shows each of several questions with its choices, and answers them all at once: a key each in order, then the one that submits', async () => {
    const questions = [
      { question: 'Which database?', choices: ['SQLite', 'Postgres', 'MySQL'] },
      { question: 'Which language?', choices: ['Rust', 'Go'] },
    ]
    const { typeInto, user } = await open('Blocked', { tool: 'AskUserQuestion', detail: 'Which database? 1 SQLite · 2 Postgres · 3 MySQL / Which language? 1 Rust · 2 Go', answerable: false, choices: [], questions })

    // Each question is its own group; the line that ran them together is gone, and so is "Answer in the terminal".
    expect(screen.getByRole('group', { name: 'Which database?' })).toBeInTheDocument()
    expect(screen.getByRole('group', { name: 'Which language?' })).toBeInTheDocument()
    expect(screen.queryByText(/1 SQLite · 2 Postgres/)).not.toBeInTheDocument()
    expect(screen.queryByText('Answer in the terminal.')).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveAttribute('placeholder', PENDING_APPROVAL_NOTE)

    // Nothing is typed until every question has an answer, so one can be changed first.
    const send = screen.getByRole('button', { name: 'Send answers' })
    expect(send).toBeDisabled()
    await user.click(screen.getByRole('radio', { name: 'SQLite' }))
    await user.click(screen.getByRole('radio', { name: 'Postgres' }))
    expect(send).toBeDisabled()
    expect(screen.getByText('1 more to answer.')).toBeInTheDocument()
    await user.click(screen.getByRole('radio', { name: 'Go' }))
    expect(typeInto).not.toHaveBeenCalled()

    await user.click(send)
    await act(async () => { await vi.advanceTimersByTimeAsync(2000) })

    expect(typeInto.mock.calls.map(([key]) => key)).toEqual(['2', '2', '1'])
  })

  it('still says the agent is waiting when its hook reported no question', async () => {
    await open('Blocked', null)

    expect(screen.getByText('claude is waiting for you')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Yes, once/ })).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message claude' })).toHaveAttribute('placeholder', TYPING_ASKING_NOTE)
  })

  it('shows no card while the agent is not stopped on a prompt', async () => {
    await open('Idle', WRITE_ASK)

    expect(screen.queryByText('Wants to use Write')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Yes, once/ })).not.toBeInTheDocument()
  })
})
