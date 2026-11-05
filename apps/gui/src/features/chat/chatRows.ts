import type { ApprovalEvent, ChatEvent, CommandEvent, EditEvent, ItemStatus, MessageEvent, SubagentEvent, TodoEvent, ToolEvent, TurnEvent } from './chatEvents'
import { describeStep } from './stepInfo'

/** What the list draws. One row per thing on screen; the events beneath keep their own ids. */
export type ChatRow =
  | { readonly kind: 'user'; readonly key: string; readonly event: MessageEvent }
  /** One agent message: the text of the agent's messages that follow each other in a turn. */
  | { readonly kind: 'agent'; readonly key: string; readonly turnId: string; readonly paragraphs: readonly string[]; readonly status: ItemStatus }
  | { readonly kind: 'step'; readonly key: string; readonly event: EditEvent | CommandEvent | TodoEvent | ToolEvent }
  /** Helper agents that follow each other are one card; its key is the first one's id, so it stays put as more join. */
  | { readonly kind: 'agents'; readonly key: string; readonly events: readonly SubagentEvent[] }
  | { readonly kind: 'approval'; readonly key: string; readonly event: ApprovalEvent; readonly step: StepRef | null }
  | { readonly kind: 'divider'; readonly key: string; readonly event: TurnEvent }
  /** The line under the last item while a turn runs. */
  | { readonly kind: 'working'; readonly key: string }

/** The tool step an approval is about: the row to scroll to and the words that name it. */
export type StepRef = { readonly key: string; readonly label: string }

export const WORKING_ROW_KEY = 'working'

const FENCE = /^\s{0,3}(```|~~~)/

/**
 * Paragraphs of a message: blocks separated by a blank line, with no empty ones. A fenced code block
 * is one block even when it holds blank lines (and one left open runs to the end of the message).
 */
export function paragraphsOf(text: string): string[] {
  const blocks: string[] = []
  let current: string[] = []
  let fence: string | null = null
  const close = () => {
    const block = current.join('\n').trim()
    if (block !== '') blocks.push(block)
    current = []
  }
  for (const line of text.split('\n')) {
    const marker = FENCE.exec(line)?.[1] ?? null
    if (fence === null && marker !== null) fence = marker
    else if (fence !== null && marker === fence) fence = null
    else if (fence === null && line.trim() === '') {
      close()
      continue
    }
    current.push(line)
  }
  close()
  return blocks
}

function mergedStatus(a: ItemStatus, b: ItemStatus): ItemStatus {
  return a === 'running' || b === 'running' ? 'running' : b
}

/**
 * The rows for a list of events. The agent's messages that follow each other in one turn become a
 * single message (v18 C.1: one agent message per turn, paragraphs never split into messages). A tool
 * step between two messages ends the first, so the text and the steps keep the order they happened in.
 */
export function buildRows(events: readonly ChatEvent[], working: boolean, root: string | null = null): ChatRow[] {
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
        rows.push({ kind: 'approval', key: event.id, event, step: null })
        break
      case 'turn':
        rows.push({ kind: 'divider', key: event.id, event })
        break
      case 'subagent': {
        const previous = rows.at(-1)
        if (previous?.kind === 'agents') rows[rows.length - 1] = { ...previous, events: [...previous.events, event] }
        else rows.push({ kind: 'agents', key: event.id, events: [event] })
        break
      }
      case 'edit':
      case 'command':
      case 'todo':
      case 'tool':
        rows.push({ kind: 'step', key: event.id, event })
    }
  }
  if (working) rows.push({ kind: 'working', key: WORKING_ROW_KEY })
  return linkApprovals(rows, events, root)
}

/** Points each approval at the row of the tool step it is about, when the provider named one. */
function linkApprovals(rows: ChatRow[], events: readonly ChatEvent[], root: string | null): ChatRow[] {
  if (!rows.some((row) => row.kind === 'approval')) return rows
  const rowOf = new Map<string, string>()
  for (const row of rows) {
    if (row.kind === 'step') rowOf.set(row.key, row.key)
    if (row.kind === 'agents') for (const event of row.events) rowOf.set(event.id, row.key)
  }
  const labelOf = new Map<string, string>()
  for (const event of events) {
    const label = describeStep(event, root)
    if (label !== null) labelOf.set(event.id, label)
  }
  const resolve = (id: string): StepRef | null => {
    // A Codex patch of several files is one tool item shown as one card per file (`id#0`, `id#1`).
    const eventId = rowOf.has(id) ? id : rowOf.has(`${id}#0`) ? `${id}#0` : null
    const key = eventId === null ? undefined : rowOf.get(eventId)
    const label = eventId === null ? undefined : labelOf.get(eventId)
    return key === undefined || label === undefined ? null : { key, label }
  }
  return rows.map((row) => (row.kind === 'approval' && row.event.stepId !== null ? { ...row, step: resolve(row.event.stepId) } : row))
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

// A first paragraph that is a code block, list, heading or quote has no lead sentence to lift out of it.
const BLOCK_START = /^(?:\s{0,3}(?:```|~~~|#{1,6}\s|>|[-*+]\s|\d+[.)]\s)|\|)/

export function splitLead(paragraphs: readonly string[]): { lead: string; rest: readonly string[] } {
  const [first, ...others] = paragraphs
  if (first === undefined) return { lead: '', rest: [] }
  if (BLOCK_START.test(first)) return { lead: '', rest: paragraphs }
  const sentence = /^[\s\S]*?[.!?](?=\s|$)/.exec(first)?.[0] ?? first
  const lead = sentence.length > LEAD_LIMIT ? first.slice(0, LEAD_LIMIT) : sentence
  const remainder = first.slice(lead.length).trim()
  return { lead, rest: remainder === '' ? others : [remainder, ...others] }
}
