import { useCallback, useEffect, useReducer } from 'react'
import { submitMessage } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { hasPendingApproval, type ChatFeed } from '@/features/chat/chatFeed'
import { reserveOperation, STALE_OPERATION } from '@/features/chat/sendOperation'
import type { ChatLink } from '@/features/workspace/model/chatView'

export const COMMENT_READ_ONLY_REASON = 'Read-only: talk to the agent in the terminal'

/** A comment the panel has not yet got to the daemon. `operation` is fixed at the first try, so a retry is the same message. */
type Waiting = { readonly text: string; operation: number | null }

// Kept outside the component so a comment written while the daemon is away is not lost when the panel closes.
const waiting = new Map<number, Waiting[]>()
const inFlight = new Set<number>()

/** For tests: the comments belong to the window's lifetime. */
export function forgetCommentDelivery(): void {
  waiting.clear()
  inFlight.clear()
}

type Input = {
  link: ChatLink
  feed: ChatFeed
  online: boolean
  refresh: () => void
}

/**
 * Sends the panel's line comments to the daemon as ordinary messages. The daemon holds one that
 * arrives while the agent works and delivers it when the turn ends, so the panel sends at once. It
 * sends nothing while an approval waits or the daemon is away; the comment waits here and goes, in
 * order, once that is over. A read-only chat takes no comments.
 */
export function useCommentDelivery({ link, feed, online, refresh }: Input) {
  const conversation = link.conversation
  const readOnly = link.readOnly === true
  const blocked = readOnly || !online || hasPendingApproval(feed)
  const [changes, wake] = useReducer((count: number) => count + 1, 0)

  const send = useCallback((text: string) => {
    waiting.set(conversation, [...(waiting.get(conversation) ?? []), { text, operation: null }])
    wake()
  }, [conversation])

  // Runs again when a comment is added or a send ends, and whenever the feed changes, which is also how a failed send is tried again.
  useEffect(() => {
    const head = waiting.get(conversation)?.[0]
    if (head === undefined || blocked || inFlight.has(conversation)) return
    head.operation ??= reserveOperation(conversation, feed)
    inFlight.add(conversation)
    submitMessage({ id: String(conversation), generation: link.generation }, head.operation, head.text)
      .then(() => {
        inFlight.delete(conversation)
        waiting.set(conversation, (waiting.get(conversation) ?? []).filter((item) => item !== head))
        refresh()
        wake()
      })
      .catch((failure: unknown) => {
        inFlight.delete(conversation)
        // The daemon passed that number (another sender used it): read, then number the comment again.
        // Any other failure waits for the next read of the feed, so a daemon that stays down is not hammered.
        if (STALE_OPERATION.test(errorMessage(failure, ''))) {
          head.operation = null
          refresh()
        }
      })
  }, [conversation, link.generation, blocked, feed, refresh, changes])

  return { send, readOnly }
}
