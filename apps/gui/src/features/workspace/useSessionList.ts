import type { UseQueryResult } from '@tanstack/react-query'
import type { Conversation } from '@/data/preview'
import type { RemoteMachine } from './model/machines'
import { type Dispatch, type SetStateAction, useEffect, useMemo, useRef, useState } from 'react'
import { daemonSaidRemoved, loadConversations, localListRevision } from '@/api/daemon'
import { wasClosed } from './closedSessions'
import { confirmGone, heldView, LINGER_SLACK_MS, nextCheckAt, trackMissing, type Missing } from './model/lingering'
import { markConversationsDisconnected } from '@/data/connection'
import { rememberSessions, withEnded } from './model/ended'
import { isListedChatSession } from './model/chatSession'
import { remoteTerminals } from './model/machines'
import type { ChatAgent } from './model/chatView'
import type { Layout } from './model/layout'
import { keepLabels, nextResumeAt, type SessionView, toSessionView } from './model/session'

/** The longest wait a browser timer takes (a signed 32-bit count of milliseconds); a longer one fires at once. */
const MAX_TIMER_MS = 2_147_483_647

/** Terminal sessions, and the chat sessions the daemon owns that are not put away (ADR 0003). */
const isInSidebar = (item: Conversation): boolean => item.kind === 'terminal' || isListedChatSession(item)

/** This window closed the session, or the daemon said why it left: its row goes at once. */
const leftWithReason = (id: string): boolean => wasClosed(id) || daemonSaidRemoved(id)

/** One log line for each session a list left out with no reason, when it is first held. */
function useLogUnexplainedMisses(missing: ReadonlyMap<string, Missing>): void {
  const logged = useRef<ReadonlyMap<string, Missing>>(new Map())
  useEffect(() => {
    for (const [id, entry] of missing) {
      if (logged.current.get(id) === entry) continue
      console.warn(`Session ${id} left the list with no reason (revision ${String(entry.miss.listedIn)} listed it, ${String(entry.miss.missedIn)} did not); its row stays until a full list confirms it is gone`)
    }
    logged.current = missing
  }, [missing])
}

/** Reads the full list from the daemon while a session is held, and drops each held session it does not name. */
function useConfirmGone(missing: ReadonlyMap<string, Missing>, setMissing: Dispatch<SetStateAction<ReadonlyMap<string, Missing>>>): void {
  const [checkedAt, setCheckedAt] = useState(0)
  const checkAt = nextCheckAt(missing, checkedAt)
  useEffect(() => {
    if (checkAt === null) return undefined
    const timer = window.setTimeout(() => {
      const askedAt = Date.now()
      loadConversations().then(
        (listed) => {
          setMissing((before) => confirmGone(before, new Set(listed.filter(isInSidebar).map((item) => item.id)), askedAt))
          setCheckedAt(askedAt)
        },
        (error: unknown) => {
          console.warn('Could not read the session list to confirm the sessions it missed; trying again', error)
          setCheckedAt(askedAt)
        },
      )
    }, Math.max(checkAt - Date.now(), 0) + LINGER_SLACK_MS)
    return () => { window.clearTimeout(timer) }
  }, [checkAt, setMissing])
}

/** The sessions the window shows, from this computer and every machine, each keeping the labels and the pane it already had. */
export function useSessionList({ sessionsQuery, online, machines, home, layout }: {
  sessionsQuery: UseQueryResult<Conversation[]>
  online: boolean
  machines: ReadonlyMap<number, RemoteMachine>
  home: string
  layout: Layout
}) {
  // Sessions started as a known agent, until the daemon says so itself (see `startAgentSession`).
  const [expectedAgents, setExpectedAgents] = useState<Record<string, ChatAgent>>({})
  const [shownLabels, setShownLabels] = useState<ReadonlyMap<string, SessionView>>(() => new Map())
  // The time the list is read against. It moves only when a limited session's time comes: a time still ahead of now is
  // ahead of an older clock too, and the timer below corrects one that passed since.
  const [clock, setClock] = useState(() => Date.now())
  const liveSessions = useMemo(() => {
    const terminals = (sessionsQuery.data ?? []).filter(isInSidebar)
    const here = online ? terminals : markConversationsDisconnected(terminals)
    return [...here, ...remoteTerminals(machines)].map((conversation) => {
      const expected = expectedAgents[conversation.id]
      return toSessionView(expected !== undefined && !conversation.agent ? { ...conversation, agent: expected } : conversation, home, clock)
    })
  }, [sessionsQuery.data, online, home, machines, expectedAgents, clock])
  // A limited session goes back to the daemon's status at the time the daemon gave, with no new list needed.
  // The clock is a dependency so a timer that fired a moment early, or was cut to the longest wait, is set again.
  const resumeAt = nextResumeAt(liveSessions)
  useEffect(() => {
    if (resumeAt === null) return undefined
    const timer = window.setTimeout(() => { setClock(Date.now()) }, Math.min(Math.max(resumeAt - Date.now(), 0), MAX_TIMER_MS))
    return () => { window.clearTimeout(timer) }
  }, [resumeAt, clock])
  const everySession = useMemo(() => liveSessions.map((view) => keepLabels(view, shownLabels.get(view.id))), [liveSessions, shownLabels])
  // What each row shows is remembered, so a later refresh keeps it (see `keepLabels`); only a change is stored, which ends the loop.
  const labelsChanged = everySession.length !== shownLabels.size || everySession.some((view) => shownLabels.get(view.id)?.title !== view.title || shownLabels.get(view.id)?.cwd !== view.cwd || shownLabels.get(view.id)?.branch !== view.branch)
  if (labelsChanged) setShownLabels(new Map(everySession.map((view) => [view.id, view])))
  // A list that misses a session with no reason must not take its row away (see `trackMissing`).
  const [previous, setPrevious] = useState(() => ({ list: everySession, revision: localListRevision() }))
  const [missing, setMissing] = useState<ReadonlyMap<string, Missing>>(() => new Map())
  if (previous.list !== everySession) {
    const revision = localListRevision()
    setPrevious({ list: everySession, revision })
    setMissing((before) => trackMissing(before, previous.list, everySession, { at: Date.now(), listedIn: previous.revision, missedIn: revision }, leftWithReason))
  }
  useLogUnexplainedMisses(missing)
  useConfirmGone(missing, setMissing)
  const held = useMemo(() => [...missing.values()].map(heldView), [missing])
  const sessions = useMemo(() => (held.length === 0 ? everySession : [...everySession, ...held]), [everySession, held])
  const sessionMap = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions])
  // A session the daemon stops listing keeps its pane, shown as ended, so a refresh that misses one
  // never closes a layout or moves your focus. A pane goes only when you close it.
  const [remembered, setRemembered] = useState<ReadonlyMap<string, SessionView>>(() => new Map())
  const [seenSessions, setSeenSessions] = useState(sessions)
  if (seenSessions !== sessions) {
    setSeenSessions(sessions)
    setRemembered((before) => rememberSessions(before, sessions, layout))
  }
  const paneSessions = useMemo(() => withEnded(sessionMap, remembered), [sessionMap, remembered])
  return { sessions, sessionMap, setExpectedAgents, paneSessions }
}
