import { describe, expect, it } from 'vitest'
import { activateArrangement, EMPTY_LAYOUT, openArrangement, splitPane, type Layout, type MakeId } from './layout'
import { tabsOf, type TabShip } from './tabs'

let counter = 0
const makeId: MakeId = (prefix) => { counter += 1; return `${prefix}-${String(counter)}` }
const ship = (pane: number): TabShip => ({ id: `t${String(pane)}`, pane, machine: 0 })

describe('the tabs of a fleet', () => {
  it('has one tab per ship, opened or not, in the ships’ creation order', () => {
    let layout: Layout = openArrangement(EMPTY_LAYOUT, 't3', makeId)
    layout = openArrangement(layout, 't1', makeId)
    const tabs = tabsOf(layout, [ship(1), ship(2), ship(3)])
    expect(tabs.map((tab) => [tab.sessionId, tab.arrangementId !== null])).toEqual([['t1', true], ['t2', false], ['t3', true]])
    expect(tabs.filter((tab) => tab.active).map((tab) => tab.sessionId)).toEqual(['t1'])
  })

  it('folds a ship split beside another into that tab, and counts the panes', () => {
    let layout = openArrangement(EMPTY_LAYOUT, 't1', makeId)
    layout = splitPane(layout, null, 'right', 't2', makeId)
    const tabs = tabsOf(layout, [ship(1), ship(2)])
    expect(tabs).toHaveLength(1)
    expect(tabs[0]).toMatchObject({ sessionId: 't1', paneCount: 2, sessionIds: ['t1', 't2'] })
  })

  it('does not move a tab when another is opened or closed', () => {
    const before = tabsOf(openArrangement(EMPTY_LAYOUT, 't2', makeId), [ship(1), ship(2), ship(3)]).map((tab) => tab.sessionId)
    const after = tabsOf(openArrangement(openArrangement(EMPTY_LAYOUT, 't2', makeId), 't3', makeId), [ship(1), ship(2), ship(3)]).map((tab) => tab.sessionId)
    expect(before).toEqual(['t1', 't2', 't3'])
    expect(after).toEqual(before)
  })

  it('shows no tab as active while the layout shows none', () => {
    const layout = activateArrangement(openArrangement(EMPTY_LAYOUT, 't1', makeId), null)
    expect(tabsOf(layout, [ship(1)]).some((tab) => tab.active)).toBe(false)
  })

  it('leaves another fleet’s ships out', () => {
    const layout = openArrangement(openArrangement(EMPTY_LAYOUT, 't1', makeId), 't9', makeId)
    expect(tabsOf(layout, [ship(1)]).map((tab) => tab.sessionId)).toEqual(['t1'])
  })
})

describe('showing an arrangement', () => {
  it('ignores one that does not exist', () => {
    const layout = openArrangement(EMPTY_LAYOUT, 't1', makeId)
    expect(activateArrangement(layout, 'nope')).toBe(layout)
  })
})
