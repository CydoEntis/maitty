import { describe, expect, it } from 'vitest'
import type { SessionStatus, SessionView } from './session'
import { byPriority, chooseLead, planTray } from './stack'

function session(id: string, status: SessionStatus): SessionView {
  return { id, status } as SessionView
}

const none = () => 0
const ids = (sessions: readonly SessionView[]) => sessions.map((item) => item.id)

describe('which session leads a project', () => {
  const sessions = [session('idle', 'idle'), session('working', 'working'), session('needs', 'needs')]

  it('is the session you are in, with "Focused"', () => {
    expect(chooseLead(sessions, 'focused', 'idle', undefined, none)?.id).toBe('idle')
  })

  it('is the one last picked in the project when you are in another project', () => {
    expect(chooseLead(sessions, 'focused', 'elsewhere', 'working', none)?.id).toBe('working')
  })

  it('is the most urgent when nothing was picked', () => {
    expect(chooseLead(sessions, 'focused', null, undefined, none)?.id).toBe('needs')
  })

  it('is always the most urgent with "Needs attention", even when you are in another session', () => {
    expect(chooseLead(sessions, 'attention', 'idle', 'idle', none)?.id).toBe('needs')
  })

  it('is nothing for an empty project', () => {
    expect(chooseLead([], 'focused', null, undefined, none)).toBeUndefined()
  })
})

describe('the order of sessions', () => {
  it('runs needs you, error, done, working, idle', () => {
    const mixed = [session('i', 'idle'), session('w', 'working'), session('d', 'done'), session('e', 'error'), session('n', 'needs')]
    expect(ids([...mixed].sort(byPriority(none)))).toEqual(['n', 'e', 'd', 'w', 'i'])
  })

  it('puts the most recently active first among equals', () => {
    const times: Record<string, number> = { a: 10, b: 30, c: 20 }
    const same = [session('a', 'working'), session('b', 'working'), session('c', 'working')]
    expect(ids([...same].sort(byPriority((id) => times[id] ?? 0)))).toEqual(['b', 'c', 'a'])
  })
})

describe('the tray under the main row', () => {
  const many = (count: number, status: SessionStatus = 'idle') => Array.from({ length: count }, (_, index) => session(`s${String(index)}`, status))

  it.each([
    ['names', 2, 2, 0],
    ['names', 3, 2, 1],
    ['names', 7, 2, 5],
    ['icons', 6, 6, 0],
    ['icons', 7, 5, 2],
    ['stacked', 3, 3, 0],
    ['stacked', 4, 2, 2],
    ['stacked', 9, 2, 7],
  ] as const)('%s with %i others shows %i and puts %i behind +N', (style, count, shown, hidden) => {
    const plan = planTray(many(count), style, none)
    expect(plan.shown).toHaveLength(shown)
    expect(plan.hidden).toBe(hidden)
  })

  it('shows the most urgent first and says when a hidden one needs you', () => {
    const others = [...many(3), session('wait', 'needs'), session('run', 'working')]
    const plan = planTray(others, 'names', none)
    expect(ids(plan.shown)).toEqual(['wait', 'run'])
    expect(plan.hiddenNeeds).toBe(false)
    const crowded = [...many(2, 'needs'), session('late', 'needs')]
    expect(planTray(crowded, 'names', none).hiddenNeeds).toBe(true)
  })
})
