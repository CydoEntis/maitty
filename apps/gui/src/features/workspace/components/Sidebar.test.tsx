import { render, screen, within } from '@testing-library/react'
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

function show(heads: SidebarProps['machineHeads'], groups: SessionGroup[]) {
  const none = vi.fn()
  render(
    <Sidebar
      groups={groups} machineHeads={heads} focusedId={null} tracks={new Map()} mode="docked" online sessionCount={0} primary="Ctrl+"
      startFolderLabel="~" canStart portalContainer={null} onToggleRail={none} canExpand tabOf={new Map()} tabPanes={new Map()}
      openIds={new Set()} paneOf={new Map()} trayStyle="stacked" leadMode="focused" side="left" width={280} onResize={none}
      onResizeEnd={none} onNewSession={none} onNewSessionIn={none} onNewWorktree={none} onShowSession={none} onEndSession={none}
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
