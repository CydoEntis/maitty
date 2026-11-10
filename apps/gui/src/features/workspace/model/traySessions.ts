import type { TraySession } from '@/api/tray'
import { displayName, type SessionView } from './session'

/** A session's line in the menu: its agent, then its name. A name alone is often only a folder, which says little away from the sidebar. */
function trayLabel(session: SessionView): string {
  const name = displayName(session)
  return session.process && session.process !== name ? `${session.process}: ${name}` : name
}

/** What the tray menu lists: the sessions that need you, then the ones that finished, each by the name the sidebar shows. */
export function traySessionsOf(sessions: readonly SessionView[]): { needs: TraySession[]; finished: TraySession[] } {
  const named = (status: SessionView['status']) =>
    sessions.filter((session) => session.status === status).map((session) => ({ id: session.id, label: trayLabel(session) }))
  return { needs: named('needs'), finished: named('done') }
}

/** One line that differs exactly when the menu would: the host is told only when it does. */
export function trayKey(tray: { needs: readonly TraySession[]; finished: readonly TraySession[] }): string {
  return JSON.stringify([tray.needs, tray.finished])
}
