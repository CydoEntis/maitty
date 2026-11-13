import type { TurnSpan } from '@/features/inspect/turnRange'
import { parseAttachmentMessage } from './attachmentMessage'
import type { ChatEvent, TurnEvent } from './chatEvents'

/** Where a chat's files can be restored from: the pane whose checkout holds them, or why they cannot be. */
export type RewindPlace = { readonly term: number } | { readonly refusal: string }

/** A sent message's Edit from here: `disabled` says why it cannot be used now, or is null when it can. */
export type EditFromHere = {
  readonly disabled: string | null
  readonly pick: (message: { readonly text: string; readonly turnId: string }) => void
}

const SNIPPET_MAX = 80

/**
 * The line the next message starts with, telling the agent its files were put back. One line, whatever the message
 * held: the agent's own conversation is not rewound, so this is how it learns the files no longer match it.
 */
export function restoreNote(message: string): string {
  const words = (parseAttachmentMessage(message)?.text ?? message).replace(/\s+/g, ' ').trim()
  const snippet = words.length > SNIPPET_MAX ? `${words.slice(0, SNIPPET_MAX)}…` : words
  return `[Note: I restored the files to how they were before my earlier message "${snippet}"; anything changed after it is undone.]`
}

/** When the turn a message started ran, by the daemon's clock; null while it runs or when the times are not the daemon's. */
export function turnSpanOf(events: readonly ChatEvent[], turnId: string): TurnSpan | null {
  const turn = events.find((event): event is TurnEvent => event.kind === 'turn' && event.turnId === turnId)
  return turn?.timed === true ? { startedAtMs: turn.startedAtMs, endedAtMs: turn.endedAtMs } : null
}

/** The confirm's words: how many files change, that what is there now is kept, and what happens to the box. */
export function confirmBody(count: number, agent: string, hasDraft: boolean): string {
  const files = count === 1 ? '1 file' : `${String(count)} files`
  const change = count === 0
    ? 'No files change: they already match how they were before this message.'
    : `${files} change back to how they were before this message. What is there now is kept as a "before a restore" snapshot.`
  const box = hasDraft ? ' The message goes in the box above what you have typed.' : ' The message goes back in the box.'
  return `${change}${box} ${agent}'s conversation is not rewound; your next message tells it the files were restored.`
}
