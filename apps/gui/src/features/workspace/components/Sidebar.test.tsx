import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Conversation, SubagentActivity } from '@/data/preview'
import { LOCAL_HEAD, type MachineHead } from '../model/machines'
import { toSessionView, type SessionView } from '../model/session'
import { Sidebar, type SidebarProps } from './Sidebar'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(id.indexOf('t') + 1)) }))

const DEVBOX: MachineHead = { ...LOCAL_HEAD, key: 'machine-4', machine: 4, name: 'devbox', label: 'devbox', chip: { glyph: '●', word: 'connected', tone: 'ok' } }

/** A session as the daemon reports it. `extra` sets what the daemon says of it. */
function session(pane: number, extra: Partial<Conversation> = {}): SessionView {
  const cwd = extra.cwd ?? `/home/me/app${String(pane)}`
  return toSessionView(
    { id: `t${String(pane)}`, generation: 0, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: 'working tree', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra },
    '/home/me',
  )
}

const child = (id: string, task: string, state: SubagentActivity['state']): SubagentActivity => ({ id, name: 'Explore', state, task, startedAt: 100, updatedAt: 160, latest: null, result: null })

function show(sessions: SessionView[], more: Partial<SidebarProps> = {}) {
  const none = vi.fn()
  const props: SidebarProps = {
    sessions, machines: [LOCAL_HEAD], focusedId: null, openSubagent: null, mode: 'docked', online: true, canStart: true, renamingId: null, portalContainer: null,
    onToggleRail: none, side: 'left', width: 264, onResize: none, onResizeEnd: none, inboxCount: 0, inboxNeeds: false, onOpenInbox: none,
    onNewAgentChat: none, onNewShell: none, onAddMachine: none, onMachineSettings: none, onUpdateMachine: none, onShowSession: none, onShowSubagent: none, onRenameSession: none,
    onSetName: none, onSplitSession: none, onSplitSubagent: none, onOpenTerminal: none, onCopyPath: none, onCopyText: none, onShowChanges: none, onGetFile: none,
    onEndSession: none, onOpenSettings: none, updateSlot: null, railUpdateSlot: null,
    ...more,
  }
  return render(<Sidebar {...props} />)
}

const rows = () => screen.getAllByRole('button', { name: /, (idle|working|needs you|done|exited|unknown)$/ })
const names = () => rows().map((row) => row.getAttribute('aria-label')?.split(',')[0])

