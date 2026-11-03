import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import App from './App'

const daemon = vi.hoisted(() => ({
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  startShellSession: vi.fn<(folder: string) => Promise<void>>(),
  archiveConversationOnDaemon: vi.fn<(conversation: { id: string }, archived: boolean) => Promise<void>>(),
  renameConversationOnDaemon: vi.fn<(conversation: { id: string }, title: string) => Promise<void>>(),
  markSeenOnDaemon: vi.fn<(conversation: { id: string }) => Promise<void>>(),
  loadDaemonRestartStatus: vi.fn(),
  restartMismatchedDaemon: vi.fn<(protocol: number) => Promise<void>>(),
  suggestProjectFolders: vi.fn<(typed: string) => Promise<string[]>>(),
  sendTerminalInput: vi.fn<(conversation: { id: string; generation: number }, input: string | Uint8Array) => Promise<void>>(),
}))

vi.mock('@/api/daemon', () => ({
  ...daemon,
  isDesktop: () => true,
  terminalOf: (id: string) => {
    const match = /^t(\d+)$/.exec(id)
    return match?.[1] === undefined ? null : Number(match[1])
  },
}))

vi.mock('./TerminalView', () => ({
  TerminalView: ({ conversation, label, focused, inputDisabled }: { conversation: Conversation; label: string; focused: boolean; inputDisabled: boolean }) => (
    <div className="terminal-screen" role="region" aria-label={label} data-focused={String(focused)} data-input-disabled={String(inputDisabled)} tabIndex={0}>
      Pane {conversation.nativePane}
    </div>
  ),
}))

const HOME = '/home/casey'

