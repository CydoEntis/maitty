import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Fleet } from '@/api/fleets'
import type { Conversation } from '@/data/preview'
import { LOCAL_HEAD } from '../model/fleets'
import type { MachineHead } from '../model/machines'
import { toSessionView, type SessionView } from '../model/session'
import { Sidebar, type SidebarProps } from './Sidebar'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(1)) }))

function fleet(id: number, name: string, color = 0, machine = 0): Fleet {
  return { key: `${String(machine)}:${String(id)}`, machine, id, name, color, worktree: false, sessionIds: [] }
}

/** A ship in a fleet. `extra` sets what the daemon says of it. */
function ship(pane: number, inFleet: Fleet, extra: Partial<Conversation> = {}): SessionView {
  const cwd = extra.cwd ?? `/home/me/app${String(pane)}`
  return toSessionView(
    { id: `t${String(pane)}`, generation: 0, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: 'working tree', provider: 'Codex', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra },
    '/home/me',
    { fleet: inFleet.key, worktree: false },
  )
}

const MAIN = fleet(1, 'Maitty', 0)
const GROWTH = fleet(2, 'AeVox launch', 1)
const DEVBOX: MachineHead = { ...LOCAL_HEAD, key: 'machine-4', machine: 4, name: 'devbox', label: 'devbox', chip: { glyph: '●', word: 'connected', tone: 'ok' } }
const DEVBOX_FLEET = fleet(1, 'build', 2, 4)

function show(sessions: SessionView[], more: Partial<SidebarProps> = {}) {
  const none = vi.fn()
  render(
    <Sidebar
      sessions={sessions} fleets={[MAIN, GROWTH]} machines={[LOCAL_HEAD]} selectedFleet={MAIN.key} focusedId={null} tracks={new Map()} mode="docked" online canStart portalContainer={null} onToggleRail={none}
      side="left" width={264} onResize={none} onResizeEnd={none} inboxCount={0} inboxNeeds={false} onOpenInbox={none}
      onNewAgentChat={none} onNewShell={none} onAddMachine={none} onSelectFleet={none} onAddFleet={none} onRenameFleet={none} viewOf={() => 'terminal'}
      onShowSession={none} onEndSession={none} onSetName={none} onRenameSession={none} onPinSession={none}
      onSplitSession={none} onCopyPath={none} onOpenSettings={none} updateSlot={null} railUpdateSlot={null}
      {...more}
    />,
  )
}

function rowAt(index: number): HTMLElement {
  const found = screen.getAllByRole('button', { name: /, (idle|working|needs you|done|exited)$/ })[index]
  if (!found) throw new Error(`no row ${String(index)}`)
  return found
}

