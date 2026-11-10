import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

const ConversationChangedSchema = z.object({
  conversation: z.number().int().nonnegative(),
  generation: z.number().int().nonnegative(),
})

/**
 * Calls back with a conversation's number each time this computer's daemon says it changed, so an
 * open chat reads it then. Says only that it changed: what changed is read as always. Never calls
 * back outside the desktop app, where the chat keeps asking on its own.
 */
export function listenConversationChanged(onChange: (conversation: number) => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined)
  return listen<unknown>('conversation-changed', (event) => {
    const changed = ConversationChangedSchema.safeParse(event.payload)
    if (changed.success) onChange(changed.data.conversation)
  })
}
