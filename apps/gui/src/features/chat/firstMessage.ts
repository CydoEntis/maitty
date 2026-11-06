/**
 * What a person typed into a chat before its agent was ready, kept per session until the chat that
 * can send it exists. A new session's chat opens at once; its conversation appears a few seconds
 * later, when the agent first reports in, and nothing typed in between may be lost.
 */
import { useSyncExternalStore } from 'react'

const heldMessages = new Map<string, string>()
const carriedDrafts = new Map<string, string>()
const listeners = new Set<() => void>()

const notify = () => { for (const listener of listeners) listener() }
const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

/** A message sent while the agent was starting; it goes out as soon as the chat can send. */
export function holdFirstMessage(sessionId: string, text: string): void {
  heldMessages.set(sessionId, text)
  notify()
}

export function heldFirstMessage(sessionId: string): string | null {
  return heldMessages.get(sessionId) ?? null
}

export function releaseFirstMessage(sessionId: string): void {
  heldMessages.delete(sessionId)
  notify()
}

/** The message held for a session, kept up to date as it is held and released; null when none or no session. */
export function useHeldFirstMessage(sessionId: string | undefined): string | null {
  return useSyncExternalStore(subscribe, () => (sessionId === undefined ? null : heldMessages.get(sessionId) ?? null), () => null)
}

/** Text typed but not sent, handed from the starting chat to the real one. Empty text clears it. */
export function carryDraft(sessionId: string, text: string): void {
  if (text === '') carriedDrafts.delete(sessionId)
  else carriedDrafts.set(sessionId, text)
}

/** The carried draft, once: reading it clears it. */
export function takeCarriedDraft(sessionId: string): string | null {
  const text = carriedDrafts.get(sessionId) ?? null
  carriedDrafts.delete(sessionId)
  return text
}

/** For tests. */
export function forgetFirstMessages(): void {
  heldMessages.clear()
  carriedDrafts.clear()
  notify()
}
