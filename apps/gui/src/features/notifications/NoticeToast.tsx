import type { ReactNode } from 'react'
import { X } from 'lucide-react'
import { StatusMark, type StatusMarkState } from '@/components/StatusMark'

export type NoticeKind = 'needs' | 'recovery' | 'finished' | 'error' | 'info'

export type NoticeAction = {
  label: string
  onClick: () => void
  /** The one button that answers the notice; filled in the notice's color. */
  primary?: boolean
}

export type NoticeProps = {
  kind: NoticeKind
  project?: string
  title: string
  body?: ReactNode
  actions?: NoticeAction[]
  /** When set, a bar along the bottom shrinks over this many milliseconds. */
  countdownMs?: number
}

const KIND: Record<NoticeKind, { label: string; mark: StatusMarkState }> = {
  needs: { label: 'Needs you', mark: 'needs-you' },
  recovery: { label: 'Needs recovery', mark: 'recovery' },
  finished: { label: 'Finished', mark: 'done' },
  error: { label: 'Error', mark: 'recovery' },
  info: { label: 'Notice', mark: 'idle' },
}

/** One notification card: kind and project, a title, optional details and buttons. */
export function NoticeToast({
  kind,
  project,
  title,
  body,
  actions = [],
  countdownMs,
  onDismiss,
}: NoticeProps & { onDismiss: () => void }) {
  const { label, mark } = KIND[kind]
  return (
    <div className={`notice-toast is-${kind}`} role="status">
      <span className="notice-tile">
        <StatusMark status={mark} />
      </span>
      <div className="notice-main">
        <div className="notice-head">
          <span className="notice-kind">{label}</span>
          {project && <span className="notice-project">· {project}</span>}
          <span className="notice-age">now</span>
          <button
            type="button"
            className="notice-dismiss"
            aria-label="Dismiss notification"
            onClick={onDismiss}
          >
            <X size={12} />
          </button>
        </div>
        <div className="notice-title">{title}</div>
        {body && <div className="notice-body">{body}</div>}
        {actions.length > 0 && (
          <div className="notice-actions">
            {actions.map((action) => (
              <button
                key={action.label}
                type="button"
                className={action.primary ? 'is-primary' : undefined}
                onClick={() => {
                  action.onClick()
                  onDismiss()
                }}
              >
                {action.label}
              </button>
            ))}
          </div>
        )}
      </div>
      {countdownMs !== undefined && (
        <span
          className="notice-countdown"
          style={{ animationDuration: `${String(countdownMs)}ms` }}
        />
      )}
    </div>
  )
}
