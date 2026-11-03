/**
 * Tabs hold layouts; panes hold sessions. A tab is one to four panes in columns. Closing a pane
 * or a tab only detaches: the session keeps running in the daemon and stays in the sidebar.
 */
export const MAX_PANES = 4

export type PaneSlot = { id: string; sessionId: string; col: number }
/** `widths` are the columns' relative widths (by column index); absent means equal. `broadcast` sends a typed line to every idle pane. */
export type Tab = { id: string; panes: PaneSlot[]; focus: string; max: string | null; widths?: number[]; broadcast?: boolean }
export type Layout = { tabs: Tab[]; activeTabId: string | null }
export type SplitDirection = 'right' | 'down'
export type MakeId = (prefix: 'tab' | 'pane') => string

export const EMPTY_LAYOUT: Layout = { tabs: [], activeTabId: null }

export function activeTab(layout: Layout): Tab | undefined {
  return layout.tabs.find((tab) => tab.id === layout.activeTabId)
}

export function focusedSessionId(layout: Layout): string | null {
  const tab = activeTab(layout)
  return tab?.panes.find((pane) => pane.id === tab.focus)?.sessionId ?? null
}

/** Sessions on screen: the panes of the tab that is showing. */
export function visibleSessionIds(layout: Layout): Set<string> {
  return new Set(activeTab(layout)?.panes.map((pane) => pane.sessionId) ?? [])
}

function columnCount(panes: readonly PaneSlot[]): number {
  return new Set(panes.map((pane) => pane.col)).size
}

/** Dragged widths belong to one arrangement of columns, so they are dropped when the columns change. */
function withValidWidths(tab: Tab): Tab {
  if (!tab.widths || tab.widths.length === columnCount(tab.panes)) return tab
  return { id: tab.id, panes: tab.panes, focus: tab.focus, max: tab.max, ...(tab.broadcast ? { broadcast: true } : {}) }
}

function replaceTab(layout: Layout, next: Tab): Layout {
  const valid = withValidWidths(next)
  return { ...layout, tabs: layout.tabs.map((tab) => (tab.id === valid.id ? valid : tab)) }
}

export function openInNewTab(layout: Layout, sessionId: string, makeId: MakeId): Layout {
  const pane: PaneSlot = { id: makeId('pane'), sessionId, col: 0 }
  const tab: Tab = { id: makeId('tab'), panes: [pane], focus: pane.id, max: null }
  return { tabs: [...layout.tabs, tab], activeTabId: tab.id }
}

/** Split right adds a column after the pane; split down adds to the pane's column. */
export function splitPane(layout: Layout, paneId: string | null, direction: SplitDirection, sessionId: string, makeId: MakeId): Layout {
  const tab = activeTab(layout)
  if (!tab || tab.panes.length >= MAX_PANES) return layout
  const index = tab.panes.findIndex((pane) => pane.id === (paneId ?? tab.focus))
  const current = tab.panes[index]
  if (!current) return layout
  const col = direction === 'right' ? current.col + 1 : current.col
  const shifted = direction === 'right' ? tab.panes.map((pane) => (pane.col > current.col ? { ...pane, col: pane.col + 1 } : pane)) : tab.panes
  const added: PaneSlot = { id: makeId('pane'), sessionId, col }
  const panes = [...shifted.slice(0, index + 1), added, ...shifted.slice(index + 1)]
  return replaceTab(layout, { ...tab, panes, focus: added.id, max: null })
}

function compactColumns(panes: PaneSlot[]): PaneSlot[] {
  const cols = [...new Set(panes.map((pane) => pane.col))].sort((a, b) => a - b)
  return panes.map((pane) => ({ ...pane, col: cols.indexOf(pane.col) }))
}

/** Detaches one pane. The last pane of a tab stays; close the tab instead. */
export function closePane(layout: Layout, paneId: string): Layout {
  const tab = layout.tabs.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  if (!tab || tab.panes.length <= 1) return layout
  const index = tab.panes.findIndex((pane) => pane.id === paneId)
  const panes = compactColumns(tab.panes.filter((pane) => pane.id !== paneId))
  const focus = tab.focus === paneId ? (panes[Math.max(0, index - 1)]?.id ?? tab.focus) : tab.focus
  return replaceTab(layout, { ...tab, panes, focus, max: tab.max === paneId ? null : tab.max })
}

