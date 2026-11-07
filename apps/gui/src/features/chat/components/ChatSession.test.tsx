import { act, cleanup, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { ACTIVE_POLL_MS, forgetChatFeeds, IDLE_POLL_MS } from '../useChatFeed'
import { forgetComposers } from '../composerStore'
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

// jsdom lays nothing out: give the list a height and a scroll position that holds what is set.
const scrolled = new WeakMap<Element, number>()
const VIEWPORT_PX = 600

beforeEach(() => {
  vi.useFakeTimers()
  forgetChatFeeds()
  forgetComposers()
  fake.submitMessage.mockReset()
  fake.cancelTurn.mockReset()
  fake.approveRequest.mockReset()
  fake.submitMessage.mockResolvedValue({})
  fake.cancelTurn.mockResolvedValue(undefined)
  fake.approveRequest.mockResolvedValue(undefined)
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => VIEWPORT_PX })
  Object.defineProperty(HTMLElement.prototype, 'scrollTop', {
    configurable: true,
    get(this: Element) { return scrolled.get(this) ?? 0 },
    set(this: Element, value: number) { scrolled.set(this, value) },
  })
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

const link = (generation = 1) => ({ conversation: FAKE_CONVERSATION, generation })

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

function conversation(count: number): FakeDaemonHistory {
  const daemon = new FakeDaemonHistory(128)
  for (let index = 0; index < count; index += 1) {
    daemon.upsert(daemon.message('t1', `m${String(index)}`, index % 2 === 0 ? 'User' : 'Assistant', `message ${String(index)}`))
  }
  fake.current = daemon
  return daemon
}

describe('the chat history', () => {
  it('draws only the rows near the view for a history of 1,500 items, ending at the newest', async () => {
    conversation(1500)
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    await settle()

    const list = screen.getByRole('region', { name: 'Conversation with claude' })
    // 1,500 items are in the conversation; only a screenful and its overscan are in the page.
    expect(list.querySelectorAll('[data-key]').length).toBeLessThan(40)
    expect(within(list).getByText('message 1499')).toBeInTheDocument()
    expect(within(list).queryByText('message 0')).not.toBeInTheDocument()
  })

  it('keys rows by the event id, so a re-read leaves the same rows', async () => {
    conversation(3)
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    const keys = () => Array.from(document.querySelectorAll('[data-key]')).map((row) => row.getAttribute('data-key'))
    const before = keys()

    await settle(IDLE_POLL_MS)

    expect(before).toEqual(['t1:m0', 't1:m1', 't1:m2'])
    expect(keys()).toEqual(before)
  })

  it('follows new messages on its next read, with no duplicates', async () => {
    const daemon = conversation(2)
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    expect(screen.getByText('message 1')).toBeInTheDocument()

    daemon.upsert(daemon.message('t1', 'm2', 'Assistant', 'a new reply'))
    await settle(IDLE_POLL_MS)

    expect(screen.getAllByText('a new reply')).toHaveLength(1)
    expect(screen.getAllByText('message 1')).toHaveLength(1)
  })

  it('polls faster while the agent works', async () => {
    const daemon = conversation(2)
    daemon.status = 'Working'
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    daemon.reads.length = 0

    await settle(ACTIVE_POLL_MS * 3)

    expect(daemon.reads.length).toBeGreaterThanOrEqual(3)
  })

  it('shows the working line and the daemon status while a turn runs', async () => {
    const daemon = conversation(2)
    daemon.status = 'Working'
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()

    expect(screen.getAllByText(/claude is working/).length).toBeGreaterThan(0)
  })

  it('says it is reconnecting when a read fails, keeps what it showed, and resumes without duplicates', async () => {
    const daemon = conversation(2)
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)

    daemon.failure = new Error('maittyd is not running')
    await settle(IDLE_POLL_MS)
    expect(screen.getByText(/Reconnecting to the daemon/)).toBeInTheDocument()
    // The reason is on screen, so a window that cannot read its chat says why instead of only waiting.
    expect(screen.getByText(/maittyd is not running/)).toBeInTheDocument()
    expect(screen.getByText('message 1')).toBeInTheDocument()

    daemon.failure = null
    daemon.upsert(daemon.message('t1', 'm2', 'User', 'sent while it was down'))
    await settle(IDLE_POLL_MS)

    expect(screen.queryByText(/Reconnecting to the daemon/)).not.toBeInTheDocument()
    expect(screen.queryByText(/maittyd is not running/)).not.toBeInTheDocument()
    expect(screen.getAllByText('message 0')).toHaveLength(1)
    expect(screen.getAllByText('sent while it was down')).toHaveLength(1)
    // The warning names the failure, never the conversation's text.
    expect(warn).toHaveBeenCalledWith('Could not read the conversation', 'maittyd is not running')
    warn.mockRestore()
  })

  it('shows a fresh conversation when the generation changes', async () => {
    const daemon = conversation(2)
    const view = render(<ChatSession link={link(1)} agent="claude" online />)
    await settle()
    expect(screen.getByText('message 1')).toBeInTheDocument()

    daemon.restart()
    daemon.upsert(daemon.message('t9', 'n0', 'User', 'a different conversation'))
    view.rerender(<ChatSession link={link(2)} agent="claude" online />)
    await settle()

    expect(screen.getByText('a different conversation')).toBeInTheDocument()
    expect(screen.queryByText('message 1')).not.toBeInTheDocument()
  })

  it('reopens at once from what it already read, without a loading state', async () => {
    conversation(2)
    const first = render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    first.unmount()

    render(<ChatSession link={link()} agent="claude" online />)

    expect(screen.queryByText('Reading the conversation…')).not.toBeInTheDocument()
    expect(screen.getByText('message 1')).toBeInTheDocument()
  })

  it('announces a change of state once, without reading the conversation out', async () => {
    const daemon = conversation(2)
    daemon.status = 'Working'
    render(<ChatSession link={link()} agent="codex" online />)
    await settle()

    const announcer = screen.getAllByRole('status').find((node) => node.getAttribute('aria-live') === 'polite')
    expect(announcer).toHaveTextContent('codex is working')
    expect(screen.getByRole('region', { name: 'Conversation with codex' })).not.toHaveAttribute('aria-live')
  })

  it('shows an item the history cannot describe as unavailable, not as an invented diff', async () => {
    const daemon = conversation(0)
    daemon.upsert(daemon.message('t1', 'u', 'User', 'edit it'))
    daemon.upsert({ source: { conversation: FAKE_CONVERSATION, turn: 't1', item: 'e' }, content: { Tool: { kind: 'commandExecution', label: 'Edit ', output: '' } }, state: 'Completed' })
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()

    expect(screen.getByText(/did not keep this change/)).toBeInTheDocument()
  })
})

