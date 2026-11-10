import { useSyncExternalStore } from 'react'
import type { AttachmentKind } from '@/api/attachments'
import { forgetOperations } from './sendOperation'

/**
 * What a conversation's composer holds between views: the unsent text and the files attached to it. It lives here, not in the
 * component, so switching to the terminal and back (which closes and reopens the chat) loses
 * nothing. Messages sent while a chat-run agent works are not kept here: the daemon holds them. Those written
 * while an agent in a terminal works are kept here as `held`, and typed into it once it is ready.
 */

/** A file kept for the message being written. `preview` is an object URL for a thumbnail, when the window holds the image's bytes. */
export type DraftAttachment = {
  readonly id: string
  readonly path: string
  readonly name: string
  readonly size: number
  readonly kind: AttachmentKind
  readonly preview: string | null
}

/** A message written while a terminal agent was busy. `id` is negative so it never meets a daemon operation number. */
export type HeldMessage = { readonly id: number; readonly text: string }

export type ComposerState = { readonly draft: string; readonly attachments: readonly DraftAttachment[]; readonly held: readonly HeldMessage[] }

const NO_ATTACHMENTS: readonly DraftAttachment[] = []
const NO_HELD: readonly HeldMessage[] = []
const EMPTY: ComposerState = { draft: '', attachments: NO_ATTACHMENTS, held: NO_HELD }
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
  if (after.draft === '' && after.attachments.length === 0 && after.held.length === 0) stored.delete(conversation)
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
 * One part of a conversation's composer. A view that needs only the files or the held messages is
 * not drawn again for each letter typed, which matters for the chat around the box.
 */
export function useComposerSlice<T>(conversation: number, pick: (state: ComposerState) => T): T {
  return useSyncExternalStore(subscribe, () => pick(read(conversation)))
}

/** The unsent text: only the box that shows it follows every change. */
export function useComposerDraft(conversation: number): string {
  return useComposerSlice(conversation, (state) => state.draft)
}

/** For tests: the store belongs to the window's lifetime. */
export function forgetComposers(): void {
  stored.clear()
  forgetOperations()
}