/** Which project a tab belongs to. Tabs of other projects stay in the layout but are not shown with this one. */
export type TabGroup = (tab: Tab) => string

const ONE_GROUP: TabGroup = () => ''

/**
 * Closes a tab. When it was the one showing, the next tab of the same project opens (the one after
 * it, else the one before), and only when the project has none left does another project's tab.
 */
export function closeTab(layout: Layout, tabId: string, groupOf: TabGroup = ONE_GROUP): Layout {
  const index = layout.tabs.findIndex((tab) => tab.id === tabId)
  const closing = layout.tabs[index]
  if (!closing) return layout
  const tabs = layout.tabs.filter((tab) => tab.id !== tabId)
  if (layout.activeTabId !== tabId) return { tabs, activeTabId: layout.activeTabId }
  const group = groupOf(closing)
  const sameGroup = layout.tabs.filter((tab) => tab.id !== tabId && groupOf(tab) === group)
  const after = sameGroup.find((tab) => layout.tabs.indexOf(tab) > index)
  const next = after ?? sameGroup.at(-1) ?? tabs[Math.min(index, tabs.length - 1)]
  return { tabs, activeTabId: next?.id ?? null }
}

/** What ⌘W closes: the focused pane, or the tab when it has one pane. */
export function closeFocused(layout: Layout, groupOf: TabGroup = ONE_GROUP): { layout: Layout; closed: 'pane' | 'tab' | null } {
  const tab = activeTab(layout)
  if (!tab) return { layout, closed: null }
  if (tab.panes.length > 1) return { layout: closePane(layout, tab.focus), closed: 'pane' }
  return { layout: closeTab(layout, tab.id, groupOf), closed: 'tab' }
}

export function focusPane(layout: Layout, paneId: string): Layout {
  const tab = layout.tabs.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  if (!tab) return layout
  return { ...replaceTab(layout, { ...tab, focus: paneId }), activeTabId: tab.id }
}

export function cycleFocus(layout: Layout, step: 1 | -1): Layout {
  const tab = activeTab(layout)
  if (!tab || tab.panes.length <= 1) return layout
  const index = tab.panes.findIndex((pane) => pane.id === tab.focus)
  const next = tab.panes[(index + step + tab.panes.length) % tab.panes.length]
  return next ? replaceTab(layout, { ...tab, focus: next.id, max: tab.max === null ? null : next.id }) : layout
}

export function selectTab(layout: Layout, tabId: string): Layout {
  return layout.tabs.some((tab) => tab.id === tabId) ? { ...layout, activeTabId: tabId } : layout
}

/** ⌃Tab: the next tab of the project that is showing, so it never lands in another project's layout. */
export function cycleTab(layout: Layout, step: 1 | -1, groupOf: TabGroup = ONE_GROUP): Layout {
  const current = activeTab(layout)
  if (!current) return layout
  const scope = layout.tabs.filter((tab) => groupOf(tab) === groupOf(current))
  if (scope.length <= 1) return layout
  const index = scope.findIndex((tab) => tab.id === current.id)
  const next = scope[(index + step + scope.length) % scope.length]
  return next ? { ...layout, activeTabId: next.id } : layout
}

/**
 * A sidebar click: focus the session's pane where it is shown, else open it in a tab of its own.
 * It is never loaded into a tab that already holds other sessions, so a project's layout stays put.
 */
export function showSession(layout: Layout, sessionId: string, makeId: MakeId): Layout {
  const current = activeTab(layout)
  const holder = (current?.panes.some((pane) => pane.sessionId === sessionId) ? current : undefined) ?? layout.tabs.find((tab) => tab.panes.some((pane) => pane.sessionId === sessionId))
  const shown = holder?.panes.find((pane) => pane.sessionId === sessionId)
  if (holder && shown) return { ...replaceTab(layout, { ...holder, focus: shown.id }), activeTabId: holder.id }
  return openInNewTab(layout, sessionId, makeId)
}

