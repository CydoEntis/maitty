import type { Conversation } from './preview'

export function markConversationsDisconnected(conversations: Conversation[]): Conversation[] {
  return conversations.map((conversation) => {
    const lastKnownStatus = conversation.lastKnownStatus ?? (conversation.status === 'unknown' ? undefined : conversation.status)
    return {
      ...conversation,
      ...(lastKnownStatus ? { lastKnownStatus } : {}),
      status: 'unknown',
    }
  })
}
