import { useEffect, useRef, useState } from 'react'
import type { SessionView } from '../model/session'

/** Longest path the host accepts. */
const MOST_PATH_CHARS = 4096

/**
 * Asks for the path of a file on a session's machine. The box starts with the session's folder, so
 * only the file's name is left to type. Nothing is browsed: the machine's files are not listed here.
 */
export function GetFileDialog({ session, onGet, onCancel }: { session: SessionView; onGet: (path: string) => void; onCancel: () => void }) {
  const machine = session.machineName ?? 'this computer'
  const start = session.cwd.endsWith('/') ? session.cwd : `${session.cwd}/`
  const [value, setValue] = useState(start)
  const input = useRef<HTMLInputElement>(null)
  useEffect(() => {
    const focusEnd = () => {
      const field = input.current
      if (field === null) return
      field.focus()
      field.setSelectionRange(field.value.length, field.value.length)
    }
    focusEnd()
    // The right-click menu that opened this is still closing, and its focus trap takes focus back as
    // it goes; claim it again once it has finished.
    const timer = window.setTimeout(() => {
      if (document.activeElement !== input.current) focusEnd()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])
  const path = value.trim()
  const ready = path !== '' && path !== start.trim()
  return (
    <div className="mx-settings-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel() }}>
      <form
        role="dialog"
        aria-modal="true"
        aria-label={`Get a file from ${machine}`}
        className="mx-rename"
        onSubmit={(event) => {
          event.preventDefault()
          if (ready) onGet(path)
        }}
        onKeyDown={(event) => {
          if (event.key !== 'Escape') return
          event.stopPropagation()
          onCancel()
        }}
      >
        <h2>Get a file from {machine}</h2>
        <p>Type the file&apos;s path on {machine}. It is saved in Downloads, in the maitty folder, up to 1 GB. A folder can&apos;t be fetched yet.</p>
        <input ref={input} aria-label={`Path on ${machine}`} value={value} maxLength={MOST_PATH_CHARS} spellCheck={false} onChange={(event) => { setValue(event.target.value) }} />
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary" disabled={!ready}>Get file</button>
        </div>
      </form>
    </div>
  )
}
