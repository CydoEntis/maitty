import type { ReactNode } from 'react'
import { StatusMark, type StatusMarkState } from '@/components/StatusMark'
import { NotificationCard, type NotificationAction, type NotificationTone } from './NotificationCard'

export type NoticeKind = 'needs' | 'recovery' | 'finished' | 'error' | 'info'

export type NoticeAction = NotificationAction

export type NoticeProps = {
  kind: NoticeKind
  project?: string
  title: string
  body?: ReactNode
  actions?: NoticeAction[]
  /** When set, a bar along the bottom shrinks over this many milliseconds. */
  countdownMs?: number
}

const KIND: Record<NoticeKind, { tone: NotificationTone; mark: StatusMarkState; verb: string }> = {
  needs: { tone: 'needs', mark: 'needs-you', verb: 'needs you' },
  recovery: { tone: 'error', mark: 'recovery', verb: 'needs recovery' },
  finished: { tone: 'done', mark: 'done', verb: 'finished' },
  error: { tone: 'error', mark: 'recovery', verb: 'failed' },
  info: { tone: 'info', mark: 'idle', verb: '' },
}

/** A card for the conversation views: the same card as every other notification, with a status mark for its icon. */
export function NoticeToast({ kind, project, title, body, actions = [], countdownMs, onDismiss }: NoticeProps & { onDismiss: () => void }) {
  const { tone, mark, verb } = KIND[kind]
  const heading = verb === '' ? title : `${title} ${verb}`
  return (
    <NotificationCard
      tone={tone}
      icon={<span className="mx-notice-mark"><StatusMark status={mark} /></span>}
      title={heading}
      {...(project || body ? { body: <>{project && <span className="mx-notice-project">{project}</span>}{body}</> } : {})}
      actions={actions}
      time="just now"
      onClose={onDismiss}
      {...(countdownMs === undefined ? {} : { countdownMs })}
    />
  )
}
