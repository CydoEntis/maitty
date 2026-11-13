import { useCallback, useState } from 'react'
import type { ClosedSession } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import type { NewChatRequest } from './components/NewChatPage'
import { reopenRequest } from './model/recentSessions'

/** The Recent sessions list: whether it is open, and reopening a session from it through the new chat's resume start. */
export function useRecentSessions(startNewChat: (request: NewChatRequest) => Promise<void>) {
  const [recentOpen, setRecentOpen] = useState(false)
  const reopenSession = useCallback((closed: ClosedSession) => {
    const request = reopenRequest(closed)
    if (request === null) return
    setRecentOpen(false)
    startNewChat(request).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not reopen the session')) })
  }, [startNewChat])
  return { recentOpen, setRecentOpen, reopenSession }
}
