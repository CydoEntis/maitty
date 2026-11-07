import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { toSessionView, type SessionView } from '../model/session'
import { Sidebar, type SidebarProps } from './Sidebar'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(1)) }))

function session(pane: number, extra: Partial<Conversation> = {}): SessionView {
  const cwd = extra.cwd ?? `/home/me/app${String(pane)}`
  return toSessionView({ id: `t${String(pane)}`, generation: 0, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: 'working tree', provider: 'Codex', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra }, '/home/me')
}

function show(sessions: SessionView[], more: Partial<SidebarProps> = {}) {
  const none = vi.fn()
  render(
    <Sidebar
      sessions={sessions} focusedId={null} tracks={new Map()} mode="docked" online canStart portalContainer={null} onToggleRail={none}
      side="left" width={264} onResize={none} onResizeEnd={none} inboxCount={0} inboxNeeds={false} onOpenInbox={none}
      onNewSession={none} onShowSession={none} onEndSession={none} onSetName={none} onRenameSession={none} onPinSession={none}
      onSplitSession={none} onCopyPath={none} onOpenSettings={none} updateSlot={null} railUpdateSlot={null}
      {...more}
    />,
  )
}

function rowAt(index: number): HTMLElement {
  const found = screen.getAllByRole('button', { name: /, idle$/ })[index]
  if (!found) throw new Error(`no row ${String(index)}`)
  return found
}

describe('the flat sidebar', () => {
  it('lists every session in the order it is given, with no project or machine headers', () => {
    show([session(1, { agent: 'claude' }), session(2), session(3, { machine: 4, machineName: 'devbox' })])
    const rows = screen.getAllByRole('button', { name: /, (idle|working|needs you)$/ })
    expect(rows).toHaveLength(3)
    expect(screen.queryByRole('region')).toBeNull()
    expect(rows[0]?.getAttribute('aria-label')).toContain('claude')
  })

  it('shows the folder badge, the machine mark and the diff counts on a row', () => {
    show([session(1, { diffAdded: 104, diffRemoved: 61 }), session(2, { machine: 4, machineName: 'devbox' })])
    expect(screen.getByText('+104')).toBeTruthy()
    expect(screen.getByText('−61')).toBeTruthy()
    expect(screen.getByTitle('This computer')).toBeTruthy()
    expect(screen.getByTitle('devbox')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Show only app1' })).toBeTruthy()
  })

  it('shows no diff counts for a checkout with no changes', () => {
    show([session(1)])
    expect(screen.queryByText(/^\+\d/)).toBeNull()
  })

  it('marks the selected row and opens a session when its row is clicked', async () => {
    const onShowSession = vi.fn()
    show([session(1), session(2)], { focusedId: 't2', onShowSession })
    const rows = screen.getAllByRole('button', { name: /, idle$/ })
    expect(rows[1]?.getAttribute('aria-current')).toBe('page')
    expect(rows[0]?.getAttribute('aria-current')).toBeNull()
    await userEvent.setup().click(rowAt(0))
    expect(onShowSession).toHaveBeenCalledWith('t1')
  })

  it('filters by name, folder, process and machine without reordering, and clears with Esc', async () => {
    const user = userEvent.setup()
    show([session(1, { cwd: '/home/me/alpha' }), session(2, { cwd: '/home/me/beta', agent: 'claude' }), session(3, { cwd: '/srv/gamma', machine: 4, machineName: 'devbox' })])
    const field = screen.getByRole('textbox', { name: 'Filter sessions' })
    await user.type(field, 'claude')
    expect(screen.getAllByRole('button', { name: /, idle$/ })).toHaveLength(1)
    await user.clear(field)
    await user.type(field, 'devbox')
    expect(screen.getAllByRole('button', { name: /, idle$/ })).toHaveLength(1)
    await user.clear(field)
    await user.type(field, 'zzz')
    expect(screen.getByText(/No session matches/)).toBeTruthy()
    await user.keyboard('{Escape}')
    expect((field as HTMLInputElement).value).toBe('')
    expect(screen.getAllByRole('button', { name: /, idle$/ })).toHaveLength(3)
  })

  it('focuses the filter on / from outside a field, but not while typing in one', async () => {
    const user = userEvent.setup()
    show([session(1)])
    await user.keyboard('/')
    expect(document.activeElement).toBe(screen.getByRole('textbox', { name: 'Filter sessions' }))
    await user.keyboard('/')
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Filter sessions' }).value).toBe('/')
  })

  it('puts a folder in the filter when its badge is clicked', () => {
    show([session(1, { cwd: '/home/me/alpha' }), session(2, { cwd: '/home/me/beta' })])
    fireEvent.click(screen.getByRole('button', { name: 'Show only alpha' }))
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Filter sessions' }).value).toBe('alpha')
    expect(screen.getAllByRole('button', { name: /, idle$/ })).toHaveLength(1)
  })

  it('shows the Inbox count and turns amber when something needs you', () => {
    show([session(1)], { inboxCount: 3, inboxNeeds: true })
    const inbox = screen.getByRole('button', { name: 'Inbox, 3, something needs you' })
    expect(inbox.className).toContain('is-needs')
    expect(within(inbox).getByText('3')).toBeTruthy()
  })

  it('opens the Inbox, a new session and Settings from the header and footer', async () => {
    const user = userEvent.setup()
    const onOpenInbox = vi.fn()
    const onNewSession = vi.fn()
    const onOpenSettings = vi.fn()
    show([], { onOpenInbox, onNewSession, onOpenSettings })
    await user.click(screen.getByRole('button', { name: 'Inbox, 0' }))
    await user.click(screen.getByRole('button', { name: 'New session' }))
    await user.click(screen.getByRole('button', { name: 'Settings' }))
    expect([onOpenInbox, onNewSession, onOpenSettings].map((mock) => mock.mock.calls.length)).toEqual([1, 1, 1])
    expect(screen.getByText(/No sessions yet/)).toBeTruthy()
  })
})

describe('the row menu', () => {
  it('offers rename, pin, split right and down, copy path and close, and pins without moving the row', () => {
    const onPinSession = vi.fn()
    const onSplitSession = vi.fn()
    show([session(1), session(2)], { onPinSession, onSplitSession })
    fireEvent.contextMenu(rowAt(1))
    const names = screen.getAllByRole('menuitem').map((item) => item.textContent)
    expect(names).toEqual(['RenameF2', 'Pin to top', 'Open in split right', 'Open in split down', 'Copy path', 'Close sessionends it'])
    fireEvent.click(screen.getByRole('menuitem', { name: 'Pin to top' }))
    expect(onPinSession).toHaveBeenCalledWith('t2', true)
    fireEvent.contextMenu(rowAt(0))
    fireEvent.click(screen.getByRole('menuitem', { name: 'Open in split down' }))
    expect(onSplitSession).toHaveBeenCalledWith('t1', 'down')
  })

  it('renames in place with F2: Enter saves, and Esc cancels', async () => {
    const user = userEvent.setup()
    const onSetName = vi.fn()
    show([session(1)], { onSetName })
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
    show([session(1, { agent: 'claude' }), session(2)], { onEndSession })
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
  it('has one icon per session, an expand button, a new session button and Settings, and no filter', () => {
    show([session(1), session(2)], { mode: 'rail' })
    expect(screen.getAllByRole('button', { name: /, idle$/ })).toHaveLength(2)
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'New session' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Settings' })).toBeTruthy()
    expect(screen.queryByRole('textbox')).toBeNull()
  })
})
