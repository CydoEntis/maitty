import { describe, expect, it } from 'vitest'
import {
  activeTab,
  dropSession,
  setColumnWidths,
  toggleBroadcast,
  cycleFocus,
  cycleTab,
  EMPTY_LAYOUT,
  focusedSessionId,
  openInNewTab,
  focusPane,
  paneColumns,
  parseLayout,
  reconcileLayout,
  removeSessions,
  selectTab,
  showSession,
  splitPane,
  swapSessions,
  toggleMaximize,
  visibleSessionIds,
  type Layout,
  type MakeId,
  type Tab,
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
  let layout = openInNewTab(EMPTY_LAYOUT, first ?? 's', makeId)
  for (const session of rest) layout = splitPane(layout, null, 'right', session, makeId)
  return { layout, makeId }
}

const cols = (layout: Layout) => activeTab(layout)?.panes.map((pane) => `${pane.sessionId}@${String(pane.col)}`)

describe('tab layout', () => {
  it('opens a session in a new tab and focuses it', () => {
    const layout = openInNewTab(EMPTY_LAYOUT, 'a', ids())
    expect(layout.tabs).toHaveLength(1)
    expect(focusedSessionId(layout)).toBe('a')
  })

  it('splits right into a new column after the pane and down into the same column', () => {
    const makeId = ids()
    let layout = openInNewTab(EMPTY_LAYOUT, 'a', makeId)
    layout = splitPane(layout, null, 'right', 'b', makeId)
    expect(cols(layout)).toEqual(['a@0', 'b@1'])
    layout = splitPane(layout, null, 'down', 'c', makeId)
    expect(cols(layout)).toEqual(['a@0', 'b@1', 'c@1'])
    const firstPane = activeTab(layout)?.panes[0]?.id ?? null
    layout = splitPane(layout, firstPane, 'right', 'd', makeId)
    expect(cols(layout)).toEqual(['a@0', 'd@1', 'b@2', 'c@2'])
    expect(focusedSessionId(layout)).toBe('d')
  })

  it('holds at most four panes', () => {
    const { layout, makeId } = withPanes('a', 'b', 'c', 'd')
    expect(splitPane(layout, null, 'down', 'e', makeId)).toBe(layout)
  })

  it('removes a pane with its session and closes the columns up', () => {
    const { layout } = withPanes('a', 'b', 'c')
    expect(cols(removeSessions(layout, new Set(['b'])))).toEqual(['a@0', 'c@1'])
  })

  it('removes the tab when its last pane goes', () => {
    const single = withPanes('a').layout
    expect(removeSessions(single, new Set(['a'])).tabs).toHaveLength(0)
  })

  it('moves focus to the previous pane when the focused one goes', () => {
    const { layout } = withPanes('a', 'b', 'c')
    expect(focusedSessionId(removeSessions(layout, new Set(['c'])))).toBe('b')
  })

  it('removes a tab and shows the one that took its place', () => {
    const makeId = ids()
    let layout = openInNewTab(EMPTY_LAYOUT, 'a', makeId)
    layout = openInNewTab(layout, 'b', makeId)
    layout = openInNewTab(layout, 'c', makeId)
    layout = cycleTab(layout, -1)
    layout = removeSessions(layout, new Set(['b']))
    expect(focusedSessionId(layout)).toBe('c')
  })

  it('cycles pane focus and tabs both ways, wrapping around', () => {
    const { layout, makeId } = withPanes('a', 'b')
    expect(focusedSessionId(cycleFocus(layout, 1))).toBe('a')
    expect(focusedSessionId(cycleFocus(layout, -1))).toBe('a')
    const two = openInNewTab(layout, 'c', makeId)
    expect(focusedSessionId(cycleTab(two, 1))).toBe('b')
    expect(focusedSessionId(cycleTab(two, -1))).toBe('b')
  })

  it('focuses a sidebar session where it is shown, else opens it in a new tab', () => {
    const { layout, makeId } = withPanes('a', 'b')
    const shown = showSession(layout, 'a', makeId)
    expect(focusedSessionId(shown)).toBe('a')
    expect(cols(shown)).toEqual(['a@0', 'b@1'])
    const opened = showSession(layout, 'z', makeId)
    expect(opened.tabs).toHaveLength(2)
    expect(cols(opened)).toEqual(['z@0'])
    expect(activeTab(layout)?.panes).toHaveLength(2)
    expect(showSession(EMPTY_LAYOUT, 'z', makeId).tabs).toHaveLength(1)
  })

  describe('tabs of one project', () => {
    const project = (tab: { panes: Array<{ sessionId: string }> }) => (tab.panes[0]?.sessionId.startsWith('m') ? 'maitty' : 'docs')
    const build = () => {
      const makeId = ids()
      let layout = openInNewTab(EMPTY_LAYOUT, 'm1', makeId)
      layout = openInNewTab(layout, 'd1', makeId)
      layout = openInNewTab(layout, 'm2', makeId)
      layout = openInNewTab(layout, 'd2', makeId)
      return layout
    }
    const at = (layout: ReturnType<typeof build>, index: number) => layout.tabs[index]?.id ?? ''

    it('cycles only through the tabs of the project that is showing', () => {
      const built = build()
      const layout = selectTab(built, at(built, 0))
      const next = cycleTab(layout, 1, project)
      expect(focusedSessionId(next)).toBe('m2')
      expect(focusedSessionId(cycleTab(next, 1, project))).toBe('m1')
      expect(focusedSessionId(cycleTab(layout, -1, project))).toBe('m2')
    })

    it('stays put when the project has a single tab', () => {
      const layout = build()
      const only = selectTab({ ...layout, tabs: layout.tabs.filter((tab) => tab.id !== at(layout, 2)) }, at(layout, 0))
      expect(cycleTab(only, 1, project).activeTabId).toBe(at(layout, 0))
    })

    it('closing the showing tab goes to the next tab of the same project, else the one before', () => {
      const layout = build()
      const first = selectTab(layout, at(layout, 0))
      expect(focusedSessionId(removeSessions(first, new Set(['m1']), project))).toBe('m2')
      const last = selectTab(layout, at(layout, 2))
      expect(focusedSessionId(removeSessions(last, new Set(['m2']), project))).toBe('m1')
    })

    it('goes to another project only when the closed one had no tab left', () => {
      const makeId = ids()
      let layout = openInNewTab(EMPTY_LAYOUT, 'd1', makeId)
      layout = openInNewTab(layout, 'm1', makeId)
      const closed = removeSessions(layout, new Set(['m1']), project)
      expect(focusedSessionId(closed)).toBe('d1')
    })

    it('leaves the showing tab alone when another one is closed', () => {
      const layout = build()
      expect(removeSessions(layout, new Set(['m1']), project).activeTabId).toBe(layout.activeTabId)
    })
  })

  it('finds a session shown in another tab and switches to it', () => {
    const makeId = ids()
    let layout = openInNewTab(EMPTY_LAYOUT, 'a', makeId)
    layout = openInNewTab(layout, 'b', makeId)
    layout = showSession(layout, 'a', makeId)
    expect(layout.activeTabId).toBe(layout.tabs[0]?.id)
    expect(visibleSessionIds(layout)).toEqual(new Set(['a']))
  })

  it('swaps the sessions of two panes, keeping their places', () => {
    const { layout } = withPanes('a', 'b')
    const [first, second] = activeTab(layout)?.panes ?? []
    const swapped = swapSessions(layout, first?.id ?? '', second?.id ?? '')
    expect(cols(swapped)).toEqual(['b@0', 'a@1'])
    expect(activeTab(swapped)?.focus).toBe(second?.id)
  })

  it('maximizes one of several panes and restores the layout', () => {
    const { layout } = withPanes('a', 'b')
    const tab = activeTab(layout)
    const first = tab?.panes[0]?.id ?? ''
    const maxed = toggleMaximize(layout, first)
    const maxedTab = activeTab(maxed)
    if (!maxedTab) throw new Error('no tab')
    expect(paneColumns(maxedTab, false).map((column) => column.map((pane) => pane.sessionId))).toEqual([['a']])
    expect(activeTab(toggleMaximize(maxed, first))?.max).toBeNull()
    const single = withPanes('a').layout
    expect(toggleMaximize(single, activeTab(single)?.focus ?? '')).toBe(single)
  })

  it('stacks panes in one column when narrow', () => {
    const { layout, makeId } = withPanes('a', 'b')
    const three = splitPane(layout, null, 'down', 'c', makeId)
    const tab = activeTab(three)
    if (!tab) throw new Error('no tab')
    expect(paneColumns(tab, false).map((column) => column.length)).toEqual([1, 2])
    expect(paneColumns(tab, true).map((column) => column.length)).toEqual([1, 1, 1])
  })

  it('drops panes and tabs whose sessions ended', () => {
    const makeId = ids()
    let layout = openInNewTab(EMPTY_LAYOUT, 'a', makeId)
    layout = splitPane(layout, null, 'right', 'b', makeId)
    layout = openInNewTab(layout, 'c', makeId)
    const reconciled = reconcileLayout(layout, new Set(['a']))
    expect(reconciled.tabs).toHaveLength(1)
    expect(cols(reconciled)).toEqual(['a@0'])
    expect(reconcileLayout(reconciled, new Set(['a']))).toBe(reconciled)
  })

  it('moves to the tab that took its place, not the first tab, when the showing tab’s session ends', () => {
    const makeId = ids()
    let layout: Layout = EMPTY_LAYOUT
    for (const session of ['a', 'b', 'c', 'd']) layout = openInNewTab(layout, session, makeId)
    layout = selectTab(layout, layout.tabs[2]?.id ?? '')

    const reconciled = reconcileLayout(layout, new Set(['a', 'b', 'd']))

    expect(reconciled.tabs.map((tab) => tab.panes[0]?.sessionId)).toEqual(['a', 'b', 'd'])
    expect(activeTab(reconciled)?.panes[0]?.sessionId).toBe('d')
  })

  it('falls back to the tab before when the last tab’s session ends', () => {
    const makeId = ids()
    let layout: Layout = EMPTY_LAYOUT
    for (const session of ['a', 'b', 'c']) layout = openInNewTab(layout, session, makeId)

    const reconciled = reconcileLayout(layout, new Set(['a', 'b']))

    expect(activeTab(reconciled)?.panes[0]?.sessionId).toBe('b')
  })

  it('prefers a tab of the same project over a nearer one of another', () => {
    const makeId = ids()
    let layout: Layout = EMPTY_LAYOUT
    for (const session of ['a1', 'b1', 'a2']) layout = openInNewTab(layout, session, makeId)
    layout = selectTab(layout, layout.tabs[1]?.id ?? '')
    const project = (tab: Tab) => (tab.panes[0]?.sessionId.startsWith('a') ? 'a' : 'b')

    const reconciled = reconcileLayout(layout, new Set(['a1', 'a2']), project)

    expect(activeTab(reconciled)?.panes[0]?.sessionId).toBe('a2')
  })

  it('moves focus to the pane before the one whose session ended', () => {
    const { layout } = withPanes('a', 'b', 'c')
    const tab = activeTab(layout)
    const middle = tab?.panes[1]?.id ?? ''
    const focused = focusPane(layout, middle)

    const reconciled = reconcileLayout(focused, new Set(['a', 'c']))

    expect(activeTab(reconciled)?.focus).toBe(tab?.panes[0]?.id)
  })

  it('puts the focused pane on screen when a maximized tab is focused elsewhere', () => {
    const { layout } = withPanes('a', 'b')
    const first = activeTab(layout)?.panes[0]?.id ?? ''
    const maximized = toggleMaximize(layout, first)

    const viaSidebar = showSession(maximized, 'b', ids())
    const second = activeTab(viaSidebar)?.panes[1]?.id
    expect(activeTab(viaSidebar)?.max).toBe(second)
    expect(activeTab(viaSidebar)?.focus).toBe(second)

    const viaFocus = focusPane(maximized, second ?? '')
    expect(activeTab(viaFocus)?.max).toBe(second)
  })

  it('leaves max alone when the tab is not maximized', () => {
    const { layout } = withPanes('a', 'b')
    const second = activeTab(layout)?.panes[1]?.id ?? ''
    expect(activeTab(focusPane(layout, second))?.max).toBeNull()
  })

  it('reads back a stored layout and ignores anything malformed', () => {
    const { layout } = withPanes('a', 'b')
    expect(parseLayout(JSON.parse(JSON.stringify(layout)))).toEqual(layout)
    expect(parseLayout({ tabs: [{ id: 't', panes: [{ id: 1 }] }, 'x'] })).toEqual(EMPTY_LAYOUT)
    expect(parseLayout(null)).toEqual(EMPTY_LAYOUT)
  })
})

