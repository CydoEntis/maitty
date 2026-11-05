import { useSyncExternalStore } from 'react'
import type { LineComment } from '@/features/chat/commentMessage'

/**
 * The comments left on a session's diff lines, kept in the window so a line can show its count
 * marker and a file its badge. The comment itself is delivered to the agent as a message (see
 * `commentMessage.ts`); this is only the window's memory of what was written, and it goes with the
 * window.
 */

export type SentComment = LineComment & {
  readonly id: number
  /** The exact message sent to the daemon. */
  readonly message: string
}

const EMPTY: readonly SentComment[] = []
const stored = new Map<number, readonly SentComment[]>()
const listeners = new Set<() => void>()
let nextId = 1

function emit(): void {
  for (const listener of listeners) listener()
}

export function addComment(conversation: number, comment: LineComment, message: string): SentComment {
  const added: SentComment = { ...comment, id: nextId, message }
  nextId += 1
  stored.set(conversation, [...(stored.get(conversation) ?? EMPTY), added])
  emit()
  return added
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

export function useComments(conversation: number): readonly SentComment[] {
  return useSyncExternalStore(subscribe, () => stored.get(conversation) ?? EMPTY)
}

/** For tests: the store belongs to the window's lifetime. */
export function forgetComments(): void {
  stored.clear()
  emit()
}

// The Esc ladder's top layer (v18 C.9) is the comment composer: it closes before anything else. The
// window's key handler asks whether one is open and, if so, cancels it.
let composerOpen = false
let cancelComposer: (() => void) | null = null

export function setCommentComposerOpen(open: boolean, cancel: (() => void) | null): void {
  composerOpen = open
  cancelComposer = open ? cancel : null
}

export function isCommentComposerOpen(): boolean {
  return composerOpen
}

export function cancelCommentComposer(): void {
  cancelComposer?.()
}
