import { displayName, type SessionStatus, type SessionView } from '@/features/workspace/model/session'

export type NotifySwitches = { needs: boolean; finished: boolean; error: boolean }
export type Notice = { sessionId: string; title: string; body: string }

const TITLES: Partial<Record<SessionStatus, (name: string) => string>> = {
  needs: (name) => `${name} needs you`,
  done: (name) => `${name} finished`,
  error: (name) => `${name} exited with an error`,
}

const WANTED: Record<'needs' | 'done' | 'error', keyof NotifySwitches> = { needs: 'needs', done: 'finished', error: 'error' }

/**
 * Which sessions just changed to a state worth a notification. `before` is what the last look saw
 * (null before the first one, so opening the app never notifies for what already was). A session
 * you are looking at is left out, and so is one coming back from "unknown" after a disconnect.
 * The text names the session and where it is, never what it was asked or is asking: a
 * notification can show on a locked screen.
 */
export function noticesFor(before: ReadonlyMap<string, SessionStatus> | null, sessions: readonly SessionView[], switches: NotifySwitches, attending: string | null): Notice[] {
  if (before === null) return []
  const notices: Notice[] = []
  for (const session of sessions) {
    const previous = before.get(session.id)
    if (previous === undefined || previous === 'unknown' || previous === session.status) continue
    if (session.status !== 'needs' && session.status !== 'done' && session.status !== 'error') continue
    if (!switches[WANTED[session.status]] || session.id === attending) continue
    const title = TITLES[session.status]?.(displayName(session))
    if (title) notices.push({ sessionId: session.id, title, body: `${session.process} in ${session.cwdLabel}` })
  }
  return notices
}
