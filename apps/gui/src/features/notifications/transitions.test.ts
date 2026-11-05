import { describe, expect, it } from 'vitest'
import type { SessionStatus, SessionView } from '@/features/workspace/model/session'
import { noticesFor, type NotifySwitches } from './transitions'

const ALL: NotifySwitches = { needs: true, finished: true, error: true }

function session(id: string, status: SessionStatus): SessionView {
  return { id, name: null, title: 'maitty', process: 'codex', cwdLabel: '~/code/maitty', status } as unknown as SessionView
}
const seen = (entries: Array<[string, SessionStatus]>) => new Map(entries)

describe('noticesFor', () => {
  it('says nothing on the first look, whatever the sessions already are', () => {
    expect(noticesFor(null, [session('t1', 'needs')], ALL, null)).toEqual([])
  })

  it('notifies when a session starts needing you, naming it and where it is but never what it asked', () => {
    const notices = noticesFor(seen([['t1', 'working']]), [session('t1', 'needs')], ALL, null)
    expect(notices).toEqual([{ sessionId: 't1', title: 'maitty needs you', body: 'codex in ~/code/maitty' }])
  })

  it('notifies for finished and for an error, each by its own switch', () => {
    const before = seen([['a', 'working'], ['b', 'working']])
    const now = [session('a', 'done'), session('b', 'error')]
    expect(noticesFor(before, now, ALL, null).map((notice) => notice.title)).toEqual(['maitty finished', 'maitty exited with an error'])
    expect(noticesFor(before, now, { ...ALL, finished: false }, null).map((notice) => notice.sessionId)).toEqual(['b'])
    expect(noticesFor(before, now, { needs: true, finished: false, error: false }, null)).toEqual([])
  })

  it('does not repeat itself while the state stays the same', () => {
    expect(noticesFor(seen([['t1', 'needs']]), [session('t1', 'needs')], ALL, null)).toEqual([])
  })

  it('leaves out the session you are looking at, and a new session, and one back from a disconnect', () => {
    const before = seen([['t1', 'working'], ['t3', 'unknown']])
    const now = [session('t1', 'needs'), session('t2', 'needs'), session('t3', 'needs')]
    expect(noticesFor(before, now, ALL, 't1')).toEqual([])
  })

  it('does not notify for states that need nothing from you', () => {
    expect(noticesFor(seen([['t1', 'working']]), [session('t1', 'idle')], ALL, null)).toEqual([])
  })

  it('starts a session of another machine with the machine in brackets, as its own notices do', () => {
    const remote = { ...session('build/t7', 'needs'), machineName: 'build', name: 'deploy' } as SessionView
    const notices = noticesFor(seen([['build/t7', 'working']]), [remote], ALL, null)
    expect(notices.map((notice) => notice.title)).toEqual(['[build] deploy needs you'])
  })
})
