import type { SessionStatus, SessionView } from './session'
import { projectOf } from './sidebar'
import { formatAge } from './tracking'

export const HOVER_CARD_DELAY_MS = 450
export const HOVER_CARD_WIDTH_PX = 290
const GAP_PX = 8
const EDGE_PX = 8
const SAID_CHARS = 140

const HEADLINE: Record<SessionStatus, string> = {
  needs: 'Needs you',
  limited: 'Limited',
  working: 'Working',
  done: 'Finished',
  error: 'Stopped',
  idle: 'Idle',
  unknown: 'Not known',
}

const FALLBACK: Record<SessionStatus, string> = {
  needs: 'Waiting on your answer.',
  limited: 'It hit its plan limit; it is told to continue when the limit resets.',
  working: 'Running now.',
  done: 'Ready to review.',
  error: 'It stopped; nothing is running.',
  idle: 'Nothing to do.',
  unknown: 'Its agent has not said what it is doing.',
}

function clipped(text: string): string {
  const line = text.trim().split('\n')[0] ?? ''
  return line.length > SAID_CHARS ? `${line.slice(0, SAID_CHARS - 1)}…` : line
}

export type HoverCardText = { title: string; meta: string; status: SessionStatus; headline: string; line: string; agent: string; changes: string }

/**
 * What the card beside a hovered row says. Each line is something the daemon reported: what the agent
 * asks when it needs you, the prompt it was last given while it works. Where it reported nothing, the
 * line only says what the status means.
 */
export function hoverCardText(session: SessionView, title: string): HoverCardText {
  const asked = session.ask === null ? session.asking : `${session.ask.tool}: ${session.ask.detail}`
  const said = session.status === 'needs' ? asked : session.status === 'working' ? session.lastPrompt : ''
  const changed = session.diffAdded + session.diffRemoved > 0
  return {
    title,
    meta: session.branch === null ? projectOf(session) : `${projectOf(session)} · ${session.branch}`,
    status: session.status,
    headline: HEADLINE[session.status],
    line: clipped(said) === '' ? FALLBACK[session.status] : clipped(said),
    agent: session.process,
    changes: changed ? `+${String(session.diffAdded)} −${String(session.diffRemoved)}` : 'No changes',
  }
}

/**
 * Where the status came from and how long ago (`hook: Stop, 3 s ago`), in the daemon's own words; null
 * when it gave no reason. The daemon writes the reason when the status changes, so its age is the
 * status's. There is none where the reason is not about the status shown: a status that is not known
 * (the reason is from before), a stopped session (the reason explains idle or done, and the stop is the
 * failed command), a limited one (the reason is for the status under the limit), and one waiting on you
 * to start its helpers (the reason is for the status under it).
 */
export function statusWhyLine(session: Pick<SessionView, 'status' | 'statusWhy' | 'statusSince' | 'helperAsk'>, now: number): string | null {
  const why = clipped(session.statusWhy)
  if (why === '' || session.status === 'unknown' || session.status === 'error' || session.status === 'limited' || session.helperAsk.length > 0) return null
  return session.statusSince === null ? why : `${why}, ${formatAge(now - session.statusSince)} ago`
}

export type Box = { left: number; right: number; top: number }

/**
 * Where the card goes: beside the row on the side away from the window's edge, its top level with the
 * row's, and moved up when it would run past the bottom of the window.
 */
export function placeHoverCard(row: Box, cardHeight: number, windowWidth: number, windowHeight: number): { left: number; top: number } {
  const fitsRight = row.right + GAP_PX + HOVER_CARD_WIDTH_PX <= windowWidth - EDGE_PX
  const left = fitsRight ? row.right + GAP_PX : Math.max(EDGE_PX, row.left - GAP_PX - HOVER_CARD_WIDTH_PX)
  const top = Math.max(EDGE_PX, Math.min(row.top, windowHeight - EDGE_PX - cardHeight))
  return { left, top }
}
