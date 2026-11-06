import { describe, expect, it } from 'vitest'
import type { SessionStatus, SessionView } from './session'
import { byPriority, chooseCard, planStack } from './stack'

function session(id: string, status: SessionStatus, pinned = false): SessionView {
  return { id, status, pinned } as SessionView
}

const none = () => 0
const ids = (sessions: readonly SessionView[]) => sessions.map((item) => item.id)

describe('which session is the card', () => {
  const sessions = [session('a', 'idle'), session('b', 'working'), session('c', 'needs')]

  it('is the session you are in, wherever it sits', () => {
    expect(chooseCard(sessions, 'b', undefined)?.id).toBe('b')
  })

  it('is the one last picked in the project when you are in another project', () => {
    expect(chooseCard(sessions, 'elsewhere', 'c')?.id).toBe('c')
  })

  it('is the first by creation order when nothing was picked, however urgent the others are', () => {
    expect(chooseCard(sessions, null, undefined)?.id).toBe('a')
  })

  it('ignores a remembered session that has gone', () => {
    expect(chooseCard(sessions, null, 'gone')?.id).toBe('a')
  })

  it('is nothing for an empty project', () => {
    expect(chooseCard([], null, undefined)).toBeUndefined()
  })

  it('does not move for a pin', () => {
    const pinned = [session('a', 'idle'), session('b', 'idle', true)]
    expect(chooseCard(pinned, null, undefined)?.id).toBe('a')
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

describe('the rows around the card', () => {
  const many = (count: number, status: SessionStatus = 'idle') => Array.from({ length: count }, (_, index) => session(`s${String(index)}`, status))

  it('keeps creation order: the sessions before the card above it, the rest after', () => {
    const all = many(5)
    const card = all[2] as SessionView
    const plan = planStack(all, card, 'tree')
    expect(ids(plan.before)).toEqual(['s0', 's1'])
    expect(ids(plan.shown)).toEqual(['s3', 's4'])
  })

  it('does not reorder for status or pins', () => {
    const all = [session('a', 'idle', true), session('b', 'needs'), session('c', 'working')]
    const plan = planStack(all, all[0] as SessionView, 'names')
    expect(ids(plan.shown)).toEqual(['b', 'c'])
  })

  it('shows every row before the card, however many', () => {
    const all = many(12)
    expect(planStack(all, all[11] as SessionView, 'tree').before).toHaveLength(11)
  })

  it.each([
    ['tree', 4, 4, 0],
    ['tree', 5, 2, 3],
    ['names', 2, 2, 0],
    ['names', 3, 2, 1],
    ['names', 7, 2, 5],
    ['icons', 6, 6, 0],
    ['icons', 7, 5, 2],
    ['stacked', 3, 3, 0],
    ['stacked', 4, 2, 2],
    ['stacked', 9, 2, 7],
  ] as const)('%s with %i after the card shows %i and puts %i behind +N', (style, count, shown, hidden) => {
    const all = many(count + 1)
    const plan = planStack(all, all[0] as SessionView, style)
    expect(plan.shown).toHaveLength(shown)
    expect(plan.hidden).toBe(hidden)
  })

  it('says when a hidden row needs you', () => {
    const all = [...many(3), session('late', 'needs')]
    expect(planStack(all, all[0] as SessionView, 'names').hiddenNeeds).toBe(true)
    expect(planStack(all, all[0] as SessionView, 'tree').hiddenNeeds).toBe(false)
  })
})