/** Dropping one pane's header on another exchanges their sessions; each pane keeps its place. */
export function swapSessions(layout: Layout, fromPaneId: string, toPaneId: string): Layout {
  const tab = layout.tabs.find((candidate) => candidate.panes.some((pane) => pane.id === fromPaneId))
  const from = tab?.panes.find((pane) => pane.id === fromPaneId)
  const to = tab?.panes.find((pane) => pane.id === toPaneId)
  if (!tab || !from || !to || from === to) return layout
  const panes = tab.panes.map((pane) => (pane.id === from.id ? { ...pane, sessionId: to.sessionId } : pane.id === to.id ? { ...pane, sessionId: from.sessionId } : pane))
  return replaceTab(layout, { ...tab, panes, focus: to.id })
}

/** Maximize shows one pane until restored; a lone pane has nothing to maximize over. */
export function toggleMaximize(layout: Layout, paneId: string): Layout {
  const tab = layout.tabs.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  if (!tab) return layout
  const maxed = tab.max === paneId
  if (!maxed && tab.panes.length <= 1) return layout
  return replaceTab(layout, { ...tab, max: maxed ? null : paneId, focus: paneId })
}

/** Drops panes whose session has ended, and tabs left empty. */
export function reconcileLayout(layout: Layout, liveSessionIds: ReadonlySet<string>, groupOf: TabGroup = ONE_GROUP): Layout {
  const tabs = layout.tabs.flatMap((tab) => {
    const panes = tab.panes.filter((pane) => liveSessionIds.has(pane.sessionId))
    if (panes.length === tab.panes.length) return [tab]
    if (panes.length === 0) return []
    const compacted = compactColumns(panes)
    const focus = compacted.some((pane) => pane.id === tab.focus) ? tab.focus : neighbourPane(tab, compacted)
    const max = compacted.some((pane) => pane.id === tab.max) ? tab.max : null
    return [withValidWidths({ ...tab, panes: compacted, focus, max })]
  })
  const changed = tabs.length !== layout.tabs.length || tabs.some((tab, index) => tab !== layout.tabs[index])
  if (!changed) return layout
  const activeTabId = tabs.some((tab) => tab.id === layout.activeTabId) ? layout.activeTabId : (nextActiveTab(layout, tabs, groupOf) ?? null)
  return { tabs, activeTabId }
}

/** When the focused pane's session ends, focus moves to the pane before it (as closing one does), else the first. */
function neighbourPane(tab: Tab, remaining: PaneSlot[]): string {
  const index = tab.panes.findIndex((pane) => pane.id === tab.focus)
  const before = tab.panes.slice(0, Math.max(0, index)).reverse().find((pane) => remaining.some((kept) => kept.id === pane.id))
  return before?.id ?? remaining[0]?.id ?? tab.focus
}

/**
 * When the showing tab's sessions all ended: the tab that took its place, else the one before it,
 * from the same project when there is one. It must not jump to whichever tab is first.
 */
function nextActiveTab(before: Layout, remaining: Tab[], groupOf: TabGroup): string | undefined {
  const index = before.tabs.findIndex((tab) => tab.id === before.activeTabId)
  const gone = before.tabs[index]
  if (!gone) return remaining[0]?.id
  const position = (tab: Tab) => before.tabs.findIndex((candidate) => candidate.id === tab.id)
  const pick = (list: Tab[]) => list.find((tab) => position(tab) > index) ?? list.at(-1)
  const sameProject = remaining.filter((tab) => groupOf(tab) === groupOf(gone))
  return (pick(sameProject) ?? pick(remaining))?.id
}

/** The tab's panes as the columns they render in: one column when maximized or narrow. */
export function paneColumns(tab: Tab, narrow: boolean): PaneSlot[][] {
  if (tab.max !== null) {
    const maxed = tab.panes.find((pane) => pane.id === tab.max)
    if (maxed) return [[maxed]]
  }
  if (narrow) return tab.panes.map((pane) => [pane])
  const columns = new Map<number, PaneSlot[]>()
  for (const pane of tab.panes) columns.set(pane.col, [...(columns.get(pane.col) ?? []), pane])
  return [...columns.entries()].sort(([a], [b]) => a - b).map(([, panes]) => panes)
}

