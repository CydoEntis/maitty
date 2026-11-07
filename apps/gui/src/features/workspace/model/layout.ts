/**
 * The window's split layouts, kept in this app only (sessions are the daemon's). An arrangement is
 * what the workspace shows for a session: one to four panes in columns, each pane naming a session
 * by id. The sidebar picks the session; the arrangement it sits in is what you see. A pane goes only
 * when you close it (`removeSessions`): one whose session stops being listed stays, as an ended pane.
 */
export const MAX_PANES = 4

export type PaneSlot = { id: string; sessionId: string; col: number }
/** `widths` are the columns' relative widths (by column index); absent means equal. `broadcast` sends a typed line to every idle pane. */
export type Arrangement = { id: string; panes: PaneSlot[]; focus: string; max: string | null; widths?: number[]; broadcast?: boolean }
export type Layout = { arrangements: Arrangement[]; activeId: string | null }
export type SplitDirection = 'right' | 'down' | 'left' | 'up'
export type MakeId = (prefix: 'tab' | 'pane') => string

export const EMPTY_LAYOUT: Layout = { arrangements: [], activeId: null }

export function activeArrangement(layout: Layout): Arrangement | undefined {
  return layout.arrangements.find((tab) => tab.id === layout.activeId)
}

export function focusedSessionId(layout: Layout): string | null {
  const tab = activeArrangement(layout)
  return tab?.panes.find((pane) => pane.id === tab.focus)?.sessionId ?? null
}

/** Sessions on screen: the panes of the tab that is showing. */
export function visibleSessionIds(layout: Layout): Set<string> {
  return new Set(activeArrangement(layout)?.panes.map((pane) => pane.sessionId) ?? [])
}

function columnCount(panes: readonly PaneSlot[]): number {
  return new Set(panes.map((pane) => pane.col)).size
}

/** Dragged widths belong to one arrangement of columns, so they are dropped when the columns change. */
function withValidWidths(tab: Arrangement): Arrangement {
  if (!tab.widths || tab.widths.length === columnCount(tab.panes)) return tab
  return { id: tab.id, panes: tab.panes, focus: tab.focus, max: tab.max, ...(tab.broadcast ? { broadcast: true } : {}) }
}

function replaceArrangement(layout: Layout, next: Arrangement): Layout {
  const valid = withValidWidths(next)
  return { ...layout, arrangements: layout.arrangements.map((tab) => (tab.id === valid.id ? valid : tab)) }
}

export function openArrangement(layout: Layout, sessionId: string, makeId: MakeId): Layout {
  const pane: PaneSlot = { id: makeId('pane'), sessionId, col: 0 }
  const tab: Arrangement = { id: makeId('tab'), panes: [pane], focus: pane.id, max: null }
  return { arrangements: [...layout.arrangements, tab], activeId: tab.id }
}

/**
 * Right adds a column after the pane and left one before it; down adds a pane below in the pane's column
 * and up one above it. The new pane takes the focus.
 */
export function splitPane(layout: Layout, paneId: string | null, direction: SplitDirection, sessionId: string, makeId: MakeId): Layout {
  const tab = activeArrangement(layout)
  if (!tab || tab.panes.length >= MAX_PANES) return layout
  const index = tab.panes.findIndex((pane) => pane.id === (paneId ?? tab.focus))
  const current = tab.panes[index]
  if (!current) return layout
  const before = direction === 'left' || direction === 'up'
  // A new column pushes every column from its place on to the right.
  const columnAt = direction === 'right' ? current.col + 1 : direction === 'left' ? current.col : null
  const shifted = columnAt === null ? tab.panes : tab.panes.map((pane) => (pane.col >= columnAt ? { ...pane, col: pane.col + 1 } : pane))
  const added: PaneSlot = { id: makeId('pane'), sessionId, col: columnAt ?? current.col }
  const at = before ? index : index + 1
  const panes = [...shifted.slice(0, at), added, ...shifted.slice(at)]
  return replaceArrangement(layout, { ...tab, panes, focus: added.id, max: null })
}

function compactColumns(panes: PaneSlot[]): PaneSlot[] {
  const cols = [...new Set(panes.map((pane) => pane.col))].sort((a, b) => a - b)
  return panes.map((pane) => ({ ...pane, col: cols.indexOf(pane.col) }))
}

