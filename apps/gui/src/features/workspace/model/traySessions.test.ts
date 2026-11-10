import { describe, expect, it } from 'vitest'
import type { SessionView } from './session'
import { trayKey, traySessionsOf } from './traySessions'

const session = (id: string, status: SessionView['status'], title: string) => ({ id, status, title }) as SessionView

describe('what the tray menu lists', () => {
  it('is the sessions that need you and the ones that finished, by the name the sidebar shows', () => {
    const tray = traySessionsOf([session('t1', 'working', 'api'), session('t2', 'needs', 'web'), session('t3', 'done', 'docs'), session('t4', 'idle', 'x')])
    expect(tray.needs.map((item) => item.id)).toEqual(['t2'])
    expect(tray.finished.map((item) => item.id)).toEqual(['t3'])
    expect(tray.needs[0]?.label).not.toBe('')
  })

  it('names the agent before the session, since a name alone is often only a folder', () => {
    const claude = { ...session('t2', 'needs', 'web'), process: 'claude' }
    expect(traySessionsOf([claude]).needs[0]?.label.startsWith('claude: ')).toBe(true)
  })

  it('has a key that changes only when the menu would', () => {
    const before = traySessionsOf([session('t2', 'needs', 'web')])
    expect(trayKey(traySessionsOf([session('t2', 'needs', 'web'), session('t9', 'working', 'other')]))).toBe(trayKey(before))
    expect(trayKey(traySessionsOf([session('t2', 'done', 'web')]))).not.toBe(trayKey(before))
  })
})