/** Reads a stored layout, keeping only well-formed tabs. */
export function parseLayout(value: unknown): Layout {
  if (typeof value !== 'object' || value === null) return EMPTY_LAYOUT
  const record = value as { tabs?: unknown; activeTabId?: unknown }
  if (!Array.isArray(record.tabs)) return EMPTY_LAYOUT
  const tabs = record.tabs.flatMap((raw: unknown): Tab[] => {
    if (typeof raw !== 'object' || raw === null) return []
    const tab = raw as { id?: unknown; panes?: unknown; focus?: unknown; max?: unknown; widths?: unknown; broadcast?: unknown }
    if (typeof tab.id !== 'string' || !Array.isArray(tab.panes)) return []
    const panes = tab.panes.flatMap((pane: unknown): PaneSlot[] => {
      if (typeof pane !== 'object' || pane === null) return []
      const slot = pane as { id?: unknown; sessionId?: unknown; col?: unknown }
      return typeof slot.id === 'string' && typeof slot.sessionId === 'string' && typeof slot.col === 'number' && Number.isInteger(slot.col) && slot.col >= 0 ? [{ id: slot.id, sessionId: slot.sessionId, col: slot.col }] : []
    }).slice(0, MAX_PANES)
    const first = panes[0]
    if (!first) return []
    const focus = typeof tab.focus === 'string' && panes.some((pane) => pane.id === tab.focus) ? tab.focus : first.id
    const max = typeof tab.max === 'string' && panes.some((pane) => pane.id === tab.max) ? tab.max : null
    const compacted = compactColumns(panes)
    const widths = Array.isArray(tab.widths) && tab.widths.length === columnCount(compacted) && tab.widths.every((width) => typeof width === 'number' && Number.isFinite(width) && width > 0) ? (tab.widths as number[]) : null
    return [{ id: tab.id, panes: compacted, focus, max, ...(widths ? { widths } : {}), ...(tab.broadcast === true ? { broadcast: true } : {}) }]
  })
  const activeTabId = typeof record.activeTabId === 'string' && tabs.some((tab) => tab.id === record.activeTabId) ? record.activeTabId : (tabs[0]?.id ?? null)
  return { tabs, activeTabId }
}

/** The narrowest a column may be dragged, in px. */
export const MIN_COLUMN_PX = 180

/** Gives the columns of a tab their relative widths; `null` goes back to equal. */
export function setColumnWidths(layout: Layout, tabId: string, widths: number[] | null): Layout {
  const tab = layout.tabs.find((candidate) => candidate.id === tabId)
  if (!tab) return layout
  const base: Tab = { id: tab.id, panes: tab.panes, focus: tab.focus, max: tab.max, ...(tab.broadcast ? { broadcast: true } : {}) }
  return replaceTab(layout, widths ? { ...base, widths } : base)
}

export function toggleBroadcast(layout: Layout, tabId: string): Layout {
  const tab = layout.tabs.find((candidate) => candidate.id === tabId)
  return tab ? replaceTab(layout, { ...tab, broadcast: !tab.broadcast }) : layout
}

/**
 * A session dragged from the sidebar onto a pane. In this tab it swaps places with the pane; shown
 * in another tab it moves here (that pane gets this pane's session); otherwise it replaces this
 * pane's session. The session that leaves keeps running, detached.
 */
export function dropSession(layout: Layout, paneId: string, sessionId: string): Layout {
  const tab = layout.tabs.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  const target = tab?.panes.find((pane) => pane.id === paneId)
  if (!tab || !target || target.sessionId === sessionId) return layout
  const here = tab.panes.find((pane) => pane.sessionId === sessionId)
  if (here) return swapSessions(layout, here.id, target.id)
  const tabs = layout.tabs.map((candidate) => {
    if (candidate.id === tab.id) return { ...candidate, panes: candidate.panes.map((pane) => (pane.id === paneId ? { ...pane, sessionId } : pane)), focus: paneId }
    // The session is moving here, so the pane it left takes this pane's old session.
    return { ...candidate, panes: candidate.panes.map((pane) => (pane.sessionId === sessionId ? { ...pane, sessionId: target.sessionId } : pane)) }
  })
  return { ...layout, tabs, activeTabId: tab.id }
}
