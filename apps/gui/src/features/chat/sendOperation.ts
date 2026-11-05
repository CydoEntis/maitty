/**
 * The operation number a send carries. The daemon uses it to tell a retry of one message from a new
 * message, so every sender of a conversation (the chat's composer, the session panel's comments)
 * draws from the same numbers and never reuses one.
 */

/** The daemon refuses an operation number it has already used or passed; a retry must pick a new one. */
export const STALE_OPERATION = /stale or reused/i

type Known = {
  /** The operation of the last message the daemon delivered, if any. */
  receiptOperation: number | null | undefined
  /** The operations of the messages the daemon holds for the running turn. */
  queuedOperations: readonly number[]
  /** The highest number this window has used for the conversation. */
  lastUsed: number
}

/** The first number above everything the daemon and this window have seen. */
export function nextOperation({ receiptOperation, queuedOperations, lastUsed }: Known): number {
  return Math.max(receiptOperation ?? 0, ...queuedOperations, lastUsed) + 1
}

// Kept per conversation, outside any component, so two senders in one window never draw the same number.
const lastUsed = new Map<number, number>()

/** The feed's view of the conversation, as far as numbering needs it. */
type Seen = {
  receipt: { readonly operation: number } | null
  queued: readonly { readonly operation: number }[]
}

/** Takes a new operation number for the conversation and records it as used. */
export function reserveOperation(conversation: number, seen: Seen): number {
  const operation = nextOperation({
    receiptOperation: seen.receipt?.operation,
    queuedOperations: seen.queued.map((message) => message.operation),
    lastUsed: lastUsed.get(conversation) ?? 0,
  })
  lastUsed.set(conversation, operation)
  return operation
}

/** For tests: the numbers belong to the window's lifetime. */
export function forgetOperations(): void {
  lastUsed.clear()
}
