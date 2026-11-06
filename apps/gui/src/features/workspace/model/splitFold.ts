import { SHELL_PROCESS, type SessionView } from './session'

/**
 * Splits of a tab that are only quiet shells fold into the row of the tab's first pane, which says how
 * many ("+2 panes"). A split that runs an agent, is working or needs you stays a row of its own, and so
 * does every session that is not a split. `tabPanes` maps each session of a tab with several panes to
 * that tab's sessions in pane order. A split whose first pane is in another project stays, so nothing is
 * folded into a row that is not on this list.
 */
export function foldSplits(
  sessions: readonly SessionView[],
  tabPanes: ReadonlyMap<string, readonly SessionView[]>,
): { visible: SessionView[]; folded: ReadonlyMap<string, number>; leadOf: ReadonlyMap<string, string> } {
  const listed = new Set(sessions.map((session) => session.id))
  const folded = new Map<string, number>()
  const leadOf = new Map<string, string>()
  const visible = sessions.filter((session) => {
    const lead = tabPanes.get(session.id)?.[0]
    const quietShell = session.process === SHELL_PROCESS && session.status === 'idle'
    if (lead === undefined || lead.id === session.id || !quietShell || !listed.has(lead.id)) return true
    folded.set(lead.id, (folded.get(lead.id) ?? 0) + 1)
    leadOf.set(session.id, lead.id)
    return false
  })
  return { visible, folded, leadOf }
}

/** What a row says about the splits folded into it: "+1 pane", "+3 panes". */
export function splitNote(count: number): string {
  return `+${String(count)} ${count === 1 ? 'pane' : 'panes'}`
}
