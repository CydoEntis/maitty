import { describe, expect, it } from 'vitest'
import {
  activeArrangement,
  dropSession,
  setColumnWidths,
  setPaneHeights,
  toggleBroadcast,
  cycleFocus,
  EMPTY_LAYOUT,
  focusedSessionId,
  openArrangement,
  focusPane,
  paneColumns,
  parseLayout,
  reconcileLayout,
  removeSessions,
  showSession,
  splitPane,
  swapSessions,
  toggleMaximize,
  visibleSessionIds,
  type Layout,
  type MakeId,
} from './layout'

function ids(): MakeId {
  let next = 0
  return (prefix) => {
    next += 1
    return `${prefix}${String(next)}`
  }
}

function withPanes(...sessions: string[]): { layout: Layout; makeId: MakeId } {
  const makeId = ids()
  const [first, ...rest] = sessions
  let layout = openArrangement(EMPTY_LAYOUT, first ?? 's', makeId)
  for (const session of rest) layout = splitPane(layout, null, 'right', session, makeId)
  return { layout, makeId }
}

/** The arrangement at this index is the one showing. */
const pick = (layout: Layout, index: number): Layout => ({ ...layout, activeId: layout.arrangements[index]?.id ?? null })

const cols = (layout: Layout) => activeArrangement(layout)?.panes.map((pane) => `${pane.sessionId}@${String(pane.col)}`)

