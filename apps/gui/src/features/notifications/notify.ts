import { createElement } from 'react'
import { toast } from '@/lib/toast'
import { NoticeToast, type NoticeProps } from './NoticeToast'
import { SessionNotice, type SessionNoticeProps } from './SessionNotice'

/** Show a notification card. Sticky ones stay until answered or dismissed. */
export function showNotice(
  notice: NoticeProps,
  options: { sticky?: boolean } = {},
): string | number {
  const duration = options.sticky ? Infinity : notice.countdownMs
  return toast.custom(
    (id) =>
      createElement(NoticeToast, {
        ...notice,
        onDismiss: () => {
          toast.dismiss(id)
        },
      }),
    duration === undefined ? {} : { duration },
  )
}

/** Takes a session's card away, e.g. once the session is no longer waiting on you. */
export function dismissSessionNotice(sessionId: string): void {
  toast.dismiss(`session-${sessionId}`)
}

/**
 * A card for a terminal session. A newer one for the same session replaces the older, so a session never
 * stacks cards; one that needs you stays until you answer it, the others go by themselves.
 */
export function showSessionNotice(sessionId: string, notice: SessionNoticeProps): string | number {
  return toast.custom(
    (id) => createElement(SessionNotice, { ...notice, onDismiss: () => { toast.dismiss(id) } }),
    { id: `session-${sessionId}`, duration: notice.countdownMs ?? Infinity },
  )
}
