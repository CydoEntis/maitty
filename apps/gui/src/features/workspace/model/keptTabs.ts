import { activeArrangement, type Arrangement, type Layout } from './layout'

/**
 * How many tabs stay mounted, out of sight, behind the one showing, and how many panes they may hold between them.
 * Each pane keeps a terminal and its GPU context, and a browser allows about sixteen contexts in all: the tab
 * showing holds up to four.
 */
export const KEPT_TABS = 2
export const KEPT_PANES = 8

/** The tabs shown most recently, newest first, with the one showing now at the front. The same list when it already is. */
export function rememberShown(recent: readonly string[], activeId: string | null): readonly string[] {
  if (activeId === null || recent[0] === activeId) return recent
  return [activeId, ...recent.filter((id) => id !== activeId)].slice(0, KEPT_TABS + 1)
}

/**
 * The tabs to keep mounted out of sight, so going back to one does not build its terminals again: the ones shown
 * last, not the one showing. A session is mounted in one place only, so a tab that shares a session with the tab
 * showing, or with one already kept, is left out.
 */
export function keptArrangements(layout: Layout, recent: readonly string[]): Arrangement[] {
  const mounted = new Set(activeArrangement(layout)?.panes.map((pane) => pane.sessionId) ?? [])
  const kept: Arrangement[] = []
  let panes = 0
  for (const id of recent) {
    if (id === layout.activeId) continue
    const tab = layout.arrangements.find((candidate) => candidate.id === id)
    if (!tab || tab.panes.some((pane) => mounted.has(pane.sessionId))) continue
    if (kept.length >= KEPT_TABS || panes + tab.panes.length > KEPT_PANES) break
    kept.push(tab)
    panes += tab.panes.length
    for (const pane of tab.panes) mounted.add(pane.sessionId)
  }
  return kept
}
