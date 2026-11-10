import { type Dispatch, type SetStateAction, useCallback } from 'react'
import { displayName, type SessionView } from './model/session'
import { toast } from '@/lib/toast'
import { sendTerminalInput } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { isChatSession } from './model/chatSession'
import { ANSWER_KEYS } from '@/features/chat/permissionAsk'

/** Answering a session that asks, resuming one that was interrupted, and typing into a session. */
export function useSessionReplies({ sessionMap, flash, showSessionById, setDismissedRestores }: {
  sessionMap: Map<string, SessionView>
  flash: (text: string) => void
  showSessionById: (sessionId: string) => void
  setDismissedRestores: Dispatch<SetStateAction<ReadonlySet<string>>>
}) {
  const sendTo = useCallback((target: SessionView, data: string) => {
    // A chat session has no terminal to type into; its answers go through the chat, by request id.
    if (isChatSession(target.conversation)) return Promise.reject(new Error('A chat session has no terminal. Use its chat.'))
    return sendTerminalInput({ id: target.id, generation: target.conversation.generation ?? 0 }, data)
  }, [])

  const answerSession = useCallback((sessionId: string, answer: 'y' | 'a' | 'n') => {
    const target = sessionMap.get(sessionId)
    if (!target) return
    // A card can outlast the question. Typing into a session that is not asking would land in its shell.
    if (target.status !== 'needs') {
      flash(`${displayName(target)} is not asking any more`)
      return
    }
    // The answer is typed into the session's own prompt; the GUI never decides for it.
    // Claude Code's permission menu numbers its choices and ignores a bare `y`; only that key is verified.
    const keys = target.process === 'claude' && answer === 'y' ? ANSWER_KEYS.approve : answer
    void sendTo(target, keys)
      .then(() => { flash(`Sent “${answer}” to ${displayName(target)}`) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not answer the session')) })
  }, [sessionMap, sendTo, flash])


  // Types the resume command into the session, as you would. Nothing resumes without this click.
  const resumeInterrupted = useCallback((target: SessionView) => {
    const command = target.interrupted?.resume
    if (!command) return
    showSessionById(target.id)
    void sendTo(target, `${command}
`)
      .then(() => { setDismissedRestores((current) => new Set(current).add(target.id)) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not resume the agent')) })
  }, [sendTo, setDismissedRestores, showSessionById])

  return { answerSession, sendTo, resumeInterrupted }
}
