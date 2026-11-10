import { LOCAL_MACHINE } from '@/api/machines'
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import App from './App'
import { toast } from 'sonner'
import { hotkeyRows } from './features/workspace/model/shortcuts'

const daemon = vi.hoisted(() => ({
  connectAgent: vi.fn(() => Promise.resolve()),
  loadAgentModels: vi.fn(() => Promise.resolve([] as { id: string; label: string; blurb: string }[])),
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  // Opening Settings > Performance asks the host for a sample; this page has nothing to show before one arrives.
  loadPerformanceSample: vi.fn<() => Promise<never>>(() => new Promise(() => {})),
  listenSessionsChanged: vi.fn(() => Promise.resolve(() => {})),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  startShellSession: vi.fn<(folder: string) => Promise<void>>(),
  startChatSession: vi.fn<(folder: string, agent: string, branch: string | null, fullAccess: boolean) => Promise<{ id: number; generation: number }>>(),
  archiveConversationOnDaemon: vi.fn<(conversation: { id: string }, archived: boolean) => Promise<void>>(),
  closeSessionWithUndo: vi.fn<(conversation: { id: string }) => Promise<void>>(),
  undoCloseSession: vi.fn<(conversation: { id: string }) => Promise<void>>(),
  renameConversationOnDaemon: vi.fn<(conversation: { id: string }, title: string) => Promise<void>>(),
  markSeenOnDaemon: vi.fn<(conversation: { id: string }) => Promise<void>>(),
  loadDaemonRestartStatus: vi.fn(),
  restartMismatchedDaemon: vi.fn<(protocol: number) => Promise<void>>(),
  openDaemonLog: vi.fn<() => Promise<void>>(),
  suggestProjectFolders: vi.fn<(typed: string) => Promise<string[]>>(),
  sendTerminalInput: vi.fn<(conversation: { id: string; generation: number }, input: string | Uint8Array) => Promise<void>>(),
  loadMachines: vi.fn(() => Promise.resolve({ names: [], error: null })),
}))

vi.mock('@/api/daemon', () => ({
  ...daemon,
  isDesktop: () => true,
  terminalOf: (id: string) => {
    const match = /^t(\d+)$/.exec(id)
    return match?.[1] === undefined ? null : Number(match[1])
  },
}))

const terminal = vi.hoisted(() => ({
  pasted: [] as Array<[string, string]>,
  drop: { current: null as null | ((paths: string[], at: { x: number; y: number }) => void) },
}))

// The window follows other machines through the host; there are none here.
const alerts = vi.hoisted(() => ({ emit: null as ((attention: { machine: number; term: number; status: 'blocked' | 'done' }) => void) | null }))

vi.mock('@/api/machines', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/machines')>()),
  loadMachinesOverview: () => Promise.resolve([]),
  listenMachineState: () => Promise.resolve(() => undefined),
  listenMachineSessions: () => Promise.resolve(() => undefined),
  listenMachineError: () => Promise.resolve(() => undefined),
  listenMachineRemoved: () => Promise.resolve(() => undefined),
  listenMachineNotice: () => Promise.resolve(() => undefined),
  listenAttention: (onAttention: (attention: { machine: number; term: number; status: 'blocked' | 'done' }) => void) => {
    alerts.emit = onAttention
    return Promise.resolve(() => undefined)
  },
  listenMachineClipboard: () => Promise.resolve(() => undefined),
}))

vi.mock('@/features/terminal/useFileDrop', () => ({
  useFileDrop: (onDrop: (paths: string[], at: { x: number; y: number }) => void) => { terminal.drop.current = onDrop },
}))

vi.mock('./TerminalView', () => ({
  TerminalView: ({ conversation, label, focused, inputDisabled, onHandle }: { conversation: Conversation; label: string; focused: boolean; inputDisabled: boolean; onHandle?: (handle: { find: () => boolean; clearFind: () => void; focus: () => void; paste: (text: string) => void } | null) => void }) => (
    <div ref={() => { onHandle?.({ find: () => false, clearFind: () => {}, focus: () => {}, paste: (text) => { terminal.pasted.push([conversation.id, text]) } }) }} className="terminal-screen" role="region" aria-label={label} data-focused={String(focused)} data-input-disabled={String(inputDisabled)} tabIndex={0}>
      Pane {conversation.nativePane}
    </div>
  ),
}))

const HOME = '/home/casey'

