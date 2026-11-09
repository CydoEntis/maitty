import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { LOCAL_HEAD, type MachineHead } from './machines'
import { toSessionView, type SessionView } from './session'
import { byUrgency, machineGroups, projectColor, projectGroups, rowTitle, rowWord } from './sidebar'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(1)) }))

function session(pane: number, extra: Partial<Conversation> = {}): SessionView {
  const cwd = extra.cwd ?? `/home/me/app${String(pane)}`
  return toSessionView({ id: `t${String(pane)}`, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: '', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra }, '/home/me')
}
const exited = { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, exit: { code: 1, at: 1 } }

describe('the sidebar’s order', () => {
  it('puts an agent that stopped above work in progress, and a failed shell command last', () => {
    const order = byUrgency([session(1, { details: exited }), session(2, { agent: 'claude', details: exited }), session(3, { agent: 'claude', status: 'working' }), session(4)])
    expect(order.map((item) => item.id)).toEqual(['t2', 't3', 't4', 't1'])
  })

  it('keeps creation order within a rank', () => {
    expect(byUrgency([session(3), session(1), session(2)]).map((item) => item.id)).toEqual(['t1', 't2', 't3'])
  })
})

describe('a row', () => {
  it('names a session by its name, a shell by its folder, an agent by its program', () => {
    expect(rowTitle(session(1, { named: true, title: 'Growth' }))).toBe('Growth')
    expect(rowTitle(session(2, { cwd: '/home/me/dev', folder: '/home/me/dev' }))).toBe('dev')
    expect(rowTitle(session(3, { agent: 'codex' }))).toBe('codex')
    expect(rowTitle(session(4, { cwd: '/home/me/dev', folder: '/home/me/dev', process: 'node-mainthread' }))).toBe('dev')
  })

  it('says stopped for an agent and exited for a shell, and nothing otherwise', () => {
    expect(rowWord(session(1, { agent: 'claude', details: exited }))).toBe('stopped')
    expect(rowWord(session(2, { details: exited }))).toBe('exited')
    expect(rowWord(session(3, { status: 'working' }))).toBe('')
  })

  it('gives a project the same one of five colours every time', () => {
    expect(projectColor('maitty')).toBe(projectColor('maitty'))
    expect(projectColor('maitty')).toMatch(/^var\(--proj-[1-5]\)$/)
  })
})

describe('machine groups', () => {
  const devbox: MachineHead = { ...LOCAL_HEAD, key: 'machine-2', machine: 2, name: 'devbox', label: 'devbox' }

  it('gives every machine its own sessions, and none at all when nothing runs', () => {
    const groups = machineGroups([session(1), session(2, { machine: 2, machineName: 'devbox' })], [LOCAL_HEAD, devbox], '')
    expect(groups.map((group) => [group.head.key, group.sessions.map((item) => item.id)])).toEqual([['machine-0', ['t1']], ['machine-2', ['t2']]])
    expect(machineGroups([], [LOCAL_HEAD], '')).toEqual([])
  })

  it('keeps every machine header with no sessions anywhere once there is another machine', () => {
    expect(machineGroups([], [LOCAL_HEAD, devbox], '').map((group) => [group.head.key, group.sessions.length])).toEqual([['machine-0', 0], ['machine-2', 0]])
  })

  it('leaves out a machine with nothing that matches the filter', () => {
    const groups = machineGroups([session(1, { cwd: '/home/me/alpha', folder: '/home/me/alpha' }), session(2, { machine: 2, machineName: 'devbox' })], [LOCAL_HEAD, devbox], 'alpha')
    expect(groups.map((group) => group.head.key)).toEqual(['machine-0'])
  })

  it('groups by project with each project’s needs-you count', () => {
    const groups = projectGroups([session(1, { cwd: '/x/a', folder: '/x/a', status: 'needs-you' }), session(2, { cwd: '/x/b', folder: '/x/b' }), session(3, { cwd: '/x/a', folder: '/x/a' })])
    expect(groups.map((group) => [group.name, group.sessions.length, group.needs])).toEqual([['a', 2, 1], ['b', 1, 0]])
  })
})