describe('the fleets section', () => {
  it('lists a machine heading with its fleets as named rows, the selected one marked', () => {
    show([ship(1, MAIN), ship(2, GROWTH)])
    const fleets = within(screen.getByRole('region', { name: 'Fleets' }))
    expect(fleets.getByText('This computer')).toBeTruthy()
    expect(fleets.getByRole('button', { name: /^Maitty, 1 ship/ }).getAttribute('aria-current')).toBe('true')
    expect(fleets.getByRole('button', { name: /^AeVox launch, 1 ship/ }).getAttribute('aria-current')).toBeNull()
  })

  it('shows the number of ships that need you on the fleet tile', () => {
    show([ship(1, GROWTH, { status: 'needs-you' }), ship(2, GROWTH, { status: 'needs-you' }), ship(3, GROWTH)])
    const row = screen.getByRole('button', { name: /^AeVox launch/ })
    expect(row.getAttribute('aria-label')).toContain('2 need you')
    expect(within(row).getByText('2')).toBeTruthy()
  })

  it('stacks at most three app icons, Claude first, then +N', () => {
    show([ship(1, MAIN), ship(2, MAIN, { agent: 'codex' }), ship(3, MAIN, { agent: 'claude' }), ship(4, MAIN, { agent: 'claude' }), ship(5, MAIN)])
    const row = screen.getByRole('button', { name: /^Maitty/ })
    const icons = row.querySelectorAll('.mx-fleet-stack .mx-app-icon')
    expect(icons).toHaveLength(3)
    expect(icons[0]?.className).toContain('is-claude')
    expect(icons[1]?.className).toContain('is-claude')
    expect(icons[2]?.className).toContain('is-codex')
    expect(within(row).getByText('+2')).toBeTruthy()
  })

  it('selects a fleet on click and renames it on a double-click', async () => {
    const user = userEvent.setup()
    const onSelectFleet = vi.fn()
    const onRenameFleet = vi.fn()
    show([], { onSelectFleet, onRenameFleet })
    await user.click(screen.getByRole('button', { name: /^AeVox launch/ }))
    expect(onSelectFleet).toHaveBeenCalledWith(GROWTH.key)
    await user.dblClick(screen.getByRole('button', { name: /^AeVox launch/ }))
    await user.type(screen.getByRole('textbox', { name: /Name for AeVox launch/ }), '{Control>}a{/Control}Launch two{Enter}')
    expect(onRenameFleet).toHaveBeenCalledWith(GROWTH, 'Launch two')
  })

  it('adds a fleet on a machine with an inline name field: Enter creates, Esc cancels, a taken name is refused', async () => {
    const user = userEvent.setup()
    const onAddFleet = vi.fn()
    show([], { onAddFleet })
    await user.click(screen.getByRole('button', { name: 'Add a fleet on This computer' }))
    await user.type(screen.getByRole('textbox', { name: /Name for the new fleet/ }), 'Docs{Enter}')
    expect(onAddFleet).toHaveBeenCalledWith(0, 'Docs')
    await user.click(screen.getByRole('button', { name: 'Add a fleet on This computer' }))
    await user.type(screen.getByRole('textbox', { name: /Name for the new fleet/ }), 'maitty{Enter}')
    expect(onAddFleet).toHaveBeenCalledTimes(1)
    await user.click(screen.getByRole('button', { name: 'Add a fleet on This computer' }))
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('textbox', { name: /Name for the new fleet/ })).toBeNull()
  })

  it('dims an offline machine and disables its +', () => {
    const away: MachineHead = { ...DEVBOX, down: true, chip: { glyph: '✕', word: 'unreachable', tone: 'bad' } }
    show([], { machines: [LOCAL_HEAD, away], fleets: [MAIN, DEVBOX_FLEET] })
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Add a fleet on devbox' }).disabled).toBe(true)
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Add a fleet on This computer' }).disabled).toBe(false)
    expect(screen.getByRole('button', { name: /^build/ }).className).toContain('is-dimmed')
    expect(screen.getByRole('img', { name: 'devbox is unreachable' })).toBeTruthy()
  })

  it('folds a section with its caret and hides the folded content from the keyboard and screen readers', async () => {
    const user = userEvent.setup()
    show([ship(1, MAIN)])
    const toggle = screen.getByRole('button', { name: 'FLEETS' })
    expect(toggle.getAttribute('aria-expanded')).toBe('true')
    await user.click(toggle)
    expect(toggle.getAttribute('aria-expanded')).toBe('false')
    const folded = screen.getByRole('region', { name: 'Fleets' }).querySelector('.mx-collapse-inner')
    expect(folded?.getAttribute('aria-hidden')).toBe('true')
    expect(folded?.hasAttribute('inert')).toBe(true)
  })
})

