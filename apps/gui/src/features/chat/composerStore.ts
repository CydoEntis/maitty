import { useSyncExternalStore } from 'react'
import { forgetOperations } from './sendOperation'

/**
 * What a conversation's composer holds between views: the unsent text. It lives here, not in the
 * component, so switching to the terminal and back (which closes and reopens the chat) loses
 * nothing. Messages sent while the agent works are not kept here: the daemon holds them.
 */

export type ComposerState = { readonly draft: string }

const EMPTY: ComposerState = { draft: '' }
const stored = new Map<number, ComposerState>()
const listeners = new Set<() => void>()

function read(conversation: number): ComposerState {
  return stored.get(conversation) ?? EMPTY
}

export function updateComposer(conversation: number, change: (state: ComposerState) => ComposerState): void {
  const before = read(conversation)
  const after = change(before)
  if (after === before) return
  if (after.draft === '') stored.delete(conversation)
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
  forgetOperations()
}
