import { useCallback, useEffect, useRef } from 'react'
import { isDesktop } from '@/api/daemon'
import { LOCAL_MACHINE, listenAttention, type Attention } from '@/api/machines'
import { enableNotificationSound, playNotificationSound } from '@/data/notificationSound'
import { sendNativeNotice } from '@/features/notifications/native'
import { sessionNoticeFor } from '@/features/notifications/noticeForSession'
import { dismissSessionNotice, showSessionNotice } from '@/features/notifications/notify'
import { noticeForAttention, noticesFor, type Notice } from '@/features/notifications/transitions'
import { isChatSession } from './model/chatSession'
import type { RemoteMachine } from './model/machines'
import type { SessionView } from './model/session'

/** How many messages this window holds for the session's agent, to be typed in as its turns end. */
const queuedFor = (session: SessionView): number => session.queued.length

/** A notification's click brings the window forward; returning later than this is not that click. */
const NOTIFICATION_CLICK_WINDOW_MS = 15_000

type SessionAlerts = {
  sessions: readonly SessionView[]
  sessionMap: ReadonlyMap<string, SessionView>
  machines: ReadonlyMap<number, RemoteMachine>
  /** The folder each session sits in. */
  projectOf: ReadonlyMap<string, { name: string; key: string }>
  online: boolean
  focusedId: string | null
  preferences: { notifyNeedsYou: boolean; notifyFinished: boolean; notifyError: boolean; notificationSound: boolean }
  showSessionById: (sessionId: string) => void
  answerSession: (sessionId: string, answer: 'y' | 'a' | 'n') => void
}

/**
 * Tells you about a session that started needing you, finished or failed while you were not looking at it: a card
 * in the window, the system's notice away from it, and a sound when the daemon says so.
 */
export function useSessionAlerts({ sessions, sessionMap, machines, projectOf, online, focusedId, preferences, showSessionById, answerSession }: SessionAlerts): void {
  const { notifyNeedsYou, notifyFinished, notifyError, notificationSound } = preferences
  const lastStatuses = useRef<ReadonlyMap<string, SessionView['status']> | null>(null)
  const lastNotified = useRef<{ sessionId: string; at: number } | null>(null)

  // Clicking a notification brings the window forward; that focus shows the session it was about.
  useEffect(() => {
    const onFocus = () => {
      const notified = lastNotified.current
      lastNotified.current = null
      if (notified && Date.now() - notified.at < NOTIFICATION_CLICK_WINDOW_MS) showSessionById(notified.sessionId)
    }
    window.addEventListener('focus', onFocus)
    return () => { window.removeEventListener('focus', onFocus) }
  }, [showSessionById])

  // In the window but on another pane: a toast you can click. Away from the window: the system's.
  const raiseNotice = useCallback((notice: Notice) => {
    // Never while reconnecting, with the service down or a restart needed: the status itself is the message, and a card could not be answered.
    if (!online) return
    if (document.hasFocus()) {
      const target = sessionMap.get(notice.sessionId)
      const card = target && sessionNoticeFor(target, projectOf.get(target.id), {
        open: () => { showSessionById(notice.sessionId) },
        // A chat session's approval is answered in its chat, by the request's id, never by typing a letter.
        allow: !isChatSession(target.conversation) ? () => { answerSession(notice.sessionId, 'y') } : null,
      })
      if (card) showSessionNotice(notice.sessionId, card)
      return
    }
    lastNotified.current = { sessionId: notice.sessionId, at: Date.now() }
    sendNativeNotice(notice.title, notice.body).catch((error: unknown) => { console.warn('Could not show a system notification', error) })
  }, [sessionMap, projectOf, showSessionById, online, answerSession])

  // A session that just started needing you, finished or failed, and that you are not looking at. A chat session's
  // alert is raised here. A terminal session's sound and system notice come from the daemon (below), which decides
  // them once per event; but it tells only one window, and only sometimes, so the window also raises the card
  // itself for a session it can see change. The card is keyed by its session, so the two never stack.
  useEffect(() => {
    const previousStatuses = lastStatuses.current
    // What another machine's agents do raises no system notice unless its entry says notices = true.
    const mine = sessions.filter((item) => item.machine === LOCAL_MACHINE || machines.get(item.machine)?.state.notices === true)
    const notices = online
      ? noticesFor(lastStatuses.current, mine, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError }, document.hasFocus() ? focusedId : null, queuedFor)
      : []
    lastStatuses.current = new Map(mine.map((item) => [item.id, item.status]))
    // A card that said "Needs you" goes when the session stops needing you.
    for (const item of sessions) if (item.status !== 'needs' && previousStatuses?.get(item.id) === 'needs') dismissSessionNotice(item.id)
    for (const notice of notices) {
      const target = mine.find((item) => item.id === notice.sessionId)
      // Away from the window, a terminal session's system notice is the daemon's; raising it here too would say it twice.
      if (target !== undefined && !isChatSession(target.conversation) && !document.hasFocus()) continue
      raiseNotice(notice)
    }
  }, [sessions, online, focusedId, notifyNeedsYou, notifyFinished, notifyError, machines, raiseNotice])

  // A terminal session the daemon says needs you, or finished. Kept in a ref so the one listener below always
  // reads the latest sessions and settings.
  const onAttention = useRef<(attention: Attention) => void>(() => undefined)
  useEffect(() => {
    onAttention.current = (attention) => {
      const target = sessions.find((item) => item.machine === attention.machine && item.pane === attention.term)
      if (!target || isChatSession(target.conversation)) return
      if (target.machine !== LOCAL_MACHINE && machines.get(target.machine)?.state.notices !== true) return
      if (document.hasFocus() && target.id === focusedId) return
      // More is queued for it: the turn ended, the session did not finish, so it neither rings nor says so.
      if (attention.status === 'done' && queuedFor(target) > 0) return
      const notice = noticeForAttention(target, attention.status, { needs: notifyNeedsYou, finished: notifyFinished, error: notifyError })
      if (!notice) return
      raiseNotice(notice)
      if (notificationSound) playNotificationSound()
    }
  }, [sessions, machines, focusedId, notifyNeedsYou, notifyFinished, notifyError, notificationSound, raiseNotice])

  useEffect(() => {
    if (!isDesktop()) return undefined
    let gone = false
    let stop: (() => void) | null = null
    listenAttention((attention) => { onAttention.current(attention) })
      .then((off) => { if (gone) off(); else stop = off })
      .catch((error: unknown) => { console.warn('Could not listen for alerts from the daemon', error) })
    return () => { gone = true; stop?.() }
  }, [])

  // A browser only lets a tone play once the person has clicked or typed in the page.
  useEffect(() => {
    if (!notificationSound) return
    const enable = () => { enableNotificationSound() }
    window.addEventListener('pointerdown', enable, { once: true })
    window.addEventListener('keydown', enable, { once: true })
    return () => {
      window.removeEventListener('pointerdown', enable)
      window.removeEventListener('keydown', enable)
    }
  }, [notificationSound])
}