describe('the ships section', () => {
  it('lists only the selected fleet, in Agents and Shells; agents show a branch, shells a folder and never a branch', () => {
    show([
      ship(1, MAIN, { agent: 'claude', repo: 'maitty', folder: '/home/me/maitty', branch: 'agent/grow-028' }),
      ship(2, MAIN, { cwd: '/home/me/notes', branch: 'main', repo: 'notes', folder: '/home/me/notes' }),
      ship(3, GROWTH, { agent: 'claude' }),
    ])
    const ships = within(screen.getByRole('region', { name: 'Ships' }))
    expect(ships.getByRole('button', { name: /^Agents/ }).textContent).toContain('1')
    expect(ships.getByRole('button', { name: /^Shells/ }).textContent).toContain('1')
    expect(ships.getAllByRole('button', { name: /, (idle|working)$/ })).toHaveLength(2)
    const agents = within(ships.getByRole('list', { name: 'Agents' }))
    expect(agents.getByText('agent/grow-028')).toBeTruthy()
    const shells = within(ships.getByRole('list', { name: 'Shells' }))
    expect(shells.getByText('~/notes', { exact: false })).toBeTruthy()
    expect(shells.queryByText('main')).toBeNull()
  })

  it('shows neither diff counts nor folder badges on a row', () => {
    show([ship(1, MAIN, { agent: 'claude', diffAdded: 104, diffRemoved: 61 })])
    expect(screen.queryByText('+104')).toBeNull()
    expect(screen.queryByRole('button', { name: /^Show only/ })).toBeNull()
  })

  it('marks the selected ship and opens it when its row is clicked', async () => {
    const onShowSession = vi.fn()
    show([ship(1, MAIN), ship(2, MAIN)], { focusedId: 't2', onShowSession })
    const rows = screen.getAllByRole('button', { name: /, idle$/ })
    expect(rows[1]?.getAttribute('aria-current')).toBe('page')
    expect(rows[0]?.getAttribute('aria-current')).toBeNull()
    await userEvent.setup().click(rowAt(0))
    expect(onShowSession).toHaveBeenCalledWith('t1')
  })

  it('says a fleet is empty, and says a filter found nothing in it while naming a fleet that matches', async () => {
    const user = userEvent.setup()
    show([ship(1, MAIN), ship(2, GROWTH)])
    const field = screen.getByRole('textbox', { name: 'Filter sessions' })
    await user.type(field, 'aevox')
    expect(screen.getByText(/Nothing in Maitty matches “aevox”\. Try AeVox launch above\./)).toBeTruthy()
    await user.clear(field)
    expect(screen.queryByText(/Nothing in Maitty/)).toBeNull()
  })

  it('shows "Nothing aboard yet" for an empty fleet', () => {
    show([ship(2, GROWTH)])
    expect(screen.getByText('Nothing aboard yet. Use + to start a session.')).toBeTruthy()
  })

  it('switches to Priority: every agent across fleets, most urgent first, each row naming its fleet', async () => {
    const user = userEvent.setup()
    show([
      ship(1, MAIN, { agent: 'claude', status: 'working' }),
      ship(2, GROWTH, { agent: 'codex', status: 'needs-you' }),
      ship(3, GROWTH, { agent: 'claude', status: 'done' }),
      ship(4, MAIN),
    ])
    await user.click(screen.getByRole('radio', { name: 'Priority' }))
    const ships = within(screen.getByRole('region', { name: 'Ships' }))
    const titles = ships.getAllByRole('button', { expanded: true }).map((button) => button.textContent).filter((text) => /^(Needs you|Working|Done|Exited|Idle)/.test(text))
    expect(titles.map((text) => text.replace(/\d+$/, ''))).toEqual(['Needs you', 'Working', 'Done'])
    expect(ships.getByRole('list', { name: 'Needs you' }).textContent).toContain('AeVox launch')
    expect(ships.queryByText('Shells')).toBeNull()
  })
})

describe('the header and footer', () => {
  it('filters on fleet names too, and clears with Esc', async () => {
    const user = userEvent.setup()
    show([ship(1, MAIN, { agent: 'claude' })])
    const field = screen.getByRole('textbox', { name: 'Filter sessions' })
    await user.type(field, 'launch')
    expect(screen.queryByRole('button', { name: /^Maitty/ })).toBeNull()
    expect(screen.getByRole('button', { name: /^AeVox launch/ })).toBeTruthy()
    await user.clear(field)
    await user.type(field, 'zzz')
    expect(screen.getByText(/No fleet matches/)).toBeTruthy()
    await user.keyboard('{Escape}')
    expect((field as HTMLInputElement).value).toBe('')
  })

  it('matches a ship by its branch', async () => {
    const user = userEvent.setup()
    show([ship(1, MAIN, { agent: 'claude', repo: 'maitty', folder: '/home/me/maitty', branch: 'agent/grow-028' }), ship(2, MAIN, { agent: 'codex' })])
    await user.type(screen.getByRole('textbox', { name: 'Filter sessions' }), 'grow-028')
    expect(screen.getAllByRole('button', { name: /, idle$/ })).toHaveLength(1)
  })

  it('focuses the filter on / from outside a field, but not while typing in one', async () => {
    const user = userEvent.setup()
    show([ship(1, MAIN)])
    await user.keyboard('/')
    expect(document.activeElement).toBe(screen.getByRole('textbox', { name: 'Filter sessions' }))
    await user.keyboard('/')
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Filter sessions' }).value).toBe('/')
  })

  it('shows the Inbox count and turns amber when something needs you', () => {
    show([ship(1, MAIN)], { inboxCount: 3, inboxNeeds: true })
    const inbox = screen.getByRole('button', { name: 'Inbox, 3, something needs you' })
    expect(inbox.className).toContain('is-needs')
    expect(within(inbox).getByText('3')).toBeTruthy()
  })

  it('starts a chat, a shell, adds a machine, opens the Inbox and Settings from the header and footer', async () => {
    const user = userEvent.setup()
    const handlers = { onOpenInbox: vi.fn(), onNewAgentChat: vi.fn(), onNewShell: vi.fn(), onAddMachine: vi.fn(), onOpenSettings: vi.fn() }
    show([], handlers)
    await user.click(screen.getByRole('button', { name: 'Inbox, 0' }))
    await user.click(screen.getByRole('button', { name: 'New agent chat' }))
    await user.click(screen.getByRole('button', { name: 'New shell' }))
    await user.click(screen.getByRole('button', { name: 'Add a remote machine' }))
    await user.click(screen.getByRole('button', { name: 'Settings' }))
    expect(Object.values(handlers).map((mock) => mock.mock.calls.length)).toEqual([1, 1, 1, 1, 1])
    expect(screen.queryByRole('button', { name: 'New session' })).toBeNull()
  })

  it('disables the starters while nothing can be started', () => {
    show([], { canStart: false })
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'New agent chat' }).disabled).toBe(true)
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'New shell' }).disabled).toBe(true)
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Add a remote machine' }).disabled).toBe(false)
  })
})

