import type { Layout } from './layout'
import type { SessionView } from './session'

/**
 * What the app remembers of sessions it has seen: every listed one, plus the ones a pane still shows
 * that the daemon has stopped listing, so an ended pane keeps its name and folder.
 */
export function rememberSessions(
  before: ReadonlyMap<string, SessionView>,
  listed: readonly SessionView[],
  layout: Layout,
): ReadonlyMap<string, SessionView> {
  const shown = new Set(layout.tabs.flatMap((tab) => tab.panes.map((pane) => pane.sessionId)))
  const next = new Map<string, SessionView>()
  for (const [id, session] of before) if (shown.has(id)) next.set(id, session)
  for (const session of listed) next.set(session.id, session)
  return next
}

/** The listed sessions, and the remembered ones that are no longer listed, marked ended and idle. */
export function withEnded(
  listed: ReadonlyMap<string, SessionView>,
  remembered: ReadonlyMap<string, SessionView>,
): ReadonlyMap<string, SessionView> {
  const all = new Map(listed)
  for (const [id, session] of remembered) if (!all.has(id)) all.set(id, { ...session, status: 'idle', ended: true })
  return all
}
