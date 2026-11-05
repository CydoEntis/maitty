import { useEffect, useSyncExternalStore } from 'react'
import type { ChatEvent } from '@/features/chat'
import { checklistOf } from './panelModel'

/**
 * The checklist progress of the conversations this window has read, for the sidebar's rows (v18 C.2,
 * C.4). The daemon reports no per-session progress, so a row can show it only for a session whose
 * conversation the window has read (its chat or its panel was open); a session never opened has no
 * row of this kind. What is kept is the last list read, not a live feed.
 */

export type ChecklistProgress = {
  readonly done: number
  readonly total: number
  /** The item in progress, when there is one. */
  readonly current: string | null
}

const stored = new Map<number, ChecklistProgress>()
const listeners = new Set<() => void>()

function same(a: ChecklistProgress | undefined, b: ChecklistProgress | null): boolean {
  if (a === undefined || b === null) return a === undefined && b === null
  return a.done === b.done && a.total === b.total && a.current === b.current
}

export function publishProgress(conversation: number, progress: ChecklistProgress | null): void {
  if (same(stored.get(conversation), progress)) return
  if (progress === null) stored.delete(conversation)
  else stored.set(conversation, progress)
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

/** The progress last read for a conversation, or `null` when none is known. */
export function useChecklistProgress(conversation: number | undefined): ChecklistProgress | null {
  return useSyncExternalStore(subscribe, () => (conversation === undefined ? null : stored.get(conversation) ?? null))
}

/** Keeps the store current from a conversation's events while the chat or panel that reads them is open. */
export function usePublishChecklistProgress(conversation: number, events: readonly ChatEvent[]): void {
  useEffect(() => {
    const { checklist } = checklistOf(events)
    publishProgress(conversation, checklist === null ? null : { done: checklist.done, total: checklist.items.length, current: checklist.items.find((item) => item.state === 'current')?.text ?? null })
  }, [conversation, events])
}

/** For tests. */
export function forgetProgress(): void {
  stored.clear()
  for (const listener of listeners) listener()
}
