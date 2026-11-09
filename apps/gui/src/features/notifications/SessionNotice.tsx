import { AppIcon, StatusBadge } from '@/features/workspace/components/glyphs'
import type { SessionStatus } from '@/features/workspace/model/session'
import { NotificationCard, type NotificationAction } from './NotificationCard'

export type SessionNoticeKind = 'needs' | 'done' | 'error'

export type SessionNoticeProps = {
  kind: SessionNoticeKind
  /** The program in the session, for its icon. */
  process: string
  status: SessionStatus
  title: string
  /** What happened to it: "needs you", "finished", "failed". */
  state: string
  body: string
  actions: readonly NotificationAction[]
  /** When set, a thin line along the bottom shrinks over this many milliseconds. */
  countdownMs?: number
}

/** The card for a session you are not looking at: its icon and status, what happened, and what you can do about it. */
export function SessionNotice({ kind, process, status, title, state, body, actions, countdownMs, onDismiss }: SessionNoticeProps & { onDismiss: () => void }) {
  return (
    <NotificationCard
      tone={kind}
      icon={<span className="mx-notice-app"><AppIcon process={process} size={26} /><StatusBadge status={status} /></span>}
      title={title}
      state={state}
      body={body}
      actions={actions}
      time="just now"
      onClose={onDismiss}
      {...(countdownMs === undefined ? {} : { countdownMs })}
    />
  )
}
