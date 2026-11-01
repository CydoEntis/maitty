import type { Conversation, SessionStatus } from './preview'

export type ConversationNotification = {
  conversationId: string
  title: string
  kind: 'attention' | 'finished'
}

export type ConversationStatusAnnouncement = {
  conversationId: string
  title: string
  kind: 'finished' | 'needs you' | 'needs recovery'
}

export function findConversationStatusAnnouncements(
  previousStatuses: ReadonlyMap<string, SessionStatus>,
  conversations: Conversation[],
): ConversationStatusAnnouncement[] {
  const announcements: ConversationStatusAnnouncement[] = []

  for (const conversation of conversations) {
    const previousStatus = previousStatuses.get(conversation.id)
    if (!previousStatus || previousStatus === conversation.status) continue

    if (conversation.status === 'needs-you') {
      announcements.push({ conversationId: conversation.id, title: conversation.title, kind: 'needs you' })
    } else if (conversation.status === 'recovery') {
      announcements.push({ conversationId: conversation.id, title: conversation.title, kind: 'needs recovery' })
    } else if (conversation.status === 'done' && previousStatus === 'working') {
      announcements.push({ conversationId: conversation.id, title: conversation.title, kind: 'finished' })
    }
  }

  return announcements
}

export function findConversationNotifications(
  previousStatuses: ReadonlyMap<string, SessionStatus>,
  conversations: Conversation[],
  selectedId: string | undefined,
): ConversationNotification[] {
  const notifications: ConversationNotification[] = []

  for (const conversation of conversations) {
    const previousStatus = previousStatuses.get(conversation.id)
    if (!previousStatus || previousStatus === conversation.status || conversation.id === selectedId) continue

    if (conversation.status === 'needs-you' || conversation.status === 'recovery') {
      notifications.push({ conversationId: conversation.id, title: conversation.title, kind: 'attention' })
    } else if (conversation.status === 'done' && previousStatus === 'working') {
      notifications.push({ conversationId: conversation.id, title: conversation.title, kind: 'finished' })
    }
  }

  return notifications
}
