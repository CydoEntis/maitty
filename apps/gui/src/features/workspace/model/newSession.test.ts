import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { toSessionView, type SessionView } from './session'
import { machineProjects, nameFromMessage } from './newSession'

vi.mock('@/api/daemon', () => ({ terminalOf: (id: string) => Number(id.slice(1)) }))

function session(pane: number, cwd: string, extra: Partial<Conversation> = {}): SessionView {
  return toSessionView({ id: `t${String(pane)}`, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: '', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra }, '/home/me')
}

describe('the New session dialog’s projects', () => {
  it('offers the focused session’s project first, then the newest, each once', () => {
    const one = session(1, '/code/api')
    const two = session(2, '/code/web')
    const three = session(3, '/code/api')
    expect(machineProjects([one, two, three], 0, two, '/home/me').map((project) => project.folder)).toEqual(['/code/web', '/code/api'])
  })

  it('offers only the machine’s own projects, and the start folder when it has none', () => {
    const remote = session(1, '/srv/app', { machine: 3, machineName: 'box' })
    expect(machineProjects([remote], 0, undefined, '/home/me/code').map((project) => project.name)).toEqual(['code'])
    expect(machineProjects([remote], 3, undefined, '~').map((project) => project.folder)).toEqual(['/srv/app'])
  })
})

describe('a session named from its first message', () => {
  it('takes the first three words and drops a closing mark', () => {
    expect(nameFromMessage('  Add draft routes, with tests.')).toBe('Add draft routes')
    expect(nameFromMessage('Fix it.')).toBe('Fix it')
  })
})
