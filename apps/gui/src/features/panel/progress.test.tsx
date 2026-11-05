import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, cleanup, render, renderHook, screen } from '@testing-library/react'
import { useState, type ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '@/features/chat/fakeDaemonHistory'
import { forgetChatFeeds } from '@/features/chat/useChatFeed'
import { SessionRow, type SessionRowProps } from '@/features/workspace/components/SessionRow'
import type { SessionView } from '@/features/workspace/model/session'
import { forgetProgress, publishProgress, useChecklistProgress } from './progressStore'
import { SessionPanel } from './SessionPanel'

const fake = vi.hoisted(() => ({ history: null as null | { read: (cursor: never) => Promise<unknown> } }))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.history === null) throw new Error('no fake daemon')
    return fake.history.read(cursor as never)
  },
  loadTargetDiff: () => Promise.resolve({ files: [], additions: 0, deletions: 0, total_files: 0, truncated: false }),
}))

beforeEach(() => {
  vi.useFakeTimers()
  forgetProgress()
  forgetChatFeeds()
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.history = null
})

const hook = renderHook

describe('the checklist progress store', () => {
  it('keeps the last list read per conversation and forgets it when the agent has none', () => {
    const { result } = hook(() => useChecklistProgress(7))
    expect(result.current).toBeNull()

    act(() => { publishProgress(7, { done: 1, total: 3, current: 'Write the fix' }) })
    expect(result.current).toEqual({ done: 1, total: 3, current: 'Write the fix' })
    expect(hook(() => useChecklistProgress(8)).result.current).toBeNull()
    expect(hook(() => useChecklistProgress(undefined)).result.current).toBeNull()

    act(() => { publishProgress(7, null) })
    expect(result.current).toBeNull()
  })

  it('does not notify for a progress that did not change', () => {
    let renders = 0
    hook(() => { renders += 1; return useChecklistProgress(7) })
    act(() => { publishProgress(7, { done: 1, total: 2, current: null }) })
    const after = renders

    act(() => { publishProgress(7, { done: 1, total: 2, current: null }) })

    expect(renders).toBe(after)
  })
})

function sessionWithChat(conversation: number | undefined): SessionView {
  return {
    id: 't2', pane: 2, machine: 0, machineName: null, cwd: '/repo', cwdLabel: '~/repo', repoRoot: '/repo', repoName: 'repo', branch: null, title: 'repo', name: null,
    pinned: false, archived: false, ended: false, promptPath: 'repo', process: 'claude', lastPrompt: '', asking: '', interrupted: null, timeline: [], usage: null,
    status: 'idle', lastKnownStatus: null,
    conversation: { id: 't2', agent: 'claude', ...(conversation === undefined ? {} : { chat: { conversation, generation: 1 } }) },
  } as unknown as SessionView
}

function row(session: SessionView) {
  const none = vi.fn()
  const hover = { close: none, open: none, show: none, hide: none } as unknown as SessionRowProps['hover']
  render(<SessionRow session={session} track={undefined} selected={false} online swapped={false} portalContainer={null} hover={hover} onShow={none} onSetName={none} onPin={none} onEnd={none} />)
}

describe('a sidebar session row', () => {
  it('shows the agent\'s checklist progress and current item under the session', () => {
    publishProgress(FAKE_CONVERSATION, { done: 2, total: 5, current: 'Write the fix' })
    row(sessionWithChat(FAKE_CONVERSATION))

    expect(screen.getByText('2 of 5 · Write the fix')).toBeInTheDocument()
    expect(screen.getByTitle(/The agent's checklist: 2 of 5 done, now: Write the fix/)).toBeInTheDocument()
  })

  it('shows nothing for a session whose conversation was never read, or that has no chat', () => {
    row(sessionWithChat(FAKE_CONVERSATION))
    expect(screen.queryByText(/ of /)).toBeNull()
    cleanup()
    publishProgress(FAKE_CONVERSATION, { done: 1, total: 2, current: null })
    row(sessionWithChat(undefined))
    expect(screen.queryByText(/ of /)).toBeNull()
  })

  it('leaves the current item out when none is in progress', () => {
    publishProgress(FAKE_CONVERSATION, { done: 3, total: 3, current: null })
    row(sessionWithChat(FAKE_CONVERSATION))

    expect(screen.getByText('3 of 3')).toBeInTheDocument()
  })
})

describe('reading a conversation in the panel', () => {
  function Providers({ children }: { children: ReactNode }) {
    const [client] = useState(() => new QueryClient())
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }

  it('publishes the checklist for the sidebar', async () => {
    const history = new FakeDaemonHistory()
    history.upsert({
      source: { conversation: FAKE_CONVERSATION, turn: 't1', item: 'todo' },
      content: { Tool: { kind: 'commandExecution', label: 'TodoWrite ', output: '', input: { todos: [{ content: 'Read', status: 'completed' }, { content: 'Fix', status: 'in_progress' }] } } },
      state: 'Completed',
    })
    fake.history = history
    const probe = hook(() => useChecklistProgress(FAKE_CONVERSATION))
    render(
      <Providers>
        <SessionPanel mode="docked" title="t" process="claude" availability={{ kind: 'ready', agent: 'claude', link: { conversation: FAKE_CONVERSATION, generation: 1 } }} root={null} folder={null} branch={null} online width={320} onWide={() => undefined} onOpenInFiles={() => undefined} onClose={() => undefined} />
      </Providers>,
    )
    await act(async () => { await vi.advanceTimersByTimeAsync(0) })

    expect(probe.result.current).toEqual({ done: 1, total: 2, current: 'Fix' })
  })
})
