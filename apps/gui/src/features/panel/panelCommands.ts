import { useSyncExternalStore } from 'react'

/**
 * Requests the rest of the window makes of the session panel. An edit block's header asks to see
 * that file's diff under Changes, opening the panel first if it is hidden (v18 C.1). The request is
 * a value with a counter, so asking twice for the same file asks twice.
 */

export type DiffRequest = { readonly file: string; readonly nonce: number }

let current: DiffRequest | null = null
let counter = 0
const listeners = new Set<() => void>()

export function requestDiff(file: string): void {
  counter += 1
  current = { file, nonce: counter }
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

export function useDiffRequest(): DiffRequest | null {
  return useSyncExternalStore(subscribe, () => current)
}

/** The panel took the request; it is not offered again to a panel that mounts later. */
export function consumeDiffRequest(nonce: number): void {
  if (current?.nonce !== nonce) return
  current = null
  for (const listener of listeners) listener()
}

/** For tests. */
export function forgetDiffRequest(): void {
  current = null
}
