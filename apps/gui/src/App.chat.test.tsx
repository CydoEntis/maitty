import { render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import App from './App'
import { FakeDaemonHistory } from './features/chat/fakeDaemonHistory'
import { forgetChatFeeds } from './features/chat/useChatFeed'

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

describe('the Terminal | Chat view of a pane', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    terminal.mounts = 0
    terminal.focused.length = 0
    forgetChatFeeds()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: false, sessionCount: 1, activeTurnCount: 0 })
  })

  it('switches to the chat and back with the button and Ctrl+Shift+E, sending nothing to the daemon and keeping the terminal mounted', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    const view = within(pane).getByRole('radiogroup', { name: 'Pane view' })
    expect(within(view).getByRole('radio', { name: /^Terminal view/ })).toBeChecked()
    vi.clearAllMocks()

    await user.click(within(view).getByRole('radio', { name: /^Chat view/ }))
    expect(within(pane).getByRole('region', { name: 'Chat' })).toBeInTheDocument()
    expect(within(view).getByRole('radio', { name: /^Chat view/ })).toBeChecked()
    // The terminal is still mounted under the chat, and was not mounted again.
    expect(within(pane).getByText('Pane 2')).toBeInTheDocument()
    expect(terminal.mounts).toBe(1)

    await user.keyboard('{Control>}{Shift>}e{/Shift}{/Control}')
    expect(within(pane).queryByRole('region', { name: 'Chat' })).not.toBeInTheDocument()
    await user.keyboard('{Control>}{Shift>}e{/Shift}{/Control}')
    expect(within(pane).getByRole('region', { name: 'Chat' })).toBeInTheDocument()

    expect(daemon.sendTerminalInput).not.toHaveBeenCalled()
    expect(daemon.resizeTerminal).not.toHaveBeenCalled()
    expect(terminal.mounts).toBe(1)
  })

  it('shows "Chat view is for agent sessions" on a shell tab, with a way back and a way to start an agent', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(3)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })

    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))
    expect(within(pane).getByRole('heading', { name: 'Chat view is for agent sessions' })).toBeInTheDocument()

    await user.click(within(pane).getByRole('button', { name: 'Start claude here' }))
    expect(daemon.sendTerminalInput).toHaveBeenCalledWith({ id: 't3', generation: 0 }, 'claude\r')
    expect(within(pane).queryByRole('region', { name: 'Chat' })).not.toBeInTheDocument()

    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))
    await user.click(within(pane).getByRole('button', { name: 'Back to terminal' }))
    expect(within(pane).queryByRole('region', { name: 'Chat' })).not.toBeInTheDocument()
  })

  it('leaves an agent without an event stream terminal-only and says so', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(4, { agent: 'gemini', process: 'gemini' })])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })

    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))
    expect(within(pane).getByText(/gemini has no event stream/)).toBeInTheDocument()
    expect(within(pane).queryByRole('button', { name: /Start claude here/ })).not.toBeInTheDocument()
  })

  it('opens a Claude session in the chat when Settings says so, and never opens a shell there', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ chatViewDefault: { claude: 'chat', codex: 'terminal' } }))
    daemon.loadConversations.mockResolvedValue([claude(2), term(3)])
    renderDesktopApp()

    await waitFor(() => { expect(screen.getAllByRole('region', { name: /^Terminal / }).length).toBeGreaterThan(0) })
    const panes = screen.getAllByRole('region', { name: /^Terminal / })
    const chats = panes.filter((pane) => within(pane).queryByRole('region', { name: 'Chat' }) !== null)
    expect(chats).toHaveLength(1)
    expect(within(chats[0] as HTMLElement).getByRole('radio', { name: /^Chat view/ })).toBeChecked()
  })

  it('shows the daemon\'s conversation in the chat and keeps the terminal untouched while it reads', async () => {
    const user = userEvent.setup()
    const history = new FakeDaemonHistory()
    history.upsert(history.message('t1', 'u', 'User', 'fix the failing test'))
    history.upsert(history.message('t1', 'a', 'Assistant', 'I found the cause. It was a stale fixture.'))
    history.upsert(history.turnStatus('t1', 'Completed', { started_at_ms: 1_000, completed_at_ms: 8_000 }))
    daemon.readConversationPage.mockImplementation((_id: number, cursor: never) => history.read(cursor))
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    vi.clearAllMocks()
    daemon.readConversationPage.mockImplementation((_id: number, cursor: never) => history.read(cursor))

    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))

    expect(await within(pane).findByText('fix the failing test')).toBeInTheDocument()
    expect(within(pane).getByText('I found the cause.')).toBeInTheDocument()
    expect(within(pane).getByRole('separator', { name: /Cooked for 7s/ })).toBeInTheDocument()
    expect(daemon.readConversationPage).toHaveBeenCalledWith(42, null)
    expect(daemon.sendTerminalInput).not.toHaveBeenCalled()
    expect(daemon.resizeTerminal).not.toHaveBeenCalled()
  })

  it('does not hand the keyboard to the terminal while the chat covers it', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([claude(2)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    await waitFor(() => { expect(terminal.focused.at(-1)).toBe(true) })

    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))
    await waitFor(() => { expect(terminal.focused.at(-1)).toBe(false) })
  })
})
