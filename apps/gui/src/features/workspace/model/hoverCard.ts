import type { SessionStatus, SessionView } from './session'
import { projectOf } from './sidebar'

export const HOVER_CARD_DELAY_MS = 450
export const HOVER_CARD_WIDTH_PX = 290
const GAP_PX = 8
const EDGE_PX = 8
const SAID_CHARS = 140

const HEADLINE: Record<SessionStatus, string> = {
  needs: 'Needs you',
  working: 'Working',
  done: 'Finished',
  error: 'Stopped',
  idle: 'Idle',
  unknown: 'Not known',
}

const FALLBACK: Record<SessionStatus, string> = {
  needs: 'Waiting on your answer.',
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
