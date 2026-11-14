import { parseTaskNotifications } from './taskNotification'
import type { ApprovalEvent, ChatEvent, CommandEvent, EditEvent, ItemStatus, MessageEvent, SubagentEvent, TodoEvent, ToolEvent, TurnEvent } from './chatEvents'
import { startsTable, streamingText } from './markdown'
import { describeStep } from './stepInfo'
import { describeWork } from './workPhase'
import { documentOf } from './writtenDocument'
import { isQuietRun, type WorkStep } from './workSummary'
import type { TurnSpan } from '@/features/inspect/turnRange'

function isDocument(event: ChatEvent): boolean {
  return event.kind === 'edit' && documentOf(event) !== null
}

/** What the list draws. One row per thing on screen; the events beneath keep their own ids. */
export type ChatRow =
  | { readonly kind: 'user'; readonly key: string; readonly event: MessageEvent }
  /** One agent message: the text of the agent's messages that follow each other in a turn. */
  | { readonly kind: 'agent'; readonly key: string; readonly turnId: string; readonly paragraphs: readonly string[]; readonly status: ItemStatus }
  /** The agent's reasoning that follows each other in a turn, as one folded row labelled Thought; its key is the first one's id. */
  | { readonly kind: 'thought'; readonly key: string; readonly turnId: string; readonly texts: readonly string[] }
  | { readonly kind: 'step'; readonly key: string; readonly event: EditEvent | CommandEvent | TodoEvent | ToolEvent }
  /** Two or more steps that follow each other, folded into one line that opens to them; its key is the first step's id. */
  | { readonly kind: 'work'; readonly key: string; readonly events: readonly WorkStep[] }
  /** Helper agents that follow each other are one card; its key is the first one's id, so it stays put as more join. */
  | { readonly kind: 'agents'; readonly key: string; readonly events: readonly SubagentEvent[] }
  | { readonly kind: 'approval'; readonly key: string; readonly event: ApprovalEvent; readonly step: StepRef | null }
  | { readonly kind: 'divider'; readonly key: string; readonly event: TurnEvent }
  /**
   * Under a turn that edited files: how many, and the lines added and removed where every diff was kept. `turn` is when
   * it ran, null when the times are not the daemon's, so its snapshots cannot be picked by them.
   */
  | { readonly kind: 'changes'; readonly key: string; readonly files: readonly string[]; readonly lines: { readonly added: number; readonly removed: number } | null; readonly turn: TurnSpan | null }
  /** What the person just sent, shown before the history has it; `echoText` of `buildRows`. */
  | { readonly kind: 'echo'; readonly key: string; readonly text: string }
  /** A helper agent reporting back (the harness's `<task-notification>`), as one quiet line. */
  | { readonly kind: 'report'; readonly key: string; readonly text: string }
  /** The line under the last item while a turn runs, with what the agent is doing (`describeWork`). */
  | { readonly kind: 'working'; readonly key: string; readonly phrase: string }

/** The tool step an approval is about: the row to scroll to and the words that name it. */
export type StepRef = { readonly key: string; readonly label: string }

export const WORKING_ROW_KEY = 'working'
export const ECHO_ROW_KEY = 'echo'

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
 * `hideSuccessfulTools` leaves out the runs `isQuietRun` allows to go (Settings › Chat).
 */
