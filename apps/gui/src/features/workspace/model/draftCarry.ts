import { sendTerminalInput } from '@/api/daemon'
import { peekComposer, updateComposer } from '@/features/chat/composerStore'
import { PASTE_END, PASTE_START } from '@/features/chat/useChatComposer'
import { isChatSession } from './chatSession'
import type { SessionView } from './session'

/**
 * Moves the text waiting in a session's chat box into its agent's own prompt, for a pane going from
 * chat to terminal: one paste and no Enter, so it can be finished there. It follows the message box's
 * rule and types only while the pane's status says the agent is idle or done, since typed text
 * would otherwise answer a question or land in a running turn. Text with files attached stays in
 * the box, because the files only travel with a message sent from it.
 */
export async function carryDraftToTerminal(session: SessionView): Promise<void> {
  const chat = session.conversation.chat
  if (chat === undefined || isChatSession(session.conversation)) return
  if (session.status !== 'idle' && session.status !== 'done') return
  const { draft, attachments } = peekComposer(chat.conversation)
  if (draft.trim() === '' || attachments.length > 0) return
  await sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${PASTE_START}${draft}${PASTE_END}`)
  // Only what was typed leaves the box: text written while the paste was on its way stays.
  updateComposer(chat.conversation, (state) => (state.draft === draft ? { ...state, draft: '' } : state))
}
