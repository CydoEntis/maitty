import type { Conversation } from '@/data/preview'

/**
 * A chat session is a conversation the daemon started itself as a structured Claude or Codex run
 * (ADR 0054). It has no terminal: the window shows its chat and nothing else, and writes to it.
 * A terminal session, and the read-only conversation that follows an agent in a pane, are not one.
 */
export function isChatSession(conversation: Pick<Conversation, 'kind' | 'execution' | 'chat'>): boolean {
  return conversation.kind !== 'terminal' && conversation.execution === 'Structured' && conversation.chat !== undefined
}

/** A chat session that is not put away (closing one stops its agent and puts it away). */
export function isListedChatSession(conversation: Pick<Conversation, 'kind' | 'execution' | 'chat' | 'archived'>): boolean {
  return isChatSession(conversation) && conversation.archived !== true
}
