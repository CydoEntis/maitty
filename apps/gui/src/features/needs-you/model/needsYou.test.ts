import { describe, expect, it } from 'vitest'
import type { SessionStatus, SessionView } from '@/features/workspace/model/session'
import { byCreation, jumpToast, nextNeedsYou } from './needsYou'

function session(pane: number, status: SessionStatus, machine = 0): SessionView {
  return { id: `${String(machine)}:${String(pane)}`, pane, machine, name: null, title: `s${String(pane)}`, status } as SessionView
}

describe('creation order', () => {
  it('goes by pane number, and breaks a tie between machines by the machine number', () => {
    expect([session(2, 'idle', 0), session(1, 'idle', 1), session(1, 'idle', 0)].sort(byCreation).map((item) => item.id)).toEqual(['0:1', '1:1', '0:2'])
  })
})

describe('nextNeedsYou', () => {
  const sessions = [session(1, 'needs'), session(2, 'working'), session(3, 'needs'), session(4, 'done'), session(5, 'needs')]

  it('is null when nothing needs you, even if something is done', () => {
    expect(nextNeedsYou([session(1, 'done'), session(2, 'error'), session(3, 'idle')], '0:1')).toBeNull()
    expect(nextNeedsYou([], null)).toBeNull()
  })

  it('starts at the first one when nothing is focused', () => {
    expect(nextNeedsYou(sessions, null)?.session.id).toBe('0:1')
  })

  it('takes the next one after the focused session that needs you', () => {
    expect(nextNeedsYou(sessions, '0:1')).toMatchObject({ position: 2, total: 3, session: { id: '0:3' } })
    expect(nextNeedsYou(sessions, '0:3')?.session.id).toBe('0:5')
  })

  it('wraps from the last one to the first', () => {
    expect(nextNeedsYou(sessions, '0:5')).toMatchObject({ position: 1, session: { id: '0:1' } })
  })

  it('goes by creation order from a focused session that does not need you', () => {
    expect(nextNeedsYou(sessions, '0:2')?.session.id).toBe('0:3')
    expect(nextNeedsYou(sessions, '0:4')?.session.id).toBe('0:5')
  })

  it('stays on the only one that needs you', () => {
    expect(nextNeedsYou([session(1, 'needs'), session(2, 'idle')], '0:1')).toMatchObject({ position: 1, total: 1, session: { id: '0:1' } })
  })

  it('walks sessions of several machines in creation order', () => {
    const many = [session(2, 'needs', 1), session(2, 'needs', 0), session(1, 'needs', 1)]
    expect(nextNeedsYou(many, '1:1')?.session.id).toBe('0:2')
    expect(nextNeedsYou(many, '0:2')?.session.id).toBe('1:2')
    expect(nextNeedsYou(many, '1:2')?.session.id).toBe('1:1')
  })

  it('does not depend on the order the list arrives in', () => {
    expect(nextNeedsYou([...sessions].reverse(), '0:1')?.session.id).toBe('0:3')
  })

  it('names the session and its place in the toast', () => {
    const target = nextNeedsYou([{ ...session(1, 'needs'), name: 'Rate limiter' }, session(2, 'needs')], '0:2')
    expect(target && jumpToast(target)).toBe('Needs you · Rate limiter · 1 of 2')
  })
})
