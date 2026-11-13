/**
 * The operation number a send carries. The daemon uses it to tell a retry of one message from a new
 * message, so every sender of a conversation draws from the same numbers and never reuses one.
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

/** 2^21: the high word's share of a JavaScript number's 53 exact integer bits. */
const HIGH_WORD_RANGE = 0x200000
const LOW_WORD_RANGE = 0x100000000

const WORDS = 2

const randomWords = (): readonly number[] => Array.from(crypto.getRandomValues(new Uint32Array(WORDS)))

/**
 * The operation a message sent into a terminal agent carries: a random nonce, never a count. The daemon keys
 * these by pane, so counters from two windows, or from one window after a reload, would collide and a second
 * message would be acknowledged and never typed. It stays within 53 bits so it reaches the daemon's u64 exactly,
 * and is never zero, which the daemon refuses.
 */
export function terminalSendOperation(random: () => readonly number[] = randomWords): number {
  for (;;) {
    const [high = 0, low = 0] = random()
    const operation = (high % HIGH_WORD_RANGE) * LOW_WORD_RANGE + low
    if (operation !== 0) return operation
  }
}

/** How long after a failed send to the pane's daemon a send of the same text is taken for its retry. */
export const FAILED_SEND_REUSE_MS = 30_000

/** A send to the pane's daemon that ended in failure: it may have been taken all the same. */
export type FailedSend = { readonly text: string; readonly operation: number; readonly at: number }

/**
 * The operation for sending `text` to the pane's daemon. A send that already ended in failure a moment ago, with the
 * same text, is a retry and keeps its number; anything else, including a send still pending, is a new message.
 */
export function paneSendOperation(failed: FailedSend | null, text: string, now: number, draw: () => number = terminalSendOperation): number {
  return failed !== null && failed.text === text && now - failed.at <= FAILED_SEND_REUSE_MS ? failed.operation : draw()
}

/** For tests: the numbers belong to the window's lifetime. */
export function forgetOperations(): void {
  lastUsed.clear()
}
