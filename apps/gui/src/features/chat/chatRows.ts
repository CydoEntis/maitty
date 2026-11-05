import type { ApprovalEvent, ChatEvent, CommandEvent, EditEvent, ItemStatus, MessageEvent, SubagentEvent, TodoEvent, ToolEvent, TurnEvent } from './chatEvents'

/** What the list draws. One row per thing on screen; the events beneath keep their own ids. */
export type ChatRow =
  | { readonly kind: 'user'; readonly key: string; readonly event: MessageEvent }
  /** One agent message: the text of the agent's messages that follow each other in a turn. */
  | { readonly kind: 'agent'; readonly key: string; readonly turnId: string; readonly paragraphs: readonly string[]; readonly status: ItemStatus }
  | { readonly kind: 'step'; readonly key: string; readonly event: EditEvent | CommandEvent | TodoEvent | SubagentEvent | ToolEvent }
  | { readonly kind: 'approval'; readonly key: string; readonly event: ApprovalEvent }
  | { readonly kind: 'divider'; readonly key: string; readonly event: TurnEvent }
  /** The line under the last item while a turn runs. */
  | { readonly kind: 'working'; readonly key: string }

export const WORKING_ROW_KEY = 'working'

/** Paragraphs of a message: blocks separated by a blank line, with no empty ones. */
export function paragraphsOf(text: string): string[] {
  return text.split(/\n[ \t]*\n/).map((block) => block.trim()).filter((block) => block !== '')
}

function mergedStatus(a: ItemStatus, b: ItemStatus): ItemStatus {
  return a === 'running' || b === 'running' ? 'running' : b
}

/**
 * The rows for a list of events. The agent's messages that follow each other in one turn become a
 * single message (v18 C.1: one agent message per turn, paragraphs never split into messages). A tool
 * step between two messages ends the first, so the text and the steps keep the order they happened in.
 */
export function buildRows(events: readonly ChatEvent[], working: boolean): ChatRow[] {
  const rows: ChatRow[] = []
  for (const event of events) {
    switch (event.kind) {
      case 'message': {
        if (event.role === 'user') {
          rows.push({ kind: 'user', key: event.id, event })
          break
        }
        const previous = rows.at(-1)
        if (previous?.kind === 'agent' && previous.turnId === event.turnId) {
          rows[rows.length - 1] = { ...previous, paragraphs: [...previous.paragraphs, ...paragraphsOf(event.text)], status: mergedStatus(previous.status, event.status) }
        } else {
          rows.push({ kind: 'agent', key: event.id, turnId: event.turnId, paragraphs: paragraphsOf(event.text), status: event.status })
        }
        break
      }
      case 'approval':
        rows.push({ kind: 'approval', key: event.id, event })
        break
      case 'turn':
        rows.push({ kind: 'divider', key: event.id, event })
        break
      case 'edit':
      case 'command':
      case 'todo':
      case 'subagent':
      case 'tool':
        rows.push({ kind: 'step', key: event.id, event })
    }
  }
  if (working) rows.push({ kind: 'working', key: WORKING_ROW_KEY })
  return rows
}

const MS_PER_SECOND = 1000
const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60

/** "7s", "1m 12s", "2h 5m": how long a turn took. */
export function formatDuration(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / MS_PER_SECOND))
  if (seconds < SECONDS_PER_MINUTE) return `${String(seconds)}s`
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE)
  if (minutes < MINUTES_PER_HOUR) return `${String(minutes)}m ${String(seconds % SECONDS_PER_MINUTE)}s`
  return `${String(Math.floor(minutes / MINUTES_PER_HOUR))}h ${String(minutes % MINUTES_PER_HOUR)}m`
}

/** The words of a turn divider. Without the daemon's times it claims no duration and no clock time. */
export function dividerText(turn: TurnEvent, clock: (ms: number) => string): string {
  const outcome = turn.outcome
  if (!turn.timed) return outcome === 'completed' ? 'Turn finished' : outcome === 'failed' ? 'Turn failed' : 'Turn interrupted'
  const took = formatDuration(turn.endedAtMs - turn.startedAtMs)
  const done = clock(turn.endedAtMs)
  if (outcome === 'completed') return `Cooked for ${took} · done ${done}`
  return `${outcome === 'failed' ? 'Failed' : 'Interrupted'} after ${took} · ${done}`
}

/** The lead sentence of an agent message: its first sentence, cut at 220 characters. The rest is the body. */
const LEAD_LIMIT = 220

export function splitLead(paragraphs: readonly string[]): { lead: string; rest: readonly string[] } {
  const [first, ...others] = paragraphs
  if (first === undefined) return { lead: '', rest: [] }
  const sentence = /^[\s\S]*?[.!?](?=\s|$)/.exec(first)?.[0] ?? first
  const lead = sentence.length > LEAD_LIMIT ? first.slice(0, LEAD_LIMIT) : sentence
  const remainder = first.slice(lead.length).trim()
  return { lead, rest: remainder === '' ? others : [remainder, ...others] }
}
