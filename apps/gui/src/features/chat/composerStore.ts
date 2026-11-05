import { useSyncExternalStore } from 'react'

/**
 * What a conversation's composer holds between views: the unsent text and the messages waiting for
 * the agent to finish. They live here, not in the component, so switching to the terminal and back
 * (which closes and reopens the chat) loses neither. This is the window's memory only; the daemon
 * has no queue yet, so nothing here survives closing the window.
 */

export type QueuedMessage = { readonly id: number; readonly text: string }
export type ComposerState = { readonly draft: string; readonly queue: readonly QueuedMessage[] }

const EMPTY: ComposerState = { draft: '', queue: [] }
const stored = new Map<number, ComposerState>()
const listeners = new Set<() => void>()
let nextId = 1

export function newQueuedMessage(text: string): QueuedMessage {
  const id = nextId
  nextId += 1
  return { id, text }
}

function read(conversation: number): ComposerState {
  return stored.get(conversation) ?? EMPTY
}

export function updateComposer(conversation: number, change: (state: ComposerState) => ComposerState): void {
  const before = read(conversation)
  const after = change(before)
  if (after === before) return
  if (after.draft === '' && after.queue.length === 0) stored.delete(conversation)
  else stored.set(conversation, after)
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

export function useComposerState(conversation: number): ComposerState {
  return useSyncExternalStore(subscribe, () => read(conversation))
}

/** For tests: the store belongs to the window's lifetime. */
export function forgetComposers(): void {
  stored.clear()
}
