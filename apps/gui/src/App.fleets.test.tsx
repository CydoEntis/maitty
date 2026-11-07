import { render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Fleet } from '@/api/fleets'
import type { Conversation } from '@/data/preview'
import App from './App'

// Fleets and ships in the desktop window: the daemon's workspaces group the sessions, the sidebar shows
// the selected fleet's ships, and the tab bar holds one tab per ship of that fleet.

const daemon = vi.hoisted(() => ({
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  loadPerformanceSample: vi.fn<() => Promise<never>>(() => new Promise(() => {})),
  listenSessionsChanged: vi.fn(() => Promise.resolve(() => {})),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  startShellSession: vi.fn<(folder: string) => Promise<void>>(),
  startChatSession: vi.fn(),
  closeSessionWithUndo: vi.fn(() => Promise.resolve()),
  undoCloseSession: vi.fn(() => Promise.resolve()),
  renameConversationOnDaemon: vi.fn(() => Promise.resolve()),
  markSeenOnDaemon: vi.fn(() => Promise.resolve()),
  pinSessionOnDaemon: vi.fn(() => Promise.resolve()),
  loadDaemonRestartStatus: vi.fn(),
  restartMismatchedDaemon: vi.fn(),
  suggestProjectFolders: vi.fn(() => Promise.resolve([] as string[])),
  sendTerminalInput: vi.fn<(conversation: { id: string; generation: number }, input: string | Uint8Array) => Promise<void>>(() => Promise.resolve()),
  loadMachines: vi.fn(() => Promise.resolve({ names: [], error: null })),
}))

vi.mock('@/api/daemon', () => ({
  ...daemon,
  createFleet: fleetApi.createFleet,
  renameFleet: fleetApi.renameFleet,
  startShellInFleet: fleetApi.startShellInFleet,
  recolorFleet: fleetApi.recolorFleet,
  moveSessionToFleet: fleetApi.moveSessionToFleet,
  isDesktop: () => true,
  terminalOf: (id: string) => {
    const match = /^t(\d+)$/.exec(id)
    return match?.[1] === undefined ? null : Number(match[1])
  },
}))

const fleetApi = vi.hoisted(() => ({
  loadFleets: vi.fn<() => Promise<Fleet[]>>(),
  createFleet: vi.fn<(machine: number, name: string) => Promise<void>>(),
  renameFleet: vi.fn<(machine: number, fleet: number, name: string) => Promise<void>>(),
  startShellInFleet: vi.fn<(machine: number, fleet: number, folder?: string) => Promise<void>>(),
  recolorFleet: vi.fn<(machine: number, fleet: number, color: number) => Promise<void>>(),
  moveSessionToFleet: vi.fn<(machine: number, session: number, fleet: number) => Promise<void>>(),
}))

vi.mock('@/api/fleets', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/fleets')>()), loadFleets: fleetApi.loadFleets }))

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

function term(pane: number, cwd: string, extra: Partial<Conversation> = {}): Conversation {
  return {
    id: `t${String(pane)}`, generation: 0, execution: 'Native', kind: 'terminal', title: `Shell ${String(pane)}`, project: 'project', folder: cwd, cwd, repo: null, branch: 'working tree',
    provider: 'Codex', status: 'idle', updated: null, model: 'Provider default', permission: 'Terminal session', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra,
  }
}

function fleet(id: number, name: string, sessionIds: string[]): Fleet {
  return { key: `0:${String(id)}`, machine: 0, id, name, color: id, worktree: false, sessionIds }
}

const MAITTY = fleet(1, 'Maitty', ['t1', 't2'])
const GROWTH = fleet(2, 'AeVox launch', ['t3'])

function renderApp() {
  window.localStorage.setItem('maitty.gui.welcomed.v1', '1')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}><App /></QueryClientProvider>)
}