describe('the row menu', () => {
  it('offers rename, pin, split right and down, copy path and close, and pins without moving the row', () => {
    const onPinSession = vi.fn()
    const onSplitSession = vi.fn()
    show([ship(1, MAIN), ship(2, MAIN)], { onPinSession, onSplitSession })
    fireEvent.contextMenu(rowAt(1))
    const names = screen.getAllByRole('menuitem').map((item) => item.textContent)
    expect(names).toEqual(['RenameF2', 'Pin', 'Open in split right', 'Open in split down', 'Copy path', 'Close sessionends it'])
    fireEvent.click(screen.getByRole('menuitem', { name: 'Pin' }))
    expect(onPinSession).toHaveBeenCalledWith('t2', true)
    fireEvent.contextMenu(rowAt(0))
    fireEvent.click(screen.getByRole('menuitem', { name: 'Open in split down' }))
    expect(onSplitSession).toHaveBeenCalledWith('t1', 'down')
  })

  it('renames in place with F2: Enter saves, and Esc cancels', async () => {
    const user = userEvent.setup()
    const onSetName = vi.fn()
    show([ship(1, MAIN)], { onSetName })
    screen.getByRole('button', { name: /, idle$/ }).focus()
    await user.keyboard('{F2}')
    await user.type(screen.getByRole('textbox', { name: /Name for/ }), 'Growth{Enter}')
    expect(onSetName).toHaveBeenCalledWith('t1', 'Growth')
    await user.keyboard('{F2}')
    await user.keyboard('{Escape}')
    expect(onSetName).toHaveBeenCalledTimes(1)
  })

  it('takes two clicks to close a session that runs an agent, and one for a plain shell', () => {
    const onEndSession = vi.fn()
    show([ship(1, MAIN, { agent: 'claude' }), ship(2, MAIN)], { onEndSession })
    fireEvent.contextMenu(rowAt(0))
    fireEvent.click(screen.getByRole('menuitem', { name: /Close session/ }))
    expect(onEndSession).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('menuitem', { name: /Close and stop claude/ }))
    expect(onEndSession).toHaveBeenCalledWith('t1')
    fireEvent.contextMenu(rowAt(1))
    fireEvent.click(screen.getByRole('menuitem', { name: /Close session/ }))
    expect(onEndSession).toHaveBeenLastCalledWith('t2')
  })
})

describe('the rail', () => {
  it('has one tile per fleet, the three starters, the Inbox, an expand button and Settings, and no filter', async () => {
    const user = userEvent.setup()
    const onSelectFleet = vi.fn()
    show([ship(1, GROWTH, { status: 'needs-you' })], { mode: 'rail', onSelectFleet })
    expect(within(screen.getByRole('group', { name: 'Fleets' })).getAllByRole('button')).toHaveLength(2)
    expect(screen.getByRole('button', { name: /^AeVox launch on This computer, 1 need you/ })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeTruthy()
    for (const name of ['New agent chat', 'New shell', 'Add a remote machine', 'Settings']) expect(screen.getByRole('button', { name })).toBeTruthy()
    expect(screen.queryByRole('textbox')).toBeNull()
    await user.click(screen.getByRole('button', { name: /^AeVox launch/ }))
    expect(onSelectFleet).toHaveBeenCalledWith(GROWTH.key)
  })
})
