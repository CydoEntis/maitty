import { useEffect, useState } from 'react'
import { Icon } from './glyphs'

export function EmptyState({ hasSessions, startFolderLabel, primary, canStart, onNewSession, onChangeStartFolder }: {
  hasSessions: boolean
  startFolderLabel: string
  primary: string
  canStart: boolean
  onNewSession: () => void
  onChangeStartFolder: () => void
}) {
  return (
    <div className="mx-empty">
      <div className="mx-empty-column">
        <span className="mx-empty-tile"><Icon name="prompt" /></span>
        <div className="mx-empty-copy">
          <h1>{hasSessions ? 'No tab open' : 'No sessions running'}</h1>
          {hasSessions ? (
            <p>Your sessions keep running in the sidebar. Pick one to show it here, or open a new shell in <span className="mono">{startFolderLabel}</span>.</p>
          ) : (
            <p>A new session opens a shell in <span className="mono">{startFolderLabel}</span>. <span className="mono">cd</span> into a project and it shows up in the sidebar. Start an agent like <span className="mono">codex</span> from any shell.</p>
          )}
        </div>
        <div className="mx-empty-actions">
          <button type="button" className="mx-primary" disabled={!canStart} onClick={onNewSession}>New session<kbd>{primary}T</kbd></button>
          <button type="button" className="mx-quiet" onClick={onChangeStartFolder}>Change start folder</button>
        </div>
        <span className="mx-empty-note-small">Sessions live in maittyd. Closing this window never stops them, and the Maitty TUI sees the same sessions.</span>
      </div>
    </div>
  )
}

export type RestartOffer = {
  label: string
  canRestart: boolean
  pending: boolean
  onRestart: () => void
}

const SECOND_MS = 1000

/** Under the tabs while maittyd is unreachable: the last known screen stays, input waits. */
export function OfflineBanner({ retryAt, detail, restart, onRetry }: { retryAt: number | null; detail: string | null; restart: RestartOffer | null; onRetry: () => void }) {
  const [now, setNow] = useState(() => Date.now())
  const [armed, setArmed] = useState(false)
  useEffect(() => {
    const timer = window.setInterval(() => { setNow(Date.now()) }, SECOND_MS)
    return () => { window.clearInterval(timer) }
  }, [])
  const seconds = retryAt === null ? null : Math.max(1, Math.ceil((retryAt - now) / SECOND_MS))
  return (
    <div className="mx-offline" role="status">
      <span className="mx-offline-dot" aria-hidden="true" />
      <span className="mx-offline-text">
        <strong>Disconnected from maittyd.</strong> Sessions keep running in the daemon. What you see is the last known screen.{seconds !== null && ` Retrying in ${String(seconds)}s.`}
        {detail && <> {detail}</>}
      </span>
      {restart && (
        <button
          type="button"
          className={`mx-offline-button${armed ? ' is-armed' : ''}`}
          disabled={!restart.canRestart || restart.pending}
          onClick={() => {
            if (!armed) {
              setArmed(true)
              return
            }
            setArmed(false)
            restart.onRestart()
          }}
        >
          {armed ? 'Click again to restart' : restart.label}
        </button>
      )}
      <button type="button" className="mx-offline-button" onClick={onRetry}>Retry now</button>
    </div>
  )
}

export function Toast({ message }: { message: string }) {
  return <div className="mx-toast" role="status">{message}</div>
}