function term(pane: number, cwd: string, extra: Partial<Conversation> = {}): Conversation {
  return {
    id: `t${String(pane)}`,
    generation: 0,
    execution: 'Native',
    kind: 'terminal',
    title: `Shell ${String(pane)}`,
    project: 'project',
    folder: cwd,
    cwd,
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

const inRepo = (pane: number, root: string, cwd: string, extra: Partial<Conversation> = {}) =>
  term(pane, cwd, { folder: root, repo: root.split('/').at(-1) ?? null, branch: 'main', ...extra })

describe('desktop terminal multiplexer', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    toast.dismiss()
    window.localStorage.clear()
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: true }))
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadConversations.mockResolvedValue([])
    daemon.startShellSession.mockResolvedValue(undefined)
    daemon.archiveConversationOnDaemon.mockResolvedValue(undefined)
    daemon.closeSessionWithUndo.mockResolvedValue(undefined)
    daemon.undoCloseSession.mockResolvedValue(undefined)
    daemon.renameConversationOnDaemon.mockResolvedValue(undefined)
    daemon.markSeenOnDaemon.mockResolvedValue(undefined)
    daemon.suggestProjectFolders.mockResolvedValue([])
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: false, sessionCount: 2, activeTurnCount: 1 })
  })

  it('explains how to start when no session is running, and opens Settings at the start folder', async () => {
    const user = userEvent.setup()
    renderDesktopApp()

    expect(await screen.findByRole('heading', { name: 'No sessions running' })).toBeInTheDocument()
    expect(screen.getByText(/A new session opens a shell in/)).toHaveTextContent('A new session opens a shell in ~. cd into a project')
    expect(screen.getByText('No agents running. Use the buttons above to start one.')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Change start folder' }))
    const dialog = screen.getByRole('dialog', { name: 'Settings' })
    expect(within(dialog).getByRole('heading', { name: 'General' })).toBeInTheDocument()
    expect(within(dialog).getByRole('textbox', { name: 'Start folder' })).toHaveValue('')
  })

  it('starts a plain shell in the home folder from New terminal and shows it', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    daemon.loadConversations.mockResolvedValue([term(5, HOME)])

    await user.keyboard('{Control>}{Alt>}{Shift>}t{/Shift}{/Alt}{/Control}')

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(HOME) })
    expect(daemon.startChatSession).not.toHaveBeenCalled()
    expect(await screen.findByRole('region', { name: 'Shell in ~' })).toHaveTextContent('Pane 5')
    expect(screen.getByRole('region', { name: /^Terminal 1: ~/ })).toBeInTheDocument()
  })

  it('opens the new agent chat page with Ctrl+T, and Escape goes back to the panes', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })

    await user.keyboard('{Control>}{Shift>}t{/Shift}{/Control}')
    const page = await screen.findByRole('region', { name: 'New agent chat' })
    // The pickers sit in the box and the tray under it, as in a chat.
    expect(within(page).getByRole('button', { name: /^Model and reasoning/ })).toBeInTheDocument()
    expect(within(page).getByRole('button', { name: /^Folder/ })).toBeInTheDocument()
    expect(daemon.startShellSession).not.toHaveBeenCalled()

    await waitFor(() => { expect(within(page).getByRole('textbox', { name: 'First message' })).toHaveFocus() })
    await user.keyboard('{Escape}')
    await waitFor(() => { expect(screen.queryByRole('region', { name: 'New agent chat' })).not.toBeInTheDocument() })
  })

  it('keeps the window’s shortcuts on the new agent chat page, and not the ones for the pane under it', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await user.keyboard('{Control>}{Shift>}t{/Shift}{/Control}')
    await screen.findByRole('region', { name: 'New agent chat' })

    await user.keyboard('{Control>}{Shift>}d{/Shift}{/Control}')
    expect(daemon.startShellSession).not.toHaveBeenCalled()

    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument()
  })

  it('applies the saved density, text sizes and sidebar width, and a drag of the edge changes and saves the width', async () => {
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'compact', sidebarTextSize: 15, tabTextSize: 12, sidebarWidth: 320 }))
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()
      const handle = await screen.findByRole('separator', { name: 'Resize sidebar' })
      const root = document.querySelector<HTMLElement>('.mx-app')
      expect(root?.style.getPropertyValue('--pad')).toBe('0.8')
      expect(root?.style.getPropertyValue('--sidebar-text')).toBe(String(15 / 13.5))
      expect(root?.style.getPropertyValue('--tab-text')).toBe(String(12 / 13.5))
      expect(root?.style.getPropertyValue('--sidebar-w')).toBe('320px')

      fireEvent.pointerDown(handle, { clientX: 320, pointerId: 1 })
      fireEvent.pointerMove(handle, { clientX: 380, pointerId: 1 })
      expect(root?.style.getPropertyValue('--sidebar-w')).toBe('380px')
      fireEvent.pointerUp(handle, { clientX: 380, pointerId: 1 })

      expect(root?.style.getPropertyValue('--sidebar-w')).toBe('380px')
      await waitFor(() => { expect(window.localStorage.getItem('maitty.gui.settings.v1')).toContain('"sidebarWidth":380') })
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('keeps the 60px rail in a window too narrow for the sidebar, and expands it over the panes', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1000 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()

      const rail = await screen.findByRole('navigation', { name: 'Sessions' })
      expect(rail).toHaveClass('is-rail')

      await user.click(within(rail).getByRole('button', { name: 'Expand sidebar' }))
      const drawer = await screen.findByRole('dialog', { name: 'Sessions' })
      const sidebar = within(drawer).getByRole('navigation', { name: 'Sessions' })
      expect(sidebar).not.toHaveClass('is-rail')

      await user.click(within(sidebar).getByRole('button', { name: /, shell, / }))
      expect(screen.queryByRole('dialog', { name: 'Sessions' })).not.toBeInTheDocument()

      await user.click(within(rail).getByRole('button', { name: 'Expand sidebar' }))
      await screen.findByRole('dialog', { name: 'Sessions' })
      await user.keyboard('{Escape}')
      expect(screen.queryByRole('dialog', { name: 'Sessions' })).not.toBeInTheDocument()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('keeps the sidebar docked in a window wide enough for it', async () => {
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()

      expect(await screen.findByRole('navigation', { name: 'Sessions' })).not.toHaveClass('is-rail')
      expect(screen.queryByRole('dialog', { name: 'Sessions' })).not.toBeInTheDocument()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('draws each known agent\'s mark on its own tile', async () => {
    const agents = ['claude', 'codex', 'deepseek', 'gemini', 'opencode']
    daemon.loadConversations.mockResolvedValue(agents.map((agent, index) => term(index + 2, `${HOME}/code/${agent}`, { agent })))
    renderDesktopApp()

    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await within(sidebar).findByRole('button', { name: /, claude, / })
    for (const agent of agents) expect(sidebar.querySelector(`.mx-app-icon.is-${agent} svg`)).not.toBeNull()
  })

  it('names each session after the agent or program it is using', async () => {
    daemon.loadConversations.mockResolvedValue([
      term(2, `${HOME}/code/a`, { agent: 'deepseek' }),
      term(3, `${HOME}/code/b`, { process: 'cargo' }),
      term(4, `${HOME}/code/c`, { process: 'zsh' }),
    ])
    renderDesktopApp()

    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    expect(await within(sidebar).findByRole('button', { name: /, deepseek, / })).toBeInTheDocument()
    expect(within(sidebar).getByRole('button', { name: /, cargo, / })).toBeInTheDocument()
    expect(within(sidebar).getByRole('button', { name: /, shell, / })).toBeInTheDocument()
  })

  it('shows a pane its status as the badge on its icon, with no chip in a corner', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { status: 'working', agent: 'codex' })])
    renderDesktopApp()

    const pane = await screen.findByRole('region', { name: /^Terminal 1: / })
    expect(pane.querySelector('.mx-pane-icon .mx-badge.is-working')).not.toBeNull()
    expect(screen.queryByRole('button', { name: /^Status:/ })).not.toBeInTheDocument()
  })

  it('draws no card and no focus ring round a lone pane', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()

    const pane = await screen.findByRole('region', { name: 'Terminal 1: ~/a, focused' })
    expect(pane).not.toHaveClass('is-focused')
    expect(pane).not.toHaveClass('is-card')
  })

  it('leaves an idle pane without a status chip and an idle row without a state line', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { status: 'idle' })])
    renderDesktopApp()

    await screen.findByRole('region', { name: /^Terminal 1: / })
    expect(screen.queryByRole('button', { name: /^Status:/ })).not.toBeInTheDocument()
    const sidebar = screen.getByRole('navigation', { name: 'Sessions' })
    const row = await within(sidebar).findByRole('button', { name: 'a, shell, idle' })
    expect(within(row).queryByText('idle')).not.toBeInTheDocument()
  })

  it('opens a menu on right-click in a pane and splits from it', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1: / })

    fireEvent.contextMenu(pane)

    const menu = await screen.findByRole('menu', { name: 'a' })
    expect(within(menu).getAllByRole('menuitem').map((item) => item.querySelector('.mx-ctx-label')?.textContent)).toEqual(['Split right', 'Split down', 'Maximize pane', 'Clear', 'Copy folder path', 'Files', 'Changes', 'Close pane'])
    daemon.startShellSession.mockResolvedValue(undefined)
    await user.click(within(menu).getByRole('menuitem', { name: /^Split right/ }))
    // A split opens a new shell beside this one.
    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalled() })
  })

  it('renames a session in place from its row menu, and the name shows on the row and the pane', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    fireEvent.contextMenu(await within(sidebar).findByRole('button', { name: /, shell, / }))
    await user.click(await screen.findByRole('menuitem', { name: /^Rename…/ }))

    // The row's own title becomes the field, with the cursor in it and the folder as its placeholder.
    const field = await within(sidebar).findByRole('textbox', { name: 'Session name' })
    await waitFor(() => { expect(field).toHaveFocus() })
    expect(field).toHaveAttribute('placeholder', 'a')
    // Once saved, the daemon reports the name, and every place that shows the session uses it.
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { named: true, title: 'Login fix' })])
    await user.keyboard('Login fix{Enter}')

    await waitFor(() => { expect(daemon.renameConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, 'Login fix') })
    expect(within(sidebar).queryByRole('textbox', { name: 'Session name' })).not.toBeInTheDocument()
    await within(sidebar).findByRole('button', { name: /^Login fix, shell, / })
    expect(document.querySelector('.mx-pane-name')).toHaveTextContent('Login fix')
  })

  it('cancels an in-place rename with Escape and sends nothing, and an empty name clears it', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { named: true, title: 'Login fix' })])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    fireEvent.contextMenu(await within(sidebar).findByRole('button', { name: /^Login fix, shell, / }))
    await user.click(await screen.findByRole('menuitem', { name: /^Rename…/ }))
    const field = await within(sidebar).findByRole('textbox', { name: 'Session name' })
    expect(field).toHaveValue('Login fix')
    await waitFor(() => { expect(field).toHaveFocus() })

    await user.keyboard('{Escape}')
    expect(within(sidebar).queryByRole('textbox', { name: 'Session name' })).not.toBeInTheDocument()
    expect(daemon.renameConversationOnDaemon).not.toHaveBeenCalled()

    fireEvent.contextMenu(within(sidebar).getByRole('button', { name: /^Login fix, shell, / }))
    await user.click(await screen.findByRole('menuitem', { name: /^Rename…/ }))
    await user.clear(await within(sidebar).findByRole('textbox', { name: 'Session name' }))
    await user.keyboard('{Enter}')
    await waitFor(() => { expect(daemon.renameConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, '') })
  })

  it('has no Archive: the menu offers none, and a session archived earlier shows as a normal row', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`, { archived: true })])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    const row = await within(sidebar).findByRole('button', { name: 'b, shell, idle' })
    expect(within(sidebar).queryByRole('region', { name: 'Archived sessions' })).not.toBeInTheDocument()

    fireEvent.contextMenu(row)

    expect(await screen.findByRole('menuitem', { name: /^Rename…/ })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Archive' })).not.toBeInTheDocument()
  })

  it('names a row after the session\'s own name, and after its program when it has none', async () => {
    daemon.loadConversations.mockResolvedValue([
      term(2, `${HOME}/a`, { agent: 'claude' }),
      term(3, `${HOME}/a`, { title: 'Growth Tests', named: true }),
    ])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    const named = await within(sidebar).findByRole('button', { name: /^Growth Tests, shell/ })

    expect(named.querySelector('.mx-row-name')).toHaveTextContent('Growth Tests')
    expect(within(sidebar).getByRole('button', { name: /, claude, / }).querySelector('.mx-row-name')).toHaveTextContent('claude')
  })

  it('moves through a session menu with the arrow keys and gives focus back when it closes', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const row = await screen.findByRole('button', { name: 'a, shell, idle' })
    row.focus()
    fireEvent.contextMenu(row)
    const open = await screen.findByRole('menuitem', { name: 'Open' })
    expect(open).toHaveFocus()

    await user.keyboard('{ArrowDown}')
    expect(screen.getByRole('menuitem', { name: /^Rename…/ })).toHaveFocus()
    await user.keyboard('{ArrowUp}{ArrowUp}')
    expect(screen.getByRole('menuitem', { name: 'Close shell' })).toHaveFocus()
    await user.keyboard('{Home}')
    expect(open).toHaveFocus()

    await user.keyboard('{Escape}')
    await waitFor(() => { expect(screen.queryByRole('menuitem', { name: 'Open' })).not.toBeInTheDocument() })
    expect(row).toHaveFocus()
  })

  it('shows no diff counts and no folder badge on a row, and leaves "working" to the badge', async () => {
    daemon.loadConversations.mockResolvedValue([inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty`, { agent: 'claude', status: 'working', branch: 'agent/grow-028', diffAdded: 104, diffRemoved: 61 })])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })

    const row = await within(sidebar).findByRole('button', { name: /, claude, working/ })

    expect(row).toHaveTextContent('claude')
    expect(row).toHaveTextContent('agent/grow-028')
    expect(row).not.toHaveTextContent('+104')
    expect(row).not.toHaveTextContent('−61')
    expect(row).not.toHaveTextContent('working')
    expect(within(sidebar).queryByRole('button', { name: 'Show only maitty' })).not.toBeInTheDocument()
  })

  it('starts a new terminal in the start folder from Settings, with ~ meaning the home folder', async () => {
    const user = userEvent.setup()
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ startFolder: '~/code' }))
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })

    await user.keyboard('{Control>}{Alt>}{Shift>}t{/Shift}{/Alt}{/Control}')

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(`${HOME}/code`) })
  })

  it('lists every session of this computer in the order it was created, each named by its folder or agent', async () => {
    daemon.loadConversations.mockResolvedValue([
      term(9, `${HOME}/Downloads`),
      inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty/crates/maitty-core`, { agent: 'codex', status: 'working' }),
      inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty`),
      inRepo(4, `${HOME}/code/docs`, `${HOME}/code/docs`, { status: 'needs-you' }),
    ])
    renderDesktopApp()

    const sidebar = await screen.findByRole('navigation', { name: 'Sessions' })
    await within(sidebar).findByRole('button', { name: 'maitty, shell, idle' })
    const rows = within(sidebar).getAllByRole('button', { name: /, (shell|codex), / }).map((row) => row.getAttribute('aria-label'))
    expect(rows).toEqual(['maitty, shell, idle', 'codex, codex, working', 'docs, shell, needs you', 'Downloads, shell, idle'])
    expect(sidebar).not.toHaveTextContent('maittyd ·')
  })

  it('opens the first session on launch, and a sidebar session that is open nowhere gets a layout of its own', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    renderDesktopApp()

    expect(await screen.findByRole('region', { name: 'Shell in ~/a' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'b, shell, idle' }))

    expect(screen.getByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Shell in ~/a' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'b, shell, idle' })).toHaveAttribute('aria-current', 'page')
    expect(screen.queryByRole('tab')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'a, shell, idle' }))
    expect(screen.getByRole('region', { name: 'Shell in ~/a' })).toBeInTheDocument()
  })

  it('keeps the terminal of the session you left mounted out of sight, and shows that same one when you go back', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    renderDesktopApp()
    const first = await screen.findByRole('region', { name: 'Shell in ~/a' })

    await user.click(screen.getByRole('button', { name: 'b, shell, idle' }))

    expect(first).toBeInTheDocument()
    const kept = first.closest('.mx-panes')
    expect(kept).toHaveClass('is-kept')
    expect(kept).toHaveAttribute('aria-hidden', 'true')
    expect(kept).toHaveAttribute('inert')
    expect(first).toHaveAttribute('data-focused', 'false')

    await user.click(screen.getByRole('button', { name: 'a, shell, idle' }))

    expect(screen.getByRole('region', { name: 'Shell in ~/a' })).toBe(first)
    expect(first.closest('.mx-panes')).not.toHaveClass('is-kept')
    expect(first).toHaveAttribute('data-focused', 'true')
    expect(screen.getByText('Pane 3').closest('.mx-panes')).toHaveClass('is-kept')
  })

  it('splits right in the focused shell’s folder, moves focus, and ends the focused pane’s session on Ctrl+Shift+W', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/code/maitty/apps' })
    daemon.loadConversations.mockResolvedValue([inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`), inRepo(7, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`)])

    await user.keyboard('{Control>}{Shift>}d{/Shift}{/Control}')

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(`${HOME}/code/maitty/apps`) })
    expect(await screen.findByRole('region', { name: 'Terminal 2: ~/code/maitty/apps, focused' })).toBeInTheDocument()
    await user.keyboard('{Control>}[[{/Control}')
    expect(screen.getByRole('region', { name: 'Terminal 1: ~/code/maitty/apps, focused' })).toBeInTheDocument()

    await user.keyboard('{Control>}{Shift>}w{/Shift}{/Control}')
    await waitFor(() => { expect(screen.getAllByRole('region', { name: /^Terminal \d/ })).toHaveLength(1) })
    expect(await screen.findByText('Closed apps')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Undo' })).toBeInTheDocument()
    expect(daemon.closeSessionWithUndo).toHaveBeenCalledTimes(1)
    expect(daemon.closeSessionWithUndo).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' }))
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('swaps two panes’ sessions when a header is dropped on another pane', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    await user.click(screen.getByRole('button', { name: 'Split down (Ctrl+Alt+Shift+D)' }))
    await screen.findByRole('region', { name: 'Shell in ~/b' })

    const first = screen.getByRole('region', { name: /^Terminal 1:/ })
    const second = screen.getByRole('region', { name: /^Terminal 2:/ })
    const header = first.querySelector('.mx-pane-head')
    if (!header) throw new Error('pane header missing')
    fireEvent.pointerDown(header, { button: 0, clientX: 10, clientY: 10 })
    fireEvent.pointerMove(window, { clientX: 40, clientY: 40 })
    fireEvent.pointerMove(second, { clientX: 60, clientY: 200 })
    fireEvent.pointerUp(second, { clientX: 60, clientY: 200 })

    expect(screen.getByRole('region', { name: 'Terminal 1: ~/b' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Terminal 2: ~/a, focused' })).toBeInTheDocument()
  })

  it('closes a pane\'s session from its × and offers Undo, which brings the pane and the session back', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await user.click(screen.getByRole('button', { name: 'Close pane · ends its session' }))

    await waitFor(() => { expect(daemon.closeSessionWithUndo).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
    expect((await screen.findAllByText('Closed ~/a')).length).toBeGreaterThan(0)
    expect(screen.queryByRole('region', { name: 'Shell in ~/a' })).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Undo' }))
    await waitFor(() => { expect(daemon.undoCloseSession).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })
    expect(await screen.findByRole('region', { name: 'Shell in ~/a' })).toBeInTheDocument()
  })

  it('keeps a layout change made while a close was still on its way to the daemon', async () => {
    const user = userEvent.setup()
    const a = term(2, `${HOME}/a`)
    const b = term(7, `${HOME}/a`)
    const c = term(9, `${HOME}/a`)
    daemon.loadConversations.mockResolvedValue([a])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    daemon.loadConversations.mockResolvedValue([a, b])
    await user.keyboard('{Control>}{Shift>}d{/Shift}{/Control}')
    await screen.findByRole('region', { name: 'Terminal 2: ~/a, focused' })
    daemon.loadConversations.mockResolvedValue([a, b, c])
    await user.keyboard('{Control>}{Shift>}d{/Shift}{/Control}')
    const third = await screen.findByRole('region', { name: 'Terminal 3: ~/a, focused' })
    let ended: () => void = () => undefined
    daemon.closeSessionWithUndo.mockReturnValue(new Promise<void>((resolve) => { ended = resolve }))

    await user.click(within(screen.getByRole('region', { name: 'Terminal 1: ~/a' })).getByRole('button', { name: 'Close pane · ends its session' }))
    await waitFor(() => { expect(daemon.closeSessionWithUndo).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
    await user.click(within(third).getByRole('button', { name: 'Maximize pane' }))
    await screen.findByRole('button', { name: 'Restore layout' })
    await act(async () => { ended(); await Promise.resolve() })

    await waitFor(() => { expect(screen.getAllByText('Closed ~/a').length).toBeGreaterThan(0) })
    expect(screen.getByRole('button', { name: 'Restore layout' })).toBeInTheDocument()
  })

  it('undoes the last close with Ctrl+Shift+Z, and leaves the key to the terminal when nothing was closed', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    const passedOn = fireEvent.keyDown(document.body, { key: 'Z', ctrlKey: true, shiftKey: true, cancelable: true })
    expect(passedOn).toBe(true)
    expect(daemon.undoCloseSession).not.toHaveBeenCalled()

    await user.click(screen.getByRole('button', { name: 'Close pane · ends its session' }))
    await screen.findAllByText('Closed ~/a')
    await user.keyboard('{Control>}{Shift>}z{/Shift}{/Control}')

    await waitFor(() => { expect(daemon.undoCloseSession).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
  })

  it('asks about the pane you are in when Ctrl+Shift+W is pressed, and Cancel closes nothing', async () => {
    const user = userEvent.setup()
    const claude = inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`, { agent: 'claude' })
    daemon.loadConversations.mockResolvedValue([claude])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Terminal 1:/ })
    daemon.loadConversations.mockResolvedValue([claude, inRepo(7, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`, { agent: 'codex' })])
    await user.keyboard('{Control>}{Shift>}d{/Shift}{/Control}')
    await screen.findByRole('region', { name: /^Terminal 2:.*focused/ })

    await user.keyboard('{Control>}{Shift>}w{/Shift}{/Control}')
    expect(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Stop codex' })).toBeInTheDocument()
    await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    await user.keyboard('{Control>}[[{/Control}')
    await user.keyboard('{Control>}{Shift>}w{/Shift}{/Control}')
    expect(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Stop claude' })).toBeInTheDocument()

    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('does not pass Esc on to the terminal when it leaves focus mode', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    fireEvent.keyDown(document.body, { key: '.', ctrlKey: true })
    await waitFor(() => { expect(screen.queryByLabelText('Status')).not.toBeInTheDocument() })

    const passedOn = fireEvent.keyDown(document.body, { key: 'Escape', cancelable: true })

    expect(passedOn).toBe(false)
    await waitFor(() => { expect(screen.getByLabelText('Status')).toBeInTheDocument() })
  })

  it('asks before closing a pane that runs an agent', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { agent: 'claude', status: 'idle' })])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await user.click(screen.getByRole('button', { name: 'Close pane · ends its session' }))
    expect(daemon.closeSessionWithUndo).not.toHaveBeenCalled()
await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Stop claude' }))

    await waitFor(() => { expect(daemon.closeSessionWithUndo).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
  })

  it('ends a shell with nothing running in it at once, without asking, and offers Undo', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const row = await screen.findByRole('button', { name: 'a, shell, idle' })

    fireEvent.contextMenu(row)
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Close shell' }))

    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    expect(await screen.findByRole('button', { name: 'Undo' })).toBeInTheDocument()
    await waitFor(() => { expect(daemon.closeSessionWithUndo).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
  })

  it('asks before the row menu’s Stop session stops an agent', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { agent: 'claude', status: 'idle' })])
    renderDesktopApp()
    const row = await screen.findByRole('button', { name: /^claude, claude, / })

    fireEvent.contextMenu(row)
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Stop session' }))
    expect(daemon.closeSessionWithUndo).not.toHaveBeenCalled()
    fireEvent.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Stop claude' }))

    await waitFor(() => { expect(daemon.closeSessionWithUndo).toHaveBeenCalledWith(expect.objectContaining({ id: 't2' })) })
  })

  it('keeps a pane when its session stops being listed, as an ended pane with its name', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    daemon.loadConversations.mockResolvedValue([])
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    expect(await screen.findByRole('status', { name: 'Session ended' })).toHaveTextContent('Restart shell')
    expect(screen.getByRole('region', { name: /^Terminal 1: ~\/a/ })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Status:/ })).not.toBeInTheDocument()
  })

  it('closes an ended pane without stopping anything', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    daemon.loadConversations.mockResolvedValue([])
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    await user.click(within(await screen.findByRole('status', { name: 'Session ended' })).getByRole('button', { name: 'Close pane' }))

    await waitFor(() => { expect(screen.queryByRole('region', { name: /^Terminal 1:/ })).not.toBeInTheDocument() })
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
    expect(daemon.closeSessionWithUndo).not.toHaveBeenCalled()
  })

  it('restarts an ended pane with a new shell in the same folder, in the same place', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    daemon.loadConversations.mockResolvedValue([])
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })
    daemon.loadConversations.mockResolvedValue([term(9, `${HOME}/a`)])

    await user.click(within(await screen.findByRole('status', { name: 'Session ended' })).getByRole('button', { name: 'Restart shell' }))

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(`${HOME}/a`) })
    await waitFor(() => { expect(screen.queryByRole('status', { name: 'Session ended' })).not.toBeInTheDocument() })
    expect(screen.getByRole('region', { name: /^Terminal 1: ~\/a/ })).toBeInTheDocument()
  })

  it('keeps every session as it was after one lost read, and reads again', async () => {
    daemon.loadConversations
      .mockResolvedValueOnce([term(2, `${HOME}/a`, { status: 'working', agent: 'codex' })])
      .mockRejectedValueOnce(new Error('connection refused'))
      .mockResolvedValue([term(2, `${HOME}/a`, { status: 'working', agent: 'codex' })])
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    expect(screen.queryByText('Reconnecting to maittyd')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'codex, codex, working' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Shell in ~/a' })).not.toHaveAttribute('data-input-disabled', 'true')
    await waitFor(() => { expect(daemon.loadConversations).toHaveBeenCalledTimes(3) }, { timeout: 3000 })
    expect(screen.queryByText('Reconnecting to maittyd')).not.toBeInTheDocument()
  })

  it('shows the disconnected banner, marks every status Unknown and holds input', async () => {
    daemon.loadConversations.mockResolvedValueOnce([term(2, `${HOME}/a`, { status: 'working', agent: 'codex' })]).mockRejectedValue(new Error('connection refused'))
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    expect(await screen.findByText('Reconnecting to maittyd')).toBeInTheDocument()
    expect(screen.getByText(/Try 2 of 5 · next in 2s/)).toBeInTheDocument()
    // The row says only that it is unknown; "last seen working" now lives in its tooltip and hover card.
    expect(screen.getByRole('button', { name: 'codex, codex, unknown' })).toBeInTheDocument()
    expect(screen.getAllByText('Reconnecting to maittyd…')).toHaveLength(2)
    expect(within(screen.getByLabelText('Status')).getByRole('status')).toHaveTextContent('Reconnecting to maittyd…')
    expect(screen.getByRole('region', { name: 'Shell in ~/a' })).toHaveAttribute('data-input-disabled', 'true')
    expect(screen.getByRole('button', { name: 'New agent chat' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'codex, codex, unknown' })).not.toHaveClass('mx-fx-working')

    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    await userEvent.setup().click(screen.getByRole('button', { name: 'Retry now' }))
    await waitFor(() => { expect(screen.queryByText('Reconnecting to maittyd')).not.toBeInTheDocument() })
  })

  it('calls the service stopped after five failed tries, and Start service tries again', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValueOnce([term(2, `${HOME}/a`)]).mockRejectedValue(new Error('connection refused'))
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    for (let attempt = 0; attempt < 5; attempt += 1) await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    const card = await screen.findByRole('region', { name: 'Maitty service stopped' })
    expect(within(card).getByText('Your session is saved. Start the service to pick up where it left off.')).toBeInTheDocument()
    expect(within(card).getByText('Details')).toBeInTheDocument()
    expect(within(screen.getByLabelText('Status')).queryByText('Service stopped')).toBeNull()
    expect(screen.queryByText('Reconnecting to maittyd')).not.toBeInTheDocument()

    daemon.openDaemonLog.mockResolvedValue(undefined)
    await user.click(within(card).getByRole('button', { name: 'Open log' }))
    expect(daemon.openDaemonLog).toHaveBeenCalledOnce()

    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    await user.click(within(card).getByRole('button', { name: 'Start service' }))
    await waitFor(() => { expect(screen.queryByRole('region', { name: 'Maitty service stopped' })).not.toBeInTheDocument() })
  })

  it('asks to restart an older service, and Later leaves a slim bar', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockRejectedValue('connecting to the local daemon: the running maitty service uses protocol v49, while this GUI needs v62')
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 62, canRestart: true, sessionCount: 2, activeTurnCount: 0 })
    renderDesktopApp()

    const card = await screen.findByRole('region', { name: 'Restart Maitty to finish updating' })
    expect(within(card).getByText('The app updated, but the service behind it is still the older one.')).toBeInTheDocument()
    expect(within(card).getByText('protocol 49 → 62')).toBeInTheDocument()
    // The card is the one message: the status bar agrees with it instead of saying it is reconnecting.
    expect(within(screen.getByLabelText('Status')).getByText('Restart needed')).toBeInTheDocument()
    expect(within(screen.getByLabelText('Status')).queryByText(/Reconnecting/)).toBeNull()
    await user.click(within(card).getByRole('button', { name: 'Later' }))
    expect(screen.queryByRole('region', { name: 'Restart Maitty to finish updating' })).not.toBeInTheDocument()
    expect(screen.getByText('Restart needed.')).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: 'Restart and restore 2 sessions' })).toBeInTheDocument()
  })

  it('leaves an older daemon running while it owns sessions it is not saving', async () => {
    daemon.loadConversations.mockRejectedValue('connecting to the local daemon: the running server is another maitty version (protocol v49, this one v59); run maitty in a terminal to restart it')
    renderDesktopApp()

    expect(await screen.findByRole('button', { name: 'Service in use' })).toBeDisabled()
    expect(screen.getByText('The app updated, but the service behind it is still the older one.')).toBeInTheDocument()
    expect(screen.getByText(/owns 2 sessions but is not saving them, so Maitty leaves it running/)).toBeInTheDocument()
    expect(daemon.restartMismatchedDaemon).not.toHaveBeenCalled()
  })

  it('asks to update the app, not restart the service, when the service is newer than the app', async () => {
    daemon.loadConversations.mockRejectedValue('connecting to the local daemon: the running maitty service uses protocol v77, while this GUI needs v76')
    renderDesktopApp()

    const card = await screen.findByRole('region', { name: 'Update Maitty to reconnect' })
    expect(within(card).getByText(/The service behind this app is newer than the app\./)).toBeInTheDocument()
    expect(within(card).getByText('protocol 76 → 77')).toBeInTheDocument()
    expect(within(card).getByRole('button', { name: 'Check again' })).toBeEnabled()
    expect(screen.queryByRole('region', { name: 'Restart Maitty to finish updating' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Service in use' })).not.toBeInTheDocument()
    expect(daemon.restartMismatchedDaemon).not.toHaveBeenCalled()
  })

  it('offers to restart an older daemon that owns saved sessions, says what survives, and asks twice', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockRejectedValue('connecting to the local daemon: the running maitty service uses protocol v49, while this GUI needs v62')
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 62, canRestart: true, sessionCount: 4, activeTurnCount: 1 })
    daemon.restartMismatchedDaemon.mockResolvedValue(undefined)
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'Restart and restore 4 sessions' }))
    expect(screen.getByText('4 sessions reopen in the same folders')).toBeInTheDocument()
    expect(screen.getByText('1 agent was working. It will offer Resume.')).toBeInTheDocument()
    expect(screen.getByText('Running shells and commands stop')).toBeInTheDocument()
    expect(daemon.restartMismatchedDaemon).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'Click again to restart' }))
    expect(daemon.restartMismatchedDaemon).toHaveBeenCalledWith(49)
  })

  it('restarts an idle older daemon only after a second click', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockRejectedValue('connecting to the local daemon: the running maitty service uses protocol v49, while this GUI needs v62')
    daemon.loadDaemonRestartStatus.mockResolvedValue({ daemonProtocol: 49, guiProtocol: 62, canRestart: true, sessionCount: 0, activeTurnCount: 0 })
    daemon.restartMismatchedDaemon.mockResolvedValue(undefined)
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'Restart service' }))
    expect(daemon.restartMismatchedDaemon).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'Click again to restart' }))
    expect(daemon.restartMismatchedDaemon).toHaveBeenCalledWith(49)
  })

  it('types the paths of files dropped on a pane into that pane, quoted, and never runs them', async () => {
    terminal.pasted.length = 0
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    const pane = document.querySelector('[data-session-id="t2"]')
    expect(pane).not.toBeNull()
    document.elementFromPoint = vi.fn(() => pane)

    act(() => { terminal.drop.current?.([`${HOME}/My Shot.png`, `${HOME}/b.png`], { x: 10, y: 10 }) })

    expect(terminal.pasted).toEqual([['t2', `${HOME}/My\\ Shot.png ${HOME}/b.png `]])
  })

  it('types a drop that lands on no pane into the focused one', async () => {
    terminal.pasted.length = 0
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    document.elementFromPoint = vi.fn(() => null)

    act(() => { terminal.drop.current?.([`${HOME}/b.png`], { x: 1, y: 1 }) })

    expect(terminal.pasted).toEqual([['t2', `${HOME}/b.png `]])
  })

  it('types nothing while the service is unreachable', async () => {
    terminal.pasted.length = 0
    daemon.loadConversations.mockResolvedValueOnce([term(2, `${HOME}/a`)]).mockRejectedValue(new Error('connection refused'))
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })
    await screen.findByText('Reconnecting to maittyd')
    document.elementFromPoint = vi.fn(() => null)

    act(() => { terminal.drop.current?.([`${HOME}/b.png`], { x: 1, y: 1 }) })

    expect(terminal.pasted).toEqual([])
  })

  it('keeps only its own keys from a focused terminal and passes the rest to the shell', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const terminal = await screen.findByRole('region', { name: 'Shell in ~/a' })

    expect(fireEvent.keyDown(terminal, { key: 'l', ctrlKey: true })).toBe(true)
    expect(fireEvent.keyDown(terminal, { key: 'c', ctrlKey: true })).toBe(true)
    // The window's sidebar key is Ctrl+Shift+B; a plain Ctrl+B belongs to the shell.
    expect(fireEvent.keyDown(terminal, { key: 'b', ctrlKey: true })).toBe(true)
    expect(fireEvent.keyDown(terminal, { key: 'B', ctrlKey: true, shiftKey: true })).toBe(false)

    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeInTheDocument()
    expect(screen.getByRole('navigation', { name: 'Sessions' })).toHaveClass('is-rail')
  })

  it('gives every button in the window a name, so an icon-only one still has a tooltip', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    const unnamed = Array.from(document.querySelectorAll<HTMLElement>('button, [role="switch"], [role="radio"]'))
      .filter((control) => (control.getAttribute('aria-label') ?? control.getAttribute('title') ?? control.textContent).trim() === '')
      .map((control) => control.className)
    expect(unnamed).toEqual([])
  })

  it('opens the inbox with Ctrl+Shift+J and Settings on Performance with Ctrl+Shift+P, and ignores a bare Ctrl+J', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('button', { name: 'New agent chat' })

    await user.keyboard('{Control>}j{/Control}')
    expect(screen.queryByRole('dialog', { name: 'Inbox' })).not.toBeInTheDocument()
    await user.keyboard('{Control>}{Shift>}j{/Shift}{/Control}')
    expect(screen.getByRole('dialog', { name: 'Inbox' })).toBeInTheDocument()
    await user.keyboard('{Escape}')

    await user.keyboard('{Control>}{Shift>}p{/Shift}{/Control}')
    const settings = screen.getByRole('dialog', { name: 'Settings' })
    expect(within(settings).getByRole('heading', { name: 'Performance' })).toBeInTheDocument()
  })

  it('opens Settings with Ctrl+comma, jumps by letter, traps focus and gives it back on Escape', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    const newSession = await screen.findByRole('button', { name: 'New agent chat' })
    newSession.focus()

    await user.keyboard('{Control>},{/Control}')
    const dialog = screen.getByRole('dialog', { name: 'Settings' })
    expect(dialog).toHaveAttribute('role', 'dialog')
    expect(within(dialog).getByRole('button', { name: /Appearance/ })).toHaveFocus()

    await user.keyboard('t')
    expect(within(dialog).getByRole('heading', { name: 'Text & cursor' })).toBeInTheDocument()
    await user.keyboard('?')
    expect(within(dialog).getByRole('group', { name: 'Ctrl Alt Shift D' }).querySelectorAll('kbd')).toHaveLength(4)

    within(dialog).getByRole('button', { name: 'Close settings' }).focus()
    const focusables = dialog.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled])')
    focusables[focusables.length - 1]?.focus()
    await user.tab()
    expect(focusables[0]).toHaveFocus()

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(newSession).toHaveFocus()
  })

  it('applies a theme from the browser to the whole window and keeps it', async () => {
    const user = userEvent.setup()
    const app = renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    const root = () => document.querySelector<HTMLElement>('.mx-app')
    expect(root()?.style.getPropertyValue('--bg')).toBe('#05131a')

    await user.keyboard('{Control>},{/Control}')
    await user.keyboard('a')
    await user.click(screen.getByRole('button', { name: 'Dark theme: Maitty Night' }))
    await user.type(screen.getByRole('textbox', { name: 'Search dark themes' }), 'tokyo')
    const listbox = await screen.findByRole('radiogroup', { name: 'Dark themes' })
    expect(within(listbox).getAllByRole('radio')).toHaveLength(1)
    await user.click(within(listbox).getByRole('radio', { name: /Tokyo Night/ }))

    expect(root()?.style.getPropertyValue('--accent')).toBe('#7DCFFF')
    app.unmount()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    expect(root()?.style.getPropertyValue('--accent')).toBe('#7DCFFF')
  })

  it('checks the start folder as it is typed and saves only a folder that exists', async () => {
    const user = userEvent.setup()
    daemon.suggestProjectFolders.mockImplementation((typed) => Promise.resolve(typed === `${HOME}/code` ? [`${HOME}/code/`, `${HOME}/codex/`] : []))
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    await user.click(screen.getByRole('button', { name: 'Change start folder' }))
    const input = screen.getByRole('textbox', { name: 'Start folder' })

    await user.type(input, '/nowhere')
    expect(await screen.findByText('Folder not found')).toBeInTheDocument()
    expect(input).toHaveAttribute('aria-invalid', 'true')

    await user.clear(input)
    await user.type(input, `${HOME}/code`)
    await waitFor(() => { expect(JSON.parse(window.localStorage.getItem('maitty.gui.settings.v1') ?? '{}')).toMatchObject({ startFolder: `${HOME}/code` }) })
    expect(screen.getByText('Saved')).toBeInTheDocument()
  })
})

