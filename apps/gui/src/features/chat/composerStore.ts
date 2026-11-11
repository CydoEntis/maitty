import { useSyncExternalStore } from 'react'
import type { AttachmentKind } from '@/api/attachments'
import { forgetOperations } from './sendOperation'

/**
 * What a conversation's composer holds between views: the unsent text and the files attached to it. It lives here, not in the
 * component, so switching to the terminal and back (which closes and reopens the chat) loses
 * nothing. Messages sent while a chat-run agent works are not kept here: the daemon holds them. Those written
 * while an agent in a terminal works are kept here as `held`, and typed into it once it is ready.
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

const NO_CONVERSATIONS: readonly number[] = []
let waiting: readonly number[] = NO_CONVERSATIONS

/** The conversations with a message held for their agent, the same list until it changes. */
function conversationsWithHeld(): readonly number[] {
  const now = [...stored].filter(([, state]) => state.held.length > 0).map(([conversation]) => conversation)
  if (now.length !== waiting.length || now.some((conversation, index) => conversation !== waiting[index])) waiting = now.length === 0 ? NO_CONVERSATIONS : now
  return waiting
}

/** Which conversations hold a message for their agent, for the window to send them whichever chat is showing. */
export function useHeldConversations(): readonly number[] {
  return useSyncExternalStore(subscribe, conversationsWithHeld)
}

/** How many messages wait for the conversation's agent in this window. */
export function heldCount(conversation: number): number {
  return read(conversation).held.length
}

/** The message boxes on screen that send their own held messages (they draw each as it goes), by conversation. */
const sending = new Map<number, number>()

/** Says a message box for the conversation is on screen and sends its held messages itself; returns the undo. */
export function claimHeld(conversation: number): () => void {
  sending.set(conversation, (sending.get(conversation) ?? 0) + 1)
  return () => {
    const left = (sending.get(conversation) ?? 1) - 1
    if (left > 0) sending.set(conversation, left)
    else sending.delete(conversation)
  }
}

export function heldClaimed(conversation: number): boolean {
  return sending.has(conversation)
}

/** For tests: the store belongs to the window's lifetime. */
export function forgetComposers(): void {
  stored.clear()
  sending.clear()
  waiting = NO_CONVERSATIONS
  lastConversation.clear()
  forgetOperations()
}
