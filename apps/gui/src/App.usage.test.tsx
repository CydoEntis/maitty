import { render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import type { UsagePeriod, UsageReport } from '@/api/usage'
import App from './App'

const daemon = vi.hoisted(() => ({
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  // The daemon numbers no list here and says no session left: a row a list misses is held.
  localListRevision: () => 0,
  daemonSaidRemoved: () => false,
  loadPerformanceSample: vi.fn<() => Promise<never>>(() => new Promise(() => {})),
  listenSessionsChanged: vi.fn(() => Promise.resolve(() => {})),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  startShellSession: vi.fn<(folder: string) => Promise<void>>(),
  archiveConversationOnDaemon: vi.fn(),
  renameConversationOnDaemon: vi.fn(),
  markSeenOnDaemon: vi.fn(),
  loadDaemonRestartStatus: vi.fn(),
  restartMismatchedDaemon: vi.fn(),
  suggestProjectFolders: vi.fn<(typed: string) => Promise<string[]>>(),
  sendTerminalInput: vi.fn(),
  loadMachines: vi.fn(() => Promise.resolve({ names: [], error: null })),
}))

vi.mock('@/api/daemon', () => ({
  ...daemon,
  isDesktop: () => true,
  // The service is the app's own release: nothing to say and nothing to restart.
  loadDaemonUpdateStatus: () => Promise.resolve({ daemonVersion: null, behind: false, canRestart: false, sessionCount: 0, activeTurnCount: 0 }),
  restartStaleDaemon: () => Promise.resolve(),
  terminalOf: (id: string) => {
    const match = /^t(\d+)$/.exec(id)
    return match?.[1] === undefined ? null : Number(match[1])
  },
}))

const usage = vi.hoisted(() => ({ loadUsageReport: vi.fn<(period: UsagePeriod) => Promise<UsageReport>>() }))
vi.mock('@/api/usage', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/usage')>()), loadUsageReport: usage.loadUsageReport }))

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

vi.mock('@/features/terminal/useFileDrop', () => ({ useFileDrop: () => {} }))

vi.mock('./TerminalView', () => ({
  TerminalView: ({ conversation, label }: { conversation: Conversation; label: string }) => (
    <div className="terminal-screen" role="region" aria-label={label} tabIndex={0}>Pane {conversation.nativePane}</div>
  ),
}))

const HOME = '/home/casey'
const NONE = { tokensIn: null, tokensOut: null, cost: null, turns: 0, workingSecs: 0 }

function term(pane: number): Conversation {
  return {
    id: `t${String(pane)}`, generation: 0, execution: 'Native', kind: 'terminal', title: `Shell ${String(pane)}`, project: 'project', folder: HOME, cwd: HOME,
    repo: null, branch: 'working tree', provider: 'Codex', status: 'idle', updated: null, model: 'Provider default', permission: 'Terminal session',
    context: null, nativePane: pane, hasDiff: false, agent: null,
  }
}

function report(): UsageReport {
  return {
    period: 'Week', from: '2026-10-29', to: '2026-11-04', retentionDays: 90,
    totals: { tokensIn: 1000, tokensOut: 200, cost: 0.5, turns: 3, workingSecs: 300 },
    sessionCount: 2, sessionsWithoutCost: 1,
    sessions: [
      { session: 'aaaaaaaa-1', term: 5, agent: 'claude', project: 'shop', firstDay: '2026-11-04', lastDay: '2026-11-04', tokensIn: 1000, tokensOut: 200, cost: 0.5, turns: 3, workingSecs: 300 },
      { session: 'bbbbbbbb-2', term: null, agent: 'codex', project: 'api', firstDay: '2026-11-03', lastDay: '2026-11-03', ...NONE },
    ],
    days: [],
  }
}

function renderDesktopApp() {
  window.localStorage.setItem('maitty.gui.welcomed.v1', '1')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(<QueryClientProvider client={client}><App /></QueryClientProvider>)
}

describe('usage and cost in the desktop app', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadConversations.mockResolvedValue([term(5)])
    daemon.suggestProjectFolders.mockResolvedValue([])
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: false, sessionCount: 1, activeTurnCount: 0 })
    usage.loadUsageReport.mockResolvedValue(report())
  })

  const openUsage = async (user: ReturnType<typeof userEvent.setup>) => {
    await screen.findByRole('button', { name: 'Settings' })
    await user.keyboard('{Control>}{Shift>}u{/Shift}{/Control}')
    return screen.findByRole('region', { name: 'Usage and cost' })
  }

  it('opens Settings on Usage & cost with Ctrl+Shift+U, and Esc closes Settings', async () => {
    const user = userEvent.setup()
    renderDesktopApp()

    await openUsage(user)
    expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Usage & cost', level: 2 })).toBeInTheDocument()
    expect(usage.loadUsageReport).toHaveBeenCalledWith('week')

    await user.keyboard('{Escape}')
    await waitFor(() => { expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument() })
  })

  it('has no usage button in the sidebar footer: Settings holds it, one key away', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('button', { name: 'Settings' })
    expect(screen.queryByRole('button', { name: 'Usage and cost' })).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(within(screen.getByRole('navigation', { name: 'Settings sections' })).getByRole('button', { name: /Usage & cost/ }))
    expect(await screen.findByRole('region', { name: 'Usage and cost' })).toBeInTheDocument()
  })

  it('opens the pane of an open session from its row, and leaves a closed session alone', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    const page = await openUsage(user)
    const table = await within(page).findByRole('table', { name: /Sessions in this period/ })

    const closed = within(table).getAllByRole('row').find((row) => row.textContent.includes('api'))
    expect(closed).toBeDefined()
    await user.click(closed as HTMLElement)
    expect(screen.getByRole('region', { name: 'Usage and cost' })).toBeInTheDocument()

    await user.click(within(table).getByRole('button', { name: /Shell 5|~|shell/i }))
    await waitFor(() => { expect(screen.queryByRole('region', { name: 'Usage and cost' })).not.toBeInTheDocument() })
    expect(await screen.findByText('Pane 5')).toBeInTheDocument()
  })

  it('keeps a monthly budget with the other preferences', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await openUsage(user)
    await user.click(await screen.findByRole('button', { name: 'Set a monthly budget' }))
    await user.type(screen.getByRole('textbox', { name: /Monthly budget/ }), '40')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => { expect(JSON.parse(window.localStorage.getItem('maitty.gui.settings.v1') ?? '{}')).toMatchObject({ monthlyBudget: 40 }) })
    expect(screen.getByText('$40.00 monthly budget')).toBeInTheDocument()
  })
})