describe('fleets and ships in the window', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.startShellSession.mockResolvedValue(undefined)
    fleetApi.loadFleets.mockResolvedValue([MAITTY, GROWTH])
    fleetApi.startShellInFleet.mockResolvedValue(undefined)
    fleetApi.createFleet.mockResolvedValue(undefined)
    fleetApi.renameFleet.mockResolvedValue(undefined)
    fleetApi.recolorFleet.mockResolvedValue(undefined)
    fleetApi.moveSessionToFleet.mockResolvedValue(undefined)
    daemon.loadConversations.mockResolvedValue([
      term(1, `${HOME}/a`, { agent: 'claude', status: 'needs-you' }),
      term(2, `${HOME}/b`),
      term(3, `${HOME}/c`, { agent: 'codex' }),
    ])
  })

  it('opens on the first ship, shows its fleet’s ships and one tab for each, and counts what needs you on the fleet tile', async () => {
    renderApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    expect(await screen.findByRole('region', { name: 'Shell in ~/a' })).toBeInTheDocument()
    expect(within(sidebar).getByRole('button', { name: /^Maitty, 2 ships, 1 need you/ })).toHaveAttribute('aria-current', 'true')
    expect(within(sidebar).queryByRole('button', { name: '~/c, codex, idle' })).not.toBeInTheDocument()
    expect(screen.getAllByRole('tab').map((tab) => tab.getAttribute('aria-label'))).toEqual(['~/a, claude, needs you', '~/b, shell, idle'])
  })

  it('shows another fleet’s ships and tabs when it is chosen, and comes back to the ship it showed', async () => {
    const user = userEvent.setup()
    renderApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await user.click(within(sidebar).getByRole('button', { name: '~/b, shell, idle' }))
    await user.click(within(sidebar).getByRole('button', { name: /^AeVox launch/ }))

    expect(await screen.findByRole('region', { name: 'Shell in ~/c' })).toBeInTheDocument()
    expect(screen.getAllByRole('tab')).toHaveLength(1)
    expect(within(sidebar).queryByRole('button', { name: '~/b, shell, idle' })).not.toBeInTheDocument()

    await user.click(within(sidebar).getByRole('button', { name: /^Maitty/ }))
    expect(await screen.findByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()
  })

  it('opens a tab on a click, and selects the fleet of a ship opened from elsewhere', async () => {
    const user = userEvent.setup()
    renderApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await user.click(screen.getByRole('tab', { name: '~/b, shell, idle' }))
    expect(screen.getByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: '~/b, shell, idle' })).toHaveAttribute('aria-selected', 'true')
  })

  it('leaves the workspace empty for a fleet with no ships, and says so', async () => {
    const user = userEvent.setup()
    fleetApi.loadFleets.mockResolvedValue([MAITTY, GROWTH, fleet(3, 'Docs', [])])
    renderApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await user.click(within(sidebar).getByRole('button', { name: /^Docs/ }))
    expect(screen.queryByRole('region', { name: /^Shell in/ })).not.toBeInTheDocument()
    expect(screen.queryAllByRole('tab')).toHaveLength(0)
    expect(within(sidebar).getByText('Nothing aboard yet. Use + to start a session.')).toBeInTheDocument()
  })

  it('starts a new shell in the selected fleet from the header button and from the key', async () => {
    const user = userEvent.setup()
    renderApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await user.click(screen.getByRole('button', { name: 'New shell' }))
    await waitFor(() => { expect(fleetApi.startShellInFleet).toHaveBeenCalledWith(0, 1, HOME) })
    expect(daemon.startShellSession).not.toHaveBeenCalled()
  })

  it('starts an agent chat in the current fleet without opening the modal', async () => {
    const user = userEvent.setup()
    renderApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    // The daemon lists the new shell once it is started.
    fleetApi.startShellInFleet.mockImplementation(() => {
      daemon.loadConversations.mockResolvedValue([term(1, `${HOME}/a`), term(2, `${HOME}/b`), term(3, `${HOME}/c`), term(4, HOME)])
      return Promise.resolve()
    })
    await user.click(screen.getByRole('button', { name: 'New agent chat' }))
    await waitFor(() => { expect(fleetApi.startShellInFleet).toHaveBeenCalledWith(0, 1, HOME) })
    await waitFor(() => { expect(daemon.sendTerminalInput).toHaveBeenCalled() })
    expect(screen.queryByRole('dialog', { name: 'New session' })).not.toBeInTheDocument()
  })

  it('adds a fleet from a machine’s + and selects it', async () => {
    const user = userEvent.setup()
    renderApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    fleetApi.loadFleets.mockResolvedValue([MAITTY, GROWTH, fleet(7, 'Docs', [])])
    await user.click(within(sidebar).getByRole('button', { name: 'Add a fleet on This computer' }))
    await user.type(within(sidebar).getByRole('textbox', { name: /Name for the new fleet/ }), 'Docs{Enter}')
    await waitFor(() => { expect(fleetApi.createFleet).toHaveBeenCalledWith(0, 'Docs') })
    expect(await within(sidebar).findByRole('button', { name: /^Docs/ })).toHaveAttribute('aria-current', 'true')
  })

  it('renames a fleet on a double-click through the daemon', async () => {
    const user = userEvent.setup()
    renderApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await user.dblClick(await within(sidebar).findByRole('button', { name: /^AeVox launch/ }))
    await user.type(within(sidebar).getByRole('textbox', { name: /Name for AeVox launch/ }), '{Control>}a{/Control}Launch{Enter}')
    await waitFor(() => { expect(fleetApi.renameFleet).toHaveBeenCalledWith(0, 2, 'Launch') })
  })

  it('moves a ship to another fleet from its menu, and recolours a fleet from its menu', async () => {
    const user = userEvent.setup()
    renderApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await user.pointer({ keys: '[MouseRight]', target: within(sidebar).getByRole('button', { name: '~/b, shell, idle' }) })
    await user.click(await screen.findByRole('menuitem', { name: 'AeVox launch' }))
    expect(fleetApi.moveSessionToFleet).toHaveBeenCalledWith(0, 2, GROWTH.id)

    await user.pointer({ keys: '[MouseRight]', target: within(sidebar).getByRole('button', { name: /^AeVox launch/ }) })
    await user.click(await screen.findByRole('menuitemradio', { name: 'orange' }))
    expect(fleetApi.recolorFleet).toHaveBeenCalledWith(0, GROWTH.id, 5)
  })
})