describe('desktop overlays and pane tools', () => {
  const root = `${HOME}/code/maitty`
  const at = (pane: number, extra: Partial<Conversation> = {}) => inRepo(pane, root, `${root}/p${String(pane)}`, extra)

  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.startShellSession.mockResolvedValue(undefined)
    daemon.sendTerminalInput.mockResolvedValue(undefined)
    daemon.loadConversations.mockResolvedValue([])
    daemon.suggestProjectFolders.mockResolvedValue([])
  })

  it('undoes and redoes typing in a text box of the desktop window, where the Linux web view has no undo of its own', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    const box = within(await screen.findByRole('dialog', { name: 'Command palette' })).getByRole('combobox')
    await user.type(box, 'codex')
    await user.keyboard('{Control>}z{/Control}')
    expect(box).toHaveValue('')
    await user.keyboard('{Control>}{Shift>}z{/Shift}{/Control}')
    expect(box).toHaveValue('codex')
  })

  it('opens the palette with Ctrl+K, finds a session by typing, opens it with Enter and closes with Escape', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`, { agent: 'codex' })])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    const dialog = await screen.findByRole('dialog', { name: 'Command palette' })
    await user.type(within(dialog).getByRole('combobox'), 'codex')
    await user.click(within(dialog).getByRole('option', { name: /codex · b/ }))

    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument()
    expect(await screen.findByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()

    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument()
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument()
  })

  it('offers themes in the palette only once something is typed, and applies the one picked', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    const dialog = await screen.findByRole('dialog', { name: 'Command palette' })
    expect(within(dialog).queryByRole('option', { name: /Tokyo/ })).not.toBeInTheDocument()

    await user.type(within(dialog).getByRole('combobox'), 'tokyo')
    await user.keyboard('{Enter}')

    await waitFor(() => { expect(JSON.parse(window.localStorage.getItem('maitty.gui.settings.v1') ?? '{}')).toMatchObject({ themeMode: 'dark', darkTheme: 'Tokyo Night' }) })
  })

  it('shows a card when a session you are not in needs you, and Allow types the answer', async () => {
    const user = userEvent.setup()
    const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    try {
      daemon.loadConversations.mockResolvedValueOnce([at(2), at(3, { status: 'working' })])
      const { client } = renderDesktopApp()
      await screen.findByRole('navigation', { name: 'Sessions' })
      daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you', details: { asking: '$ git push origin main', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null } } })])
      await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

      // The window sees the status change and raises the card itself; the daemon's own alert for the same event replaces it, never stacks.
      const card = (await screen.findByText(/needs you$/)).closest('.mx-notice')
      expect(card).not.toBeNull()
      act(() => { alerts.emit?.({ machine: LOCAL_MACHINE, term: 3, status: 'blocked' }) })
      expect(screen.getAllByText(/needs you$/)).toHaveLength(1)
      expect(card).toHaveTextContent('$ git push origin main')
      expect(card).toHaveTextContent('just now')
      expect(within(card as HTMLElement).getAllByRole('button').map((button) => button.textContent)).toEqual(['', 'Allow', 'Open session'])

      await user.click(within(card as HTMLElement).getByRole('button', { name: 'Allow' }))
      expect(daemon.sendTerminalInput).toHaveBeenCalledWith(expect.objectContaining({ id: 't3' }), 'y')
    } finally {
      focus.mockRestore()
    }
  })

  it('Allow on a Claude session sends 1, the key its permission menu takes, not y', async () => {
    const user = userEvent.setup()
    const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    try {
      daemon.loadConversations.mockResolvedValueOnce([at(2), at(3, { status: 'working', agent: 'claude' })])
      const { client } = renderDesktopApp()
      await screen.findByRole('navigation', { name: 'Sessions' })
      daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you', agent: 'claude', details: { asking: '', ask: { tool: 'Bash', detail: 'git push origin main', answerable: true }, interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null } } })])
      await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

      const card = (await screen.findByText(/needs you$/)).closest('.mx-notice')
      expect(card).not.toBeNull()
      expect(card).toHaveTextContent('Bash · git push origin main')
      await user.click(within(card as HTMLElement).getByRole('button', { name: 'Allow' }))
      expect(daemon.sendTerminalInput).toHaveBeenCalledWith(expect.objectContaining({ id: 't3' }), '1')
    } finally {
      focus.mockRestore()
    }
  })

  it('lists what needs you in the inbox as plain rows: a row opens its session and a finished one can be cleared', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you', summary: 'Allow git push origin main?' }), at(4, { status: 'done' })])
    renderDesktopApp()
    const open = await screen.findByRole('button', { name: /^Inbox, 2, something needs you$/ })
    expect(open.closest('.mx-sidebar-head')).not.toBeNull()

    await user.click(open)
    const inbox = await screen.findByRole('dialog', { name: 'Inbox' })
    const rows = within(inbox).getAllByRole('listitem')
    expect(rows).toHaveLength(2)
    expect(rows[0]).toHaveTextContent('Needs you')
    expect(rows[0]).toHaveTextContent('Allow git push origin main?')
    // No preview of the session and no answer buttons: the row opens the session, where the answer is given.
    expect(within(inbox).queryByRole('button', { name: /^Yes, once/ })).not.toBeInTheDocument()

    await user.click(within(rows[1] as HTMLElement).getByRole('button', { name: /^Clear / }))
    expect(within(inbox).getAllByRole('listitem')).toHaveLength(1)

    await user.click(within(inbox).getByRole('button', { name: /needs you$/ }))
    await waitFor(() => { expect(screen.queryByRole('dialog', { name: 'Inbox' })).not.toBeInTheDocument() })
  })

  it('shows a session in the inbox again when it comes back to a state you cleared, after it moved on', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2, { status: 'done' })])
    const { client } = renderDesktopApp()
    await user.click(await screen.findByRole('button', { name: /^Inbox, 1/ }))
    const inbox = await screen.findByRole('dialog', { name: 'Inbox' })
    await user.click(within(within(inbox).getByRole('listitem')).getByRole('button', { name: /^Clear / }))
    expect(within(inbox).getByText(/all caught up/)).toBeInTheDocument()

    daemon.loadConversations.mockResolvedValue([at(2, { status: 'working' })])
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })
    // The window has to see the session working before it is done again, or nothing moved on.
    // The inbox is a modal now, so the page behind it is hidden from the accessibility tree.
    await screen.findByRole('button', { name: /, working$/, hidden: true })
    daemon.loadConversations.mockResolvedValue([at(2, { status: 'done' })])
    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    await waitFor(() => { expect(within(screen.getByRole('dialog', { name: 'Inbox' })).queryByText(/all caught up/)).not.toBeInTheDocument() })
  })

  it('marks a finished session seen once you are in its pane, and leaves a needs-you one alone', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { status: 'done' })])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    act(() => { window.dispatchEvent(new Event('focus')) })

    await waitFor(() => { expect(daemon.markSeenOnDaemon).toHaveBeenCalledWith({ id: 't2' }) })
  })

  it('does not mark a needs-you session seen just because you are in its pane', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { status: 'needs-you' })])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    act(() => { window.dispatchEvent(new Event('focus')) })

    expect(daemon.markSeenOnDaemon).not.toHaveBeenCalled()
  })

  it('greets a first launch, saves the start folder, shows the keys, and never greets again', async () => {
    const user = userEvent.setup()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    const app = renderDesktopApp({ firstRun: true })

    const dialog = await screen.findByRole('dialog', { name: 'Welcome to Maitty' })
    expect(dialog).toHaveTextContent('Closing this window never stops an agent')
    // Global shortcuts are ignored while it is open.
    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument()

    await user.type(within(dialog).getByRole('textbox', { name: 'New sessions start in' }), '/definitely/not/here')
    await user.click(within(dialog).getByRole('button', { name: 'Continue' }))
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('That folder was not found.')

    await user.clear(within(dialog).getByRole('textbox', { name: 'New sessions start in' }))
    await user.click(within(dialog).getByRole('button', { name: 'Continue' }))
    const set = await screen.findByRole('dialog', { name: "You're set" })
    expect(within(set).getAllByRole('listitem')).toHaveLength(6)
    expect(set).toHaveTextContent('New session')

    await user.click(within(set).getByRole('button', { name: 'Start a session' }))
    expect(screen.queryByRole('dialog', { name: "You're set" })).not.toBeInTheDocument()
    await screen.findByRole('region', { name: 'New agent chat' })

    app.unmount()
    renderDesktopApp({ firstRun: true })
    await screen.findByRole('heading', { name: 'No sessions running' })
    expect(screen.queryByRole('dialog', { name: 'Welcome to Maitty' })).not.toBeInTheDocument()
  })

  it('skips the welcome with Escape and does not greet a window that already has settings', async () => {
    const user = userEvent.setup()
    renderDesktopApp({ firstRun: true })
    await screen.findByRole('dialog', { name: 'Welcome to Maitty' })

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog', { name: 'Welcome to Maitty' })).not.toBeInTheDocument()

    window.localStorage.removeItem('maitty.gui.welcomed.v1')
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ accent: 'blue' }))
    cleanup()
    renderDesktopApp({ firstRun: true })
    await screen.findByRole('heading', { name: 'No sessions running' })
    expect(screen.queryByRole('dialog', { name: 'Welcome to Maitty' })).not.toBeInTheDocument()
  })

  it('shows every shortcut from the keymap on Ctrl+/, ignores other shortcuts while it is open, and closes with the key or Esc', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })

    await user.keyboard('{Control>}/{/Control}')
    const dialog = await screen.findByRole('dialog', { name: 'Keyboard shortcuts' })
    for (const row of hotkeyRows('linux')) expect(dialog).toHaveTextContent(row.action)
    for (const group of ['Sessions', 'Panes', 'Find & jump', 'Window']) expect(within(dialog).getByRole('region', { name: group })).toBeInTheDocument()

    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument()

    await user.keyboard('{Control>}/{/Control}')
    expect(screen.queryByRole('dialog', { name: 'Keyboard shortcuts' })).not.toBeInTheDocument()

    await user.keyboard('{Control>}/{/Control}')
    await screen.findByRole('dialog', { name: 'Keyboard shortcuts' })
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog', { name: 'Keyboard shortcuts' })).not.toBeInTheDocument()
  })

  it('keeps the status bar to quick controls: theme, layout style, text size and the shortcuts', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: true }))
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([
      inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty`, { agent: 'claude', status: 'working' }),
      inRepo(4, `${HOME}/code/maitty`, `${HOME}/code/maitty/crates`, { agent: 'codex', status: 'needs-you' }),
    ])
    renderDesktopApp()
    const bar = await screen.findByLabelText('Status')

    expect(bar).not.toHaveTextContent('maittyd')
    expect(bar).not.toHaveTextContent('working')
    expect(bar).not.toHaveTextContent('needs you')
    expect(bar).not.toHaveTextContent('main')
    expect(bar).toHaveTextContent('13px')

    await user.click(within(bar).getByRole('button', { name: 'Larger text' }))
    expect(bar).toHaveTextContent('14px')
    await user.click(within(bar).getByRole('button', { name: 'Smaller text' }))
    await user.click(within(bar).getByRole('button', { name: 'Smaller text' }))
    expect(bar).toHaveTextContent('12px')
    expect(within(bar).getByRole('button', { name: 'Smaller text' })).toBeDisabled()
    await user.click(within(bar).getByRole('button', { name: /^Text size 12px/ }))
    expect(bar).toHaveTextContent('14px')

    await user.click(within(bar).getByRole('button', { name: 'Keyboard shortcuts' }))
    expect(await screen.findByRole('dialog', { name: 'Keyboard shortcuts' })).toBeInTheDocument()
  })

  it('changes the layout style from the status bar', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: true }))
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const bar = await screen.findByLabelText('Status')

    await user.click(within(bar).getByRole('button', { name: /^Layout style/ }))
    await user.click(await screen.findByRole('menuitemradio', { name: 'Mono Type' }))

    expect(document.documentElement.dataset.style).toBe('mono')
    expect(document.documentElement.dataset.finish).toBe('flat')
    expect(within(bar).getByRole('button', { name: 'Layout style: mono' })).toBeInTheDocument()
  })

  it('picks a theme from the status bar and opens the full list in Settings', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: true }))
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const bar = await screen.findByLabelText('Status')

    await user.click(within(bar).getByRole('button', { name: /^Theme:/ }))
    await user.click(await screen.findByRole('menuitem', { name: /All themes in Settings/ }))

    expect(await screen.findByRole('dialog', { name: /Settings/ })).toBeInTheDocument()
  })

  it('mirrors the window when the sidebar is moved to the right, from the command palette', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    const app = document.querySelector('.mx-app')
    expect(app).toHaveAttribute('data-side', 'left')

    await user.keyboard('{Control>}{Shift>}k{/Shift}{/Control}')
    await user.type(await screen.findByRole('combobox'), 'sidebar to the right')
    await user.keyboard('{Enter}')

    await waitFor(() => { expect(app).toHaveAttribute('data-side', 'right') })
    expect(JSON.parse(window.localStorage.getItem('maitty.gui.settings.v1') ?? '{}')).toMatchObject({ sidebarSide: 'right' })
  })

  it('hides the status bar in focus mode and when Settings turns it off', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: true }))
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    const app = renderDesktopApp()
    await screen.findByLabelText('Status')

    await user.keyboard('{Control>}.{/Control}')
    expect(screen.queryByLabelText('Status')).not.toBeInTheDocument()
    await user.keyboard('{Escape}')
    await screen.findByLabelText('Status')

    app.unmount()
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: false }))
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    expect(screen.queryByLabelText('Status')).not.toBeInTheDocument()
  })

  it('says it is all caught up when nothing needs you', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2)])
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'Inbox, 0' }))

    expect(await screen.findByText('You\'re all caught up')).toBeInTheDocument()
  })

  it('hides the sidebar in focus mode and brings it back with Escape', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you' })])
    renderDesktopApp()
    await screen.findByRole('navigation', { name: 'Sessions' })

    await user.keyboard('{Control>}.{/Control}')
    const pill = await screen.findByRole('status', { name: '' }, { timeout: 1000 }).catch(() => null)
    expect(pill ?? document.querySelector('.mx-focus-pill')).not.toBeNull()
    expect(screen.queryByRole('navigation', { name: 'Sessions' })).not.toBeInTheDocument()
    expect(document.querySelector('.mx-focus-pill')).toHaveTextContent('1 need you elsewhere')

    await user.keyboard('{Escape}')
    expect(await screen.findByRole('navigation', { name: 'Sessions' })).toBeInTheDocument()
  })

  it('moves a session dragged from the sidebar into a pane, and the old one keeps running', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1: / })
    const row = screen.getByRole('button', { name: 'b, shell, idle' })
    fireEvent.pointerDown(row, { button: 0, clientX: 5, clientY: 5 })
    fireEvent.pointerMove(window, { clientX: 40, clientY: 40 })
    fireEvent.pointerMove(pane, { clientX: 300, clientY: 200 })
    expect(pane).toHaveClass('is-drop-target')
    fireEvent.pointerUp(pane, { clientX: 300, clientY: 200 })

    expect(await screen.findByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()
    expect(screen.queryByText('Session moved into this pane')).toBeInTheDocument()
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('broadcasts typing from one pane to the tab\'s idle panes only', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Shell in/ })
    daemon.loadConversations.mockResolvedValue([at(2), at(7), at(8, { status: 'working' })])
    await user.keyboard('{Control>}{Shift>}d{/Shift}{/Control}')
    await screen.findByRole('region', { name: /^Terminal 2:/ })

    await user.keyboard('{Control>}{Alt>}{Shift>}b{/Shift}{/Alt}{/Control}')

    expect(await screen.findAllByText('◉ Broadcast')).toHaveLength(2)
    await user.keyboard('{Control>}{Alt>}{Shift>}b{/Shift}{/Alt}{/Control}')
    await waitFor(() => { expect(screen.queryByText('◉ Broadcast')).not.toBeInTheDocument() })
  })
})

function renderDesktopApp({ firstRun = false }: { firstRun?: boolean } = {}) {
  // Only a first run is greeted; every other test starts as an ordinary launch.
  if (!firstRun) window.localStorage.setItem('maitty.gui.welcomed.v1', '1')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const view = render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  )
  return { client, unmount: view.unmount }
}
