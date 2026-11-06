import { useEffect, useMemo, useRef, useState } from 'react'
import type { FolderListing } from '@/api/folders'
import { errorMessage } from '@/lib/error-message'
import {
  DEFAULT_REMOTE_FOLDER, parentOf, splitTyped, subfolderSuggestions, withoutTrailingSlash,
  type FolderSuggestion,
} from '../model/folderPath'

/** How long typing pauses before the machine is asked about the folder. */
const ASK_AFTER_MS = 150
/** The most suggestions shown at once. */
const MOST_SUGGESTIONS = 8

type Check = { state: 'checking' } | { state: 'found'; items: number } | { state: 'missing' } | { state: 'unknown'; why: string }

/**
 * Asks for a folder on another machine, and checks it there. The system's folder picker only sees
 * this computer, so the folder is typed; the machine says whether it exists, lists the folders that
 * continue what is typed, and the machine's recent folders are offered too. A folder that is not
 * there cannot be started in.
 */
export function RemoteSessionDialog({ machineName, list, recent, onStart, onCancel }: {
  machineName: string
  /** The folders inside a path on this machine. */
  list: (path: string) => Promise<FolderListing>
  /** Folders sessions of this machine are in, most recent first. */
  recent: readonly string[]
  onStart: (folder: string) => Promise<void>
  onCancel: () => void
}) {
  const [folder, setFolder] = useState(DEFAULT_REMOTE_FOLDER)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [answer, setAnswer] = useState<{ path: string; check: Check } | null>(null)
  const [insideAnswer, setInsideAnswer] = useState<{ path: string; listing: FolderListing | null } | null>(null)
  const [highlight, setHighlight] = useState(-1)
  const input = useRef<HTMLInputElement>(null)
  const asked = useRef(new Map<string, Promise<FolderListing>>())
  useEffect(() => {
    input.current?.focus()
    input.current?.select()
  }, [])

  const typed = withoutTrailingSlash(folder)
  const { folder: parent, part } = splitTyped(folder)
  const parentPath = withoutTrailingSlash(parent)
  // An answer counts only for the folder it was about; until the new one comes the old one is not shown.
  const check: Check = answer?.path === typed ? answer.check : { state: 'checking' }
  const inside = insideAnswer?.path === parentPath ? insideAnswer.listing : null

  useEffect(() => {
    let current = true
    const ask = (path: string) => {
      const known = asked.current.get(path) ?? list(path)
      asked.current.set(path, known)
      return known
    }
    const timer = window.setTimeout(() => {
      ask(typed).then(
        (listing) => {
          if (!current) return
          setAnswer({ path: typed, check: listing.exists ? { state: 'found', items: listing.entries.length } : { state: 'missing' } })
        },
        (failure: unknown) => {
          asked.current.delete(typed)
          if (current) setAnswer({ path: typed, check: { state: 'unknown', why: errorMessage(failure, `${machineName} did not answer`) } })
        },
      )
      ask(parentPath).then(
        (listing) => { if (current) setInsideAnswer({ path: parentPath, listing }) },
        () => { if (current) setInsideAnswer({ path: parentPath, listing: null }) },
      )
    }, ASK_AFTER_MS)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [typed, parentPath, list, machineName])

  const suggestions = useMemo<FolderSuggestion[]>(() => {
    const lower = folder.trim().toLowerCase()
    const fromRecent = recent
      .filter((path) => path.toLowerCase().startsWith(lower) && withoutTrailingSlash(path) !== typed)
      .map((path) => ({ kind: 'recent' as const, folder: path, label: path }))
    const fromListing = subfolderSuggestions(inside, parent, part)
    return [...fromRecent, ...fromListing].slice(0, MOST_SUGGESTIONS)
  }, [folder, recent, inside, parent, part, typed])

  const canStart = check.state === 'found' && !busy && typed !== ''
  const pick = (suggestion: FolderSuggestion) => {
    setFolder(suggestion.folder)
    setHighlight(-1)
    input.current?.focus()
  }
  const submit = () => {
    if (!canStart) return
    setBusy(true)
    setError(null)
    onStart(typed).catch((failure: unknown) => { setError(errorMessage(failure, `Could not start a session on ${machineName}`)) }).finally(() => { setBusy(false) })
  }
  const status =
    check.state === 'found' ? `${String(check.items)} ${check.items === 1 ? 'item' : 'items'}`
      : check.state === 'missing' ? `Folder not found on ${machineName}`
        : check.state === 'unknown' ? check.why
          : 'Checking…'
  const missing = check.state === 'missing'

  return (
    <div className="mx-settings-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel() }}>
      <form
        role="dialog"
        aria-modal="true"
        aria-label={`New session on ${machineName}`}
        className="mx-rename mx-folder-dialog"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.stopPropagation()
            onCancel()
          } else if (event.key === 'Enter' && suggestions[highlight]) {
            event.preventDefault()
            pick(suggestions[highlight])
          } else if (event.key === 'ArrowDown' && suggestions.length > 0) {
            event.preventDefault()
            setHighlight((current) => Math.min(current + 1, suggestions.length - 1))
          } else if (event.key === 'ArrowUp' && suggestions.length > 0) {
            event.preventDefault()
            setHighlight((current) => Math.max(current - 1, -1))
          } else if (event.key === 'Tab' && !event.shiftKey) {
            const choice = suggestions[highlight] ?? (suggestions.length === 1 ? suggestions[0] : undefined)
            if (choice) {
              event.preventDefault()
              pick(choice)
            }
          }
        }}
      >
        <h2>New terminal on {machineName}</h2>
        <p>Opens a shell on {machineName}. Chats run on this computer only, so sessions there are terminals.</p>
        <div className="mx-folder-row">
          <input
            ref={input}
            aria-label={`Folder on ${machineName}`}
            aria-invalid={missing}
            aria-describedby="mx-folder-status"
            className={missing ? 'is-missing' : undefined}
            value={folder}
            spellCheck={false}
            onChange={(event) => { setFolder(event.target.value); setHighlight(-1) }}
          />
          <button type="button" className="mx-text-button" onClick={() => { setFolder(parentOf(folder)); setHighlight(-1) }}>Up</button>
          <button type="button" className="mx-text-button" onClick={() => { setFolder(DEFAULT_REMOTE_FOLDER); setHighlight(-1) }}>Home</button>
        </div>
        <p id="mx-folder-status" role="status" className={missing ? 'mx-folder-bad' : undefined}>{status}</p>
        {suggestions.length > 0 && (
          <ul role="listbox" aria-label="Folders" className="mx-folder-suggestions">
            {suggestions.map((suggestion, index) => (
              <li
                key={`${suggestion.kind}:${suggestion.folder}`}
                role="option"
                aria-selected={index === highlight}
                className={index === highlight ? 'is-on' : undefined}
                onMouseDown={(event) => { event.preventDefault(); pick(suggestion) }}
              >
                <span aria-hidden="true">{suggestion.kind === 'recent' ? '◷' : '▸'}</span>
                <span>{suggestion.label}</span>
                {suggestion.kind === 'recent' && <small>recent</small>}
              </li>
            ))}
          </ul>
        )}
        {error !== null && <p role="alert">{error}</p>}
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary" disabled={!canStart}>Open terminal ↵</button>
        </div>
      </form>
    </div>
  )
}
