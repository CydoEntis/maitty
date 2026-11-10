import { sendTerminalInput, submitMessage } from '@/api/daemon'
import { reserveOperation } from '@/features/chat/sendOperation'
import { PASTE_END, PASTE_START } from '@/features/chat/useChatComposer'
import { peekChatFeed } from '@/features/chat/useChatFeed'
import { isChatSession } from './chatSession'
import type { SessionView } from './session'

/** The agent's name as a button says it: `claude` is Claude. */
export function agentName(process: string): string {
  return process.charAt(0).toUpperCase() + process.slice(1)
}

/**
 * Sends the session's agent a message from outside its message box (a comment on a diff line). A chat
 * session takes it as its next message; an agent in a terminal has it pasted in and entered, as the
 * message box does. An agent that is asking something is not typed into: the text would answer its question.
 */
export async function sendAgentMessage(session: SessionView, text: string): Promise<void> {
  if (session.status === 'needs') throw new Error(`${agentName(session.process)} is asking something. Answer it first.`)
  const chat = session.conversation.chat
  if (isChatSession(session.conversation) && chat !== undefined) {
    // The message is numbered after everything the chat has seen; without the chat's own reading there is no safe number.
    const feed = peekChatFeed(chat.conversation)
    if (feed === undefined) throw new Error('Open the chat once, then send the comment again.')
    await submitMessage({ id: String(chat.conversation), generation: chat.generation }, reserveOperation(chat.conversation, feed), text)
    return
  }
  await sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${PASTE_START}${text}${PASTE_END}\r`)
}

const COMMIT_SUBJECT_MAX = 72

/** A first commit message for the session's changes: its name, else the first line of what the agent was last asked. */
export function suggestedCommitMessage(session: Pick<SessionView, 'name' | 'lastPrompt'>): string {
  const line = (session.name ?? session.lastPrompt.split('\n')[0] ?? '').trim()
  return line.length > COMMIT_SUBJECT_MAX ? `${line.slice(0, COMMIT_SUBJECT_MAX - 1).trimEnd()}…` : line
}
