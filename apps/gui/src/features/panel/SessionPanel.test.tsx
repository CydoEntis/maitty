import { act, cleanup, fireEvent, render, renderHook, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useState, type ReactElement, type ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '@/features/chat/fakeDaemonHistory'
import { forgetChatFeeds } from '@/features/chat/useChatFeed'
import { SessionPanel } from './SessionPanel'
import { PANEL_STORAGE_KEY, useSessionPanelState } from './useSessionPanelState'

const fake = vi.hoisted(() => ({ current: null as null | { read: (cursor: never) => Promise<unknown> } }))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
  loadTargetDiff: () => Promise.resolve({ files: [], additions: 0, deletions: 0, total_files: 0, truncated: false }),
}))

const base = { root: null, branch: null, online: true, chatOpen: false, width: 320, onWide: () => undefined } as const

function Providers({ children }: { children: ReactNode }) {
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false } } }))
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

function renderPanel(ui: ReactElement) {
  return render(ui, { wrapper: Providers })
}

beforeEach(() => {
  vi.useFakeTimers()
  forgetChatFeeds()
  window.localStorage.clear()
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

type Entry = Record<string, unknown>
const tool = (item: string, label: string, input: unknown, output = '', state = 'Completed'): Entry => ({
  source: { conversation: FAKE_CONVERSATION, turn: 't1', item },
  content: { Tool: { kind: 'commandExecution', label, output, input } },
  state,
})

const ready = { kind: 'ready', agent: 'claude', link: { conversation: FAKE_CONVERSATION, generation: 1 } } as const

function open(entries: Entry[], status: 'Idle' | 'Working' = 'Idle') {
  const daemon = new FakeDaemonHistory()
  for (const entry of entries) daemon.upsert(entry)
  daemon.status = status
  fake.current = daemon
  const onClose = vi.fn()
  renderPanel(<SessionPanel {...base} mode="docked" title="fix tests" process="claude" availability={ready} onClose={onClose} />)
  return { daemon, onClose }
}

describe('the session panel for an agent session', () => {
  it('shows the agent\'s checklist with its count, progress and marks', async () => {
    open([tool('todo', 'TodoWrite ', { todos: [{ content: 'Read the code', status: 'completed' }, { content: 'Write the fix', status: 'in_progress' }, { content: 'Run tests', status: 'pending' }] })])
    await settle()

    const card = screen.getByRole('region', { name: 'Checklist' })
    expect(within(card).getByText('Checklist · 1 of 3')).toBeInTheDocument()
    expect(within(card).getByRole('progressbar', { name: 'Checklist progress' })).toHaveAttribute('aria-valuenow', '1')
    expect(within(card).getAllByRole('listitem').map((item) => item.textContent)).toEqual(['Read the code', 'Write the fix', 'Run tests'])
    expect(within(card).getByText('Read the code')).toHaveClass('line-through')
    expect(within(card).getByText('Write the fix')).toHaveClass('font-semibold')
  })

  it('shows the current item running with its time only while the agent works', async () => {
    vi.setSystemTime(100_000)
    const todo = tool('todo', 'TodoWrite ', { todos: [{ content: 'Write the fix', status: 'in_progress' }] })
    todo.started_at_ms = 70_000
    open([todo], 'Working')
    await settle()

    expect(within(screen.getByRole('region', { name: 'Checklist' })).getByText('0:30')).toBeInTheDocument()
  })

  it('shows the helper agents under "Agents · N working"', async () => {
    open([
      tool('h1', 'Task ', { description: 'Find call sites', subagent_type: 'Explore' }, '', 'Running'),
      tool('h2', 'Task ', { description: 'Audit parser' }, 'ok'),
    ])
    await settle()

    const card = screen.getByRole('group', { name: 'Helper agents' })
    expect(within(card).getByText('Agents · 1 working')).toBeInTheDocument()
    expect(within(card).getByText('Find call sites')).toBeInTheDocument()
    expect(within(card).getByText('Audit parser')).toBeInTheDocument()
  })

  it('says "Nothing to track yet" for an agent that made neither', async () => {
    open([tool('r', 'Read ', { file_path: 'a.ts' })])
    await settle()

    expect(screen.getByText('Nothing to track yet')).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Checklist' })).toBeNull()
  })

  it('does not invent items for a checklist the history did not keep', async () => {
    open([tool('todo', 'TodoWrite ', undefined)])
    await settle()

    expect(screen.getByText(/did not keep its items/)).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Checklist' })).toBeNull()
  })

  it('names the session, hides on request, and the hide button is named with its key', async () => {
    const { onClose } = open([])
    await settle()

    expect(screen.getByRole('heading', { name: 'fix tests' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /^Hide session panel/ }))
    expect(onClose).toHaveBeenCalledOnce()
  })
})

describe('a session with nothing to track', () => {
  it.each([
    [{ kind: 'shell' } as const, /shell session/],
    [{ kind: 'unsupported', agent: 'gemini' } as const, /no event stream/],
    [{ kind: 'remote', agent: 'claude', machineName: 'devbox' } as const, /local-only/],
    [{ kind: 'no-events', agent: 'codex' } as const, /no conversation/],
  ])('says why for %j and reads no conversation', (availability, why) => {
    renderPanel(<SessionPanel {...base} mode="docked" title="shell" process="shell" availability={availability} onClose={() => undefined} />)

    expect(screen.getByText('Nothing to track yet')).toBeInTheDocument()
    expect(screen.getByText(why)).toBeInTheDocument()
  })
})

describe('docked and overlay', () => {
  it('draws the overlay over a scrim that closes it, and keeps the docked form scrim-free', () => {
    const onClose = vi.fn()
    const { container, rerender } = renderPanel(<SessionPanel {...base} mode="overlay" title="t" process="shell" availability={{ kind: 'shell' }} onClose={onClose} />)
    const scrim = container.querySelector('.mx-drawer-scrim')
    expect(scrim).not.toBeNull()

    fireEvent.mouseDown(within(container).getByLabelText('Session panel'))
    expect(onClose).not.toHaveBeenCalled()
    fireEvent.mouseDown(scrim as Element)
    expect(onClose).toHaveBeenCalledOnce()

    rerender(<SessionPanel {...base} mode="docked" title="t" process="shell" availability={{ kind: 'shell' }} onClose={onClose} />)
    expect(container.querySelector('.mx-drawer-scrim')).toBeNull()
  })
})

describe('the panel state', () => {
  it('docks on a wide window and remembers the choice', () => {
    const first = renderHook(() => useSessionPanelState(false))
    expect(first.result.current).toMatchObject({ open: false, mode: 'docked' })

    act(() => { first.result.current.toggle() })
    expect(first.result.current.open).toBe(true)
    expect(window.localStorage.getItem(PANEL_STORAGE_KEY)).toBe('open')

    first.unmount()
    expect(renderHook(() => useSessionPanelState(false)).result.current.open).toBe(true)
  })

  it('is an overlay on a compact window, closed by default, and never remembered', () => {
    window.localStorage.setItem(PANEL_STORAGE_KEY, 'open')
    const { result } = renderHook(() => useSessionPanelState(true))

    expect(result.current).toMatchObject({ open: false, mode: 'overlay' })
    act(() => { result.current.toggle() })
    expect(result.current.open).toBe(true)
    act(() => { result.current.close() })
    expect(result.current.open).toBe(false)
    expect(window.localStorage.getItem(PANEL_STORAGE_KEY)).toBe('open')
  })

  it('keeps the docked and overlay choices apart as the window changes size', () => {
    const { result, rerender } = renderHook(({ compact }) => useSessionPanelState(compact), { initialProps: { compact: false } })
    act(() => { result.current.toggle() })

    rerender({ compact: true })
    expect(result.current.open).toBe(false)
    rerender({ compact: false })
    expect(result.current.open).toBe(true)
  })

  it('works when storage is unavailable', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked') })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('blocked') })
    const { result } = renderHook(() => useSessionPanelState(false))

    expect(result.current.open).toBe(false)
    act(() => { result.current.toggle() })
    expect(result.current.open).toBe(true)
    vi.restoreAllMocks()
  })
})
