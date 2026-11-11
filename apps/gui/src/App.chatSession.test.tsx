import { render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import App from './App'
import { FakeDaemonHistory } from './features/chat/fakeDaemonHistory'
import { forgetChatFeeds } from './features/chat/useChatFeed'
import { forgetComposers } from './features/chat/composerStore'
import { forgetOperations } from './features/chat/sendOperation'

// A chat session (ADR 0003) in the desktop window: a structured conversation the daemon owns, listed
// beside terminal sessions, with a chat that can be written to and no terminal.

const daemon = vi.hoisted(() => ({
  connectAgent: vi.fn(() => Promise.resolve()),
  loadAgentModels: vi.fn(() => Promise.resolve([] as { id: string; label: string; blurb: string }[])),
  // Not a mock: a chat asks for its agent's commands as it opens, whatever a test resets.
  loadAgentCommands: () => Promise.resolve([] as { name: string; description: string; hint: string; aliases: string[] }[]),
  // The service is the app's own release: nothing to say and nothing to restart.
  loadDaemonUpdateStatus: () => Promise.resolve({ daemonVersion: null, behind: false, canRestart: false, sessionCount: 0, activeTurnCount: 0 }),
  restartStaleDaemon: () => Promise.resolve(),
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  loadPerformanceSample: vi.fn<() => Promise<never>>(() => new Promise(() => {})),
  listenSessionsChanged: vi.fn(() => Promise.resolve(() => {})),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  startShellSession: vi.fn(),
  archiveConversationOnDaemon: vi.fn(() => Promise.resolve()),
  closeChatSession: vi.fn(() => Promise.resolve()),
  startChatSession: vi.fn<(folder: string, agent: string, branch?: string | null) => Promise<{ id: number; generation: number }>>(),
  renameConversationOnDaemon: vi.fn(),
  markSeenOnDaemon: vi.fn(() => Promise.resolve()),
  loadDaemonRestartStatus: vi.fn(),
  restartMismatchedDaemon: vi.fn(),
  suggestProjectFolders: vi.fn(() => Promise.resolve([] as string[])),
  sendTerminalInput: vi.fn<(conversation: { id: string; generation: number }, input: string | Uint8Array) => Promise<void>>(() => Promise.resolve()),
  resizeTerminal: vi.fn(() => Promise.resolve()),
  loadMachines: vi.fn(() => Promise.resolve({ names: [], error: null })),
  readConversationPage: vi.fn(),
  submitMessage: vi.fn(() => Promise.resolve({})),
  approveRequest: vi.fn(() => Promise.resolve()),
  answerUserInput: vi.fn(() => Promise.resolve()),
  cancelTurn: vi.fn(() => Promise.resolve()),
  dequeueMessage: vi.fn(() => Promise.resolve()),
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
  listenAttention: () => Promise.resolve(() => undefined),
  listenMachineClipboard: () => Promise.resolve(() => undefined),
}))

vi.mock('@/features/terminal/useFileDrop', () => ({ useFileDrop: () => undefined }))

vi.mock('./TerminalView', () => ({
  TerminalView: ({ conversation, label }: { conversation: Conversation; label: string }) => (
    <div className="terminal-screen" role="region" aria-label={label} tabIndex={0}>Pane {conversation.nativePane}</div>
  ),
}))

const HOME = '/home/casey'
const REPO = `${HOME}/app`

function term(pane: number, extra: Partial<Conversation> = {}): Conversation {
  return {
    id: `t${String(pane)}`, generation: 0, execution: 'Native', kind: 'terminal', title: `Shell ${String(pane)}`, project: 'app', folder: REPO, cwd: REPO, repo: 'app',
    branch: 'main', provider: 'Codex', status: 'idle', updated: null, model: '', permission: 'Terminal session', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra,
  }
}

function chatSession(id: number, extra: Partial<Conversation> = {}): Conversation {
  return {
    id: String(id), generation: 1, execution: 'Structured', title: `Claude conversation ${String(id)}`, project: 'app', folder: REPO, cwd: `${HOME}/app-trees/agent-x`, repo: 'app',
    branch: 'agent/x', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: null, hasDiff: false, agent: 'claude', process: 'claude',
    chat: { conversation: id, generation: 1 }, ...extra,
  }
}

function renderDesktopApp() {
  window.localStorage.setItem('maitty.gui.welcomed.v1', '1')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(<QueryClientProvider client={client}><App /></QueryClientProvider>)
}

function serveHistory() {
  const history = new FakeDaemonHistory()
  history.upsert(history.message('t1', 'u', 'User', 'fix the failing test'))
  history.upsert(history.message('t1', 'a', 'Assistant', 'On it.'))
  daemon.readConversationPage.mockImplementation((_id: number, cursor: never) => history.read(cursor))
  return history
}

describe('a chat session in the desktop window', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    forgetChatFeeds()
    forgetComposers()
    forgetOperations()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: false, sessionCount: 1, activeTurnCount: 0 })
  })

  it('is a row beside the terminal sessions, and opens as a chat with the composer on and no terminal', async () => {
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(2), chatSession(44)])
    renderDesktopApp()

    const nav = await screen.findByRole('navigation', { name: 'Sessions' })
    expect(within(nav).getByRole('button', { name: /, claude, / })).toBeInTheDocument()

    // Opening it: its pane is the chat and nothing else.
    await userEvent.setup().click(within(nav).getByRole('button', { name: /, claude, / }))
    const chat = await screen.findByRole('region', { name: 'Chat' })
    expect(await within(chat).findByText('On it.')).toBeInTheDocument()
    // A chat session has no terminal, so its pane offers no switch to one.
    expect(screen.queryByRole('radiogroup', { name: 'Pane view' })).not.toBeInTheDocument()
    expect(within(chat).getByRole('textbox')).toBeEnabled()
    expect(screen.queryByRole('radiogroup', { name: 'Pane view' })).not.toBeInTheDocument()
    // No terminal was attached for it: the only terminal on screen is the shell's, if that is shown at all.
    expect(screen.queryByText('Pane null')).not.toBeInTheDocument()
  })

  it('sends a message, with the operation number the daemon expects', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([chatSession(44)])
    renderDesktopApp()
    const chat = await screen.findByRole('region', { name: 'Chat' })
    await within(chat).findByText('On it.')

    await user.type(within(chat).getByRole('textbox'), 'run the tests{Enter}')
    await waitFor(() => { expect(daemon.submitMessage).toHaveBeenCalledWith({ id: '44', generation: 1 }, 1, 'run the tests') })
    expect(daemon.sendTerminalInput).not.toHaveBeenCalled()
  })

  it('shows a pending approval with y, a and n, answered by its id, never typed into a terminal', async () => {
    const user = userEvent.setup()
    const history = serveHistory()
    history.status = 'Blocked'
    history.approvals = [{ request_id: 'r-1', kind: 'Command', turn_id: 't1', command: 'rm -rf build', choices: ['Accept', 'AcceptForSession', 'Decline'], questions: [] }]
    daemon.loadConversations.mockResolvedValue([chatSession(44, { status: 'needs-you' })])
    renderDesktopApp()
    const chat = await screen.findByRole('region', { name: 'Chat' })

    const card = await within(chat).findByRole('region', { name: /Approval/ })
    await user.click(within(card).getByRole('button', { name: /^Yes, once/ }))
    expect(daemon.approveRequest).toHaveBeenCalledWith({ id: '44', generation: 1 }, 'r-1', 'Accept')
    expect(daemon.sendTerminalInput).not.toHaveBeenCalled()
  })

  it('closing it stops its agent first and does not close a terminal pane', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([chatSession(44)])
    renderDesktopApp()
    const chat = await screen.findByRole('region', { name: 'Chat' })
    await within(chat).findByText('On it.')

    const pane = screen.getByRole('region', { name: /^Terminal 1/ })
    // An agent is running in it, so closing asks first.
    await user.click(within(pane).getByRole('button', { name: 'Close pane · ends its session' }))
    expect(daemon.closeChatSession).not.toHaveBeenCalled()
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Stop claude' }))
    await waitFor(() => { expect(daemon.closeChatSession).toHaveBeenCalledTimes(1) })
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('stays out of the way of the typed-command start: Start claude here still types into the shell', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(3, { repo: null, folder: HOME, cwd: HOME })])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    await user.keyboard('{Control>}{Shift>}e{/Shift}{/Control}')

    await user.click(within(pane).getByRole('button', { name: 'Start claude here' }))
    expect(daemon.sendTerminalInput).toHaveBeenCalledWith({ id: 't3', generation: 0 }, 'claude\r')
    expect(daemon.startChatSession).not.toHaveBeenCalled()
  })

  it('"Start a Claude chat" on a shell starts a chat session in its folder and opens it, typing nothing', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(3, { repo: null, folder: HOME, cwd: HOME })])
    daemon.startChatSession.mockImplementation(() => {
      daemon.loadConversations.mockResolvedValue([term(3, { repo: null, folder: HOME, cwd: HOME }), chatSession(44, { cwd: HOME, folder: HOME, repo: null })])
      return Promise.resolve({ id: 44, generation: 1 })
    })
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    await user.keyboard('{Control>}{Shift>}e{/Shift}{/Control}')

    await user.click(within(pane).getByRole('button', { name: 'Start a Claude chat' }))

    await waitFor(() => { expect(daemon.startChatSession).toHaveBeenCalledWith(HOME, 'claude', null, false) })
    expect(daemon.sendTerminalInput).not.toHaveBeenCalled()
    // The chat session opens in a pane of its own, with no switch to a terminal it does not have.
    await waitFor(() => { expect(screen.getAllByRole('region', { name: /^Terminal \d/ }).some((shown) => within(shown).queryByRole('region', { name: 'Chat' }) !== null && within(shown).queryByRole('radiogroup', { name: 'Pane view' }) === null)).toBe(true) })
  })
})

