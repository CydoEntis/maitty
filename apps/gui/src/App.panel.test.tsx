import { act, render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import App from './App'
import { FakeDaemonHistory } from './features/chat/fakeDaemonHistory'
import { forgetChatFeeds } from './features/chat/useChatFeed'
import { forgetComposers } from './features/chat/composerStore'
import { PANEL_STORAGE_KEY } from './features/panel/useSessionPanelState'

const daemon = vi.hoisted(() => ({
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  loadPerformanceSample: vi.fn<() => Promise<never>>(() => new Promise(() => {})),
  listenSessionsChanged: vi.fn(() => Promise.resolve(() => {})),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  startShellSession: vi.fn<(folder: string) => Promise<void>>(),
  archiveConversationOnDaemon: vi.fn(),
  renameConversationOnDaemon: vi.fn(),
  markSeenOnDaemon: vi.fn(() => Promise.resolve()),
  pinSessionOnDaemon: vi.fn(),
  loadDaemonRestartStatus: vi.fn(),
  restartMismatchedDaemon: vi.fn(),
  suggestProjectFolders: vi.fn(() => Promise.resolve([] as string[])),
  sendTerminalInput: vi.fn<(conversation: { id: string; generation: number }, input: string | Uint8Array) => Promise<void>>(() => Promise.resolve()),
  resizeTerminal: vi.fn(() => Promise.resolve()),
  loadMachines: vi.fn(() => Promise.resolve({ names: [], error: null })),
  readConversationPage: vi.fn(),
  submitMessage: vi.fn(() => Promise.resolve({})),
  approveRequest: vi.fn(() => Promise.resolve()),
  cancelTurn: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/api/daemon', () => ({
  ...daemon,
  isDesktop: () => true,
  terminalOf: (id: string) => {
    const match = /^t(\d+)$/.exec(id)
    return match?.[1] === undefined ? null : Number(match[1])
  },
}))

vi.mock('@/api/machines', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/machines')>()),
  loadMachinesOverview: () => Promise.resolve([]),
  listenMachineState: () => Promise.resolve(() => undefined),
  listenMachineSessions: () => Promise.resolve(() => undefined),
  listenMachineError: () => Promise.resolve(() => undefined),
  listenMachineRemoved: () => Promise.resolve(() => undefined),
  listenMachineNotice: () => Promise.resolve(() => undefined),
  listenMachineClipboard: () => Promise.resolve(() => undefined),
}))

vi.mock('@/features/terminal/useFileDrop', () => ({ useFileDrop: () => undefined }))

const terminal = vi.hoisted(() => ({ mounts: 0, focused: [] as boolean[] }))

vi.mock('./TerminalView', async () => {
  const { useEffect } = await import('react')
  return {
    TerminalView: ({ conversation, label, focused }: { conversation: Conversation; label: string; focused: boolean }) => {
      useEffect(() => { terminal.mounts += 1 }, [])
      terminal.focused.push(focused)
      return <div className="terminal-screen" role="region" aria-label={label} tabIndex={0}>Pane {conversation.nativePane}</div>
    },
  }
})

const HOME = '/home/casey'

function term(pane: number, extra: Partial<Conversation> = {}): Conversation {
  return {
    id: `t${String(pane)}`,
    generation: 0,
    execution: 'Native',
    kind: 'terminal',
    title: `Shell ${String(pane)}`,
    project: 'project',
    folder: HOME,
    cwd: HOME,
    repo: null,
    branch: 'working tree',
    provider: 'Codex',
    status: 'idle',
    updated: null,
    model: 'Provider default',
    permission: 'Terminal session',
    context: null,
    nativePane: pane,
    hasDiff: false,
    agent: null,
    ...extra,
  }
}

const claude = (pane: number, extra: Partial<Conversation> = {}) =>
  term(pane, { provider: 'Claude', agent: 'claude', process: 'claude', chat: { conversation: 40 + pane, generation: 1 }, ...extra })

