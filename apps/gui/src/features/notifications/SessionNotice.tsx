export type SessionNoticeKind = 'needs' | 'done' | 'error'

export type SessionNoticeAction = {
  label: string
  onClick: () => void
  /** The one button that answers the notice; filled amber. */
  primary?: boolean
}

export type SessionNoticeProps = {
  kind: SessionNoticeKind
  title: string
  body: string
  actions: readonly SessionNoticeAction[]
  /** When set, a thin line along the bottom shrinks over this many milliseconds. */
  countdownMs?: number
}

/** A notification card for a session you are not looking at: the app, the project, what happened, and what you can do about it. */
export function SessionNotice({ kind, title, body, actions, countdownMs, onDismiss }: SessionNoticeProps & { onDismiss: () => void }) {
  return (
    <div className={`mx-notice is-${kind}`} role="status">
      <span className="mx-notice-tile" aria-hidden="true">&gt;_</span>
      <div className="mx-notice-main">
        <div className="mx-notice-head">
          <span className="mx-notice-app">MAITTY</span>
          <span className="mx-notice-age">now</span>
        </div>
        <div className="mx-notice-title">{title}</div>
        <div className="mx-notice-body">{body}</div>
        <div className="mx-notice-actions">
          {actions.map((action) => (
            <button key={action.label} type="button" className={action.primary ? 'is-primary' : undefined} onClick={() => { action.onClick(); onDismiss() }}>{action.label}</button>
          ))}
          <button type="button" className="is-quiet" onClick={onDismiss}>Dismiss</button>
        </div>
      </div>
      {countdownMs !== undefined && <span className="mx-notice-countdown" style={{ animationDuration: `${String(countdownMs)}ms` }} />}
    </div>
  )
}
