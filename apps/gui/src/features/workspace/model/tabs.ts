import type { Layout } from './layout'
import type { SessionView } from './session'

/**
 * The tabs of the selected fleet: one per ship. A ship that a split put beside another's pane is
 * part of that tab (its pane count says so), not a tab of its own. The tabs sit in the ships'
 * creation order, so opening, splitting or closing never moves another tab.
 */
export type Tab = {
  /** The ship the tab is named after: the first pane's session. */
  sessionId: string
  /** The layout arrangement behind it; null for a ship nobody has opened yet. */
  arrangementId: string | null
  /** Every ship in the tab's panes. */
  sessionIds: string[]
  paneCount: number
  active: boolean
}

/** What a tab needs of a ship: its id and where it sits in creation order. */
export type TabShip = Pick<SessionView, 'id' | 'pane' | 'machine'>

export function tabsOf(layout: Layout, ships: readonly TabShip[]): Tab[] {
  const byId = new Map(ships.map((ship) => [ship.id, ship]))
  const tabs: Array<Tab & { head: TabShip }> = []
  for (const arrangement of layout.arrangements) {
    const head = arrangement.panes[0] ? byId.get(arrangement.panes[0].sessionId) : undefined
    if (!head) continue
    const sessionIds = arrangement.panes.map((pane) => pane.sessionId)
    tabs.push({ head, sessionId: head.id, arrangementId: arrangement.id, sessionIds, paneCount: arrangement.panes.length, active: arrangement.id === layout.activeId })
  }
  const placed = new Set(layout.arrangements.flatMap((arrangement) => arrangement.panes.map((pane) => pane.sessionId)))
  for (const ship of ships) {
    if (!placed.has(ship.id)) tabs.push({ head: ship, sessionId: ship.id, arrangementId: null, sessionIds: [ship.id], paneCount: 1, active: false })
  }
  const order = new Map([...tabs.map((tab) => tab.head)].sort((a, b) => a.pane - b.pane || a.machine - b.machine).map((head, index) => [head.id, index]))
  return tabs
    .sort((a, b) => (order.get(a.head.id) ?? 0) - (order.get(b.head.id) ?? 0))
    .map((tab) => ({ sessionId: tab.sessionId, arrangementId: tab.arrangementId, sessionIds: tab.sessionIds, paneCount: tab.paneCount, active: tab.active }))
}