export function focusPane(layout: Layout, paneId: string): Layout {
  const tab = layout.arrangements.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  if (!tab) return layout
  // In a maximized tab the focused pane is the one on screen, so focus and maximize move together.
  return { ...replaceArrangement(layout, { ...tab, focus: paneId, max: tab.max === null ? null : paneId }), activeId: tab.id }
}

export function cycleFocus(layout: Layout, step: 1 | -1): Layout {
  const tab = activeArrangement(layout)
  if (!tab || tab.panes.length <= 1) return layout
  const index = tab.panes.findIndex((pane) => pane.id === tab.focus)
  const next = tab.panes[(index + step + tab.panes.length) % tab.panes.length]
  return next ? replaceArrangement(layout, { ...tab, focus: next.id, max: tab.max === null ? null : next.id }) : layout
}

/**
 * A sidebar click: focus the session's pane where it is shown, else open it in a tab of its own.
 * It is never loaded into a tab that already holds other sessions, so a project's layout stays put.
 */
export function showSession(layout: Layout, sessionId: string, makeId: MakeId): Layout {
  const current = activeArrangement(layout)
  const holder = (current?.panes.some((pane) => pane.sessionId === sessionId) ? current : undefined) ?? layout.arrangements.find((tab) => tab.panes.some((pane) => pane.sessionId === sessionId))
  const shown = holder?.panes.find((pane) => pane.sessionId === sessionId)
  if (holder && shown) return { ...replaceArrangement(layout, { ...holder, focus: shown.id, max: holder.max === null ? null : shown.id }), activeId: holder.id }
  return openArrangement(layout, sessionId, makeId)
}

/** Dropping one pane's header on another exchanges their sessions; each pane keeps its place. */
export function swapSessions(layout: Layout, fromPaneId: string, toPaneId: string): Layout {
  const tab = layout.arrangements.find((candidate) => candidate.panes.some((pane) => pane.id === fromPaneId))
  const from = tab?.panes.find((pane) => pane.id === fromPaneId)
  const to = tab?.panes.find((pane) => pane.id === toPaneId)
  if (!tab || !from || !to || from === to) return layout
  const panes = tab.panes.map((pane) => (pane.id === from.id ? { ...pane, sessionId: to.sessionId } : pane.id === to.id ? { ...pane, sessionId: from.sessionId } : pane))
  return replaceArrangement(layout, { ...tab, panes, focus: to.id })
}

/** Maximize shows one pane until restored; a lone pane has nothing to maximize over. */
export function toggleMaximize(layout: Layout, paneId: string): Layout {
  const tab = layout.arrangements.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  if (!tab) return layout
  const maxed = tab.max === paneId
  if (!maxed && tab.panes.length <= 1) return layout
  return replaceArrangement(layout, { ...tab, max: maxed ? null : paneId, focus: paneId })
}

/** Drops panes whose session has ended, and tabs left empty. */
export function reconcileLayout(layout: Layout, liveSessionIds: ReadonlySet<string>): Layout {
  const tabs = layout.arrangements.flatMap((tab) => {
    const panes = tab.panes.filter((pane) => liveSessionIds.has(pane.sessionId))
    if (panes.length === tab.panes.length) return [tab]
    if (panes.length === 0) return []
    const compacted = compactColumns(panes)
    const focus = compacted.some((pane) => pane.id === tab.focus) ? tab.focus : neighbourPane(tab, compacted)
    const max = compacted.some((pane) => pane.id === tab.max) ? tab.max : null
    return [withValidWidths({ ...tab, panes: compacted, focus, max })]
  })
  const changed = tabs.length !== layout.arrangements.length || tabs.some((tab, index) => tab !== layout.arrangements[index])
  if (!changed) return layout
  const activeId = tabs.some((tab) => tab.id === layout.activeId) ? layout.activeId : (nextActiveArrangement(layout, tabs) ?? null)
  return { arrangements: tabs, activeId }
}

/**
 * Takes out the panes of these sessions and nothing else. A session that merely stopped being
 * listed keeps its pane (it shows as ended), so only what is closed on purpose goes.
 */
export function removeSessions(layout: Layout, gone: ReadonlySet<string>): Layout {
  const kept = new Set(layout.arrangements.flatMap((tab) => tab.panes.map((pane) => pane.sessionId)).filter((id) => !gone.has(id)))
  return reconcileLayout(layout, kept)
}

