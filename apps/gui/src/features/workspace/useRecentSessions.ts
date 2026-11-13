import { useCallback, useState } from 'react'
import type { ClosedSession } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import type { NewChatRequest } from './components/NewChatPage'
import { reopenRequest } from './model/recentSessions'

/** The Recent sessions list: whether it is open, and reopening a session from it through the new chat's resume start. */
export function useRecentSessions(startNewChat: (request: NewChatRequest) => Promise<void>, starting: boolean) {
  const [recentOpen, setRecentOpen] = useState(false)
  const reopenSession = useCallback((closed: ClosedSession) => {
    const request = reopenRequest(closed)
    if (request === null) return
    // A start already under way would drop this one without a word, so the list stays and says why.
    if (starting) {
      toast.message('A session is already starting')
      return
    }
    setRecentOpen(false)
    startNewChat(request).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not reopen the session')) })
  }, [startNewChat, starting])
  return { recentOpen, setRecentOpen, reopenSession }
}