function renderDesktopApp() {
  window.localStorage.setItem('maitty.gui.welcomed.v1', '1')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(<QueryClientProvider client={client}><App /></QueryClientProvider>)
}


function setWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', { configurable: true, writable: true, value: width })
  window.dispatchEvent(new Event('resize'))
}

function history() {
  const fake = new FakeDaemonHistory()
  fake.upsert({
    source: { conversation: 5, turn: 't1', item: 'todo' },
    content: { Tool: { kind: 'commandExecution', label: 'TodoWrite ', output: '', input: { todos: [{ content: 'Write the fix', status: 'in_progress' }, { content: 'Run tests', status: 'pending' }] } } },
    state: 'Completed',
  })
  daemon.readConversationPage.mockImplementation((_id: number, cursor: never) => fake.read(cursor))
  return fake
}

describe('the session panel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    forgetChatFeeds()
    forgetComposers()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: false, sessionCount: 1, activeTurnCount: 0 })
  })

  it('docks on a wide window with Ctrl+\\ and shows the agent\'s checklist, then hides with the same key', async () => {
    setWidth(1400)
    const user = userEvent.setup()
    history()
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })
    expect(screen.queryByRole('complementary', { name: 'Session panel' })).not.toBeInTheDocument()

    await user.keyboard('{Control>}\\{/Control}')

    const panel = await screen.findByRole('complementary', { name: 'Session panel' })
    expect(await within(panel).findByText('Checklist · 0 of 2')).toBeInTheDocument()
    expect(panel.closest('.mx-drawer-scrim')).toBeNull()
    expect(window.localStorage.getItem(PANEL_STORAGE_KEY)).toBe('open')

    await user.keyboard('{Control>}\\{/Control}')
    expect(screen.queryByRole('complementary', { name: 'Session panel' })).not.toBeInTheDocument()
  })

  it('opens from the pane\'s toggle button and closes from its Hide button', async () => {
    setWidth(1400)
    const user = userEvent.setup()
    history()
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })

    await user.click(within(pane).getByRole('button', { name: /^Show session panel/ }))
    const panel = await screen.findByRole('complementary', { name: 'Session panel' })
    expect(within(pane).getByRole('button', { name: /^Hide session panel/ })).toHaveAttribute('aria-pressed', 'true')

    await user.click(within(panel).getByRole('button', { name: /^Hide session panel/ }))
    expect(screen.queryByRole('complementary', { name: 'Session panel' })).not.toBeInTheDocument()
  })

  it('is an overlay on a compact window that Esc closes before anything else', async () => {
    setWidth(900)
    const user = userEvent.setup()
    history()
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })

    await user.keyboard('{Control>}\\{/Control}')

    const panel = await screen.findByRole('complementary', { name: 'Session panel' })
    expect(panel.closest('.mx-drawer-scrim')).not.toBeNull()
    expect(window.localStorage.getItem(PANEL_STORAGE_KEY)).toBeNull()

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('complementary', { name: 'Session panel' })).not.toBeInTheDocument()
  })

  it('says why there is nothing to track for a shell session, and reads no conversation', async () => {
    setWidth(1400)
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(3)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })

    await user.keyboard('{Control>}\\{/Control}')

    const panel = await screen.findByRole('complementary', { name: 'Session panel' })
    expect(within(panel).getByText('Nothing to track yet')).toBeInTheDocument()
    expect(within(panel).getByText(/shell session/)).toBeInTheDocument()
    expect(daemon.readConversationPage).not.toHaveBeenCalled()
  })

  it('keeps the docked panel through a narrow spell, without opening the overlay by itself', async () => {
    setWidth(1400)
    const user = userEvent.setup()
    history()
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })
    await user.keyboard('{Control>}\\{/Control}')
    await screen.findByRole('complementary', { name: 'Session panel' })

    act(() => { setWidth(900) })
    expect(screen.queryByRole('complementary', { name: 'Session panel' })).not.toBeInTheDocument()
    act(() => { setWidth(1400) })
    expect(await screen.findByRole('complementary', { name: 'Session panel' })).toBeInTheDocument()
  })
})
