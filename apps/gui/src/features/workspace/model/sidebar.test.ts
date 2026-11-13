import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { LOCAL_HEAD, type MachineHead } from './machines'
import { toSessionView, type SessionView } from './session'
import { byUrgency, canPutAway, canSnooze, inProject, machineGroups, projectChoices, projectColor, projectGroups, projectSections, rowTitle, rowWord, splitPutAway, statusSections, terminalSessions } from './sidebar'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(1)) }))

function session(pane: number, extra: Partial<Conversation> = {}): SessionView {
  const cwd = extra.cwd ?? `/home/me/app${String(pane)}`
  return toSessionView({ id: `t${String(pane)}`, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: '', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra }, '/home/me')
}
const exited = { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, exit: { code: 1, at: 1 } }

const NOW = new Date(2026, 0, 5, 9, 30).getTime()
const RESUME = new Date(2026, 0, 5, 14, 5).getTime()
/** An agent the daemon says continues at 14:05, seen at 9:30. */
function limited(pane: number): SessionView {
  return toSessionView({ ...session(pane, { agent: 'claude', details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, resumeAt: RESUME / 1000 } }).conversation }, '/home/me', NOW)
}

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

  it('says until when a session stopped by its plan limit is limited', () => {
    expect(rowWord(limited(1), NOW)).toBe('limited until 14:05')
  })

  it('gives a project the same one of five colours every time', () => {
    expect(projectColor('maitty')).toBe(projectColor('maitty'))
    expect(projectColor('maitty')).toMatch(/^var\(--proj-[1-5]\)$/)
  })
})

describe('a snoozed session', () => {
  const UNTIL = new Date(2026, 0, 5, 14, 5).getTime()
  const details = { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, snoozedUntil: UNTIL / 1000 }
  const snoozed = (pane: number, extra: Partial<Conversation> = {}) => toSessionView({ ...session(pane, { agent: 'claude', status: 'done', details, ...extra }).conversation }, '/home/me', NOW)

  it('is listed under Put away, with when it comes back, and not in a section', () => {
    const away = snoozed(1)
    expect(away.snoozedUntil).toBe(UNTIL)
    expect(splitPutAway([away, session(2)]).putAway.map((item) => item.id)).toEqual(['t1'])
    expect(rowWord(away, NOW)).toBe('until 14:05')
  })

  it('names the day when it comes back on another one', () => {
    const tomorrow = toSessionView({ ...session(1, { agent: 'claude', status: 'done', details: { ...details, snoozedUntil: new Date(2026, 0, 6, 9, 0).getTime() / 1000 } }).conversation }, '/home/me', NOW)
    expect(rowWord(tomorrow, NOW)).toBe('until Tue 09:00')
  })

  it('is listed again once its time has passed', () => {
    const over = toSessionView({ ...snoozed(1).conversation }, '/home/me', UNTIL + 1000)
    expect(over.snoozedUntil).toBeNull()
    expect(splitPutAway([over]).putAway).toEqual([])
  })

  it('is listed under Needs you, not Put away, while it asks', () => {
    expect(splitPutAway([snoozed(1, { status: 'needs-you' })]).putAway).toEqual([])
  })

  it('can be snoozed only when finished, idle or held by its plan limit, and not already away', () => {
    expect(canSnooze(session(1, { agent: 'claude', status: 'done' }))).toBe(true)
    expect(canSnooze(session(2, { agent: 'claude' }))).toBe(true)
    expect(canSnooze(limited(3))).toBe(true)
    expect(canSnooze(session(4, { agent: 'claude', status: 'working' }))).toBe(false)
    expect(canSnooze(session(5, { agent: 'claude', status: 'needs-you' }))).toBe(false)
    expect(canSnooze(session(6, { agent: 'claude', details: exited }))).toBe(false)
    expect(canSnooze(session(7, { agent: 'claude', archived: true }))).toBe(false)
    expect(canSnooze(snoozed(8))).toBe(false)
    expect(canPutAway(snoozed(8))).toBe(false)
    const terminal = session(9, { agent: 'claude', status: 'done' })
    const chat = Object.fromEntries(Object.entries(terminal.conversation).filter(([key]) => key !== 'kind')) as Conversation
    expect(canSnooze({ ...terminal, conversation: { ...chat, execution: 'Structured', chat: { conversation: 1, generation: 1 } } })).toBe(false)
  })
})

describe('machine groups', () => {
  const devbox: MachineHead = { ...LOCAL_HEAD, key: 'machine-2', machine: 2, name: 'devbox', label: 'devbox' }

  it('gives every machine its own sessions, and none at all when nothing runs', () => {
    const groups = machineGroups([session(1), session(2, { machine: 2, machineName: 'devbox' })], [LOCAL_HEAD, devbox])
    expect(groups.map((group) => [group.head.key, group.sessions.map((item) => item.id)])).toEqual([['machine-0', ['t1']], ['machine-2', ['t2']]])
    expect(machineGroups([], [LOCAL_HEAD])).toEqual([])
  })

  it('keeps every machine header with no sessions anywhere once there is another machine', () => {
    expect(machineGroups([], [LOCAL_HEAD, devbox]).map((group) => [group.head.key, group.sessions.length])).toEqual([['machine-0', 0], ['machine-2', 0]])
  })

  it('groups by project with each project’s needs-you count', () => {
    const groups = projectGroups([session(1, { cwd: '/x/a', folder: '/x/a', status: 'needs-you' }), session(2, { cwd: '/x/b', folder: '/x/b' }), session(3, { cwd: '/x/a', folder: '/x/a' })])
    expect(groups.map((group) => [group.name, group.sessions.length, group.needs])).toEqual([['a', 2, 1], ['b', 1, 0]])
  })
})

