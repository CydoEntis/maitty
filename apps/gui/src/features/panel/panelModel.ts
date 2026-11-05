import type { ChatEvent, EditEvent, SubagentEvent, TodoEvent, TodoItem } from '@/features/chat'
import type { ChatAvailability } from '@/features/workspace/model/chatView'

/**
 * What the session panel shows, derived from the chat projection's events (v18 C.2). Pure. The
 * panel adds nothing of its own: the checklist is the agent's own to-do list, and the helper agents
 * and touched files are what the daemon's history recorded.
 */

export type Checklist = {
  readonly items: readonly TodoItem[]
  readonly done: number
  readonly event: TodoEvent
}

/**
 * The agent's current checklist: the last to-do list in the conversation that carried its items.
 * `null` when there is none. `unreadable` is true when the history held a checklist whose items it
 * did not keep, so the panel can say that instead of claiming there is none.
 */
export function checklistOf(events: readonly ChatEvent[]): { checklist: Checklist | null; unreadable: boolean } {
  let unreadable = false
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index]
    if (event?.kind !== 'todo') continue
    if (event.items === null) {
      unreadable = true
      continue
    }
    return { checklist: { items: event.items, done: event.items.filter((item) => item.state === 'done').length, event }, unreadable: false }
  }
  return { checklist: null, unreadable }
}

/**
 * When the current item became current: the time of the earliest list in an unbroken run of lists
 * that all name it as current. The agent only reports changes when it rewrites its list, so this is
 * as exact as the data is. `null` when there is no current item.
 */
export function currentItemSince(events: readonly ChatEvent[], checklist: Checklist): number | null {
  const current = checklist.items.find((item) => item.state === 'current')
  if (current === undefined) return null
  let since = timeOf(checklist.event)
  let seenLast = false
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index]
    if (event?.kind !== 'todo' || event.items === null) continue
    if (event === checklist.event) {
      seenLast = true
      continue
    }
    if (!seenLast) continue
    if (!event.items.some((item) => item.state === 'current' && item.text === current.text)) break
    since = timeOf(event)
  }
  return since
}

function timeOf(event: TodoEvent): number {
  return event.timing.startedAtMs ?? event.observed.firstSeenAtMs
}

export function helperAgentsOf(events: readonly ChatEvent[]): SubagentEvent[] {
  return events.filter((event): event is SubagentEvent => event.kind === 'subagent')
}

export type TouchedFile = {
  readonly file: string
  /** `edited` for a change to a file that existed, `created` for a new one, `read` for a file only looked at. */
  readonly kind: 'edited' | 'created' | 'read'
  readonly added: number
  readonly removed: number
}

const READ_TOOLS: readonly string[] = ['Read', 'NotebookRead']

function editKind(event: EditEvent): 'edited' | 'created' {
  return event.change === 'add' ? 'created' : 'edited'
}

/**
 * The files the agent touched, in the order it first touched them. An edit outranks a read of the
 * same file. Edits are summed per file, and a file created and then edited stays created.
 */
export function touchedFilesOf(events: readonly ChatEvent[]): TouchedFile[] {
  const files = new Map<string, TouchedFile>()
  for (const event of events) {
    if (event.kind === 'edit' && event.file !== null) {
      const before = files.get(event.file)
      const diff = event.diff.status === 'available' ? event.diff : null
      files.set(event.file, {
        file: event.file,
        kind: before?.kind === 'created' ? 'created' : editKind(event),
        added: (before?.kind === 'read' ? 0 : before?.added ?? 0) + (diff?.added ?? 0),
        removed: (before?.kind === 'read' ? 0 : before?.removed ?? 0) + (diff?.removed ?? 0),
      })
    } else if (event.kind === 'tool' && READ_TOOLS.includes(event.name) && event.target !== null && !files.has(event.target)) {
      files.set(event.target, { file: event.target, kind: 'read', added: 0, removed: 0 })
    }
  }
  return [...files.values()]
}

/** One plain sentence on why a session has nothing for the panel to track (v18 C.2, "No agent / no events"). */
export function emptyReason(availability: Exclude<ChatAvailability, { kind: 'ready' }>): string {
  switch (availability.kind) {
    case 'shell':
      return 'This is a shell session, so there is no agent whose work could be tracked.'
    case 'unsupported':
      return `${availability.agent} has no event stream, so its checklist and helpers cannot be shown.`
    case 'remote':
      return `This session runs on ${availability.machineName}. The session panel is local-only in this version.`
    case 'no-events':
      return `The daemon has no conversation for this ${availability.agent} session, so there are no events to track.`
  }
}