/** Folds the docked sidebar to its rail, as a user does. */
async function collapseSidebar() {
  await userEvent.setup().click(await screen.findByRole('button', { name: 'Collapse sidebar' }))
}

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
    window.localStorage.clear()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.loadConversations.mockResolvedValue([])
    daemon.startShellSession.mockResolvedValue(undefined)
    daemon.archiveConversationOnDaemon.mockResolvedValue(undefined)
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
    expect(screen.getByText('No sessions. Projects show up here when a session cds into a git repo.')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Change start folder' }))
    const dialog = screen.getByRole('dialog', { name: 'Settings' })
    expect(within(dialog).getByRole('heading', { name: 'General' })).toBeInTheDocument()
    expect(within(dialog).getByRole('textbox', { name: 'Start folder' })).toHaveValue('')
  })

  it('starts a plain shell in the home folder with Ctrl+T and opens it in a new tab', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    daemon.loadConversations.mockResolvedValue([term(5, HOME)])

    await user.keyboard('{Control>}t{/Control}')

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(HOME) })
    expect(await screen.findByRole('tab', { selected: true })).toHaveTextContent('shell ~')
    expect(screen.getByRole('region', { name: 'Shell in ~' })).toHaveTextContent('Pane 5')
    expect(screen.getByRole('region', { name: /^Terminal 1: ~/ })).toBeInTheDocument()
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
      expect(root?.style.getPropertyValue('--sidebar-fs')).toBe('15px')
      expect(root?.style.getPropertyValue('--tab-fs')).toBe('12px')
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

  it('slides the sidebar in over the panes from a Sessions button in a window too narrow for it', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1000 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()

      const open = await screen.findByRole('button', { name: 'Sessions' })
      expect(screen.queryByRole('navigation', { name: 'Sessions by project' })).not.toBeInTheDocument()

      await user.click(open)
      const drawer = await screen.findByRole('dialog', { name: 'Sessions' })
      const sidebar = within(drawer).getByRole('navigation', { name: 'Sessions by project' })
      expect(sidebar).not.toHaveClass('is-rail')

      await user.click(within(sidebar).getByRole('button', { name: /, shell, / }))
      expect(screen.queryByRole('dialog', { name: 'Sessions' })).not.toBeInTheDocument()

      await user.click(open)
      await screen.findByRole('dialog', { name: 'Sessions' })
      await user.keyboard('{Escape}')
      expect(screen.queryByRole('dialog', { name: 'Sessions' })).not.toBeInTheDocument()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('keeps the sidebar docked, with no Sessions button, in a window wide enough for it', async () => {
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()

      expect(await screen.findByRole('navigation', { name: 'Sessions by project' })).not.toHaveClass('is-rail')
      expect(screen.queryByRole('button', { name: 'Sessions' })).not.toBeInTheDocument()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('draws each known agent\'s mark on its own tile', async () => {
    const agents = ['claude', 'codex', 'deepseek', 'gemini', 'opencode']
    daemon.loadConversations.mockResolvedValue(agents.map((agent, index) => term(index + 2, `${HOME}/code/${agent}`, { agent })))
    renderDesktopApp()

    const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
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

    const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
    expect(await within(sidebar).findByRole('button', { name: /, deepseek, / })).toBeInTheDocument()
    expect(within(sidebar).getByRole('button', { name: /, cargo, / })).toBeInTheDocument()
    expect(within(sidebar).getByRole('button', { name: /, shell, / })).toBeInTheDocument()
  })

  it('shows each pane its status in the bottom-right corner, with a popover that explains it', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { status: 'working' })])
    renderDesktopApp()

    const chip = await screen.findByRole('button', { name: 'Status: Working' })
    expect(chip).toHaveAccessibleDescription(/A program is running in this session/)
    expect(chip.closest('.mx-pane-status-wrap')).toBeInTheDocument()
  })

  it('opens a menu on right-click in a pane and splits from it', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1: / })

    fireEvent.contextMenu(pane)

    expect(await screen.findByRole('menuitem', { name: /^Close session/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /^Close pane/ })).toHaveAttribute('aria-disabled', 'true')
    daemon.startShellSession.mockResolvedValue(undefined)
    await user.click(screen.getByRole('menuitem', { name: /^Split right/ }))
    // A split opens a new shell beside this one.
    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalled() })
  })

  it('opens a menu on right-click on a tab', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const tab = await screen.findByRole('tab', { selected: true })

    fireEvent.contextMenu(tab)

    expect(await screen.findByRole('menuitem', { name: /^New tab/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /^Close tab/ })).toBeInTheDocument()
  })

  it('shows an icon for each pane in a tab and lists the panes when the tab is hovered', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { agent: 'claude' })])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { agent: 'claude' }), term(7, `${HOME}/a`)])
    expect(screen.getByRole('tab', { selected: true }).querySelectorAll('.mx-tab-icons .mx-app-icon')).toHaveLength(1)

    await user.keyboard('{Control>}d{/Control}')

    const tab = await screen.findByRole('tab', { selected: true })
    await waitFor(() => { expect(tab.querySelectorAll('.mx-tab-icons .mx-app-icon')).toHaveLength(2) })
    expect(tab).toHaveTextContent('+1')
    await user.hover(tab)
    const list = await waitFor(() => {
      const found = document.querySelector('.mx-rail-popover.is-tab')
      expect(found).not.toBeNull()
      return found
    })
    expect(list).toHaveTextContent('2 panes in this tab')
    expect(list).toHaveTextContent('claude')
    expect(list).toHaveTextContent('shell')
    await user.unhover(tab)
    expect(document.querySelector('.mx-rail-popover.is-tab')).toBeNull()
  })

  it('renames a session from its row menu, and the name shows on the row, the tab and the pane', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
    fireEvent.contextMenu(await within(sidebar).findByRole('button', { name: /, shell, / }))

    await user.click(await screen.findByRole('menuitem', { name: /^Rename/ }))
    // The field already has the cursor: the menu must not have taken focus back to the row.
    await screen.findByRole('textbox', { name: 'Session name' })
    await waitFor(() => { expect(screen.getByRole('textbox', { name: 'Session name' })).toHaveFocus() })
    // Once saved, the daemon reports the name, and every place that shows the session uses it.
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { named: true, title: 'Login fix' })])
    await user.keyboard('Login fix{Enter}')

    await waitFor(() => { expect(daemon.renameConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, 'Login fix') })
    expect(screen.queryByRole('dialog', { name: 'Rename session' })).not.toBeInTheDocument()
    await within(sidebar).findByRole('button', { name: /^Login fix, shell, / })
    expect(screen.getByRole('tab', { selected: true })).toHaveTextContent('Login fix')
    expect(document.querySelector('.mx-pane-name')).toHaveTextContent('Login fix')
  })

  it('cancels a rename with Escape and sends nothing, and an empty name clears it', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { named: true, title: 'Login fix' })])
    renderDesktopApp()
    const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
    fireEvent.contextMenu(await within(sidebar).findByRole('button', { name: /^Login fix, shell, / }))
    await user.click(await screen.findByRole('menuitem', { name: /^Rename/ }))
    expect(await screen.findByRole('textbox', { name: 'Session name' })).toHaveValue('Login fix')

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog', { name: 'Rename session' })).not.toBeInTheDocument()
    expect(daemon.renameConversationOnDaemon).not.toHaveBeenCalled()

    fireEvent.contextMenu(within(sidebar).getByRole('button', { name: /^Login fix, shell, / }))
    await user.click(await screen.findByRole('menuitem', { name: /^Rename/ }))
    await user.clear(await screen.findByRole('textbox', { name: 'Session name' }))
    await user.keyboard('{Enter}')
    await waitFor(() => { expect(daemon.renameConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, '') })
  })

  it('opens a menu on right-click on a collapsed session, with rename and close', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()
      await collapseSidebar()
      const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })

      fireEvent.contextMenu(await within(sidebar).findByRole('button', { name: /, shell, / }))

      expect(await screen.findByRole('menuitem', { name: /^Show in pane/ })).toBeInTheDocument()
      expect(screen.getByRole('menuitem', { name: /^Close session/ })).toBeInTheDocument()
      await user.click(screen.getByRole('menuitem', { name: /^Rename/ }))
      expect(await screen.findByRole('dialog', { name: 'Rename session' })).toBeInTheDocument()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('says what a collapsed session is doing in a popover beside it', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty`, { agent: 'claude', status: 'needs-you', summary: 'Fix the login redirect so it keeps the query string' })])
      renderDesktopApp()
      await collapseSidebar()
      const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
      const row = await within(sidebar).findByRole('button', { name: /, claude, / })

      await user.hover(row)

      // The pane's own status tooltip says the same thing, so pick the rail's popover by its class.
      // It opens after a short hover, not at once.
      expect(document.querySelector('.mx-railpop.is-session')).toBeNull()
      const card = await waitFor(() => {
        const found = document.querySelector('.mx-railpop.is-session')
        expect(found).not.toBeNull()
        return found
      })
      expect(card).toHaveTextContent('maitty')
      expect(card).toHaveTextContent('needs you')
      expect(card).toHaveTextContent('claude · ~/code/maitty')
      expect(card).toHaveTextContent('Fix the login redirect so it keeps the query string')
      await user.unhover(row)
      await waitFor(() => { expect(document.querySelector('.mx-railpop.is-session')).toBeNull() })
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('shows no prompt line in a popover for a session that has been given none', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
      renderDesktopApp()
      await collapseSidebar()
      const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })

      await user.hover(await within(sidebar).findByRole('button', { name: /, shell, / }))

      const card = await waitFor(() => {
        const found = document.querySelector('.mx-railpop.is-session')
        expect(found).not.toBeNull()
        return found
      })
      expect(card).toHaveTextContent('idle')
      expect(card?.querySelector('.mx-rp-well')).toBeNull()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('says what a collapsed folder is in a popover beside it', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/personal/dynamic-ui`), inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty`)])
      renderDesktopApp()
      await collapseSidebar()
      const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })

      await user.hover(within(sidebar).getByRole('button', { name: /^dynamic-ui · 1 session/ }))

      // Each pane also has a status tooltip, so find the folder's popover by what only it shows.
      const path = await screen.findByText('~/personal/dynamic-ui', { selector: '.mx-rp-path' })
      const popover = path.closest('[role="tooltip"]')
      expect(popover).toHaveTextContent('dynamic-ui')
      expect(popover).toHaveTextContent('shell')
      expect(popover).toHaveTextContent('idle')
      expect(popover).toHaveTextContent('Click the folder to collapse')
      await user.unhover(within(sidebar).getByRole('button', { name: /^dynamic-ui · 1 session/ }))
      await waitFor(() => { expect(screen.queryByText('~/personal/dynamic-ui', { selector: '.mx-rp-path' })).not.toBeInTheDocument() })
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('draws each collapsed project as a tinted group with its two letters, and marks the focused one', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([
        inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty`, { status: 'needs-you' }),
        inRepo(4, `${HOME}/code/maitty`, `${HOME}/code/maitty`),
        term(2, `${HOME}/personal/dynamic-ui`, { status: 'working' }),
      ])
      renderDesktopApp()
      await collapseSidebar()
      const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })

      const groups = Array.from(sidebar.querySelectorAll('.mx-rail-group'))
      expect(groups).toHaveLength(2)
      expect(Array.from(sidebar.querySelectorAll('.mx-rail-folder-text')).map((label) => label.textContent).sort()).toEqual(['DU', 'Ma'])
      // Open groups show a hairline under the tile, then their sessions; only one holds the focused session.
      expect(sidebar.querySelectorAll('.mx-rail-group.has-focused')).toHaveLength(1)
      expect(sidebar.querySelectorAll('.mx-rail-group.is-open .mx-rail-hairline')).toHaveLength(2)
      expect(sidebar.querySelector('.mx-rail-needs')).not.toBeNull()
      expect(sidebar.querySelector('.mx-rail-count')).toBeNull()

      // Folding a project leaves its tile with a count, and a spinner while something in it works.
      await user.click(within(sidebar).getByRole('button', { name: /^dynamic-ui · 1 session/ }))
      const folded = within(sidebar).getByRole('button', { name: /^dynamic-ui · 1 session/ }).closest('.mx-rail-group')
      expect(folded).not.toHaveClass('is-open')
      expect(folded?.querySelector('.mx-rail-count')).toHaveTextContent('1')
      expect(folded?.querySelector('.mx-rail-spin')).not.toBeNull()
      expect(folded?.querySelector('.mx-rail-hairline')).toBeNull()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('keeps a rail popover open while the pointer is on it, and opens a session from its list', async () => {
    const user = userEvent.setup()
    const wide = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1400 })
    try {
      daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/personal/dynamic-ui`), term(3, `${HOME}/personal/dynamic-ui`)])
      renderDesktopApp()
      await collapseSidebar()
      const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
      const folder = within(sidebar).getByRole('button', { name: /^dynamic-ui · 2 sessions/ })

      await user.hover(folder)
      const path = await screen.findByText('~/personal/dynamic-ui', { selector: '.mx-rp-path' })
      const popover = path.closest<HTMLElement>('[role="tooltip"]')
      expect(popover).not.toBeNull()
      await user.unhover(folder)
      await user.hover(popover as HTMLElement)
      await new Promise((resolve) => { window.setTimeout(resolve, 300) })
      expect(popover).toBeInTheDocument()

      const rows = within(popover as HTMLElement).getAllByRole('button')
      expect(rows).toHaveLength(2)
      await user.click(rows[1] as HTMLElement)
      expect(popover).not.toBeInTheDocument()
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: wide })
    }
  })

  it('starts a new session in the start folder from Settings, with ~ meaning the home folder', async () => {
    const user = userEvent.setup()
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ startFolder: '~/code' }))
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })

    await user.click(screen.getByRole('button', { name: 'New session' }))

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(`${HOME}/code`) })
  })

  it('groups sessions by git repo or plain folder, sorted by name, each with a main row and a tray for the rest', async () => {
    daemon.loadConversations.mockResolvedValue([
      term(9, `${HOME}/Downloads`),
      inRepo(3, `${HOME}/code/maitty`, `${HOME}/code/maitty/crates/maitty-core`, { agent: 'codex', status: 'working' }),
      inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty`),
      inRepo(4, `${HOME}/code/docs`, `${HOME}/code/docs`, { status: 'needs-you' }),
    ])
    renderDesktopApp()

    const sidebar = await screen.findByRole('navigation', { name: 'Sessions by project' })
    await within(sidebar).findByRole('button', { name: '(root), shell, idle' })
    const headers = within(sidebar).getAllByRole('button', { expanded: true }).map((button) => button.textContent)
    expect(headers[0]).toMatch(/^docsmain1 needs you1$/)
    expect(headers[1]).toMatch(/^Downloads1$/)
    expect(headers[2]).toMatch(/^maittymain2$/)
    const rows = within(sidebar).getAllByRole('button', { name: /, (shell|codex), / }).map((row) => row.getAttribute('aria-label'))
    expect(rows).toEqual(['(root), shell, needs you', '~/Downloads, shell, idle', '(root), shell, idle', 'crates/maitty-core, codex, working, detached'])
    expect(within(sidebar).getByText(/· needs you/)).toHaveClass('is-needs')
    expect(within(sidebar).getByRole('status')).toHaveTextContent('maittyd · 4 sessions')
  })

  it('opens the first session on launch, and a sidebar session that is open nowhere gets a tab of its own', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    renderDesktopApp()

    expect(await screen.findByRole('region', { name: 'Shell in ~/a' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: '~/b, shell, idle' }))

    expect(screen.getByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Shell in ~/a' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: '~/b, shell, idle' })).toHaveAttribute('aria-current', 'page')
    // ~/a is another project, so its tab is not on this bar; picking it in the sidebar goes back.
    expect(screen.getAllByRole('tab')).toHaveLength(1)
    await user.click(screen.getByRole('button', { name: '~/a, shell, idle' }))
    expect(screen.getByRole('region', { name: 'Shell in ~/a' })).toBeInTheDocument()
    expect(screen.getAllByRole('tab')).toHaveLength(1)
  })

  it('splits right in the focused shell’s folder, moves focus, and closes a pane without ending its session', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/code/maitty/apps' })
    daemon.loadConversations.mockResolvedValue([inRepo(2, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`), inRepo(7, `${HOME}/code/maitty`, `${HOME}/code/maitty/apps`)])

    await user.keyboard('{Control>}d{/Control}')

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(`${HOME}/code/maitty/apps`) })
    expect(await screen.findByRole('region', { name: 'Terminal 2: ~/code/maitty/apps, focused' })).toBeInTheDocument()
    // A tab with two panes shows an icon for each and +1 for the one beyond the first.
    expect(screen.getByRole('tab', { selected: true })).toHaveTextContent('+1')

    await user.keyboard('{Control>}[[{/Control}')
    expect(screen.getByRole('region', { name: 'Terminal 1: ~/code/maitty/apps, focused' })).toBeInTheDocument()

    await user.keyboard('{Control>}w{/Control}')
    await waitFor(() => { expect(screen.getAllByRole('region', { name: /^Terminal \d/ })).toHaveLength(1) })
    expect(screen.getByText('Pane closed · the session keeps running')).toBeInTheDocument()
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'apps, shell, idle' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'apps, shell, idle, detached' })).toBeInTheDocument()
  })

  it('swaps two panes’ sessions when a header is dropped on another pane', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    await user.click(screen.getByRole('button', { name: 'Split down (Ctrl+Shift+D)' }))
    await screen.findByRole('region', { name: 'Shell in ~/b' })

    const first = screen.getByRole('region', { name: /^Terminal 1:/ })
    const second = screen.getByRole('region', { name: /^Terminal 2:/ })
    const header = first.querySelector('.mx-pane-head')
    if (!header) throw new Error('pane header missing')
    fireEvent.dragStart(header, { dataTransfer: { setData: vi.fn(), effectAllowed: '' } })
    fireEvent.dragOver(second, { dataTransfer: { types: ['text/plain'] } })
    fireEvent.drop(second, { dataTransfer: { getData: () => '' } })

    expect(screen.getByRole('region', { name: 'Terminal 1: ~/b' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Terminal 2: ~/a, focused' })).toBeInTheDocument()
  })

  it('closes a tab without ending its sessions, and ends a session only after a second click', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    await screen.findByRole('tab', { selected: true })

    await user.click(screen.getByRole('button', { name: 'Close tab. Sessions keep running.' }))
    expect(screen.queryByRole('tab')).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'No tab open' })).toBeInTheDocument()
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()

    await user.click(screen.getByRole('button', { name: 'Close session ~/a' }))
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'End session' }))
    await waitFor(() => { expect(daemon.archiveConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, true) })
    expect(await screen.findByText('Session ended')).toBeInTheDocument()
  })

  it('ends a session from its row’s context menu', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const row = await screen.findByRole('button', { name: '~/a, shell, idle' })

    fireEvent.contextMenu(row)
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Close session' }))
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
    fireEvent.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'End session' }))

    await waitFor(() => { expect(daemon.archiveConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, true) })
  })

  it('ends every session in a project from its menu, after one confirmation', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/a`)])
    renderDesktopApp()
    const [head] = await screen.findAllByRole('button', { expanded: true })

    fireEvent.contextMenu(head as HTMLElement)
    fireEvent.click(await screen.findByRole('menuitem', { name: /Close project/ }))
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
    const dialog = await screen.findByRole('alertdialog')
    expect(dialog).toHaveTextContent('End 2 sessions')
    fireEvent.click(within(dialog).getByRole('button', { name: 'End 2 sessions' }))

    await waitFor(() => { expect(daemon.archiveConversationOnDaemon).toHaveBeenCalledTimes(2) })
    expect(daemon.archiveConversationOnDaemon).toHaveBeenCalledWith({ id: 't2' }, true)
    expect(daemon.archiveConversationOnDaemon).toHaveBeenCalledWith({ id: 't3' }, true)
  })

  it('keeps a session when ending it is cancelled', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'Close session ~/a' }))
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Cancel' }))

    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('shows the disconnected banner, marks every status Unknown and holds input', async () => {
    daemon.loadConversations.mockResolvedValueOnce([term(2, `${HOME}/a`, { status: 'working', agent: 'codex' })]).mockRejectedValue(new Error('connection refused'))
    const { client } = renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await act(async () => { await client.refetchQueries({ queryKey: ['conversations'] }) })

    expect(await screen.findByText('Disconnected from maittyd.')).toBeInTheDocument()
    expect(screen.getByText(/What you see is the last known screen\. Retrying in 1s\./)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '~/a, codex, unknown' })).toHaveTextContent('codex · last seen working')
    expect(screen.getByText('Reconnecting to maittyd…')).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Shell in ~/a' })).toHaveAttribute('data-input-disabled', 'true')
    expect(screen.getByRole('button', { name: 'New session' })).toBeDisabled()
    expect(screen.getByRole('button', { name: '~/a, codex, unknown' })).not.toHaveClass('mx-fx-working')

    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    await userEvent.setup().click(screen.getByRole('button', { name: 'Retry now' }))
    await waitFor(() => { expect(screen.queryByText('Disconnected from maittyd.')).not.toBeInTheDocument() })
  })

  it('leaves an older daemon running while it owns sessions', async () => {
    daemon.loadConversations.mockRejectedValue('connecting to the local daemon: the running server is another maitty version (protocol v49, this one v59); run maitty in a terminal to restart it')
    renderDesktopApp()

    expect(await screen.findByRole('button', { name: 'Service in use' })).toBeDisabled()
    expect(screen.getByText(/service uses protocol v49, but this GUI needs v59/)).toBeInTheDocument()
    expect(screen.getByText(/owns 2 sessions, so Maitty leaves it running/)).toBeInTheDocument()
    expect(daemon.restartMismatchedDaemon).not.toHaveBeenCalled()
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

  it('keeps only its own keys from a focused terminal and passes the rest to the shell', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`)])
    renderDesktopApp()
    const terminal = await screen.findByRole('region', { name: 'Shell in ~/a' })

    expect(fireEvent.keyDown(terminal, { key: 'l', ctrlKey: true })).toBe(true)
    expect(fireEvent.keyDown(terminal, { key: 'c', ctrlKey: true })).toBe(true)
    expect(fireEvent.keyDown(terminal, { key: 'b', ctrlKey: true })).toBe(false)

    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeInTheDocument()
    expect(screen.getByRole('navigation', { name: 'Sessions by project' })).toHaveClass('is-rail')
  })

  it('moves between a project\'s tabs with Ctrl+Tab and Ctrl+Shift+Tab', async () => {
    const user = userEvent.setup()
    const root = `${HOME}/code/maitty`
    daemon.loadConversations.mockResolvedValue([inRepo(2, root, `${root}/a`)])
    renderDesktopApp()
    await screen.findByRole('tab', { selected: true })
    daemon.loadConversations.mockResolvedValue([inRepo(2, root, `${root}/a`), inRepo(3, root, `${root}/b`)])
    await user.click(screen.getByRole('button', { name: 'New session in maitty' }))
    await waitFor(() => { expect(screen.getAllByRole('tab')).toHaveLength(2) })
    expect(screen.getByRole('tab', { selected: true })).toHaveTextContent('maitty/b')

    await user.keyboard('{Control>}{Tab}{/Control}')
    expect(screen.getByRole('tab', { selected: true })).toHaveTextContent('maitty/a')
    await user.keyboard('{Control>}{Shift>}{Tab}{/Shift}{/Control}')
    expect(screen.getByRole('tab', { selected: true })).toHaveTextContent('maitty/b')
  })

  it('opens Settings with Ctrl+comma, jumps by letter, traps focus and gives it back on Escape', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    const newSession = await screen.findByRole('button', { name: 'New session' })
    newSession.focus()

    await user.keyboard('{Control>},{/Control}')
    const dialog = screen.getByRole('dialog', { name: 'Settings' })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    expect(within(dialog).getByRole('button', { name: /General/ })).toHaveFocus()

    await user.keyboard('t')
    expect(within(dialog).getByRole('heading', { name: 'Text & cursor' })).toBeInTheDocument()
    await user.keyboard('?')
    expect(within(dialog).getByRole('group', { name: 'Ctrl Shift D' }).querySelectorAll('kbd')).toHaveLength(3)

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
    expect(root()?.style.getPropertyValue('--bg')).toBe('#090C10')

    await user.keyboard('{Control>},{/Control}')
    await user.keyboard('a')
    await user.click(screen.getByRole('button', { name: 'Dark theme: Midnight' }))
    await user.type(screen.getByRole('textbox', { name: 'Search dark themes' }), 'nord')
    const listbox = screen.getByRole('listbox', { name: 'Dark themes' })
    expect(within(listbox).getAllByRole('option')).toHaveLength(1)
    await user.click(within(listbox).getByRole('option', { name: /Nord/ }))

    expect(root()?.style.getPropertyValue('--accent')).toBe('#88C0D0')
    app.unmount()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    expect(root()?.style.getPropertyValue('--accent')).toBe('#88C0D0')
  })

  it('runs status effects only while they are switched on and motion is allowed', async () => {
    const user = userEvent.setup()
    // Glass keeps the sweeps on until they are switched off; Flat and Tiled start with them off.
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'glass', effectWorking: true, effectNeedsYou: true, effectDone: true }))
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`, { status: 'working', agent: 'codex' })])
    renderDesktopApp()
    const row = await screen.findByRole('button', { name: '~/a, codex, working' })
    expect(row).toHaveClass('mx-fx-working')
    expect(screen.getByRole('tab', { selected: true })).toHaveClass('mx-fx-working', 'mx-fx-strong')

    await user.keyboard('{Control>},{/Control}')
    await user.keyboard('a')
    await user.click(screen.getByRole('switch', { name: 'Working' }))
    expect(screen.getByRole('button', { name: '~/a, codex, working' })).not.toHaveClass('mx-fx-working')

    await user.click(screen.getByRole('switch', { name: 'Reduce motion' }))
    expect(document.querySelector('.mx-app')).toHaveAttribute('data-motion', 'reduced')
    expect(screen.getByRole('switch', { name: 'Needs you' })).toBeDisabled()
    expect(screen.getByText('Off while Reduce motion is on.')).toBeInTheDocument()
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

