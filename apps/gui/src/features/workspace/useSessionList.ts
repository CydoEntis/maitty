import type { UseQueryResult } from '@tanstack/react-query'
import type { Conversation } from '@/data/preview'
import type { RemoteMachine } from './model/machines'
import { useEffect, useMemo, useState } from 'react'
import { wasClosed } from './closedSessions'
import { heldView, LINGER_MS, LINGER_SLACK_MS, pruneMissing, trackMissing, type Missing } from './model/lingering'
import { markConversationsDisconnected } from '@/data/connection'
import { rememberSessions, withEnded } from './model/ended'
import { isListedChatSession } from './model/chatSession'
import { remoteTerminals } from './model/machines'
import type { ChatAgent } from './model/chatView'
import type { Layout } from './model/layout'
import { keepLabels, type SessionView, toSessionView } from './model/session'

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
  const liveSessions = useMemo(() => {
    // Terminal sessions, and the chat sessions the daemon owns that are not put away (ADR 0003).
    const terminals = (sessionsQuery.data ?? []).filter((item) => item.kind === 'terminal' || isListedChatSession(item))
    const here = online ? terminals : markConversationsDisconnected(terminals)
    return [...here, ...remoteTerminals(machines)].map((conversation) => {
      const expected = expectedAgents[conversation.id]
      return toSessionView(expected !== undefined && !conversation.agent ? { ...conversation, agent: expected } : conversation, home)
    })
  }, [sessionsQuery.data, online, home, machines, expectedAgents])
  const everySession = useMemo(() => liveSessions.map((view) => keepLabels(view, shownLabels.get(view.id))), [liveSessions, shownLabels])
  // What each row shows is remembered, so a later refresh keeps it (see `keepLabels`); only a change is stored, which ends the loop.
  const labelsChanged = everySession.length !== shownLabels.size || everySession.some((view) => shownLabels.get(view.id)?.title !== view.title || shownLabels.get(view.id)?.cwd !== view.cwd || shownLabels.get(view.id)?.branch !== view.branch)
  if (labelsChanged) setShownLabels(new Map(everySession.map((view) => [view.id, view])))
  // A list that misses a session for a moment must not take its row away (see `trackMissing`).
  const [previousList, setPreviousList] = useState(everySession)
  const [missing, setMissing] = useState<ReadonlyMap<string, Missing>>(() => new Map())
  if (previousList !== everySession) {
    setPreviousList(everySession)
    setMissing((before) => trackMissing(before, previousList, everySession, Date.now(), wasClosed))
  }
  useEffect(() => {
    if (missing.size === 0) return undefined
    const timer = window.setTimeout(() => { setMissing((before) => pruneMissing(before, Date.now())) }, LINGER_MS + LINGER_SLACK_MS)
    return () => { window.clearTimeout(timer) }
  }, [missing])
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
