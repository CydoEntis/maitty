import { useSyncExternalStore } from 'react'
import type { AttachmentKind } from '@/api/attachments'
import { forgetOperations } from './sendOperation'

/**
 * What a conversation's composer holds between views: the unsent text and the files attached to it. It lives here, not in the
 * component, so switching to the terminal and back (which closes and reopens the chat) loses
 * nothing. Messages sent while an agent works are not kept here: its daemon holds them, for a chat-run agent
 * and for one in a terminal alike, so they go in whether or not this window is open.
 */

/**
 * A file kept for the message being written. `preview` is an object URL for a thumbnail, when the window holds the image's bytes.
 * `size` is null for a file that came back with a message taken out of the queue: the message names its path, not its size.
 */
export type DraftAttachment = {
  readonly id: string
  readonly path: string
  readonly name: string
  readonly size: number | null
  readonly kind: AttachmentKind
  readonly preview: string | null
}

export type ComposerState = { readonly draft: string; readonly attachments: readonly DraftAttachment[] }

const NO_ATTACHMENTS: readonly DraftAttachment[] = []
const EMPTY: ComposerState = { draft: '', attachments: NO_ATTACHMENTS }
const stored = new Map<number, ComposerState>()
const listeners = new Set<() => void>()

/** The state now, outside React, for code that must see the latest. */
export function peekComposer(conversation: number): ComposerState {
  return read(conversation)
}

function read(conversation: number): ComposerState {
  return stored.get(conversation) ?? EMPTY
}

export function updateComposer(conversation: number, change: (state: ComposerState) => ComposerState): void {
  const before = read(conversation)
  const after = change(before)
  if (after === before) return
  if (after.draft === '' && after.attachments.length === 0) stored.delete(conversation)
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

/**
 * One part of a conversation's composer. A view that needs only the files is
 * not drawn again for each letter typed, which matters for the chat around the box.
 */
export function useComposerSlice<T>(conversation: number, pick: (state: ComposerState) => T): T {
  return useSyncExternalStore(subscribe, () => pick(read(conversation)))
}

/** The unsent text: only the box that shows it follows every change. */
export function useComposerDraft(conversation: number): string {
  return useComposerSlice(conversation, (state) => state.draft)
}

/** The conversation each session's chat last had, so what its box holds can follow it to a new one. */
const lastConversation = new Map<string, number>()

/**
 * Says which conversation a session's chat is now. The daemon gives a pane's agent a new conversation
 * number when it stops recognising the agent for a moment and then finds it again, and the box is kept by
 * that number: without this the text being written vanished with the old one. What the box held moves
 * to the new number, unless something was already written there.
 */
export function followComposer(sessionId: string, conversation: number): void {
  const before = lastConversation.get(sessionId)
  lastConversation.set(sessionId, conversation)
  if (before === undefined || before === conversation) return
  const state = stored.get(before)
  if (state === undefined || stored.has(conversation)) return
  stored.delete(before)
  stored.set(conversation, state)
  for (const listener of listeners) listener()
}

/** For tests: the store belongs to the window's lifetime. */
export function forgetComposers(): void {
  stored.clear()
  lastConversation.clear()
  forgetOperations()
}