describe('desktop sidebar projects', () => {
  const root = `${HOME}/code/maitty`
  const at = (pane: number, extra: Partial<Conversation> = {}) => inRepo(pane, root, `${root}/p${String(pane)}`, extra)

  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    daemon.defaultSessionFolder.mockResolvedValue(HOME)
    daemon.startShellSession.mockResolvedValue(undefined)
    daemon.loadConversations.mockResolvedValue([])
    daemon.suggestProjectFolders.mockResolvedValue([])
  })

  const sidebar = () => screen.findByRole('navigation', { name: 'Sessions by project' })

  it('leads with the session you are in and tucks the others into a tray, most urgent first', async () => {
    daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'working' }), at(4, { status: 'needs-you' })])
    renderDesktopApp()

    const nav = await sidebar()
    const tray = await within(nav).findByRole('group', { name: 'Other sessions in maitty' })
    expect(within(nav).getByRole('button', { name: 'maitty/p2, shell, idle'.replace('maitty/', '') })).toHaveAttribute('aria-current', 'page')
    const items = within(tray).getAllByRole('button').map((item) => item.getAttribute('aria-label'))
    expect(items).toEqual(['p4, shell, needs you, detached', 'p3, shell, working, detached'])
  })

  it('folds sessions beyond what fits behind +N, and lists every one in a menu that opens that session', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2), at(3), at(4), at(5, { status: 'needs-you' }), at(6, { status: 'needs-you' }), at(7, { status: 'needs-you' })])
    renderDesktopApp()

    const nav = await sidebar()
    const more = await within(nav).findByRole('button', { name: /^All 6 sessions in maitty, some hidden ones need you$/ })
    expect(more).toHaveTextContent('+3')
    expect(more).toHaveAttribute('aria-expanded', 'false')
    expect(within(nav).getByRole('group', { name: 'Other sessions in maitty' }).querySelectorAll('.mx-tray-item')).toHaveLength(2)

    await user.click(more)
    const menu = await screen.findByRole('menu', { name: 'maitty' })
    expect(within(menu).getAllByRole('menuitem')).toHaveLength(6)
    expect(menu).toHaveTextContent('6 sessions')
    expect(within(menu).getAllByRole('menuitem')[0]).toHaveTextContent('p5')

    await user.click(within(menu).getByRole('menuitem', { name: /p4/ }))
    expect(screen.queryByRole('menu', { name: 'maitty' })).not.toBeInTheDocument()
    expect(await screen.findByRole('region', { name: 'Shell in ~/code/maitty/p4' })).toBeInTheDocument()
  })

  it('closes the +N menu with Escape', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'names' }))
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2), at(3), at(4), at(5)])
    renderDesktopApp()

    await user.click(await within(await sidebar()).findByRole('button', { name: /^All 4 sessions in maitty/ }))
    expect(await screen.findByRole('menu', { name: 'maitty' })).toBeInTheDocument()
    await user.keyboard('{Escape}')

    expect(screen.queryByRole('menu', { name: 'maitty' })).not.toBeInTheDocument()
  })

  it('lays the tray out as icons or as one line per session, as chosen in Settings', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'icons' }))
    daemon.loadConversations.mockResolvedValue([at(2), at(3), at(4)])
    const first = renderDesktopApp()
    let nav = await sidebar()
    await within(nav).findByRole('group', { name: 'Other sessions in maitty' })
    expect(nav.querySelectorAll('.mx-tray-item.is-icons')).toHaveLength(2)
    expect(nav.querySelector('.mx-tray-proc')).toBeNull()
    first.unmount()

    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'stacked' }))
    renderDesktopApp()
    nav = await sidebar()
    await within(nav).findByRole('group', { name: 'Other sessions in maitty' })
    expect(nav.querySelectorAll('.mx-tray-item.is-stacked')).toHaveLength(2)
    expect(nav.querySelector('.mx-tray-where')).toHaveTextContent('p')
  })

  it('can lead with the session that needs you most instead of the one you are in', async () => {
    window.localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ leadMode: 'attention' }))
    daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you' })])
    renderDesktopApp()

    const nav = await sidebar()
    expect(await within(nav).findByRole('button', { name: 'p3, shell, needs you' })).toBeInTheDocument()
    expect(within(nav).getByRole('group', { name: 'Other sessions in maitty' })).toHaveTextContent('shell')
    expect(within(nav).getByRole('button', { name: 'p2, shell, idle, pane 1' })).toBeInTheDocument()
  })

  it('saves the tray style and the lead from Settings › Appearance', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    await user.keyboard('{Control>},{/Control}')
    await user.keyboard('a')

    await user.click(screen.getByRole('radio', { name: 'Icons' }))
    await user.click(screen.getByRole('radio', { name: 'Needs attention' }))

    const saved = JSON.parse(window.localStorage.getItem('maitty.gui.settings.v1') ?? '{}') as { trayStyle?: string; leadMode?: string }
    expect(saved).toMatchObject({ trayStyle: 'icons', leadMode: 'attention' })
  })

  it('opens the + after the tabs in the project you are in', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2)])
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'New session in maitty' }))

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(root) })
  })
})

