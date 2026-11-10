import { describe, expect, it } from 'vitest'
import { keptArrangements, KEPT_PANES, KEPT_TABS, rememberShown } from './keptTabs'
import type { Arrangement, Layout } from './layout'

const tab = (id: string, ...sessions: string[]): Arrangement => ({
  id,
  panes: sessions.map((sessionId, col) => ({ id: `${id}-${sessionId}`, sessionId, col })),
  focus: `${id}-${sessions[0] ?? ''}`,
  max: null,
})

const layout = (activeId: string | null, ...arrangements: Arrangement[]): Layout => ({ arrangements, activeId })

const idsOf = (tabs: readonly Arrangement[]) => tabs.map((kept) => kept.id)

describe('the tabs shown most recently', () => {
  it('puts the tab showing now first, once', () => {
    expect(rememberShown(['a', 'b'], 'c')).toEqual(['c', 'a', 'b'])
    expect(rememberShown(['a', 'b'], 'b')).toEqual(['b', 'a'])
  })

  it('is the same list when the tab showing has not changed, or nothing shows', () => {
    const recent = ['a', 'b']
    expect(rememberShown(recent, 'a')).toBe(recent)
    expect(rememberShown(recent, null)).toBe(recent)
  })

  it('forgets the ones shown longest ago', () => {
    const many = ['a', 'b', 'c', 'd', 'e'].reduce<readonly string[]>((recent, id) => rememberShown(recent, id), [])
    expect(many).toHaveLength(KEPT_TABS + 1)
    expect(many[0]).toBe('e')
  })
})

describe('the tabs kept mounted out of sight', () => {
  it('are the ones shown last, without the one showing', () => {
    const now = layout('c', tab('a', 's1'), tab('b', 's2'), tab('c', 's3'))
    expect(idsOf(keptArrangements(now, ['c', 'b', 'a']))).toEqual(['b', 'a'])
  })

  it('are none before any other tab was shown', () => {
    expect(keptArrangements(layout('a', tab('a', 's1'), tab('b', 's2')), ['a'])).toEqual([])
  })

  it('leave out a tab that was closed', () => {
    const now = layout('c', tab('a', 's1'), tab('c', 's3'))
    expect(idsOf(keptArrangements(now, ['c', 'b', 'a']))).toEqual(['a'])
  })

  it('leave out a tab whose session is in the tab showing, which mounts it', () => {
    const now = layout('c', tab('a', 's1'), tab('b', 's2'), tab('c', 's3', 's2'))
    expect(idsOf(keptArrangements(now, ['c', 'b', 'a']))).toEqual(['a'])
  })

  it('leave out a tab whose session a kept tab already mounts', () => {
    const now = layout('c', tab('a', 's1', 's2'), tab('b', 's2'), tab('c', 's3'))
    expect(idsOf(keptArrangements(now, ['c', 'b', 'a']))).toEqual(['b'])
  })

  it('hold no more panes between them than the terminals the window can keep', () => {
    const wide = (id: string) => tab(id, ...Array.from({ length: KEPT_PANES - 1 }, (_, pane) => `${id}${String(pane)}`))
    const now = layout('c', wide('a'), wide('b'), tab('c', 's3'))
    expect(idsOf(keptArrangements(now, ['c', 'b', 'a']))).toEqual(['b'])
  })
})
