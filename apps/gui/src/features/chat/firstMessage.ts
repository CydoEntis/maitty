/**
 * What a person typed into a chat before its agent was ready, kept per session until the chat that
 * can send it exists. A new session's chat opens at once; its conversation appears a few seconds
 * later, when the agent first reports in, and nothing typed in between may be lost.
 */
import { useSyncExternalStore } from 'react'
import type { ReasoningPick } from './agentChoice'

const heldMessages = new Map<string, string>()
const heldByDaemon = new Set<string>()
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
  heldByDaemon.delete(sessionId)
  notify()
}

/**
 * Hands a held first message to the session's daemon, which types it in once the agent takes input, so it is
 * delivered even if this window closes first. The window keeps showing it. If the daemon does not take it, the
 * window sends it itself, as it does for a session that has no terminal for the daemon to type into.
 */
export function entrustFirstMessage(sessionId: string, text: string, hand: (sessionId: string, text: string) => Promise<void>): void {
  heldByDaemon.add(sessionId)
  notify()
  // `hand` may refuse before it asks (a session that is not a terminal), which must not stop the window sending it.
  void Promise.resolve().then(() => hand(sessionId, text)).catch((error: unknown) => {
    console.warn('The daemon did not take the first message, so the window sends it', error instanceof Error ? error.message : String(error))
    heldByDaemon.delete(sessionId)
    notify()
  })
}

/** Whether the session's daemon has the held message, and will send it. */
export function useHeldByDaemon(sessionId: string | undefined): boolean {
  return useSyncExternalStore(subscribe, () => sessionId !== undefined && heldByDaemon.has(sessionId), () => false)
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

/**
 * What a new chat was started with: the agent's model and effort, and Claude's permission mode. Kept per session so the message
 * box draws the same chips before the agent reports in and after, and the real chat starts from what was picked.
 */
export type StartChoice = { readonly model: string; readonly reasoning: ReasoningPick; readonly mode: string | null }

const startChoices = new Map<string, StartChoice>()

export function holdStartChoice(sessionId: string, choice: StartChoice): void {
  startChoices.set(sessionId, choice)
  notify()
}

export function startChoiceOf(sessionId: string | undefined): StartChoice | null {
  return sessionId === undefined ? null : startChoices.get(sessionId) ?? null
}

export function releaseStartChoice(sessionId: string): void {
  if (startChoices.delete(sessionId)) notify()
}

export function useStartChoice(sessionId: string | undefined): StartChoice | null {
  return useSyncExternalStore(subscribe, () => startChoiceOf(sessionId), () => null)
}

const draftKeys = new Map<string, number>()
let nextDraftKey = -2

/**
 * The key a starting session's attached files are kept under in the composer store. No conversation exists yet, and a
 * real one is never negative; -1 belongs to the new-chat page.
 */
export function draftKeyFor(sessionId: string): number {
  const known = draftKeys.get(sessionId)
  if (known !== undefined) return known
  const key = nextDraftKey
  nextDraftKey -= 1
  draftKeys.set(sessionId, key)
  return key
}

/** For tests. */
export function forgetFirstMessages(): void {
  heldMessages.clear()
  heldByDaemon.clear()
  carriedDrafts.clear()
  startChoices.clear()
  notify()
}