describe('the sidebar', () => {
  it('groups sessions by machine, this computer first, each with its count', () => {
    show([session(1, { agent: 'claude', named: true, title: 'Here' }), session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There' })], { machines: [LOCAL_HEAD, DEVBOX] })
    const here = screen.getByRole('list', { name: 'Sessions on This computer' })
    const there = screen.getByRole('list', { name: 'Sessions on devbox' })
    expect(within(here).getByRole('button', { name: /^Here,/ })).toBeTruthy()
    expect(within(there).getByRole('button', { name: /^There,/ })).toBeTruthy()
    expect(screen.getByRole('button', { name: /^This computer/, expanded: true })).toHaveTextContent('1')
  })

  it('lists the most urgent first: needs you, a stopped agent, working, done, then idle and an exited shell last', () => {
    show([
      session(1, { named: true, title: 'Exited shell', details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, exit: { code: 1, at: 1 } } }),
      session(2, { agent: 'claude', named: true, title: 'Idle agent' }),
      session(3, { agent: 'claude', named: true, title: 'Working', status: 'working' }),
      session(4, { agent: 'claude', named: true, title: 'Asking', status: 'needs-you' }),
      session(5, { agent: 'claude', named: true, title: 'Finished', status: 'done' }),
    ])
    expect(names()).toEqual(['Asking', 'Working', 'Finished', 'Idle agent', 'Exited shell'])
    expect(screen.getByRole('button', { name: /^Exited shell/ })).toHaveTextContent('exited')
  })

  it('shows the project and branch under the name, and names a shell by its folder', () => {
    show([session(1, { agent: 'claude', named: true, title: 'Growth', repo: 'AeVoxGrowth', branch: 'agent/grow-025', folder: '/home/me/AeVoxGrowth', cwd: '/home/me/AeVoxGrowth' }), session(2, { cwd: '/home/me/dev', folder: '/home/me/dev' })])
    const growth = screen.getByRole('button', { name: /^Growth,/ })
    expect(growth).toHaveTextContent('AeVoxGrowth')
    expect(growth).toHaveTextContent('agent/grow-025')
    expect(screen.getByRole('button', { name: /^dev, shell/ })).toBeTruthy()
  })

  it('groups a machine by project from its menu, and back to one list', async () => {
    const user = userEvent.setup()
    show([session(1, { cwd: '/home/me/a', folder: '/home/me/a' }), session(2, { cwd: '/home/me/b', folder: '/home/me/b' })])
    fireEvent.contextMenu(screen.getByRole('button', { name: /^This computer/ }))
    await user.click(screen.getByRole('menuitem', { name: 'Group by project' }))
    expect(screen.getByRole('list', { name: 'a' })).toBeTruthy()
    expect(screen.getByRole('list', { name: 'b' })).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Show one list, most urgent first' }))
    expect(screen.getByRole('list', { name: 'Sessions on This computer' })).toBeTruthy()
  })

  it('folds a machine away from its heading', async () => {
    const user = userEvent.setup()
    show([session(1)])
    await user.click(screen.getByRole('button', { name: /^This computer/ }))
    expect(screen.getByRole('button', { name: /^This computer/ })).toHaveAttribute('aria-expanded', 'false')
  })
})

describe('a machine that needs maitty installed or updated', () => {
  const needing = (action: MachineHead['updateAction']): MachineHead => ({ ...DEVBOX, chip: { glyph: '▲', word: 'needs update', tone: 'bad' }, down: true, updateAction: action })
  const machineMenuItems = () => {
    fireEvent.contextMenu(screen.getByRole('button', { name: /^devbox/ }))
    return within(screen.getByRole('menu', { name: 'devbox' })).getAllByRole('menuitem').map((item) => item.textContent)
  }

  it('says not installed under its header, with an Install button there', async () => {
    const user = userEvent.setup()
    const onUpdateMachine = vi.fn()
    show([], { machines: [LOCAL_HEAD, needing('install')], onUpdateMachine })
    expect(screen.getByText('not installed')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Install Maitty on devbox' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'install')
  })

  it('offers Install Maitty first in its menu', async () => {
    const user = userEvent.setup()
    const onUpdateMachine = vi.fn()
    show([], { machines: [LOCAL_HEAD, needing('install')], onUpdateMachine })
    expect(machineMenuItems()[0]).toBe('Install Maitty')
    await user.click(screen.getByRole('menuitem', { name: 'Install Maitty' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'install')
  })

  it('says needs update under its header and offers Update Maitty for another version', async () => {
    const user = userEvent.setup()
    const onUpdateMachine = vi.fn()
    show([session(1)], { machines: [LOCAL_HEAD, needing('update')], onUpdateMachine })
    expect(screen.getByText('needs update')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Update Maitty on devbox' })).toBeInTheDocument()
    expect(machineMenuItems()).toContain('Update Maitty')
    await user.click(screen.getByRole('menuitem', { name: 'Update Maitty' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'update')
  })

  it('offers neither on a machine that is up to date, nor on this computer', () => {
    show([session(1)], { machines: [LOCAL_HEAD, DEVBOX] })
    expect(machineMenuItems().filter((item) => item.endsWith('Maitty'))).toEqual([])
    fireEvent.contextMenu(screen.getByRole('button', { name: /^This computer/ }))
    expect(screen.queryByRole('menuitem', { name: /Maitty$/ })).toBeNull()
    expect(screen.queryByText(/needs update|not installed/)).toBeNull()
    expect(screen.queryByRole('button', { name: /Maitty on/ })).toBeNull()
  })
})

describe('subagents', () => {
  const parent = (extra: Partial<Conversation> = {}) => session(1, { agent: 'claude', named: true, title: 'Growth', status: 'working', details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, subagents: [child('a', 'Check migration order', 'Completed'), child('b', 'Write drafts tests', 'Working'), child('c', 'Load test', 'AwaitingApproval')] }, ...extra })

  it('shows small icons under a session that is not selected, saying each one’s state', () => {
    show([parent()])
    expect(screen.getByRole('img', { name: 'Subagents: Check migration order done, Write drafts tests working, Load test needs you' })).toBeTruthy()
    expect(screen.queryByRole('group', { name: 'Subagents' })).toBeNull()
  })

  it('lists them under the selected session, and opening one asks for its own chat', async () => {
    const user = userEvent.setup()
    const onShowSubagent = vi.fn()
    show([parent()], { focusedId: 't1', onShowSubagent })
    const list = screen.getByRole('group', { name: 'Subagents' })
    await user.click(within(list).getByRole('button', { name: 'Write drafts tests, working' }))
    expect(onShowSubagent).toHaveBeenCalledWith('t1', 'b')
  })

  it('marks the subagent whose chat is open', () => {
    show([parent()], { focusedId: 't1', openSubagent: { sessionId: 't1', subagentId: 'a' } })
    expect(screen.getByRole('button', { name: 'Check migration order, done' })).toHaveAttribute('aria-current', 'page')
  })

  it('offers Open, Open in split right, Copy summary and a Stop it cannot do yet on right-click', async () => {
    const user = userEvent.setup()
    const onSplitSubagent = vi.fn()
    show([parent()], { focusedId: 't1', onSplitSubagent })
    fireEvent.contextMenu(screen.getByRole('button', { name: 'Write drafts tests, working' }))
    const menu = screen.getByRole('menu', { name: 'Write drafts tests' })
    expect(within(menu).getByRole('menuitem', { name: 'Stop subagent' })).toBeDisabled()
    await user.click(within(menu).getByRole('menuitem', { name: 'Open in split right' }))
    expect(onSplitSubagent).toHaveBeenCalledWith('t1', 'b')
  })
})

describe('a session row', () => {
  it('starts renaming on a double-click', async () => {
    const user = userEvent.setup()
    const onRenameSession = vi.fn()
    show([session(1, { agent: 'claude' })], { onRenameSession })
    await user.dblClick(screen.getByRole('button', { name: /^claude,/ }))
    expect(onRenameSession).toHaveBeenCalledWith('t1')
  })

  it('offers no Get a file on a session of this computer', () => {
    show([session(1, { agent: 'claude' })])
    fireEvent.contextMenu(screen.getByRole('button', { name: /^claude,/ }))
    expect(screen.queryByRole('menuitem', { name: /Get a file/ })).toBeNull()
  })

  it('offers Get a file on a session of another machine, and asks for that session', async () => {
    const user = userEvent.setup()
    const onGetFile = vi.fn()
    show([session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There' })], { machines: [LOCAL_HEAD, DEVBOX], onGetFile })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^There,/ }))
    await user.click(screen.getByRole('menuitem', { name: 'Get a file…' }))
    expect(onGetFile).toHaveBeenCalledWith('t2')
  })

  it('cannot get a file while the daemon is offline', () => {
    show([session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There' })], { machines: [LOCAL_HEAD, DEVBOX], online: false })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^There,/ }))
    expect(screen.getByRole('menuitem', { name: 'Get a file…' })).toBeDisabled()
  })

  it('types the name in place while renaming, Enter saves and Escape cancels', async () => {
    const user = userEvent.setup()
    const onSetName = vi.fn()
    const onRenameSession = vi.fn()
    show([session(1, { agent: 'claude' })], { renamingId: 't1', onSetName, onRenameSession })
    const field = screen.getByRole('textbox', { name: 'Session name' })
    await user.type(field, 'Login fix{Enter}')
    expect(onSetName).toHaveBeenCalledWith('t1', 'Login fix')
  })

  it('offers the session menu on right-click, and arms Stop session before it stops an agent', async () => {
    const user = userEvent.setup()
    const onEndSession = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Growth' }), session(2)], { focusedId: 't2', onEndSession })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Growth,/ }))
    const menu = screen.getByRole('menu', { name: 'Growth' })
    expect(within(menu).getAllByRole('menuitem').map((item) => item.textContent)).toEqual(['Open', 'Open in split right', 'Open in split down', 'Open terminal here', 'Rename…F2', 'Copy folder path', 'Show changes', 'Stop session'])
    await user.click(within(menu).getByRole('menuitem', { name: 'Stop session' }))
    expect(onEndSession).not.toHaveBeenCalled()
    await user.click(within(menu).getByRole('menuitem', { name: 'Stop claude? Click again' }))
    expect(onEndSession).toHaveBeenCalledWith('t1')
  })

  it('offers Close shell, not Stop, for a shell', () => {
    show([session(1)])
    fireEvent.contextMenu(screen.getByRole('button', { name: /^app1,/ }))
    expect(screen.getByRole('menuitem', { name: 'Close shell' })).toBeTruthy()
    expect(screen.queryByRole('menuitem', { name: 'Open terminal here' })).toBeNull()
  })
})

describe('the filter', () => {
  it('narrows the list by name and says when nothing matches', async () => {
    const user = userEvent.setup()
    show([session(1, { named: true, title: 'Alpha' }), session(2, { named: true, title: 'Beta' })])
    await user.type(screen.getByRole('textbox', { name: 'Filter sessions' }), 'alp')
    expect(names()).toEqual(['Alpha'])
    await user.type(screen.getByRole('textbox', { name: 'Filter sessions' }), 'zzz')
    expect(screen.getByText('No agent matches “alpzzz”.')).toBeTruthy()
  })

  it('says how to start one when nothing runs', () => {
    show([])
    expect(screen.getByText('No agents running. Use the buttons above to start one.')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /^This computer/ })).toBeNull()
  })
})
