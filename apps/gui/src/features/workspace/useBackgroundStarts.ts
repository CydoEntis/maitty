import { useCallback, useEffect, useRef, useState } from 'react'
import type { DraftAttachment } from '@/features/chat/composerStore'
import { AGENT_LABEL } from '@/features/chat/modelName'
import { revokePreviews } from '@/features/chat/useAttachmentIntake'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { useLatest } from '@/lib/useLatest'
import type { NewChatRequest, NewChatStartOptions } from './components/NewChatPage'
import { nameFromMessage } from './model/newSession'

/** What a start in the background took out of the message box, given back as it was when the start fails. */
export type StartDraft = { text: string; attachments: readonly DraftAttachment[] }

/**
 * A start in the background that failed: named by its message (and its agent, when one prompt went to several), with the
 * reason and what it took from the box. Every start of one prompt shares a `group` and its draft; `draft` is null where
 * there is nothing to put back: the box still holds the message, or another failed start of the prompt put it back.
 */
export type FailedStart = { key: number; group: number; name: string; agent: string | null; reason: string; draft: StartDraft | null }

/** How a failed start is named: by the words its session would have been named with, and its agent when one prompt went to several. */
export const failedStartLabel = ({ name, agent }: Pick<FailedStart, 'name' | 'agent'>): string =>
  `${name === '' ? 'the empty chat' : `“${name}”`}${agent === null ? '' : ` with ${agent}`}`

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
  // A prompt's files are let go once none of its starts is running or failed and still holding them; putting them back
  // hands them to the box, and its group stops counting.
  const holders = useRef(new Map<number, number>())
  const failedNow = useLatest(() => failed)
  // Files of a failed start that was never put back are not kept once the page is gone.
  useEffect(() => {
    open.current = true
    return () => {
      open.current = false
      for (const start of failedNow()) if (start.draft !== null) revokePreviews(start.draft.attachments)
    }
  }, [failedNow])

  const release = useCallback((group: number, draft: StartDraft) => {
    const left = holders.current.get(group)
    if (left === undefined) return
    if (left > 1) holders.current.set(group, left - 1)
    else {
      holders.current.delete(group)
      revokePreviews(draft.attachments)
    }
  }, [])

  /** Starts each request with the same message; `draft` is what goes back to the box should one fail, null when the box keeps it. */
  const launch = useCallback((requests: readonly NewChatRequest[], draft: StartDraft | null) => {
    lastKey.current += 1
    const group = lastKey.current
    if (draft !== null) holders.current.set(group, requests.length)
    const several = requests.length > 1
    for (const request of requests) {
      setPending((count) => count + 1)
      onStart(request, { background: true })
        .then(() => { if (draft !== null) release(group, draft) })
        .catch((failure: unknown) => {
          const reason = errorMessage(failure, 'Could not start the chat')
          const start = { name: nameFromMessage(request.message), agent: several ? AGENT_LABEL[request.agent] : null }
          if (!open.current) {
            if (draft !== null) release(group, draft)
            toast.error(`Could not start ${failedStartLabel(start)}: ${reason}`)
            return
          }
          lastKey.current += 1
          const key = lastKey.current
          // Once another start of the prompt put it back, the box holds the draft: this one has nothing to give back.
          setFailed((now) => [...now, { ...start, key, group, reason, draft: holders.current.has(group) ? draft : null }])
        })
        .finally(() => { setPending((count) => count - 1) })
    }
  }, [onStart, release])

  const forget = (key: number) => { setFailed((now) => now.filter((start) => start.key !== key)) }
  // A prompt's message goes back once, whichever of its failed starts asks: the others keep only Dismiss.
  const takeBack = (key: number): StartDraft | null => {
    const start = failed.find((candidate) => candidate.key === key)
    if (start?.draft == null) return null
    holders.current.delete(start.group)
    setFailed((now) => now.filter((other) => other.key !== key).map((other) => (other.group === start.group ? { ...other, draft: null } : other)))
    return start.draft
  }
  const dismiss = (key: number) => {
    const start = failed.find((candidate) => candidate.key === key)
    if (start?.draft != null) release(start.group, start.draft)
    forget(key)
  }
  return { pending, failed, launch, takeBack, dismiss }
}