export function buildRows(events: readonly ChatEvent[], working: boolean, root: string | null = null, echoText: string | null = null, echoThinking = true, hideSuccessfulTools = false): ChatRow[] {
  const rows: ChatRow[] = []
  const edits = new Map<string, EditEvent[]>()
  for (const event of events) {
    if (event.kind === 'edit') edits.set(event.turnId, [...(edits.get(event.turnId) ?? []), event])
    switch (event.kind) {
      case 'message': {
        if (event.role === 'user') {
          const reports = parseTaskNotifications(event.text)
          if (reports !== null) rows.push(...reports.map((text, index): ChatRow => ({ kind: 'report', key: `${event.id}:${String(index)}`, text })))
          else rows.push({ kind: 'user', key: event.id, event })
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
      case 'reasoning': {
        const previous = rows.at(-1)
        if (previous?.kind === 'thought' && previous.turnId === event.turnId) rows[rows.length - 1] = { ...previous, texts: [...previous.texts, event.text] }
        else rows.push({ kind: 'thought', key: event.id, turnId: event.turnId, texts: [event.text] })
        break
      }
      case 'approval':
        rows.push({ kind: 'approval', key: event.id, event, step: null })
        break
      case 'turn': {
        rows.push({ kind: 'divider', key: event.id, event })
        const changed = changesOf(edits.get(event.turnId) ?? [])
        if (changed !== null) rows.push({ kind: 'changes', key: `changes:${event.turnId}`, ...changed, turn: event.timed ? { startedAtMs: event.startedAtMs, endedAtMs: event.endedAtMs } : null })
        break
      }
      case 'subagent': {
        const previous = rows.at(-1)
        if (previous?.kind === 'agents') rows[rows.length - 1] = { ...previous, events: [...previous.events, event] }
        else rows.push({ kind: 'agents', key: event.id, events: [event] })
        break
      }
      case 'todo':
        rows.push({ kind: 'step', key: event.id, event })
        break
      case 'edit':
      case 'command':
      case 'tool': {
        const previous = rows.at(-1)
        // A document the agent wrote is something to read, so it stands on its own instead of folding into a run's summary.
        if (isDocument(event)) rows.push({ kind: 'step', key: event.id, event })
        else if (previous?.kind === 'work') rows[rows.length - 1] = { ...previous, events: [...previous.events, event] }
        else if (previous?.kind === 'step' && previous.event.kind !== 'todo' && !isDocument(previous.event)) rows[rows.length - 1] = { kind: 'work', key: previous.key, events: [previous.event, event] }
        else rows.push({ kind: 'step', key: event.id, event })
      }
    }
  }
  const shown = hideSuccessfulTools ? withoutQuietRuns(rows) : rows
  // A message just sent is drawn at once, and the agent is taken to be at work on it, until the history says so itself.
  if (echoText !== null) shown.push({ kind: 'echo', key: ECHO_ROW_KEY, text: echoText })
  if (working || (echoText !== null && echoThinking)) shown.push({ kind: 'working', key: WORKING_ROW_KEY, phrase: describeWork(events, root) })
  return linkApprovals(shown, events, root)
}

function isQuietRow(row: ChatRow): boolean {
  if (row.kind === 'work') return isQuietRun(row.events)
  return row.kind === 'step' && row.event.kind !== 'todo' && isQuietRun([row.event])
}

/**
 * The rows without the runs of reads and searches that succeeded. The two parts of a turn's message
 * that such a run stood between read as the one message they would have been without it, and so do two thoughts.
 * A thought between two messages stays a row of its own, so they stay two messages.
 */
function withoutQuietRuns(rows: readonly ChatRow[]): ChatRow[] {
  const kept: ChatRow[] = []
  for (const row of rows) {
    if (isQuietRow(row)) continue
    const previous = kept.at(-1)
    if (row.kind === 'agent' && previous?.kind === 'agent' && previous.turnId === row.turnId) {
      kept[kept.length - 1] = { ...previous, paragraphs: [...previous.paragraphs, ...row.paragraphs], status: mergedStatus(previous.status, row.status) }
    } else if (row.kind === 'thought' && previous?.kind === 'thought' && previous.turnId === row.turnId) {
      kept[kept.length - 1] = { ...previous, texts: [...previous.texts, ...row.texts] }
    } else kept.push(row)
  }
  return kept
}

/** The files a turn's edits touched, each once, and their line counts when no diff is missing. */
function changesOf(edits: readonly EditEvent[]): { files: string[]; lines: { added: number; removed: number } | null } | null {
  const files = [...new Set(edits.flatMap((edit) => (edit.status === 'completed' && edit.file !== null ? [edit.file] : [])))]
  if (files.length === 0) return null
  const counted = edits.filter((edit) => edit.status === 'completed' && edit.file !== null)
  const lines = counted.every((edit) => edit.diff.status === 'available' && !edit.diff.truncated)
    ? counted.reduce((sum, edit) => (edit.diff.status === 'available' ? { added: sum.added + edit.diff.added, removed: sum.removed + edit.diff.removed } : sum), { added: 0, removed: 0 })
    : null
  return { files, lines }
}

/** Points each approval at the row of the tool step it is about, when the provider named one. */
function linkApprovals(rows: ChatRow[], events: readonly ChatEvent[], root: string | null): ChatRow[] {
  if (!rows.some((row) => row.kind === 'approval')) return rows
  const rowOf = new Map<string, string>()
  for (const row of rows) {
    if (row.kind === 'step') rowOf.set(row.key, row.key)
    if (row.kind === 'agents' || row.kind === 'work') for (const event of row.events) rowOf.set(event.id, row.key)
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
  if (!turn.timed) return outcome === 'completed' ? 'Turn finished' : outcome === 'failed' ? 'Turn failed' : 'Turn stopped'
  const took = formatDuration(turn.endedAtMs - turn.startedAtMs)
  const done = clock(turn.endedAtMs)
  if (outcome === 'completed') return `Finished in ${took} · ${done}`
  return `${outcome === 'failed' ? 'Failed' : 'Stopped'} after ${took} · ${done}`
}

/** The lead sentence of an agent message: its first sentence, cut at 220 characters. The rest is the body. */
const LEAD_LIMIT = 220

// A first paragraph that is a code block, list, heading, quote or table has no lead sentence to lift out of it.
const BLOCK_START = /^(?:\s{0,3}(?:```|~~~|#{1,6}\s|>|[-*+]\s|\d+[.)]\s)|\|)/

/** The paragraphs of a reply that is still arriving, ready to read: only the last can be unfinished (`streamingText`). */
export function streamingParagraphs(paragraphs: readonly string[]): string[] {
  const last = paragraphs.at(-1)
  if (last === undefined) return []
  const shown = streamingText(last)
  const earlier = paragraphs.slice(0, -1)
  return shown.trim() === '' ? earlier : [...earlier, shown]
}

export function splitLead(paragraphs: readonly string[]): { lead: string; rest: readonly string[] } {
  const [first, ...others] = paragraphs
  if (first === undefined) return { lead: '', rest: [] }
  const [firstLine = '', secondLine] = first.split('\n')
  if (BLOCK_START.test(first) || startsTable(firstLine, secondLine)) return { lead: '', rest: paragraphs }
  const sentence = /^[\s\S]*?[.!?](?=\s|$)/.exec(first)?.[0] ?? first
  const lead = sentence.length > LEAD_LIMIT ? first.slice(0, LEAD_LIMIT) : sentence
  const remainder = first.slice(lead.length).trim()
  return { lead, rest: remainder === '' ? others : [remainder, ...others] }
}
