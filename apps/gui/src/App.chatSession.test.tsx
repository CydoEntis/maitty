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

// A chat session (ADR 0054) in the desktop window: a structured conversation the daemon owns, listed
// beside terminal sessions, with a chat that can be written to and no terminal.

const daemon = vi.hoisted(() => ({
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

const branchAgent = vi.hoisted(() => ({ newBranchAgent: vi.fn(() => Promise.resolve()) }))
vi.mock('@/features/worktrees/api', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/features/worktrees/api')>()), ...branchAgent }))

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

  it('is a row beside the terminal sessions of its project, and opens as a chat with the composer on and no terminal', async () => {
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(2), chatSession(44)])
    renderDesktopApp()

    const nav = await screen.findByRole('navigation', { name: 'Sessions by project' })
    expect(within(nav).getByRole('button', { name: /, claude, / })).toBeInTheDocument()

    // Opening it: its pane is the chat and nothing else.
    await userEvent.setup().click(within(nav).getByRole('button', { name: /, claude, / }))
    const chat = await screen.findByRole('region', { name: 'Chat' })
    expect(await within(chat).findByText('On it.')).toBeInTheDocument()
    expect(within(chat).getByText(/Chat session · no terminal/)).toBeInTheDocument()
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

    const card = await within(chat).findByRole('group', { name: /Approval/ })
    await user.click(within(card).getByRole('button', { name: 'Yes, once (y)' }))
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
    // An agent is running in it, so the first press arms the close and the second ends it.
    await user.click(within(pane).getByRole('button', { name: 'Close pane · ends its session' }))
    await user.click(within(pane).getByRole('button', { name: 'Click again to close and stop claude' }))
    await waitFor(() => { expect(daemon.closeChatSession).toHaveBeenCalledTimes(1) })
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('stays out of the way of the typed-command start: Start claude here still types into the shell', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(3, { repo: null, folder: HOME, cwd: HOME })])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))

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
    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))

    await user.click(within(pane).getByRole('button', { name: 'Start a Claude chat' }))

    await waitFor(() => { expect(daemon.startChatSession).toHaveBeenCalledWith(HOME, 'claude', null, false) })
    expect(daemon.sendTerminalInput).not.toHaveBeenCalled()
    const chats = await screen.findAllByRole('region', { name: 'Chat' })
    expect(chats.some((chat) => within(chat).queryByText(/Chat session · no terminal/) !== null)).toBe(true)
  })

  it('the New agent dialog starts a chat session on a branch when Chat is chosen, and a terminal one when Terminal is chosen', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(2)])
    daemon.startChatSession.mockImplementation(() => {
      daemon.loadConversations.mockResolvedValue([term(2), chatSession(44)])
      return Promise.resolve({ id: 44, generation: 1 })
    })
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })

    const openDialog = async () => {
      await user.click(await screen.findByRole('button', { name: 'New agent on a branch' }))
      return screen.findByRole('dialog', { name: /New agent on a branch/ })
    }

    // Chat is the default; Terminal is one click away.
    let dialog = await openDialog()
    expect(within(dialog).getByRole('radio', { name: 'Chat' })).toBeChecked()
    await user.click(within(dialog).getByRole('radio', { name: 'Terminal' }))
    await user.type(within(dialog).getByRole('textbox', { name: 'Branch' }), 'agent/t')
    await user.click(within(dialog).getByRole('button', { name: /^Start/ }))
    await waitFor(() => { expect(branchAgent.newBranchAgent).toHaveBeenCalledWith({ project: REPO, branch: 'agent/t', agent: 'claude', prompt: '' }) })
    expect(daemon.startChatSession).not.toHaveBeenCalled()

    // Chat is the default again.
    dialog = await openDialog()
    expect(within(dialog).getByRole('radio', { name: 'Chat' })).toBeChecked()
    await user.type(within(dialog).getByRole('textbox', { name: 'First prompt' }), 'Fix the login bug')
    await user.click(within(dialog).getByRole('button', { name: /^Start/ }))

    await waitFor(() => { expect(daemon.startChatSession).toHaveBeenCalledWith(REPO, 'claude', 'agent/fix-login-bug', false) })
    // The first prompt is an ordinary send once the start has answered.
    await waitFor(() => { expect(daemon.submitMessage).toHaveBeenCalledWith({ id: '44', generation: 1 }, 1, 'Fix the login bug') })
    expect(branchAgent.newBranchAgent).toHaveBeenCalledTimes(1)
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
    daemon.loadConversations.mockResolvedValue([term(2)])
    daemon.startChatSession.mockImplementation(() => {
      daemon.loadConversations.mockResolvedValue([term(2), chatSession(44)])
      return Promise.resolve({ id: 44, generation: 1 })
    })
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1/ })

    await user.click(await screen.findByRole('button', { name: 'New agent on a branch' }))
    const dialog = await screen.findByRole('dialog', { name: /New agent on a branch/ })
    await user.click(within(dialog).getByRole('checkbox', { name: 'Full access (skip permission prompts)' }))
    await user.type(within(dialog).getByRole('textbox', { name: 'Branch' }), 'agent/yolo')
    await user.click(within(dialog).getByRole('button', { name: /^Start/ }))

    await waitFor(() => { expect(daemon.startChatSession).toHaveBeenCalledWith(REPO, 'claude', 'agent/yolo', true) })
    expect(await screen.findByText('Full access')).toBeInTheDocument()
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

  it('stays read-only: the composer is off with its reason, and it is not a chat session', async () => {
    const user = userEvent.setup()
    serveHistory()
    daemon.loadConversations.mockResolvedValue([term(2, { provider: 'Claude', agent: 'claude', process: 'claude', chat: { conversation: 41, generation: 1, readOnly: true } })])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1/ })
    // The terminal view with its switch is still there.
    await user.click(within(pane).getByRole('radio', { name: /^Chat view/ }))
    const chat = await within(pane).findByRole('region', { name: 'Chat' })
    await within(chat).findByText('On it.')

    expect(within(chat).getByRole('textbox')).toBeDisabled()
    expect(within(chat).getByText(/terminal/i)).toBeInTheDocument()
    expect(within(chat).queryByText(/Chat session · no terminal/)).not.toBeInTheDocument()
    expect(within(pane).getByRole('radiogroup', { name: 'Pane view' })).toBeInTheDocument()
  })
})