describe('tab layout', () => {
  it('opens a session in a new tab and focuses it', () => {
    const layout = openArrangement(EMPTY_LAYOUT, 'a', ids())
    expect(layout.arrangements).toHaveLength(1)
    expect(focusedSessionId(layout)).toBe('a')
  })

  it('splits right into a new column after the pane and down into the same column', () => {
    const makeId = ids()
    let layout = openArrangement(EMPTY_LAYOUT, 'a', makeId)
    layout = splitPane(layout, null, 'right', 'b', makeId)
    expect(cols(layout)).toEqual(['a@0', 'b@1'])
    layout = splitPane(layout, null, 'down', 'c', makeId)
    expect(cols(layout)).toEqual(['a@0', 'b@1', 'c@1'])
    const firstPane = activeArrangement(layout)?.panes[0]?.id ?? null
    layout = splitPane(layout, firstPane, 'right', 'd', makeId)
    expect(cols(layout)).toEqual(['a@0', 'd@1', 'b@2', 'c@2'])
    expect(focusedSessionId(layout)).toBe('d')
  })

  it('splits left and up as well: the new pane goes before the one split', () => {
    const makeId = ids()
    let layout = openArrangement(EMPTY_LAYOUT, 'a', makeId)
    layout = splitPane(layout, null, 'left', 'b', makeId)
    expect(cols(layout)).toEqual(['b@0', 'a@1'])
    expect(focusedSessionId(layout)).toBe('b')
    layout = splitPane(layout, null, 'up', 'c', makeId)
    expect(cols(layout)).toEqual(['c@0', 'b@0', 'a@1'])
    const last = activeArrangement(layout)?.panes[2]?.id ?? null
    layout = splitPane(layout, last, 'left', 'd', makeId)
    expect(cols(layout)).toEqual(['c@0', 'b@0', 'd@1', 'a@2'])
  })

  it('holds at most four panes', () => {
    const { layout, makeId } = withPanes('a', 'b', 'c', 'd')
    expect(splitPane(layout, null, 'down', 'e', makeId)).toBe(layout)
  })

  it('removes a pane with its session and closes the columns up', () => {
    const { layout } = withPanes('a', 'b', 'c')
    expect(cols(removeSessions(layout, new Set(['b'])))).toEqual(['a@0', 'c@1'])
  })

  it('removes the arrangement when its last pane goes', () => {
    const single = withPanes('a').layout
    expect(removeSessions(single, new Set(['a'])).arrangements).toHaveLength(0)
  })

  it('moves focus to the previous pane when the focused one goes', () => {
    const { layout } = withPanes('a', 'b', 'c')
    expect(focusedSessionId(removeSessions(layout, new Set(['c'])))).toBe('b')
  })

  it('removes an arrangement and shows the one that took its place', () => {
    const makeId = ids()
    let layout = openArrangement(EMPTY_LAYOUT, 'a', makeId)
    layout = openArrangement(layout, 'b', makeId)
    layout = openArrangement(layout, 'c', makeId)
    layout = pick(layout, 1)
    layout = removeSessions(layout, new Set(['b']))
    expect(focusedSessionId(layout)).toBe('c')
  })

  it('cycles pane focus both ways, wrapping around', () => {
    const { layout } = withPanes('a', 'b')
    expect(focusedSessionId(cycleFocus(layout, 1))).toBe('a')
    expect(focusedSessionId(cycleFocus(layout, -1))).toBe('a')
  })

  it('focuses a sidebar session where it is shown, else gives it a layout of its own', () => {
    const { layout, makeId } = withPanes('a', 'b')
    const shown = showSession(layout, 'a', makeId)
    expect(focusedSessionId(shown)).toBe('a')
    expect(cols(shown)).toEqual(['a@0', 'b@1'])
    const opened = showSession(layout, 'z', makeId)
    expect(opened.arrangements).toHaveLength(2)
    expect(cols(opened)).toEqual(['z@0'])
    expect(activeArrangement(layout)?.panes).toHaveLength(2)
    expect(showSession(EMPTY_LAYOUT, 'z', makeId).arrangements).toHaveLength(1)
  })

  it('finds a session shown in another tab and switches to it', () => {
    const makeId = ids()
    let layout = openArrangement(EMPTY_LAYOUT, 'a', makeId)
    layout = openArrangement(layout, 'b', makeId)
    layout = showSession(layout, 'a', makeId)
    expect(layout.activeId).toBe(layout.arrangements[0]?.id)
    expect(visibleSessionIds(layout)).toEqual(new Set(['a']))
  })

  it('swaps the sessions of two panes, keeping their places', () => {
    const { layout } = withPanes('a', 'b')
    const [first, second] = activeArrangement(layout)?.panes ?? []
    const swapped = swapSessions(layout, first?.id ?? '', second?.id ?? '')
    expect(cols(swapped)).toEqual(['b@0', 'a@1'])
    expect(activeArrangement(swapped)?.focus).toBe(second?.id)
  })

  it('maximizes one of several panes and restores the layout', () => {
    const { layout } = withPanes('a', 'b')
    const tab = activeArrangement(layout)
    const first = tab?.panes[0]?.id ?? ''
    const maxed = toggleMaximize(layout, first)
    const maxedTab = activeArrangement(maxed)
    if (!maxedTab) throw new Error('no tab')
    expect(paneColumns(maxedTab, false).map((column) => column.map((pane) => pane.sessionId))).toEqual([['a']])
    expect(activeArrangement(toggleMaximize(maxed, first))?.max).toBeNull()
    const single = withPanes('a').layout
    expect(toggleMaximize(single, activeArrangement(single)?.focus ?? '')).toBe(single)
  })

  it('stacks panes in one column when narrow', () => {
    const { layout, makeId } = withPanes('a', 'b')
    const three = splitPane(layout, null, 'down', 'c', makeId)
    const tab = activeArrangement(three)
    if (!tab) throw new Error('no tab')
    expect(paneColumns(tab, false).map((column) => column.length)).toEqual([1, 2])
    expect(paneColumns(tab, true).map((column) => column.length)).toEqual([1, 1, 1])
  })

  it('drops panes and tabs whose sessions ended', () => {
    const makeId = ids()
    let layout = openArrangement(EMPTY_LAYOUT, 'a', makeId)
    layout = splitPane(layout, null, 'right', 'b', makeId)
    layout = openArrangement(layout, 'c', makeId)
    const reconciled = reconcileLayout(layout, new Set(['a']))
    expect(reconciled.arrangements).toHaveLength(1)
    expect(cols(reconciled)).toEqual(['a@0'])
    expect(reconcileLayout(reconciled, new Set(['a']))).toBe(reconciled)
  })

  it('moves to the tab that took its place, not the first tab, when the showing tab’s session ends', () => {
    const makeId = ids()
    let layout: Layout = EMPTY_LAYOUT
    for (const session of ['a', 'b', 'c', 'd']) layout = openArrangement(layout, session, makeId)
    layout = pick(layout, 2)

    const reconciled = reconcileLayout(layout, new Set(['a', 'b', 'd']))

    expect(reconciled.arrangements.map((tab) => tab.panes[0]?.sessionId)).toEqual(['a', 'b', 'd'])
    expect(activeArrangement(reconciled)?.panes[0]?.sessionId).toBe('d')
  })

  it('falls back to the tab before when the last tab’s session ends', () => {
    const makeId = ids()
    let layout: Layout = EMPTY_LAYOUT
    for (const session of ['a', 'b', 'c']) layout = openArrangement(layout, session, makeId)

    const reconciled = reconcileLayout(layout, new Set(['a', 'b']))

    expect(activeArrangement(reconciled)?.panes[0]?.sessionId).toBe('b')
  })

  it('moves focus to the pane before the one whose session ended', () => {
    const { layout } = withPanes('a', 'b', 'c')
    const tab = activeArrangement(layout)
    const middle = tab?.panes[1]?.id ?? ''
    const focused = focusPane(layout, middle)

    const reconciled = reconcileLayout(focused, new Set(['a', 'c']))

    expect(activeArrangement(reconciled)?.focus).toBe(tab?.panes[0]?.id)
  })

  it('puts the focused pane on screen when a maximized tab is focused elsewhere', () => {
    const { layout } = withPanes('a', 'b')
    const first = activeArrangement(layout)?.panes[0]?.id ?? ''
    const maximized = toggleMaximize(layout, first)

    const viaSidebar = showSession(maximized, 'b', ids())
    const second = activeArrangement(viaSidebar)?.panes[1]?.id
    expect(activeArrangement(viaSidebar)?.max).toBe(second)
    expect(activeArrangement(viaSidebar)?.focus).toBe(second)

    const viaFocus = focusPane(maximized, second ?? '')
    expect(activeArrangement(viaFocus)?.max).toBe(second)
  })

  it('leaves max alone when the tab is not maximized', () => {
    const { layout } = withPanes('a', 'b')
    const second = activeArrangement(layout)?.panes[1]?.id ?? ''
    expect(activeArrangement(focusPane(layout, second))?.max).toBeNull()
  })

  it('reads back a stored layout and ignores anything malformed', () => {
    const { layout } = withPanes('a', 'b')
    expect(parseLayout(JSON.parse(JSON.stringify(layout)))).toEqual(layout)
    expect(parseLayout({ arrangements: [{ id: 't', panes: [{ id: 1 }] }, 'x'] })).toEqual(EMPTY_LAYOUT)
    expect(parseLayout(null)).toEqual(EMPTY_LAYOUT)
  })
})