describe('a subagent’s own chat', () => {
  const subagent = { id: 'b', name: 'Write drafts tests', kind: 'Explore', status: 'done' as const, task: 'Write drafts tests for the routes.', latest: null, result: 'Wrote 12 tests.\n\nThey all pass.', startedAt: 100, updatedAt: 170 }

  it('shows what it was asked and its report under a bar that names it, instead of the session’s chat', async () => {
    conversation(2)
    render(<ChatSession link={link()} agent="claude" online subagent={subagent} />)
    await settle()

    expect(screen.getAllByRole('status').some((node) => node.textContent.startsWith('Subagent'))).toBe(true)
    expect(screen.getByText('Write drafts tests', { selector: 'strong' })).toBeInTheDocument()
    expect(screen.getByText('· done in 1m 10s')).toBeInTheDocument()
    const log = screen.getByRole('log', { name: 'Subagent Write drafts tests' })
    expect(within(log).getByText('Write drafts tests for the routes.')).toBeInTheDocument()
    expect(within(log).getByText('Wrote 12 tests.')).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Conversation with claude' })).not.toBeInTheDocument()
  })

  it('goes back to the main session from its bar', async () => {
    conversation(2)
    const onBackToMain = vi.fn()
    render(<ChatSession link={link()} agent="claude" online subagent={subagent} onBackToMain={onBackToMain} />)
    await settle()

    act(() => { screen.getByRole('button', { name: 'Back to main session' }).click() })
    expect(onBackToMain).toHaveBeenCalled()
  })
})