describe('full access in a chat session', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
  })

  it('starts the chat with full access when the switch is on, and shows it on the chat afterwards', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(3, { repo: null, folder: HOME, cwd: HOME })])
    daemon.startChatSession.mockImplementation(() => {
      daemon.loadConversations.mockResolvedValue([term(3, { repo: null, folder: HOME, cwd: HOME }), chatSession(44, { cwd: HOME, folder: HOME, repo: null })])
      return Promise.resolve({ id: 44, generation: 1 })
    })
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    await user.keyboard('{Control>}{Shift>}e{/Shift}{/Control}')

    await user.click(within(pane).getByRole('checkbox', { name: 'Full access (skip permission prompts)' }))
    await user.click(within(pane).getByRole('button', { name: 'Start a Claude chat' }))

    await waitFor(() => { expect(daemon.startChatSession).toHaveBeenCalledWith(HOME, 'claude', null, true) })
    expect(await screen.findByText('Full access')).toBeInTheDocument()
  })

  it('has no New agent on a branch button: the sidebar offers New session and chats start from a shell', async () => {
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(2)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })
    expect(screen.queryByRole('button', { name: 'New agent on a branch' })).toBeNull()
  })
})

describe('the chat that follows an agent in a pane', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    forgetChatFeeds()
    forgetComposers()
    forgetOperations()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: false, sessionCount: 1, activeTurnCount: 0 })
  })

  it('types into the agent\'s terminal from the composer when the agent is idle, so the chat and the terminal are one session', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(2, { provider: 'Claude', agent: 'claude', process: 'claude', chat: { conversation: 41, generation: 1, readOnly: true } })])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    // An agent opens as its chat, and the switch to its terminal is there.
    const chat = await within(pane).findByRole('region', { name: 'Chat' })
    await within(chat).findByText('On it.')

    const composer = within(chat).getByRole('textbox')
    await waitFor(() => { expect(composer).toBeEnabled() })
    expect(within(pane).getByRole('radiogroup', { name: 'Pane view' })).toBeInTheDocument()

    await user.type(composer, 'run the tests{Enter}')

    // One paste then Enter, into the terminal; nothing goes to the daemon's conversation.
    await waitFor(() => { expect(daemon.sendTerminalInput).toHaveBeenCalledWith({ id: 't2', generation: 0 }, '\x1b[200~run the tests\x1b[201~\r') })
    expect(daemon.submitMessage).not.toHaveBeenCalled()
  })
})
