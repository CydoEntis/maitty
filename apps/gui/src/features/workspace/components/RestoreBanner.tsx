import { displayName, type SessionView } from '../model/session'

type Props = {
  /** Sessions whose agent was mid-turn when the daemon restarted and has not been dismissed. */
  sessions: readonly SessionView[]
  online: boolean
  /** Types the session's resume command into it. The banner never does it by itself. */
  onResume: (session: SessionView) => void
  onDismiss: (sessionId: string) => void
}

/** Offers to bring back agents that were working when Maitty restarted. Nothing resumes until you click. */
export function RestoreBanner({ sessions, online, onResume, onDismiss }: Props) {
  if (sessions.length === 0) return null
  return (
    <div className="mx-restore" role="region" aria-label="Interrupted agents">
      {sessions.map((session) => (
        <div key={session.id} className="mx-restore-row">
          <span className="mx-restore-text">
            <strong>{session.interrupted?.agent}</strong> was working in {displayName(session)} when Maitty restarted.
          </span>
          <code className="mx-restore-command">{session.interrupted?.resume}</code>
          <button type="button" className="mx-text-button is-primary" disabled={!online} onClick={() => { onResume(session) }}>Resume</button>
          <button type="button" className="mx-text-button" onClick={() => { onDismiss(session.id) }}>Dismiss</button>
        </div>
      ))}
    </div>
  )
}
