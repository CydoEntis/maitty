import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Conversation, SubagentActivity } from '@/data/preview'
import { writeMachineNicknames } from '@/data/machineNicknames'
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
    sessions, machines: [LOCAL_HEAD], order: 'creation', focusedId: null, mode: 'docked', online: true, canStart: true, renamingId: null, portalContainer: null,
    onToggleRail: none, side: 'left', width: 264, onResize: none, onResizeEnd: none, grouping: 'machine', onGrouping: none, onSearch: none,
    onNewAgentChat: none, onNewShell: none, onAddMachine: none, onMachineSettings: none, onRenameMachine: none, onUpdateMachine: none, onShowSession: none, onRenameSession: none,
    onSetName: none, onSplitSession: none, onCopyPath: none, onCopyText: none, onShowChanges: none, onMarkDone: none, onPutAway: none, onBringBack: none, onSnooze: none, onUnsnooze: none, onGetFile: none,
    onEndSession: none, onOpenSettings: none, onRecentSessions: none, updateSlot: null, railUpdateSlot: null,
    ...more,
  }
  const view = render(<Sidebar {...props} />)
  // Draws the same sidebar again with some props changed, as the window does when the selection moves.
  return { ...view, again: (next: Partial<SidebarProps>) => { view.rerender(<Sidebar {...props} {...next} />) } }
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

  it('leaves the machine off a row under its own machine heading', () => {
    show([session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There', repo: 'app2', branch: 'feat/remote-branch' })], { machines: [LOCAL_HEAD, DEVBOX] })
    const text = screen.getByRole('button', { name: /^There,/ }).querySelector('.mx-row-text')?.textContent ?? ''
    expect(text).toMatch(/^There/)
    expect(text).not.toContain('devbox')
  })

  const MIXED = () => [
    session(1, { named: true, title: 'Exited shell', details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, exit: { code: 1, at: 1 } } }),
    session(2, { agent: 'claude', named: true, title: 'Idle agent' }),
    session(3, { agent: 'claude', named: true, title: 'Working', status: 'working' }),
    session(4, { agent: 'claude', named: true, title: 'Asking', status: 'needs-you' }),
    session(5, { agent: 'claude', named: true, title: 'Finished', status: 'done' }),
  ]

  it('keeps sessions in the order they were created, so a row never moves when its state changes', () => {
    show(MIXED())
    expect(names()).toEqual(['Exited shell', 'Idle agent', 'Working', 'Asking', 'Finished'])
    expect(screen.getByRole('button', { name: /^Exited shell/ })).toHaveTextContent('exited')
  })

  it('lists a session stopped by its plan limit under Needs you, saying until when', () => {
    const resume = new Date(2026, 0, 5, 14, 5).getTime()
    const details = { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, resumeAt: resume / 1000 }
    const today = vi.spyOn(Date, 'now').mockReturnValue(resume - 60_000)
    try {
      const limited = session(6, { agent: 'claude', named: true, title: 'Out of plan', status: 'done', details })
      show([session(2, { agent: 'claude', named: true, title: 'Idle agent' }), limited], { grouping: 'status' })
      const row = within(screen.getByRole('list', { name: 'Needs you' })).getByRole('button', { name: 'Out of plan, claude, limited until 14:05' })
      expect(row).toHaveTextContent('limited until 14:05')
    } finally {
      today.mockRestore()
    }
  })

  it('moves only a session that needs you to the top when the order is priority', () => {
    show(MIXED(), { order: 'priority' })
    expect(names()).toEqual(['Asking', 'Exited shell', 'Idle agent', 'Working', 'Finished'])
  })

  it('puts the name first and the project under it, leaving the branch to the hover card', () => {
    show([session(1, { agent: 'claude', named: true, title: 'Growth', repo: 'AeVoxGrowth', branch: 'agent/grow-025', folder: '/home/me/AeVoxGrowth', cwd: '/home/me/AeVoxGrowth' }), session(2, { cwd: '/home/me/dev', folder: '/home/me/dev' })])
    const growth = screen.getByRole('button', { name: /^Growth,/ })
    expect(growth.querySelector('.mx-row-name')?.textContent).toBe('Growth')
    expect(growth.querySelector('.mx-row-project-name')?.textContent).toBe('AeVoxGrowth')
    expect(growth).not.toHaveTextContent('agent/grow-025')
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

  it('offers Install in its header, and its dot says it is not installed', async () => {
    const user = userEvent.setup()
    const onUpdateMachine = vi.fn()
    show([], { machines: [LOCAL_HEAD, needing('install')], onUpdateMachine })
    expect(screen.getByRole('img', { name: /^devbox is / })).toHaveClass('is-warn')
    await user.click(screen.getByRole('button', { name: 'Install Maitty on devbox' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'install')
  })

  it('says why its + is off while the machine is down', () => {
    show([], { machines: [LOCAL_HEAD, needing('update')] })
    const add = screen.getByRole('button', { name: 'New agent chat on devbox' })
    expect(add).toBeDisabled()
    expect(add).toHaveAttribute('title', expect.stringContaining('Cannot start a chat on devbox: '))
  })

  it('offers Install Maitty first in its menu', async () => {
    const user = userEvent.setup()
    const onUpdateMachine = vi.fn()
    show([], { machines: [LOCAL_HEAD, needing('install')], onUpdateMachine })
    expect(machineMenuItems()[0]).toBe('Install Maitty')
    await user.click(screen.getByRole('menuitem', { name: 'Install Maitty' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'install')
  })

  it('offers Update in its header for another version, and offers Update Maitty in its menu', async () => {
    const user = userEvent.setup()
    const onUpdateMachine = vi.fn()
    show([session(1)], { machines: [LOCAL_HEAD, needing('update')], onUpdateMachine })
    expect(screen.getByRole('img', { name: 'devbox is needs update' })).toHaveClass('is-warn')
    expect(screen.getByRole('button', { name: 'Update Maitty on devbox' })).toBeInTheDocument()
    expect(machineMenuItems()).toContain('Update Maitty')
    await user.click(screen.getByRole('menuitem', { name: 'Update Maitty' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'update')
  })

  it('shows how far its update has got in the header, in place of the Update button', () => {
    show([], { machines: [LOCAL_HEAD, needing('update')], updateProgress: { devbox: 55 } })
    expect(screen.getByText('55%')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Update Maitty on devbox' })).toBeNull()
    expect(screen.getByRole('progressbar', { name: 'Updating devbox' })).toHaveAttribute('aria-valuenow', '55')
  })

  it('shows no progress for a machine that is not updating', () => {
    show([], { machines: [LOCAL_HEAD, needing('update')], updateProgress: { other: 55 } })
    expect(screen.queryByRole('progressbar')).toBeNull()
    expect(screen.getByRole('button', { name: 'Update Maitty on devbox' })).toBeInTheDocument()
  })

  it('offers neither on a machine that is up to date, nor on this computer', () => {
    show([session(1)], { machines: [LOCAL_HEAD, DEVBOX] })
    expect(machineMenuItems().filter((item) => item.endsWith('Maitty'))).toEqual([])
    fireEvent.contextMenu(screen.getByRole('button', { name: /^This computer/ }))
    expect(screen.queryByRole('menuitem', { name: /Maitty$/ })).toBeNull()
    expect(screen.queryByRole('img', { name: /needs update|not installed/ })).toBeNull()
    expect(screen.queryByRole('button', { name: /Maitty on/ })).toBeNull()
  })
})

describe('subagents', () => {
  it('lists none under a session: its running subagents float over its pane instead', () => {
    show([session(1, { agent: 'claude', named: true, title: 'Growth', status: 'working', details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, subagents: [child('b', 'Write drafts tests', 'Working')] } })], { focusedId: 't1' })
    expect(screen.queryByRole('group', { name: 'Subagents' })).toBeNull()
    expect(screen.queryByText('Write drafts tests')).toBeNull()
  })
})


describe('a session row', () => {
  it('says how long an agent has been in its state, and nothing for a shell or when the daemon did not say', () => {
    const details = (since: number) => ({ asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, since })
    const fiveMinutesAgo = Math.floor(Date.now() / 1000) - 5 * 60
    show([
      session(1, { agent: 'claude', named: true, title: 'Timed', details: details(fiveMinutesAgo) }),
      session(2, { named: true, title: 'A shell', details: details(fiveMinutesAgo) }),
      session(3, { agent: 'claude', named: true, title: 'Untimed', details: details(0) }),
    ])
    expect(screen.getByRole('button', { name: /^Timed,/ })).toHaveTextContent('5m')
    expect(screen.getByRole('button', { name: /^A shell,/ })).not.toHaveTextContent('5m')
    expect(screen.getByRole('button', { name: /^Untimed,/ }).querySelector('.mx-row-when')).toBeNull()
  })

  it('offers Mark as done on a finished agent only, and asks for that session', async () => {
    const onMarkDone = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Finished', status: 'done' }), session(2, { agent: 'claude', named: true, title: 'Busy', status: 'working' })], { onMarkDone })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Busy,/ }))
    expect(screen.queryByRole('menuitem', { name: 'Mark as done' })).toBeNull()
    await userEvent.keyboard('{Escape}')
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Finished,/ }))
    await userEvent.click(screen.getByRole('menuitem', { name: 'Mark as done' }))
    expect(onMarkDone).toHaveBeenCalledWith('t1')
  })

  it('offers Put away on a finished or idle session, not on a busy one, and asks for that session', async () => {
    const onPutAway = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Finished', status: 'done' }), session(2, { named: true, title: 'Shell' }), session(3, { agent: 'claude', named: true, title: 'Busy', status: 'working' })], { onPutAway })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Busy,/ }))
    expect(screen.queryByRole('menuitem', { name: 'Put away' })).toBeNull()
    await userEvent.keyboard('{Escape}')
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Shell,/ }))
    await userEvent.click(screen.getByRole('menuitem', { name: 'Put away' }))
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Finished,/ }))
    await userEvent.click(screen.getByRole('menuitem', { name: 'Put away' }))
    expect(onPutAway.mock.calls).toEqual([['t2'], ['t1']])
  })

  it('lists put-away sessions, terminals included, only under a folded Put away section at the bottom', async () => {
    const onBringBack = vi.fn()
    show([
      session(1, { agent: 'claude', named: true, title: 'Away agent', status: 'done', archived: true }),
      session(2, { named: true, title: 'Away shell', archived: true }),
      session(3, { agent: 'claude', named: true, title: 'Here' }),
    ], { grouping: 'status', onBringBack })
    expect(names()).toEqual(['Here'])
    const head = screen.getByRole('button', { name: /^Put away/, expanded: false })
    expect(head).toHaveTextContent('2')
    await userEvent.click(head)
    const list = screen.getByRole('list', { name: 'Put away' })
    expect(within(list).getAllByRole('button', { name: /, (idle|done)$/ }).map((row) => row.getAttribute('aria-label')?.split(',')[0])).toEqual(['Away agent', 'Away shell'])
    expect(screen.queryByRole('list', { name: 'Terminals' })).toBeNull()
    await userEvent.click(within(list).getByRole('button', { name: 'Bring back Away shell' }))
    expect(onBringBack).toHaveBeenCalledWith('t2')
  })

  it('offers Bring back in the menu of a put-away session', async () => {
    const onBringBack = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Away', archived: true })], { onBringBack })
    await userEvent.click(screen.getByRole('button', { name: /^Put away/ }))
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Away,/ }))
    expect(screen.queryByRole('menuitem', { name: 'Put away' })).toBeNull()
    await userEvent.click(screen.getByRole('menuitem', { name: 'Bring back' }))
    expect(onBringBack).toHaveBeenCalledWith('t1')
  })

  describe('snoozing', () => {
    const UNTIL = new Date(2026, 0, 5, 14, 5).getTime()
    const resets = Math.ceil(Date.now() / 1000) + 4 * 3600
    const withDetails = (extra: object) => ({ asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, ...extra })

    it('offers an hour and tomorrow at 9:00, and the limit reset only for a session held by its plan limit', async () => {
      const onSnooze = vi.fn()
      show([session(1, { agent: 'claude', named: true, title: 'Finished', status: 'done' }), session(2, { agent: 'claude', named: true, title: 'Held', details: withDetails({ resumeAt: resets }) })], { onSnooze })
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Finished,/ }))
      expect(screen.getAllByRole('menuitem').map((item) => item.textContent).filter((text) => text.startsWith('Snooze'))).toEqual(['Snooze 1 hour', 'Snooze until tomorrow 9:00'])
      await userEvent.click(screen.getByRole('menuitem', { name: 'Snooze 1 hour' }))
      expect(onSnooze).toHaveBeenCalledTimes(1)
      expect(onSnooze.mock.calls[0]?.[0]).toBe('t1')
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Held,/ }))
      await userEvent.click(screen.getByRole('menuitem', { name: 'Snooze until the limit resets' }))
      expect(onSnooze).toHaveBeenLastCalledWith('t2', resets)
    })

    it('offers no snooze on a busy session or one put away', () => {
      show([session(1, { agent: 'claude', named: true, title: 'Busy', status: 'working' })])
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Busy,/ }))
      expect(screen.queryByRole('menuitem', { name: /^Snooze/ })).toBeNull()
    })

    it('lists a snoozed session under Put away with when it returns, and Unsnooze wakes it', async () => {
      const onUnsnooze = vi.fn()
      const snoozed = toSessionView({ ...session(1, { agent: 'claude', named: true, title: 'Napping', status: 'done', details: withDetails({ snoozedUntil: UNTIL / 1000 }) }).conversation }, '/home/me', UNTIL - 3_600_000)
      // Which day it names depends on when the test runs; the clock is the point.
      show([snoozed, session(2, { agent: 'claude', named: true, title: 'Here' })], { grouping: 'status', onUnsnooze })
      expect(names()).toEqual(['Here'])
      await userEvent.click(screen.getByRole('button', { name: /^Put away/ }))
      const list = screen.getByRole('list', { name: 'Put away' })
      expect(within(list).getByRole('button', { name: /^Napping,/ })).toHaveTextContent(/until (\w{3} )?14:05/)
      fireEvent.contextMenu(within(list).getByRole('button', { name: /^Napping,/ }))
      expect(screen.queryByRole('menuitem', { name: 'Bring back' })).toBeNull()
      expect(screen.queryByRole('menuitem', { name: 'Put away' })).toBeNull()
      expect(screen.queryByRole('menuitem', { name: /^Snooze/ })).toBeNull()
      await userEvent.click(screen.getByRole('menuitem', { name: 'Unsnooze' }))
      await userEvent.click(within(list).getByRole('button', { name: 'Unsnooze Napping' }))
      expect(onUnsnooze.mock.calls).toEqual([['t1'], ['t1']])
    })
  })

  it('lists a put-away session that needs you under Needs you, not under Put away', () => {
    show([session(1, { agent: 'claude', named: true, title: 'Asking', status: 'needs-you', archived: true })], { grouping: 'status' })
    expect(within(screen.getByRole('list', { name: 'Needs you' })).getByRole('button', { name: /^Asking,/ })).toBeTruthy()
    expect(screen.queryByRole('button', { name: /^Put away/ })).toBeNull()
  })

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
    expect(screen.getByRole('menuitem', { name: 'Get a file…' })).toHaveAttribute('aria-disabled', 'true')
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

  it('shows a card beside a row the pointer rests on, and puts it away when the pointer leaves', () => {
    vi.useFakeTimers()
    try {
      show([session(1, { agent: 'claude', named: true, title: 'Growth', status: 'needs-you' })])
      const row = screen.getByRole('button', { name: /^Growth,/ }).closest('.mx-row-wrap') as HTMLElement
      fireEvent.mouseEnter(row)
      expect(screen.queryByRole('tooltip')).toBeNull()
      act(() => { vi.advanceTimersByTime(450) })
      const card = screen.getByRole('tooltip')
      expect(card).toHaveTextContent('Growth')
      expect(card).toHaveTextContent('Needs you')
      expect(card).toHaveTextContent('No changes')
      fireEvent.mouseLeave(row)
      expect(screen.queryByRole('tooltip')).toBeNull()
    } finally {
      vi.useRealTimers()
    }
  })

  it('offers the session menu on right-click, and hands Stop session to the app, which asks first', async () => {
    const user = userEvent.setup()
    const onEndSession = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Growth' }), session(2)], { focusedId: 't2', onEndSession })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Growth,/ }))
    const menu = screen.getByRole('menu', { name: 'Growth' })
    expect(within(menu).getAllByRole('menuitem').map((item) => item.textContent)).toEqual(['Open', 'Open in split right', 'Open in split down', 'Show changes', 'Rename…F2', 'Copy folder path', 'Put away', 'Snooze 1 hour', 'Snooze until tomorrow 9:00', 'Stop session'])
    await user.click(within(menu).getByRole('menuitem', { name: 'Stop session' }))
    expect(onEndSession).toHaveBeenCalledWith('t1')
  })

  it("offers this computer's sessions a read-only list of their specialist agents", async () => {
    const onSpecialists = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Growth' })], { onSpecialists })
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Growth,/ }))
    await userEvent.click(screen.getByRole('menuitem', { name: 'Specialist agents…' }))
    expect(onSpecialists).toHaveBeenCalledWith('t1')
  })

  it('offers Close shell, not Stop, for a shell', () => {
    show([session(1)])
    fireEvent.contextMenu(screen.getByRole('button', { name: /^app1,/ }))
    expect(screen.getByRole('menuitem', { name: 'Close shell' })).toBeTruthy()
    expect(screen.queryByRole('menuitem', { name: 'New terminal' })).toBeNull()
  })
})

