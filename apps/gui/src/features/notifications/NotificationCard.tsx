import type { ReactNode } from 'react'
import { X } from 'lucide-react'

export type NotificationTone = 'needs' | 'done' | 'error' | 'info'

export type NotificationAction = {
  label: string
  onClick: () => void
  /** The one button that answers the card; filled in the card's colour. */
  primary?: boolean
}

export type NotificationCardProps = {
  tone: NotificationTone
  /** The agent's icon with its status badge, or any mark that says who this is about. */
  icon: ReactNode
  title: string
  body?: ReactNode
  actions?: readonly NotificationAction[]
  /** How long ago, in words ("just now"). */
  time: string
  onClose: () => void
  /** When set, a thin line along the bottom shrinks over this many milliseconds. */
  countdownMs?: number
}

/**
 * The one notification card: who it is about, what happened, what you can do, and when. Every toast in
 * the app (a session that needs you, one that finished, a failure, a plain message) is drawn with it,
 * on the float material, so they cannot drift apart.
 */
export function NotificationCard({ tone, icon, title, body, actions = [], time, onClose, countdownMs }: NotificationCardProps) {
  return (
    <div className={`mx-notice is-${tone}`} role="status">
      <span className="mx-notice-icon">{icon}</span>
      <div className="mx-notice-main">
        <div className="mx-notice-head">
          <span className="mx-notice-title">{title}</span>
          <button type="button" className="mx-notice-close" aria-label="Dismiss notification" onClick={onClose}>
            <X size={14} />
          </button>
        </div>
        {body !== undefined && <div className="mx-notice-body">{body}</div>}
        <div className="mx-notice-foot">
          {actions.map((action) => (
            <button
              key={action.label}
              type="button"
              className={action.primary ? 'is-primary' : undefined}
              onClick={() => {
                action.onClick()
                onClose()
              }}
            >
              {action.label}
            </button>
          ))}
          <span className="mx-notice-time">{time}</span>
        </div>
      </div>
      {countdownMs !== undefined && <span className="mx-notice-countdown" style={{ animationDuration: `${String(countdownMs)}ms` }} />}
    </div>
  )
}
