import { describe, expect, it } from 'vitest'
import type { SessionStatus, SessionView } from '@/features/workspace/model/session'
import { jumpToast, machineCount, needsYouCount, nextNeedsYou, overviewSections, sectionOf } from './overview'

function session(pane: number, status: SessionStatus, machine = 0): SessionView {
  return { id: `${String(machine)}:${String(pane)}`, pane, machine, name: null, title: `s${String(pane)}`, status } as SessionView
}
const ids = (list: readonly SessionView[]) => list.map((item) => item.id)
const section = (sessions: readonly SessionView[], name: string) => ids(overviewSections(sessions).find((group) => group.section === name)?.sessions ?? [])

describe('sections', () => {
  it('lists the five sections in the fixed order, empty or not', () => {
    expect(overviewSections([]).map((group) => group.section)).toEqual(['needs', 'working', 'done', 'idle', 'unknown'])
  })

  it.each([
    ['needs', 'needs'],
    ['working', 'working'],
    ['done', 'done'],
    ['error', 'done'],
    ['idle', 'idle'],
    ['unknown', 'unknown'],
  ] as const)('puts %s in %s', (status, expected) => {
    expect(sectionOf(status)).toBe(expected)
  })

  it('keeps creation order inside a section, whatever order the sessions arrive in', () => {
    const sessions = [session(5, 'working'), session(2, 'working'), session(9, 'working')]
    expect(section(sessions, 'working')).toEqual(['0:2', '0:5', '0:9'])
  })

  it('breaks a tie between machines by the machine number', () => {
    const sessions = [session(3, 'needs', 2), session(3, 'needs', 0), session(3, 'needs', 1)]
    expect(section(sessions, 'needs')).toEqual(['0:3', '1:3', '2:3'])
  })

  it('mixes finished and exited sessions in Done by creation order only', () => {
    expect(section([session(4, 'error'), session(1, 'done')], 'done')).toEqual(['0:1', '0:4'])
  })

  it('moves a card between sections on a status change without reordering the rest', () => {
    const before = [session(1, 'working'), session(2, 'working'), session(3, 'working')]
    const after = [session(1, 'working'), session(2, 'needs'), session(3, 'working')]
    expect(section(before, 'working')).toEqual(['0:1', '0:2', '0:3'])
    expect(section(after, 'working')).toEqual(['0:1', '0:3'])
    expect(section(after, 'needs')).toEqual(['0:2'])
  })

  it('does not change its input', () => {
    const sessions = [session(2, 'idle'), session(1, 'idle')]
    overviewSections(sessions)
    expect(ids(sessions)).toEqual(['0:2', '0:1'])
  })
})

describe('counts', () => {
  it('counts only the sessions that need you', () => {
    expect(needsYouCount([session(1, 'needs'), session(2, 'done'), session(3, 'needs'), session(4, 'error')])).toBe(2)
  })

  it('counts this computer as one machine and each other once', () => {
    expect(machineCount([session(1, 'idle'), session(2, 'idle'), session(1, 'idle', 1), session(1, 'idle', 2)])).toBe(3)
    expect(machineCount([])).toBe(0)
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

  it('walks sessions of several machines in the Overview order', () => {
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
