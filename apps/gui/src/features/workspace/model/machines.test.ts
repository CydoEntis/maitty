import { afterEach, describe, expect, it } from 'vitest'
import { conversationsFromSnapshot } from '@/api/daemon'
import { forgetMachines, parseSessionId, registerMachine, type MachineKind, type MachineState } from '@/api/machines'
import type { Conversation } from '@/data/preview'
import type { Layout } from './layout'
import { idsOfMachinesNotHeard, remoteTerminals, type RemoteMachine } from './machines'
import { groupSessions, toSessionView } from './session'

afterEach(() => { forgetMachines() })

function term(id: number, cwd: string) {
  return {
    id, cols: 80, rows: 24, title: 't', process: 'bash', agent: null, status: 'Idle', cwd, summary: '', name: 'n',
    label: '', pinned: false, archived: false, model: '', said: '', branch: null, linked: false, subagents: [],
    conversation_id: null, popup: false, remote: null, root: null, top: null, since: 0, asleep: false, win32_input: false,
  }
}

function snapshot(terms: ReturnType<typeof term>[]) {
  return { terms: Object.fromEntries(terms.map((item) => [String(item.id), item])), workspaces: [], conversations: [] }
}

function state(machine: number, name: string, kind: MachineKind = 'connected'): MachineState {
  return { machine, name, state: kind, why: null, attempt: null }
}

function machineOf(id: number, name: string, terms: ReturnType<typeof term>[], kind: MachineKind = 'connected', heard = true): RemoteMachine {
  registerMachine(name, id)
  return { state: state(id, name, kind), conversations: conversationsFromSnapshot(snapshot(terms), { id, name }), heard }
}

describe('merging the snapshots of several machines', () => {
  it('keeps the pane that two machines both number 5 apart, by the machine in its id', () => {
    const here = conversationsFromSnapshot(snapshot([term(5, '/home/me/app')]))
    const build = machineOf(2, 'build', [term(5, '/srv/app')])
    const box = machineOf(1, 'box', [term(5, '/home/x/app'), term(6, '/home/x/other')])
    const all = [...here, ...remoteTerminals(new Map([[2, build], [1, box]]))]

    expect(all.map((item) => item.id)).toEqual(['t5', 'box/t5', 'box/t6', 'build/t5'])
    expect(new Set(all.map((item) => item.id)).size).toBe(all.length)
    expect(all.map((item) => [item.machine ?? 0, item.nativePane])).toEqual([[0, 5], [1, 5], [1, 6], [2, 5]])
    expect(all[0]?.machineName).toBeUndefined()
    expect(all[1]?.machineName).toBe('box')
  })

  it('shows what a machine last listed, as unknown, while its line is down', () => {
    const busy = { ...term(3, '/srv'), status: 'Working' }
    const down = machineOf(1, 'box', [busy], 'unreachable')
    const [session] = remoteTerminals(new Map([[1, down]]))
    expect(session?.status).toBe('unknown')
    expect(session?.lastKnownStatus).toBe('working')
    const up = machineOf(1, 'box', [busy], 'connected')
    expect(remoteTerminals(new Map([[1, up]]))[0]?.status).toBe('working')
  })

  it('says a session of this computer is exactly what it was, with one machine', () => {
    const [session] = conversationsFromSnapshot(snapshot([term(12, '/home/me/app')]))
    expect(session?.id).toBe('t12')
    expect(session).not.toHaveProperty('machine')
    expect(session).not.toHaveProperty('machineName')
    expect(remoteTerminals(new Map())).toEqual([])
  })

  it('does not let one machine\'s unreadable pane take another\'s last good state', () => {
    const broken = { ...term(9, '/srv'), status: 'Nonsense' }
    const first = machineOf(1, 'box', [term(9, '/srv/box-9')])
    expect(first.conversations[0]?.cwd).toBe('/srv/box-9')
    // The same pane number on another machine reads fine; box's now fails and keeps box's own.
    machineOf(2, 'build', [term(9, '/srv/build-9')])
    const again = conversationsFromSnapshot(snapshot([broken]), { id: 1, name: 'box' })
    expect(again[0]?.cwd).toBe('/srv/box-9')
  })

  it('names the sessions of another machine in its id so a saved layout survives a restart', () => {
    registerMachine('build', 2)
    expect(parseSessionId('t5')).toEqual({ machine: 0, machineName: null, term: 5 })
    expect(parseSessionId('build/t5')).toEqual({ machine: 2, machineName: 'build', term: 5 })
    expect(parseSessionId('BUILD/t5')?.machine).toBe(2)
    // A different run gives the machine another number; the id still finds it.
    registerMachine('build', 7)
    expect(parseSessionId('build/t5')?.machine).toBe(7)
  })
})

describe('the sidebar groups of several machines', () => {
  it('never joins a folder of another machine to a folder here with the same path', () => {
    const here = conversationsFromSnapshot(snapshot([term(1, '/work/app')]))
    const there = remoteTerminals(new Map([[1, machineOf(1, 'box', [term(1, '/work/app')])]]))
    const views = [...here, ...there].map((conversation) => toSessionView(conversation, '/home/me'))
    const groups = groupSessions(views, 'folder', null, '/home/me')
    expect(groups).toHaveLength(2)
    expect(groups.map((group) => group.name).sort()).toEqual(['app', 'box · app'])
    expect(groups.find((group) => group.name === 'box · app')?.path).toBe('/work/app')
  })

  it('writes the home folder as ~ only for this computer', () => {
    const here = toSessionView(conversationsFromSnapshot(snapshot([term(1, '/home/me/app')]))[0] as Conversation, '/home/me')
    const there = toSessionView(machineOf(1, 'box', [term(1, '/home/me/app')]).conversations[0] as Conversation, '/home/me')
    expect(here.cwdLabel).toBe('~/app')
    expect(there.cwdLabel).toBe('/home/me/app')
    expect(there.machine).toBe(1)
    expect(there.machineName).toBe('box')
  })
})

describe('a saved layout at launch', () => {
  const layout: Layout = {
    tabs: [{
      id: 'tab1',
      panes: [
        { id: 'p1', sessionId: 't3', col: 0 },
        { id: 'p2', sessionId: 'box/t4', col: 1 },
        { id: 'p3', sessionId: 'build/t9', col: 2 },
      ],
      focus: 'p1',
      max: null,
    }],
    activeTabId: 'tab1',
  }

  it('keeps the panes of machines that have not answered yet', () => {
    expect(idsOfMachinesNotHeard(layout, new Map())).toEqual(['box/t4', 'build/t9'])
  })

  it('lets the panes of a machine that answered be judged by what it listed', () => {
    const box = machineOf(1, 'box', [term(4, '/srv')], 'connected', true)
    const build = machineOf(2, 'build', [], 'connecting', false)
    expect(idsOfMachinesNotHeard(layout, new Map([[1, box], [2, build]]))).toEqual(['build/t9'])
  })
})