describe('column widths', () => {
  it('keeps dragged widths until the columns change, then goes back to equal', () => {
    const { layout } = withPanes('a', 'b')
    const tabId = layout.activeId ?? ''
    const sized = setColumnWidths(layout, tabId, [2, 1])
    expect(activeArrangement(sized)?.widths).toEqual([2, 1])
    expect(activeArrangement(setColumnWidths(sized, tabId, null))?.widths).toBeUndefined()

    expect(activeArrangement(removeSessions(sized, new Set(['b'])))?.widths).toBeUndefined()
    expect(activeArrangement(reconcileLayout(sized, new Set(['a'])))?.widths).toBeUndefined()
  })

  it('survives a reload only when it matches the columns', () => {
    const { layout } = withPanes('a', 'b')
    const sized = setColumnWidths(layout, layout.activeId ?? '', [3, 1])
    expect(activeArrangement(parseLayout(JSON.parse(JSON.stringify(sized))))?.widths).toEqual([3, 1])
    const bad = JSON.parse(JSON.stringify(sized)) as { arrangements: Array<{ widths: unknown }> }
    if (bad.arrangements[0]) bad.arrangements[0].widths = [1, -2]
    expect(activeArrangement(parseLayout(bad))?.widths).toBeUndefined()
  })
})

