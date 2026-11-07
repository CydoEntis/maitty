import { describe, expect, it, vi } from 'vitest'
import type { Fleet } from '@/api/fleets'
import type { Conversation } from '@/data/preview'
import { aboard, daemonColorOf, emptyShipsNote, fleetColorOf, FLEET_COLORS, fleetKeyOf, iconStack, initialsOf, isFreeFleetName, NO_FLEET_ID, priorityGroups, shipsOf, sidebarMachines, visibleFleets, withHomeFleets, LOCAL_HEAD } from './fleets'
import { toSessionView, type SessionView } from './session'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(1)) }))

function fleet(id: number, name: string, machine = 0): Fleet {
  return { key: `${String(machine)}:${String(id)}`, machine, id, name, color: id, worktree: false, sessionIds: [] }
}

function ship(pane: number, inFleet: Fleet | null, extra: Partial<Conversation> = {}): SessionView {
  return toSessionView(
    { id: `t${String(pane)}`, generation: 0, kind: 'terminal', title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: 'working tree', provider: 'Codex', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra },
    '/home/me',
    inFleet ? { fleet: inFleet.key, worktree: false } : undefined,
  )
}

const A = fleet(1, 'Maitty')
const B = fleet(2, 'AeVox launch')

describe('fleet colours and initials', () => {
  it('never offers amber or red, and wraps the daemon palette onto six colours', () => {
    expect(FLEET_COLORS).toEqual(['teal', 'violet', 'blue', 'green', 'pink', 'orange'])
    // The indexes the host accepts for a recolour read back as the colour that was chosen.
    for (const name of FLEET_COLORS) expect(fleetColorOf(daemonColorOf(name))).toBe(name)
    expect(daemonColorOf('teal')).toBe(3)
    // An index saved before fleets wraps onto the six.
    expect(fleetColorOf(6)).toBe('teal')
    expect(fleetColorOf(4)).toBe('pink')
  })

  it('takes two letters: one from each of two words, else the first two', () => {
    expect(initialsOf('AeVox launch')).toBe('AL')
    expect(initialsOf('Maitty')).toBe('MA')
    expect(initialsOf('docs-site')).toBe('DS')
    expect(initialsOf('  ')).toBe('?')
  })
})

describe('which fleet a ship is in', () => {
  it('is the one the daemon holds it in, else its machine’s first fleet', () => {
    expect(fleetKeyOf(0, B.key, [A, B])).toBe(B.key)
    expect(fleetKeyOf(0, null, [A, B])).toBe(A.key)
    expect(fleetKeyOf(0, '0:99', [A, B])).toBe(A.key)
    expect(fleetKeyOf(4, null, [A, B])).toBeNull()
  })

  it('adds a Main fleet for a machine whose sessions the daemon lists in no fleet', () => {
    const lonely = ship(1, null)
    const fleets = withHomeFleets([], [lonely])
    expect(fleets).toHaveLength(1)
    expect(fleets[0]).toMatchObject({ name: 'Main', machine: 0, id: NO_FLEET_ID })
    expect(withHomeFleets([A], [lonely])).toEqual([A])
  })

  it('lists the ships of a fleet in the order given', () => {
    const ships = [ship(1, A), ship(2, B), ship(3, A)]
    expect(aboard(ships, [A, B], A.key).map((s) => s.id)).toEqual(['t1', 't3'])
  })
})

describe('the machines section', () => {
  it('always lists this computer, even alone', () => {
    expect(sidebarMachines([])).toEqual([LOCAL_HEAD])
  })
})

describe('the stacked icons', () => {
  it('shows Claude first, then Codex, then shells, three at most, then the rest as +N', () => {
    const ships = [ship(1, A), ship(2, A, { agent: 'codex' }), ship(3, A, { agent: 'claude' }), ship(4, A, { agent: 'claude' }), ship(5, A)]
    expect(iconStack(ships)).toEqual({ processes: ['claude', 'claude', 'codex'], more: 2 })
    expect(iconStack([ship(1, A)])).toEqual({ processes: ['shell'], more: 0 })
  })
})

describe('agents and shells', () => {
  it('splits a fleet’s ships into the two subgroups', () => {
    const { agents, shells } = shipsOf([ship(1, A), ship(2, A, { agent: 'claude' })])
    expect(agents.map((s) => s.id)).toEqual(['t2'])
    expect(shells.map((s) => s.id)).toEqual(['t1'])
  })
})

describe('the Priority view', () => {
  it('groups every agent across fleets as Needs you, Exited, Working, Done, Idle, and leaves shells out', () => {
    const ships = [
      ship(1, A, { agent: 'claude', status: 'working' }),
      ship(2, B, { agent: 'codex', status: 'needs-you' }),
      ship(3, B, { agent: 'claude', status: 'done' }),
      ship(4, A),
      ship(5, A, { agent: 'claude' }),
      ship(6, A, { agent: 'claude', details: { asking: '', ask: null, interrupted: null, exit: { code: 1, at: 0 }, timeline: [], usage: { context: null, cost: null, tokens: null } } }),
    ]
    const groups = priorityGroups(ships)
    expect(groups.map((group) => [group.title, group.ships.map((s) => s.id)])).toEqual([
      ['Needs you', ['t2']],
      ['Exited', ['t6']],
      ['Working', ['t1']],
      ['Done', ['t3']],
      ['Idle', ['t5']],
    ])
  })

  it('puts a session whose daemon is gone with the idle ones', () => {
    const gone = ship(1, A, { agent: 'claude', status: 'working' })
    expect(priorityGroups([{ ...gone, status: 'unknown' }]).map((group) => group.id)).toEqual(['idle'])
  })
})

describe('the filter', () => {
  it('keeps a fleet whose name matches or that holds a ship that does', () => {
    const ships = [ship(1, A, { agent: 'claude', branch: 'agent/grow-028', repo: 'maitty', folder: '/home/me/maitty' }), ship(2, B)]
    expect(visibleFleets([A, B], ships, 'aevox').map((f) => f.name)).toEqual(['AeVox launch'])
    expect(visibleFleets([A, B], ships, 'grow-028').map((f) => f.name)).toEqual(['Maitty'])
    expect(visibleFleets([A, B], ships, '')).toHaveLength(2)
  })

  it('words an empty fleet and a filter that found nothing, naming a fleet that matches', () => {
    expect(emptyShipsNote({ fleetName: 'Maitty', query: '', hasShips: false, otherMatch: null })).toBe('Nothing aboard yet. Use + to start a session.')
    expect(emptyShipsNote({ fleetName: 'Maitty', query: 'api', hasShips: true, otherMatch: 'API' })).toBe('Nothing in Maitty matches “api”. Try API above.')
    expect(emptyShipsNote({ fleetName: 'Maitty', query: 'api', hasShips: true, otherMatch: null })).toBe('Nothing in Maitty matches “api”.')
  })

  it('refuses a fleet name already used on that machine, case aside, and an empty one', () => {
    expect(isFreeFleetName([A, B], 0, 'maitty')).toBe(false)
    expect(isFreeFleetName([A, B], 0, '  ')).toBe(false)
    expect(isFreeFleetName([A, B], 0, 'Docs')).toBe(true)
    expect(isFreeFleetName([A, B], 4, 'Maitty')).toBe(true)
  })
})
