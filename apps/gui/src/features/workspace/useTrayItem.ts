import { useEffect, useMemo } from 'react'
import { listenTrayOpenSession, setTraySessions } from '@/api/tray'
import { useLatest } from '@/lib/useLatest'
import { trayKey, traySessionsOf } from './model/traySessions'
import type { SessionView } from './model/session'

/**
 * Keeps the tray (or menu-bar) menu showing the sessions that need you and the ones that finished, and shows the
 * session chosen there. `sessions` are the ones the Inbox lists, so one you cleared there is not in the menu either.
 */
export function useTrayItem(sessions: readonly SessionView[], showSessionById: (sessionId: string) => void): void {
  const tray = useMemo(() => traySessionsOf(sessions), [sessions])
  const key = trayKey(tray)
  const tell = useLatest(() => setTraySessions(tray.needs, tray.finished))
  useEffect(() => {
    tell().catch((error: unknown) => { console.warn('Could not update the tray menu', error) })
  }, [key, tell])

  const show = useLatest(showSessionById)
  useEffect(() => {
    let gone = false
    let stop: (() => void) | null = null
    listenTrayOpenSession(show)
      .then((off) => { if (gone) off(); else stop = off })
      .catch((error: unknown) => { console.warn('Could not listen to the tray menu', error) })
    return () => { gone = true; stop?.() }
  }, [show])
}
