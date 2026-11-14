import type { ChatEvent } from './chatEvents'
import type { ChatRow } from './chatRows'

function stepText(event: Extract<ChatEvent, { kind: 'edit' | 'command' | 'todo' | 'tool' }>): string {
  if (event.kind === 'command') return `${event.command}\n${event.output}`
  if (event.kind === 'edit') return event.file ?? ''
  if (event.kind === 'todo') return (event.items ?? []).map((item) => item.text).join('\n')
  return `${event.name} ${event.target ?? ''}`
}

/** What a search looks through in a row: the words a person can read in it. */
export function rowText(row: ChatRow): string {
  switch (row.kind) {
    case 'user':
      return row.event.text
    case 'agent':
      return row.paragraphs.join('\n')
    case 'echo':
    case 'report':
      return row.text
    case 'step':
      return stepText(row.event)
    case 'work':
      return row.events.map(stepText).join('\n')
    case 'agents':
      return row.events.map((event) => `${event.title ?? ''}\n${event.prompt ?? ''}\n${event.report}`).join('\n')
    // An approval is answered in its tray, and the divider and the working line say nothing of the conversation.
    case 'changes':
      return row.files.join('\n')
    // A thought is folded, so what it says is not yet a word a person can see; it is left out of the search.
    case 'thought':
    case 'approval':
    case 'divider':
    case 'working':
      return ''
  }
}

/**
 * The key of the next row holding `query`, going down from the row after `from` (or up from the one before it),
 * and round past the end. With no `from` the search starts at the newest row, which is where the reader is.
 * Null when no row holds it. Letter case is ignored.
 */
export function findRow(rows: readonly ChatRow[], query: string, from: string | null, direction: 'next' | 'previous'): string | null {
  const wanted = query.trim().toLowerCase()
  if (wanted === '' || rows.length === 0) return null
  const step = direction === 'next' ? 1 : -1
  const at = from === null ? -1 : rows.findIndex((row) => row.key === from)
  // Without a place to start from, the first try is the newest row itself.
  const start = at === -1 ? rows.length - 1 : at + step
  for (let tried = 0; tried < rows.length; tried += 1) {
    const row = rows[(((start + tried * step) % rows.length) + rows.length) % rows.length]
    if (row && rowText(row).toLowerCase().includes(wanted)) return row.key
  }
  return null
}

/** What Find asks of the chat a pane shows. */
export type ChatFind = {
  /** Shows the next or previous row holding the text; false when none does. */
  find: (query: string, direction: 'next' | 'previous') => boolean
  clear: () => void
}

const open = new Map<string, ChatFind>()

/** The chat a session shows makes itself searchable for as long as it is on screen. */
export function registerChatFind(sessionId: string, find: ChatFind): () => void {
  open.set(sessionId, find)
  return () => { if (open.get(sessionId) === find) open.delete(sessionId) }
}

export function chatFindOf(sessionId: string): ChatFind | null {
  return open.get(sessionId) ?? null
}
