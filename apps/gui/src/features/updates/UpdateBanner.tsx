import type { UpdaterState } from './useUpdater'

type Props = {
  state: UpdaterState
  /** Hides the offer for this run. A download in progress or an installed update stays visible. */
  dismissed: boolean
  onInstall: () => void
  onRestart: () => void
  onDismiss: () => void
}

const MAX_NOTES = 4
const PERCENT = 100

function noteLines(notes: string): string[] {
  return notes.split('\n').map((line) => line.replace(/^-\s*/, '').trim()).filter(Boolean).slice(0, MAX_NOTES)
}

/** Offers a newer version, and installs it only when you click. */
export function UpdateBanner({ state, dismissed, onInstall, onRestart, onDismiss }: Props) {
  if (state.status === 'available' && !dismissed) {
    const lines = noteLines(state.notes)
    return (
      <div className="mx-restore mx-update" role="region" aria-label="Update available">
        <div className="mx-restore-row">
          <span className="mx-restore-text"><strong>Maitty {state.version}</strong> is available.</span>
          <button type="button" className="mx-text-button is-primary" onClick={onInstall}>Update</button>
          <button type="button" className="mx-text-button" onClick={onDismiss}>Later</button>
        </div>
        {lines.length > 0 && <ul className="mx-update-notes">{lines.map((line) => <li key={line}>{line}</li>)}</ul>}
      </div>
    )
  }
  if (state.status === 'downloading') {
    const percent = state.progress === null ? null : Math.round(state.progress * PERCENT)
    return (
      <div className="mx-restore mx-update" role="status" aria-label="Updating">
        <div className="mx-restore-row">
          <span className="mx-restore-text">Downloading Maitty {state.version}{percent === null ? '…' : `… ${String(percent)}%`}</span>
        </div>
      </div>
    )
  }
  if (state.status === 'ready') {
    return (
      <div className="mx-restore mx-update" role="status" aria-label="Update installed">
        <div className="mx-restore-row">
          <span className="mx-restore-text">Maitty {state.version} is installed. Restarting the window never ends your sessions.</span>
          <button type="button" className="mx-text-button is-primary" onClick={onRestart}>Restart now</button>
        </div>
      </div>
    )
  }
  return null
}
