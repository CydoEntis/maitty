import { useEffect, useRef } from 'react'
import { sendTerminalInput } from '@/api/daemon'
import { heldClaimed, peekComposer, updateComposer, useHeldConversations } from '@/features/chat/composerStore'
import { announceOutsideSend } from '@/features/chat/outsideSends'
import { HELD_GAP_MS, typeMessage } from '@/features/chat/useChatComposer'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { isChatSession } from './model/chatSession'
import type { SessionView } from './model/session'

/**
 * Sends the messages queued for an agent in a terminal when its chat is not on screen. They are held by the window
 * (the daemon holds a chat session's own), and the message box sends them while it shows; without this they waited
 * until the chat was opened again, so a queue stopped after each turn and the session read "finished" with work
 * still to do. One goes when the pane's status says the agent is idle or done (never into a question), and the
 * next waits for the turn that one starts.
 */
export function useHeldMessages({ sessions, online }: { sessions: readonly SessionView[]; online: boolean }): void {
  const waiting = useHeldConversations()
  const sentAt = useRef(new Map<number, number>())
  const going = useRef(new Set<number>())
  useEffect(() => {
    if (!online || waiting.length === 0) return
    const timers = sessions.flatMap((session) => {
      const chat = session.conversation.chat
      if (chat?.readOnly !== true || isChatSession(session.conversation)) return []
      const conversation = chat.conversation
      if (!waiting.includes(conversation) || heldClaimed(conversation) || going.current.has(conversation)) return []
      if (session.status !== 'idle' && session.status !== 'done') return []
      const wait = Math.max(0, (sentAt.current.get(conversation) ?? 0) + HELD_GAP_MS - Date.now())
      return [window.setTimeout(() => {
        const first = peekComposer(conversation).held[0]
        if (first === undefined || heldClaimed(conversation) || going.current.has(conversation)) return
        going.current.add(conversation)
        sentAt.current.set(conversation, Date.now())
        updateComposer(conversation, (state) => ({ ...state, held: state.held.filter((message) => message.id !== first.id) }))
        typeMessage((text) => sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, text), session.process, first.text)
          .then(
            () => { announceOutsideSend(conversation, first.text) },
            (failure: unknown) => {
              // One that did not go is put back at the front, to be tried again.
              updateComposer(conversation, (state) => ({ ...state, held: [first, ...state.held] }))
              toast.error(errorMessage(failure, 'Could not send a queued message'))
            },
          )
          .finally(() => { going.current.delete(conversation) })
      }, wait)]
    })
    return () => { for (const timer of timers) window.clearTimeout(timer) }
  }, [sessions, online, waiting])
}