describe('column widths', () => {
  it('keeps dragged widths until the columns change, then goes back to equal', () => {
    const { layout } = withPanes('a', 'b')
    const tabId = layout.activeTabId ?? ''
    const sized = setColumnWidths(layout, tabId, [2, 1])
    expect(activeTab(sized)?.widths).toEqual([2, 1])
    expect(activeTab(setColumnWidths(sized, tabId, null))?.widths).toBeUndefined()

    expect(activeTab(removeSessions(sized, new Set(['b'])))?.widths).toBeUndefined()
    expect(activeTab(reconcileLayout(sized, new Set(['a'])))?.widths).toBeUndefined()
  })

  it('survives a reload only when it matches the columns', () => {
    const { layout } = withPanes('a', 'b')
    const sized = setColumnWidths(layout, layout.activeTabId ?? '', [3, 1])
    expect(activeTab(parseLayout(JSON.parse(JSON.stringify(sized))))?.widths).toEqual([3, 1])
    const bad = JSON.parse(JSON.stringify(sized)) as { tabs: Array<{ widths: unknown }> }
    if (bad.tabs[0]) bad.tabs[0].widths = [1, -2]
    expect(activeTab(parseLayout(bad))?.widths).toBeUndefined()
  })
})

describe('broadcast', () => {
  it('turns on and off for one tab and is remembered', () => {
    const { layout } = withPanes('a', 'b')
    const on = toggleBroadcast(layout, layout.activeTabId ?? '')
    expect(activeTab(on)?.broadcast).toBe(true)
    expect(activeTab(parseLayout(JSON.parse(JSON.stringify(on))))?.broadcast).toBe(true)
    expect(activeTab(toggleBroadcast(on, layout.activeTabId ?? ''))?.broadcast).toBe(false)
  })
})

describe('dropping a session on a pane', () => {
  it('swaps with the pane when the session is already in this tab', () => {
    const { layout } = withPanes('a', 'b')
    const first = activeTab(layout)?.panes[0]?.id ?? ''
    expect(cols(dropSession(layout, first, 'b'))).toEqual(['b@0', 'a@1'])
  })

  it('moves a session from another tab here, and the pane it left takes this pane\'s session', () => {
    const makeId = ids()
    let layout = openInNewTab(EMPTY_LAYOUT, 'a', makeId)
    layout = openInNewTab(layout, 'b', makeId)
    const firstTab = layout.tabs[0]
    layout = selectTab(layout, firstTab?.id ?? '')
    const result = dropSession(layout, firstTab?.panes[0]?.id ?? '', 'b')
    expect(result.tabs.map((tab) => tab.panes.map((pane) => pane.sessionId))).toEqual([['b'], ['a']])
    expect(result.activeTabId).toBe(firstTab?.id)
  })

  it('replaces the pane\'s session when the session is shown nowhere', () => {
    const { layout } = withPanes('a', 'b')
    const first = activeTab(layout)?.panes[0]?.id ?? ''
    expect(cols(dropSession(layout, first, 'z'))).toEqual(['z@0', 'b@1'])
  })
})
