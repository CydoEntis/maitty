type Listener = (text: string) => void

const listeners = new Map<number, Set<Listener>>()

/**
 * A message that went to a conversation's agent from outside its message box (a comment on a diff line).
 * The chat hears of it here, so it can draw the message at once as it does one typed in the box.
 */
export function announceOutsideSend(conversation: number, text: string): void {
  for (const listener of listeners.get(conversation) ?? []) listener(text)
}

/** Listens for such messages to one conversation; the returned function stops listening. */
export function onOutsideSend(conversation: number, listener: Listener): () => void {
  const set = listeners.get(conversation) ?? new Set<Listener>()
  listeners.set(conversation, set)
  set.add(listener)
  return () => {
    set.delete(listener)
    if (set.size === 0) listeners.delete(conversation)
  }
}
