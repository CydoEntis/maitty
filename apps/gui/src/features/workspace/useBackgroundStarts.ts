import { useCallback, useEffect, useRef, useState } from 'react'
import type { DraftAttachment } from '@/features/chat/composerStore'
import { revokePreviews } from '@/features/chat/useAttachmentIntake'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { useLatest } from '@/lib/useLatest'
import type { NewChatRequest, NewChatStartOptions } from './components/NewChatPage'
import { nameFromMessage } from './model/newSession'

/** What a start in the background took out of the message box, given back as it was when the start fails. */
export type StartDraft = { text: string; attachments: readonly DraftAttachment[] }

/** A start in the background that failed: named by its message, with the reason and what it took from the box. */
export type FailedStart = { key: number; name: string; reason: string; draft: StartDraft }

/** How a failed start is named: by the words its session would have been named with. */
export const failedStartLabel = (name: string): string => (name === '' ? 'the empty chat' : `“${name}”`)

/**
 * The new chat page's starts in the background: each runs on its own while the page is written in again. `pending`
 * counts the ones still starting; a failed one is kept in `failed` until it is taken back or dismissed, and is a toast
 * instead once the page is gone.
 */
export function useBackgroundStarts(onStart: (request: NewChatRequest, options?: NewChatStartOptions) => Promise<void>) {
  const [pending, setPending] = useState(0)
  const [failed, setFailed] = useState<readonly FailedStart[]>([])
  const open = useRef(true)
  const lastKey = useRef(0)
  const failedNow = useLatest(() => failed)
  // Files of a failed start that was never put back are not kept once the page is gone.
  useEffect(() => {
    open.current = true
    return () => {
      open.current = false
      for (const start of failedNow()) revokePreviews(start.draft.attachments)
    }
  }, [failedNow])

  const launch = useCallback((request: NewChatRequest, draft: StartDraft) => {
    setPending((count) => count + 1)
    onStart(request, { background: true })
      .then(() => { revokePreviews(draft.attachments) })
      .catch((failure: unknown) => {
        const reason = errorMessage(failure, 'Could not start the chat')
        const name = nameFromMessage(draft.text)
        if (!open.current) {
          revokePreviews(draft.attachments)
          toast.error(`Could not start ${failedStartLabel(name)}: ${reason}`)
          return
        }
        lastKey.current += 1
        const key = lastKey.current
        setFailed((now) => [...now, { key, name, reason, draft }])
      })
      .finally(() => { setPending((count) => count - 1) })
  }, [onStart])

  const forget = (key: number) => { setFailed((now) => now.filter((start) => start.key !== key)) }
  const takeBack = (key: number): StartDraft | null => {
    forget(key)
    return failed.find((start) => start.key === key)?.draft ?? null
  }
  const dismiss = (key: number) => {
    revokePreviews(failed.find((start) => start.key === key)?.draft.attachments ?? [])
    forget(key)
  }
  return { pending, failed, launch, takeBack, dismiss }
}