/** When the focused pane's session ends, focus moves to the pane before it (as closing one does), else the first. */
function neighbourPane(tab: Arrangement, remaining: PaneSlot[]): string {
  const index = tab.panes.findIndex((pane) => pane.id === tab.focus)
  const before = tab.panes.slice(0, Math.max(0, index)).reverse().find((pane) => remaining.some((kept) => kept.id === pane.id))
  return before?.id ?? remaining[0]?.id ?? tab.focus
}

/** When the showing arrangement's sessions all ended: the one that took its place, else the one before it. */
function nextActiveArrangement(before: Layout, remaining: Arrangement[]): string | undefined {
  const index = before.arrangements.findIndex((tab) => tab.id === before.activeId)
  const position = (tab: Arrangement) => before.arrangements.findIndex((candidate) => candidate.id === tab.id)
  return (remaining.find((tab) => position(tab) > index) ?? remaining.at(-1))?.id
}

/** The tab's panes as the columns they render in: one column when maximized or narrow. */
export function paneColumns(tab: Arrangement, narrow: boolean): PaneSlot[][] {
  if (tab.max !== null) {
    const maxed = tab.panes.find((pane) => pane.id === tab.max)
    if (maxed) return [[maxed]]
  }
  if (narrow) return tab.panes.map((pane) => [pane])
  const columns = new Map<number, PaneSlot[]>()
  for (const pane of tab.panes) columns.set(pane.col, [...(columns.get(pane.col) ?? []), pane])
  return [...columns.entries()].sort(([a], [b]) => a - b).map(([, panes]) => panes)
}

/** Reads a stored layout, keeping only well-formed arrangements. */
export function parseLayout(value: unknown): Layout {
  if (typeof value !== 'object' || value === null) return EMPTY_LAYOUT
  const record = value as { arrangements?: unknown; activeId?: unknown }
  if (!Array.isArray(record.arrangements)) return EMPTY_LAYOUT
  const arrangements = record.arrangements.flatMap((raw: unknown): Arrangement[] => {
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
  const activeId = typeof record.activeId === 'string' && arrangements.some((tab) => tab.id === record.activeId) ? record.activeId : (arrangements[0]?.id ?? null)
  return { arrangements, activeId }
}

/** The narrowest a column may be dragged, in px. */
export const MIN_COLUMN_PX = 180

/** Gives the columns of a tab their relative widths; `null` goes back to equal. */
export function setColumnWidths(layout: Layout, tabId: string, widths: number[] | null): Layout {
  const tab = layout.arrangements.find((candidate) => candidate.id === tabId)
  if (!tab) return layout
  const base: Arrangement = { id: tab.id, panes: tab.panes, focus: tab.focus, max: tab.max, ...(tab.broadcast ? { broadcast: true } : {}) }
  return replaceArrangement(layout, widths ? { ...base, widths } : base)
}

export function toggleBroadcast(layout: Layout, tabId: string): Layout {
  const tab = layout.arrangements.find((candidate) => candidate.id === tabId)
  return tab ? replaceArrangement(layout, { ...tab, broadcast: !tab.broadcast }) : layout
}

/**
 * A session dragged from the sidebar onto a pane. In this tab it swaps places with the pane; shown
 * in another tab it moves here (that pane gets this pane's session); otherwise it replaces this
 * pane's session. The session that leaves keeps running, detached.
 */
export function dropSession(layout: Layout, paneId: string, sessionId: string): Layout {
  const tab = layout.arrangements.find((candidate) => candidate.panes.some((pane) => pane.id === paneId))
  const target = tab?.panes.find((pane) => pane.id === paneId)
  if (!tab || !target || target.sessionId === sessionId) return layout
  const here = tab.panes.find((pane) => pane.sessionId === sessionId)
  if (here) return swapSessions(layout, here.id, target.id)
  const arrangements = layout.arrangements.map((candidate) => {
    if (candidate.id === tab.id) return { ...candidate, panes: candidate.panes.map((pane) => (pane.id === paneId ? { ...pane, sessionId } : pane)), focus: paneId }
    // The session is moving here, so the pane it left takes this pane's old session.
    return { ...candidate, panes: candidate.panes.map((pane) => (pane.sessionId === sessionId ? { ...pane, sessionId: target.sessionId } : pane)) }
  })
  return { ...layout, arrangements, activeId: tab.id }
}