describe('status sections', () => {
  it('lists needs you, working, finished and idle in that order, and leaves out a status nothing has', () => {
    const sections = statusSections([session(1, { agent: 'codex' }), session(2, { agent: 'claude', status: 'done' }), session(3, { agent: 'claude', status: 'needs-you' })])
    expect(sections.map((section) => [section.name, section.sessions.map((item) => item.id)])).toEqual([['Needs you', ['t3']], ['Finished', ['t2']], ['Idle', ['t1']]])
  })

  it('puts an agent that stopped with what needs you', () => {
    const sections = statusSections([session(2, { agent: 'claude', details: exited })])
    expect(sections.map((section) => [section.key, section.sessions.map((item) => item.id)])).toEqual([['needs', ['t2']]])
  })

  it('puts a session stopped by its plan limit with what needs you', () => {
    const sections = statusSections([session(1, { agent: 'claude', status: 'working' }), limited(2)])
    expect(sections.map((section) => [section.key, section.sessions.map((item) => item.id)])).toEqual([['needs', ['t2']], ['working', ['t1']]])
  })
})

describe('terminals', () => {
  it('are left out of the status and project sections and listed on their own, as created', () => {
    const all = [session(3, { process: 'pnpm' }), session(1), session(2, { agent: 'claude' }), session(4, { details: exited })]
    expect(terminalSessions(all).map((item) => item.id)).toEqual(['t1', 't3', 't4'])
    expect(statusSections(all).flatMap((section) => section.sessions.map((item) => item.id))).toEqual(['t2'])
    expect(projectSections(all, [LOCAL_HEAD]).flatMap((section) => section.sessions.map((item) => item.id))).toEqual(['t2'])
  })

  it('stay with the agents while they need you', () => {
    const all = [session(1, { status: 'needs-you' }), session(2)]
    expect(terminalSessions(all).map((item) => item.id)).toEqual(['t2'])
    expect(statusSections(all).map((section) => [section.key, section.sessions.map((item) => item.id)])).toEqual([['needs', ['t1']]])
  })
})

describe('projects', () => {
  const devbox: MachineHead = { ...LOCAL_HEAD, key: 'machine-2', machine: 2, name: 'devbox', label: 'devbox' }
  const sessions = () => [session(1, { agent: 'claude', cwd: '/x/b', folder: '/x/b' }), session(2, { agent: 'claude', cwd: '/x/a', folder: '/x/a' }), session(3, { agent: 'claude', machine: 2, machineName: 'devbox', cwd: '/x/a', folder: '/x/a' })]

  it('lists every project by name with how many sessions it has', () => {
    expect(projectChoices(sessions()).map((choice) => [choice.name, choice.count])).toEqual([['a', 2], ['b', 1]])
  })

  it('narrows to one project, or to none when no project is picked', () => {
    expect(inProject(sessions(), 'a').map((item) => item.id)).toEqual(['t2', 't3'])
    expect(inProject(sessions(), null)).toHaveLength(3)
  })

  it('gives a project on two machines a heading for each, this computer first', () => {
    expect(projectSections(sessions(), [LOCAL_HEAD, devbox]).map((section) => [section.name, section.head.name, section.sessions.length])).toEqual([['a', null, 1], ['a', 'devbox', 1], ['b', null, 1]])
  })
})

describe('putting a session away', () => {
  const away = (pane: number, extra: Partial<Conversation> = {}) => session(pane, { archived: true, ...extra })

  it('lists put-away agents and terminals only under Put away, as created', () => {
    const { listed, putAway } = splitPutAway([away(3, { agent: 'claude' }), session(1, { agent: 'claude' }), away(2), session(4)])
    expect(putAway.map((item) => item.id)).toEqual(['t2', 't3'])
    expect(listed.map((item) => item.id)).toEqual(['t1', 't4'])
    expect(statusSections(listed).flatMap((section) => section.sessions.map((item) => item.id))).toEqual(['t1'])
    expect(terminalSessions(listed).map((item) => item.id)).toEqual(['t4'])
  })

  it('lists a put-away session that needs you under Needs you, still put away on the daemon', () => {
    const asking = away(1, { agent: 'claude', status: 'needs-you' })
    const { listed, putAway } = splitPutAway([asking])
    expect(putAway).toEqual([])
    expect(statusSections(listed).map((section) => [section.key, section.sessions.map((item) => item.id)])).toEqual([['needs', ['t1']]])
    expect(listed[0]?.archived).toBe(true)
  })

  it('lists a put-away agent that stopped under Needs you, and keeps an exited shell put away', () => {
    const { listed, putAway } = splitPutAway([away(1, { agent: 'claude', details: exited }), away(2, { details: exited })])
    expect(listed.map((item) => item.id)).toEqual(['t1'])
    expect(putAway.map((item) => item.id)).toEqual(['t2'])
  })

  it('offers Put away for a finished or idle session that is not put away already', () => {
    expect([session(1, { status: 'done' }), session(2), session(3, { status: 'working' }), session(4, { status: 'needs-you' }), away(5)].map(canPutAway)).toEqual([true, true, false, false, false])
  })

  it('does not offer Put away for a chat session, whose daemon archive closes it', () => {
    const chat: Conversation = { id: '1', title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', branch: '', provider: 'Claude', status: 'done', updated: null, model: '', permission: '', context: null, nativePane: null, hasDiff: false, execution: 'Structured', chat: { conversation: 1, generation: 1 } }
    expect(canPutAway(toSessionView(chat, '/home/me'))).toBe(false)
  })
})