describe('desktop overlays and tab tools', () => {
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

  it('opens the palette with Ctrl+K, finds a session by typing, opens it with Enter and closes with Escape', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`, { agent: 'codex' })])
    renderDesktopApp()
    await screen.findByRole('region', { name: 'Shell in ~/a' })

    await user.keyboard('{Control>}k{/Control}')
    const dialog = await screen.findByRole('dialog', { name: 'Command palette' })
    await user.type(within(dialog).getByRole('combobox'), 'codex')
    await user.click(within(dialog).getByRole('option', { name: /~\/b · codex/ }))

    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument()
    expect(await screen.findByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()

    await user.keyboard('{Control>}k{/Control}')
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument()
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument()
  })

  it('offers themes in the palette only once something is typed, and applies the one picked', async () => {
    const user = userEvent.setup()
    renderDesktopApp()
    await screen.findByRole('heading', { name: 'No sessions running' })
    await user.keyboard('{Control>}k{/Control}')
    const dialog = await screen.findByRole('dialog', { name: 'Command palette' })
    expect(within(dialog).queryByRole('option', { name: /Abyss/ })).not.toBeInTheDocument()

    await user.type(within(dialog).getByRole('combobox'), 'abyss')
    await user.keyboard('{Enter}')

    await waitFor(() => { expect(JSON.parse(window.localStorage.getItem('maitty.gui.settings.v1') ?? '{}')).toMatchObject({ themeMode: 'dark', darkTheme: 'Abyss' }) })
  })

  it('lists what needs you in the inbox and types the answer into that session', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you', summary: 'Allow git push origin main?' }), at(4, { status: 'done' })])
    renderDesktopApp()
    const open = await screen.findByRole('button', { name: /^Inbox, 2, something needs you$/ })

    await user.click(open)
    const inbox = await screen.findByRole('dialog', { name: 'Inbox' })
    const cards = within(inbox).getAllByRole('article')
    expect(cards).toHaveLength(2)
    expect(cards[0]).toHaveTextContent('Needs you')
    expect(cards[0]).toHaveTextContent('Allow git push origin main?')

    await user.click(within(cards[0] as HTMLElement).getByRole('button', { name: /^Yes, once/ }))
    expect(daemon.sendTerminalInput).toHaveBeenCalledWith(expect.objectContaining({ id: 't3' }), 'y')

    await user.click(within(cards[1] as HTMLElement).getByRole('button', { name: 'Clear' }))
    expect(within(inbox).getAllByRole('article')).toHaveLength(1)
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog', { name: 'Inbox' })).not.toBeInTheDocument()
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
    await user.keyboard('{Control>}k{/Control}')
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
    expect(daemon.startShellSession).toHaveBeenCalled()

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

  it('says it is all caught up when nothing needs you', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2)])
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'Inbox, 0' }))

    expect(await screen.findByText('You\'re all caught up')).toBeInTheDocument()
  })

  it('hides the sidebar and tabs in focus mode and brings them back with Escape', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2), at(3, { status: 'needs-you' })])
    renderDesktopApp()
    await screen.findByRole('navigation', { name: 'Sessions by project' })

    await user.keyboard('{Control>}.{/Control}')
    const pill = await screen.findByRole('status', { name: '' }, { timeout: 1000 }).catch(() => null)
    expect(pill ?? document.querySelector('.mx-focus-pill')).not.toBeNull()
    expect(screen.queryByRole('navigation', { name: 'Sessions by project' })).not.toBeInTheDocument()
    expect(screen.queryByRole('tablist')).not.toBeInTheDocument()
    expect(document.querySelector('.mx-focus-pill')).toHaveTextContent('1 need you elsewhere')

    await user.keyboard('{Escape}')
    expect(await screen.findByRole('navigation', { name: 'Sessions by project' })).toBeInTheDocument()
  })

  it('moves a session dragged from the sidebar into a pane, and the old one keeps running', async () => {
    daemon.loadConversations.mockResolvedValue([term(2, `${HOME}/a`), term(3, `${HOME}/b`)])
    renderDesktopApp()
    const pane = await screen.findByRole('region', { name: /^Terminal 1: / })
    const row = screen.getByRole('button', { name: '~/b, shell, idle' })
    expect(row).toHaveAttribute('draggable', 'true')

    fireEvent.dragOver(pane, { dataTransfer: { types: ['application/x-maitty-session'] } })
    expect(pane).toHaveClass('is-drop-target')
    fireEvent.drop(pane, { dataTransfer: { getData: (type: string) => (type === 'application/x-maitty-session' ? 't3' : '') } })

    expect(await screen.findByRole('region', { name: 'Shell in ~/b' })).toBeInTheDocument()
    expect(screen.queryByText('Session moved into this pane')).toBeInTheDocument()
    expect(daemon.archiveConversationOnDaemon).not.toHaveBeenCalled()
  })

  it('opens a menu on a project header with a new session in its folder', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2)])
    renderDesktopApp()
    const nav = await screen.findByRole('navigation', { name: 'Sessions by project' })

    fireEvent.contextMenu(await within(nav).findByRole('button', { name: /^maitty/, expanded: true }))
    expect(await screen.findByText('maitty', { selector: '.mx-menu-caps' })).toBeInTheDocument()
    await user.click(screen.getByRole('menuitem', { name: 'New session here' }))

    await waitFor(() => { expect(daemon.startShellSession).toHaveBeenCalledWith(root) })
  })

  it('broadcasts typing from one pane to the tab\'s idle panes only', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([at(2)])
    renderDesktopApp()
    await screen.findByRole('region', { name: /^Shell in/ })
    daemon.loadConversations.mockResolvedValue([at(2), at(7), at(8, { status: 'working' })])
    await user.keyboard('{Control>}d{/Control}')
    await screen.findByRole('region', { name: /^Terminal 2:/ })

    await user.click(screen.getAllByRole('button', { name: 'Broadcast typing to this tab' })[0] as HTMLElement)

    expect(await screen.findAllByText('◉ Broadcast')).toHaveLength(2)
    expect(screen.getAllByRole('button', { name: 'Stop broadcasting' })).toHaveLength(2)
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