describe('broadcast', () => {
  it('turns on and off for one tab and is remembered', () => {
    const { layout } = withPanes('a', 'b')
    const on = toggleBroadcast(layout, layout.activeId ?? '')
    expect(activeArrangement(on)?.broadcast).toBe(true)
    expect(activeArrangement(parseLayout(JSON.parse(JSON.stringify(on))))?.broadcast).toBe(true)
    expect(activeArrangement(toggleBroadcast(on, layout.activeId ?? ''))?.broadcast).toBe(false)
  })
})

describe('dropping a session on a pane', () => {
  it('swaps with the pane when the session is already in this tab', () => {
    const { layout } = withPanes('a', 'b')
    const first = activeArrangement(layout)?.panes[0]?.id ?? ''
    expect(cols(dropSession(layout, first, 'b'))).toEqual(['b@0', 'a@1'])
  })

  it('moves a session from another tab here, and the pane it left takes this pane\'s session', () => {
    const makeId = ids()
    let layout = openArrangement(EMPTY_LAYOUT, 'a', makeId)
    layout = openArrangement(layout, 'b', makeId)
    const firstTab = layout.arrangements[0]
    layout = pick(layout, 0)
    const result = dropSession(layout, firstTab?.panes[0]?.id ?? '', 'b')
    expect(result.arrangements.map((tab) => tab.panes.map((pane) => pane.sessionId))).toEqual([['b'], ['a']])
    expect(result.activeId).toBe(firstTab?.id)
  })

  it('replaces the pane\'s session when the session is shown nowhere', () => {
    const { layout } = withPanes('a', 'b')
    const first = activeArrangement(layout)?.panes[0]?.id ?? ''
    expect(cols(dropSession(layout, first, 'z'))).toEqual(['z@0', 'b@1'])
  })
})

describe('mixed splits', () => {
  it('gives a pane split right half of its column’s width', () => {
    const { layout, makeId } = withPanes('a', 'b')
    const three = splitPane(layout, null, 'right', 'c', makeId)
    expect(activeArrangement(three)?.widths).toEqual([0.5, 0.25, 0.25])
  })

  it('gives a pane split down half of its pane’s height, in a column of its own', () => {
    const { layout, makeId } = withPanes('a', 'b')
    const stacked = splitPane(layout, null, 'down', 'c', makeId)
    const twice = splitPane(stacked, null, 'down', 'd', makeId)
    expect(activeArrangement(twice)?.panes.map((pane) => [pane.sessionId, pane.col, pane.h])).toEqual([['a', 0, undefined], ['b', 1, undefined], ['c', 1, 0.5], ['d', 1, 0.5]])
  })

  it('sets the heights of one column’s panes, and evens them out again', () => {
    const { layout, makeId } = withPanes('a')
    const stacked = splitPane(layout, null, 'down', 'b', makeId)
    const tab = activeArrangement(stacked)
    if (!tab) throw new Error('no tab')
    const sized = setPaneHeights(stacked, tab.id, 0, [300, 100])
    expect(activeArrangement(sized)?.panes.map((pane) => pane.h)).toEqual([300, 100])
    expect(activeArrangement(setPaneHeights(sized, tab.id, 0, null))?.panes.map((pane) => pane.h)).toEqual([undefined, undefined])
  })

  it('keeps a stored pane height when it reads a saved layout back', () => {
    const saved = parseLayout({ activeId: 't', arrangements: [{ id: 't', focus: 'p1', max: null, panes: [{ id: 'p1', sessionId: 'a', col: 0, h: 2 }, { id: 'p2', sessionId: 'b', col: 0, h: -1 }] }] })
    expect(activeArrangement(saved)?.panes.map((pane) => pane.h)).toEqual([2, undefined])
  })
})
