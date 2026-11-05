import { useEffect, useRef, useState } from 'react'
import { errorMessage } from '@/lib/error-message'

/** Where a session on another machine starts when nothing else is typed: its home folder. */
export const DEFAULT_REMOTE_FOLDER = '~'

/**
 * Asks for a folder on another machine. The system's folder picker only sees this computer, so
 * the folder is typed; one that is not there opens in that machine's home folder.
 */
export function RemoteSessionDialog({ machineName, onStart, onCancel }: { machineName: string; onStart: (folder: string) => Promise<void>; onCancel: () => void }) {
  const [folder, setFolder] = useState(DEFAULT_REMOTE_FOLDER)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const input = useRef<HTMLInputElement>(null)
  useEffect(() => {
    input.current?.focus()
    input.current?.select()
  }, [])
  const submit = () => {
    setBusy(true)
    setError(null)
    onStart(folder.trim()).catch((failure: unknown) => { setError(errorMessage(failure, `Could not start a session on ${machineName}`)) }).finally(() => { setBusy(false) })
  }
  return (
    <div className="mx-settings-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel() }}>
      <form
        role="dialog"
        aria-modal="true"
        aria-label={`New session on ${machineName}`}
        className="mx-rename"
        onSubmit={(event) => { event.preventDefault(); if (!busy && folder.trim() !== '') submit() }}
        onKeyDown={(event) => {
          if (event.key !== 'Escape') return
          event.stopPropagation()
          onCancel()
        }}
      >
        <h2>New session on {machineName}</h2>
        <p>Type a folder on {machineName}. One that is not there opens in its home folder.</p>
        <input ref={input} aria-label={`Folder on ${machineName}`} value={folder} spellCheck={false} onChange={(event) => { setFolder(event.target.value) }} />
        {error !== null && <p role="alert">{error}</p>}
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary" disabled={busy || folder.trim() === ''}>Start</button>
        </div>
      </form>
    </div>
  )
}
