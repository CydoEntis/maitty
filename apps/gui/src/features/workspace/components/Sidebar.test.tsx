import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { SessionGroup } from '../model/session'
import { machineHeads, type RemoteMachine } from '../model/machines'
import { Sidebar, type SidebarProps } from './Sidebar'

function machine(id: number, name: string, kind: RemoteMachine['state']['state'], why: string | null = null, attempt: number | null = null): [number, RemoteMachine] {
  return [id, { state: { machine: id, name, state: kind, why, attempt }, conversations: [], heard: true }]
}

function group(key: string, name: string, machineName: string | null): SessionGroup {
  return { key, name, kind: 'folder', path: `/${name}`, machineName, sessions: [], containsFocused: false, needsCount: 0, workingCount: 0 }
}

function show(heads: SidebarProps['machineHeads'], groups: SessionGroup[], more: Partial<SidebarProps> = {}) {
  const none = vi.fn()
  render(
    <Sidebar
      groups={groups} machineHeads={heads} focusedId={null} tracks={new Map()} mode="docked" online sessionCount={0}
      startFolderLabel="~" canStart portalContainer={null} onToggleRail={none} canExpand tabOf={new Map()} tabPanes={new Map()}
      openIds={new Set()} paneOf={new Map()} trayStyle="stacked" side="left" width={280} onResize={none}
      onResizeEnd={none} onNewSession={none} onNewSessionWith={none} defaultAgent="claude" onNewSessionIn={none} onNewTerminal={none} onNewTerminalIn={none} onLocalOnly={none} onNewSessionOn={none} onNewWorktree={none}
      {...more} onShowSession={none} onEndSession={none}
      onEndProject={none} onSetName={none} onRenameSession={none} onPinSession={none} onOpenSettings={none} updateSlot={null} railUpdateSlot={null}
    />,
  )
}

describe('the sidebar with machines', () => {
  it('has no machine header while this computer is the only machine', () => {
    show([], [group('a', 'app', null)])
    expect(screen.queryByText('this computer')).toBeNull()
  })

  it('puts each machine\'s folders under its own header, this computer first', () => {
    show(machineHeads(new Map([machine(1, 'build', 'connected')])), [group('a', 'app', null), group('b', 'svc', 'build')])
    const sections = screen.getAllByRole('region')
    expect(sections.map((section) => section.getAttribute('aria-label'))).toEqual(['this computer', 'build'])
    const [here, there] = sections
    if (here === undefined || there === undefined) throw new Error('two machine sections expected')
    expect(within(here).getByText('app')).toBeTruthy()
    expect(within(here).queryByText('svc')).toBeNull()
    expect(within(there).getByText('svc')).toBeTruthy()
    expect(within(there).getByText(/connected/)).toBeTruthy()
  })

  it('opens a terminal on the machine whose header + is pressed, and not on one that is not connected', () => {
    const onNewSessionOn = vi.fn()
    show(machineHeads(new Map([machine(1, 'build', 'connected'), machine(2, 'lost', 'unreachable')])), [], { onNewSessionOn })
    expect(screen.queryByRole('button', { name: 'New terminal on this computer' })).toBeNull()
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'New terminal on lost' }).disabled).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'New terminal on build' }))
    expect(onNewSessionOn).toHaveBeenCalledWith(1, 'build')
  })

  it('tells a connected machine with nothing running where to start, and a machine that is down why it cannot', () => {
    show(machineHeads(new Map([machine(1, 'build', 'connected'), machine(2, 'lost', 'unreachable')])), [])
    expect(screen.getByText('No sessions. Use + to open a terminal here.')).toBeTruthy()
    expect(screen.getAllByText('No sessions. Use + to open a terminal here.')).toHaveLength(1)
    expect(screen.getByRole('button', { name: 'New terminal on lost' }).getAttribute('title')).toBe('lost is unreachable · can’t open a terminal')
  })

  it('tells a machine that is down by its word, with the fix under it', () => {
    show(machineHeads(new Map([machine(1, 'build', 'needs-login', 'run ssh build once')])), [])
    expect(screen.getByText(/needs login/)).toBeTruthy()
    expect(screen.getByText('run ssh build once')).toBeTruthy()
  })

  it('dates the outage of a machine that is down, and says nothing of one that is up or still dialling', () => {
    const down = machine(1, 'build', 'unreachable', 'fix it')
    down[1].state.since = Date.now() - 120_000
    show(machineHeads(new Map([down, machine(2, 'up', 'connected'), machine(3, 'new', 'connecting')])), [])
    expect(screen.getAllByText(/^disconnected/)).toHaveLength(1)
    expect(screen.getByText('disconnected since 2m ago')).toBeTruthy()
  })

  it('does not print a fix for a machine that is only reconnecting', () => {
    show(machineHeads(new Map([machine(1, 'build', 'reconnecting', 'leftover', 2)])), [])
    expect(screen.getByText(/reconnecting #2/)).toBeTruthy()
    expect(screen.queryByText('leftover')).toBeNull()
  })
})

describe('the sidebar after New agent on a branch was removed', () => {
  it('has no such button at the top, in the rail, or in a project menu', async () => {
    show([], [{ ...group('/code/app', 'app', null), kind: 'repo' }])
    expect(screen.queryByRole('button', { name: 'New agent on a branch' })).toBeNull()
    fireEvent.contextMenu(screen.getByRole('button', { name: /app/ }))
    await screen.findByRole('menu')
    expect(screen.queryByRole('menuitem', { name: /New agent on a branch/ })).toBeNull()
  })
})
