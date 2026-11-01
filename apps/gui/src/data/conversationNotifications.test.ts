import { describe, expect, it } from 'vitest'
import { previewConversations, type Conversation, type SessionStatus } from './preview'
import { findConversationNotifications, findConversationStatusAnnouncements } from './conversationNotifications'

describe('conversation notifications', () => {
  it('announces finished, attention and recovery status changes for any conversation', () => {
    const previousStatuses = new Map<string, SessionStatus>([
      ['codex-43', 'working'],
      ['codex-41', 'idle'],
      ['claude-18', 'working'],
      ['codex-12', 'done'],
    ])
    const conversations = [
      conversation('codex-43', 'done'),
      conversation('codex-41', 'needs-you'),
      conversation('claude-18', 'recovery'),
      conversation('codex-12', 'working'),
    ]

    expect(findConversationStatusAnnouncements(previousStatuses, conversations)).toEqual([
      { conversationId: 'codex-43', title: 'codex-43', kind: 'finished' },
      { conversationId: 'codex-41', title: 'codex-41', kind: 'needs you' },
      { conversationId: 'claude-18', title: 'claude-18', kind: 'needs recovery' },
    ])
  })

  it('notifies when another conversation needs attention or finishes', () => {
    const previousStatuses = new Map<string, SessionStatus>([
      ['codex-43', 'working'],
      ['codex-41', 'idle'],
      ['claude-18', 'working'],
    ])
    const conversations = [
      conversation('codex-43', 'done'),
      conversation('codex-41', 'needs-you'),
      conversation('claude-18', 'recovery'),
    ]

    expect(findConversationNotifications(previousStatuses, conversations, undefined)).toEqual([
      { conversationId: 'codex-43', title: 'codex-43', kind: 'finished' },
      { conversationId: 'codex-41', title: 'codex-41', kind: 'attention' },
      { conversationId: 'claude-18', title: 'claude-18', kind: 'attention' },
    ])
  })

  it('does not notify for the selected conversation, unchanged status or idle work', () => {
    const previousStatuses = new Map<string, SessionStatus>([
      ['codex-43', 'working'],
      ['codex-41', 'idle'],
      ['claude-18', 'working'],
    ])
    const conversations = [
      conversation('codex-43', 'needs-you'),
      conversation('codex-41', 'idle'),
      conversation('claude-18', 'working'),
    ]

    expect(findConversationNotifications(previousStatuses, conversations, 'codex-43')).toEqual([])
  })
})

function conversation(id: string, status: SessionStatus): Conversation {
  const preview = previewConversations.find((item) => item.id === id)
  if (!preview) throw new Error(`Missing preview conversation ${id}`)
  return { ...preview, title: id, status }
}
