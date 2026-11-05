import { useSyncExternalStore } from 'react'
import type { AttachmentKind } from '@/api/attachments'
import { forgetOperations } from './sendOperation'

/**
 * What a conversation's composer holds between views: the unsent text and the files attached to it. It lives here, not in the
 * component, so switching to the terminal and back (which closes and reopens the chat) loses
 * nothing. Messages sent while the agent works are not kept here: the daemon holds them.
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

/** For tests: the store belongs to the window's lifetime. */
export function forgetComposers(): void {
  stored.clear()
  forgetOperations()
}
