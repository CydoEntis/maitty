import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetChatFeeds, IDLE_POLL_MS } from '../useChatFeed'
import { forgetComposers } from '../composerStore'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  approveRequest: vi.fn<(conversation: { id: string; generation: number }, requestId: number | string, decision: string) => Promise<void>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  submitMessage: vi.fn().mockResolvedValue({}),
  cancelTurn: vi.fn().mockResolvedValue(undefined),
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
  fake.approveRequest.mockReset()
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

const link = () => ({ conversation: FAKE_CONVERSATION, generation: 1 })

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

type Entry = Record<string, unknown>
const toolEntry = (turn: string, item: string, label: string, output: string, input?: unknown, extra: Record<string, unknown> = {}): Entry => ({
  source: { conversation: FAKE_CONVERSATION, turn, item },
  content: { Tool: { kind: 'commandExecution', label, output, ...(input === undefined ? {} : { input }), ...extra } },
  state: 'Completed',
})

/** Every kind of row the chat draws, in a repeating order, so measured heights differ row to row. */
function mixedConversation(count: number): FakeDaemonHistory {
  const daemon = new FakeDaemonHistory(128)
  for (let index = 0; index < count; index += 1) {
    const turn = `t${String(Math.floor(index / 7))}`
    const item = `i${String(index)}`
    switch (index % 7) {
      case 0: daemon.upsert(daemon.message(turn, item, 'User', `question ${String(index)}`)); break
      case 1: daemon.upsert(daemon.message(turn, item, 'Assistant', `Done ${String(index)}. Here is **why**.\n\n\`\`\`sh\nls\n\nls -la\n\`\`\``)); break
      case 2: daemon.upsert(toolEntry(turn, item, 'pnpm test', 'a\nb\nc\nd\ne\nf', undefined, { exit_code: 0, duration_ms: 900 })); break
      case 3: daemon.upsert(toolEntry(turn, item, 'Edit ', '', { file_path: `/repo/src/f${String(index)}.ts`, old_string: 'a\nb', new_string: 'c\nd\ne' })); break
      case 4: daemon.upsert(toolEntry(turn, item, 'TodoWrite ', '', { todos: [{ content: 'one', status: 'completed' }, { content: 'two', status: 'in_progress' }] })); break
      case 5: daemon.upsert(toolEntry(turn, item, 'Task ', 'report', { description: `Helper ${String(index)}`, prompt: 'go', subagent_type: 'Explore' })); break
      default: daemon.upsert(toolEntry(turn, item, 'Read ', 'text', { file_path: '/repo/README.md' }))
    }
  }
  fake.current = daemon
  return daemon
}

describe('a long mixed conversation', () => {
  it('draws only the rows near the view for 1,500 mixed items, ending at the newest', async () => {
    mixedConversation(1500)
    render(<ChatSession link={link()} agent="claude" online root="/repo" />)
    await settle()
    await settle()

    const list = screen.getByRole('region', { name: 'Conversation with claude' })
    expect(list.querySelectorAll('[data-key]').length).toBeLessThan(40)
    // The last item (index 1499, 1499 % 7 = 1) is an agent message with a code block.
    expect(within(list).getByText(/Done 1499/)).toBeInTheDocument()
    expect(within(list).queryByText('question 0')).not.toBeInTheDocument()
  })

  it('keeps every row keyed by a stable id when the same history is read again', async () => {
    mixedConversation(40)
    render(<ChatSession link={link()} agent="claude" online root="/repo" />)
    await settle()
    const keys = () => Array.from(document.querySelectorAll('[data-key]')).map((row) => row.getAttribute('data-key'))
    const before = keys()

    await settle(IDLE_POLL_MS)

    expect(before.length).toBeGreaterThan(0)
    expect(keys()).toEqual(before)
  })
})

describe('approvals in the chat', () => {
  const request = (id: string, over: Record<string, unknown> = {}) => ({
    request_id: id,
    kind: 'Command',
    turn_id: 't1',
    item_id: 'step',
    command: 'rm -rf build',
    reason: null,
    changes: [],
    choices: ['Accept', 'AcceptForSession', 'Decline'],
    ...over,
  })

  function withStep(): FakeDaemonHistory {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message('t1', 'u', 'User', 'clean it'))
    daemon.upsert(toolEntry('t1', 'step', 'rm -rf build', ''))
    fake.current = daemon
    return daemon
  }

  it('answers by request id when y is pressed on the focused card', async () => {
    const daemon = withStep()
    daemon.approvals = [request('req-1')]
    daemon.status = 'Blocked'
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()

    screen.getByRole('group', { name: /Approval/ }).focus()
    fireEvent.keyDown(document.activeElement as Element, { key: 'y' })
    await settle()

    expect(fake.approveRequest).toHaveBeenCalledExactlyOnceWith({ id: String(FAKE_CONVERSATION), generation: 1 }, 'req-1', 'Accept')
  })

  it('shows an approval answered in the terminal as answered, with the choice the daemon kept', async () => {
    const daemon = withStep()
    daemon.answeredApprovals = [{ request: request('req-2'), decision: 'AcceptForSession', answered_at_ms: Date.UTC(2026, 10, 4, 10, 11) }]
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()

    const card = screen.getByRole('group', { name: /Approval/ })
    expect(card).toHaveTextContent('You answered: This session')
    expect(within(card).queryByRole('button', { name: /This session/ })).toBeNull()
  })

  it('moves a request that disappears from the pending list to answered once the daemon records it', async () => {
    const daemon = withStep()
    daemon.approvals = [request('req-3')]
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    expect(screen.getByRole('button', { name: 'No (n)' })).toBeInTheDocument()

    daemon.approvals = []
    daemon.answeredApprovals = [{ request: request('req-3'), decision: 'Decline' }]
    daemon.upsert(daemon.message('t1', 'u2', 'User', 'ping'))
    await settle(IDLE_POLL_MS)

    expect(screen.getByRole('group', { name: /Approval/ })).toHaveTextContent('You answered: No')
    expect(screen.queryByRole('button', { name: 'No (n)' })).toBeNull()
  })

  it('links the request to its tool step and scrolls the list there', async () => {
    const daemon = withStep()
    for (let index = 0; index < 60; index += 1) daemon.upsert(daemon.message('t1', `m${String(index)}`, 'Assistant', `filler ${String(index)}`))
    daemon.upsert(toolEntry('t2', 'x', 'ls', ''))
    daemon.approvals = [request('req-4', { turn_id: 't2', item_id: 'x', command: 'ls' })]
    render(<ChatSession link={link()} agent="claude" online />)
    await settle()
    await settle()

    fireEvent.click(screen.getByRole('button', { name: /Go to the step this is about: ls/ }))
    await settle()

    // Following the newest row stops, so the jump-to-latest button appears.
    expect(screen.getByRole('button', { name: 'Jump to the latest message' })).toBeInTheDocument()
  })
})