describe('a machine heading', () => {
  it('starts an agent chat on that machine when + is clicked, and opens the full menu on right-click', async () => {
    const user = userEvent.setup()
    const onNewAgentChat = vi.fn()
    show([session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There' })], { machines: [LOCAL_HEAD, DEVBOX], onNewAgentChat })
    await user.click(screen.getByRole('button', { name: 'New agent chat on devbox' }))
    expect(onNewAgentChat).toHaveBeenCalledWith(4)
    fireEvent.contextMenu(screen.getByRole('button', { name: 'New agent chat on devbox' }))
    expect(await screen.findByRole('menuitem', { name: /New shell/ })).toBeTruthy()
  })

  it('collapses its sessions when the name is clicked', async () => {
    const user = userEvent.setup()
    show([session(1, { agent: 'claude', named: true, title: 'Here' })])
    await user.click(screen.getByRole('button', { name: /^This computer/ }))
    expect(screen.getByRole('button', { name: /^This computer/, expanded: false })).toBeTruthy()
  })
})

describe('closing a session from its row', () => {
  it('has a close button on every row that asks to end that session, and a shell says Close', async () => {
    const user = userEvent.setup()
    const onEndSession = vi.fn()
    show([session(1, { agent: 'claude', named: true, title: 'Growth' }), session(2, { cwd: '/home/me/a', folder: '/home/me/a' })], { onEndSession })

    await user.click(screen.getByRole('button', { name: 'Stop Growth' }))
    expect(onEndSession).toHaveBeenLastCalledWith('t1')
    expect(screen.getByRole('button', { name: /^Close / })).toBeTruthy()
  })

  it('is off while the daemon is not reachable', () => {
    show([session(1, { agent: 'claude', named: true, title: 'Growth' })], { online: false })
    expect(screen.getByRole('button', { name: 'Stop Growth' })).toBeDisabled()
  })
})

describe('a draft chat', () => {
  it('is listed last on its machine while it waits for a first message, as a twin of a session row, and the empty note steps aside', () => {
    show([], { draft: { machine: 0 } })
    const draft = screen.getByRole('status', { name: 'New chat, waiting for the first message' })
    expect((screen.getByRole('button', { name: /^This computer/ }).closest('.mx-machine') as HTMLElement).contains(draft)).toBe(true)
    expect(screen.queryByText('No agents running. New chat starts one.')).toBeNull()
    expect(draft.querySelector('.mx-row')).not.toBeNull()
  })

  it('stays after every session of its machine, wherever they sort', () => {
    show([session(1, { agent: 'claude', named: true, title: 'First' }), session(2, { agent: 'claude', named: true, title: 'Second', status: 'needs-you' })], { draft: { machine: 0 }, order: 'priority' })
    const draft = screen.getByRole('status', { name: 'New chat, waiting for the first message' })
    const rows = Array.from((screen.getByRole('button', { name: /^This computer/ }).closest('.mx-machine') as HTMLElement).querySelectorAll('.mx-row-wrap'))
    expect(rows.at(-1)).toBe(draft)
  })

  it('is not listed once there is no draft, nor on another machine', () => {
    show([session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There' })], { machines: [LOCAL_HEAD, DEVBOX], draft: { machine: 0 } })
    expect(screen.getAllByRole('status', { name: /New chat/ })).toHaveLength(1)
    expect(within(screen.getByRole('list', { name: 'Sessions on devbox' })).queryByRole('status')).toBeNull()
  })
})

describe('with nothing running', () => {
  it('says how to start one, under no heading', () => {
    show([])
    expect(screen.getByText('No agents running. New chat starts one.')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /^This computer/ })).toBeNull()
  })
})

describe('grouped by status', () => {
  const mixed = () => [
    session(1, { agent: 'claude', named: true, title: 'Idle one' }),
    session(2, { agent: 'claude', named: true, title: 'Busy', status: 'working' }),
    session(3, { agent: 'claude', named: true, title: 'Asking', status: 'needs-you' }),
    session(4, { agent: 'claude', named: true, title: 'Finished one', status: 'done' }),
  ]

  it('lists what needs you, then working, finished and idle, each with its count, and leaves out a status nothing has', () => {
    show(mixed(), { grouping: 'status' })
    expect(screen.getAllByRole('list').map((list) => list.getAttribute('aria-label'))).toEqual(['Needs you', 'Working', 'Finished', 'Idle'])
    expect(within(screen.getByRole('list', { name: 'Needs you' })).getByRole('button', { name: /^Asking,/ })).toBeTruthy()
    expect(screen.getByRole('button', { name: /^Needs you\s*1$/ })).toBeTruthy()
    cleanup()
    show([session(1, { agent: 'claude', named: true, title: 'Idle one' })], { grouping: 'status' })
    expect(screen.getAllByRole('list').map((list) => list.getAttribute('aria-label'))).toEqual(['Idle'])
  })

  it('lists terminals in a section of their own at the bottom, whatever they are doing', () => {
    show([session(9, { named: true, title: 'Dev server', status: 'working' }), ...mixed()], { grouping: 'status' })
    expect(screen.getAllByRole('list').map((list) => list.getAttribute('aria-label'))).toEqual(['Needs you', 'Working', 'Finished', 'Idle', 'Terminals'])
    expect(within(screen.getByRole('list', { name: 'Terminals' })).getByRole('button', { name: /^Dev server,/ })).toBeTruthy()
    // Terminals are pinned over the sidebar's foot, outside the list of agents that scrolls.
    expect(screen.getByRole('list', { name: 'Terminals' }).closest('.mx-sidebar-list')).toBeNull()
    expect(screen.getByRole('list', { name: 'Terminals' }).closest('.mx-sidebar-terminals')).not.toBeNull()
    expect(within(screen.getByRole('list', { name: 'Working' })).queryByRole('button', { name: /^Dev server,/ })).toBeNull()
  })

  it('says which other machine a row runs on, since no heading does', () => {
    show([session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There' })], { machines: [LOCAL_HEAD, DEVBOX], grouping: 'status' })
    expect(within(screen.getByRole('button', { name: /^There,/ })).getByText('devbox')).toBeTruthy()
  })

  it('names another machine by its given name, with its own name as the tooltip, and this computer not at all', () => {
    writeMachineNicknames({ devbox: 'Kingdom' })
    try {
      show([
        session(1, { agent: 'claude', named: true, title: 'Here', repo: 'app1', branch: 'feat/local-branch' }),
        session(2, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There', repo: 'app2', branch: 'feat/remote-branch' }),
      ], { machines: [LOCAL_HEAD, DEVBOX], grouping: 'status' })
      const there = screen.getByRole('button', { name: /^There,/ })
      expect(within(there).getByText('Kingdom')).toHaveAttribute('title', 'devbox')
      const here = screen.getByRole('button', { name: /^Here,/ })
      expect(here.querySelector('.mx-row-machine')).toBeNull()
    } finally {
      writeMachineNicknames({})
    }
  })

  it('folds a section away from its heading', async () => {
    const user = userEvent.setup()
    show(mixed(), { grouping: 'status' })
    const head = screen.getByRole('button', { name: /^Working/ })
    expect(head).toHaveAttribute('aria-expanded', 'true')
    await user.click(head)
    expect(head).toHaveAttribute('aria-expanded', 'false')
  })

  it('lists a draft chat first, above every section', () => {
    show(mixed(), { grouping: 'status', draft: { machine: 0 } })
    const draft = screen.getByRole('status', { name: 'New chat, waiting for the first message' })
    expect(Array.from(document.querySelectorAll('.mx-row-wrap'))[0]).toBe(draft)
  })

  it('still offers Update for a machine that needs the app, under the list', () => {
    const onUpdateMachine = vi.fn()
    show(mixed(), { grouping: 'status', machines: [LOCAL_HEAD, { ...DEVBOX, updateAction: 'update' }], onUpdateMachine })
    const notes = screen.getByRole('group', { name: 'Machines that need Maitty' })
    fireEvent.click(within(notes).getByRole('button', { name: 'Update Maitty on devbox' }))
    expect(onUpdateMachine).toHaveBeenCalledWith('devbox', 'update')
  })
})

describe('grouped by project', () => {
  it('gives every project a heading, and a project on another machine one that names the machine', () => {
    show([
      session(1, { agent: 'claude', named: true, title: 'Here', cwd: '/home/me/shop' }),
      session(2, { agent: 'claude', named: true, title: 'Also here', cwd: '/home/me/shop' }),
      session(3, { machine: 4, machineName: 'devbox', agent: 'claude', named: true, title: 'There', cwd: '/home/me/shop' }),
    ], { machines: [LOCAL_HEAD, DEVBOX], grouping: 'project' })
    expect(within(screen.getByRole('list', { name: 'shop' })).getAllByRole('listitem')).toHaveLength(2)
    expect(within(screen.getByRole('list', { name: 'shop on devbox' })).getByRole('button', { name: /^There,/ })).toBeTruthy()
  })
})

describe('the top of the sidebar', () => {
  it('starts a chat on this computer from New chat, and opens the search from Search', async () => {
    const user = userEvent.setup()
    const onNewAgentChat = vi.fn()
    const onSearch = vi.fn()
    show([], { onNewAgentChat, onSearch })
    await user.click(screen.getByRole('button', { name: /^New chat/ }))
    expect(onNewAgentChat).toHaveBeenCalledWith(0)
    await user.click(screen.getByRole('button', { name: /^Search/ }))
    expect(onSearch).toHaveBeenCalledTimes(1)
  })

  it('opens Recent sessions from the rail; docked, it is in the Maitty menu instead', async () => {
    const user = userEvent.setup()
    const onRecentSessions = vi.fn()
    const { again } = show([], { onRecentSessions })
    expect(screen.queryByRole('button', { name: 'Recent sessions' })).toBeNull()
    again({ mode: 'rail' })
    await user.click(screen.getByRole('button', { name: 'Recent sessions' }))
    expect(onRecentSessions).toHaveBeenCalledTimes(1)
  })

  it('offers a shell and a remote machine from a right-click on New chat', async () => {
    const user = userEvent.setup()
    const onNewShell = vi.fn()
    show([], { onNewShell })
    fireEvent.contextMenu(screen.getByRole('button', { name: 'New chat' }))
    expect(screen.getByRole('menuitem', { name: /Add a remote machine/ })).toBeTruthy()
    await user.click(screen.getByRole('menuitem', { name: /^New shell/ }))
    expect(onNewShell).toHaveBeenCalledWith(0)
  })

  it('narrows the list to one project, and back to all of them', async () => {
    const user = userEvent.setup()
    show([session(1, { named: true, title: 'Alpha', cwd: '/home/me/shop' }), session(2, { named: true, title: 'Beta', cwd: '/home/me/docs' })], { grouping: 'status' })
    await user.click(screen.getByRole('button', { name: 'Filter and group: All projects, by status' }))
    await user.click(screen.getByRole('menuitem', { name: /^docs/ }))
    expect(names()).toEqual(['Beta'])
    await user.click(screen.getByRole('button', { name: 'Filter and group: docs, by status' }))
    await user.click(screen.getByRole('menuitem', { name: /^All projects/ }))
    expect(names()).toEqual(['Alpha', 'Beta'])
  })

  it('asks for another grouping from the same menu', async () => {
    const user = userEvent.setup()
    const onGrouping = vi.fn()
    show([], { grouping: 'status', onGrouping })
    await user.click(screen.getByRole('button', { name: /^Filter and group/ }))
    await user.click(screen.getByRole('menuitem', { name: /^Group by project/ }))
    expect(onGrouping).toHaveBeenCalledWith('project')
  })
})
