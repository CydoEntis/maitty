import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { ResultsList, ResultsOption } from '@/components/ui/results-list'
import type { FolderListing } from '@/api/folders'
import { errorMessage } from '@/lib/error-message'

/** Longest path the host accepts. */
const MOST_PATH_CHARS = 4096

/** Where browsing another machine starts: its home folder. */
const START_FOLDER = '~'

type Listed = { asked: string; result: { state: 'ready'; value: FolderListing } | { state: 'failed'; why: string } }

/** `folder` and `name` joined with one slash. */
function join(folder: string, name: string): string {
  return `${folder.replace(/\/+$/, '')}/${name}`
}

/** The folder above `folder`; the root stays the root. */
function parentOf(folder: string): string {
  const trimmed = folder.replace(/\/+$/, '')
  const cut = trimmed.lastIndexOf('/')
  return cut <= 0 ? '/' : trimmed.slice(0, cut)
}

type Props = {
  /** What the machine is called in the window. */
  machine: string
  list: (path: string) => Promise<FolderListing>
  onPick: (folder: string) => void
  onCancel: () => void
}

/**
 * Chooses a folder on another machine, whose disk the system's own folder picker cannot see. A person
 * clicks through the machine's folders, narrows them by typing a few letters, or types a whole path.
 * Every name shown came from the machine's daemon.
 */
export function PickFolderDialog({ machine, list, onPick, onCancel }: Props) {
  const [asked, setAsked] = useState(START_FOLDER)
  const [typed, setTyped] = useState(START_FOLDER)
  // Each answer is kept with the folder it is for, so one for a folder the person has left is never shown.
  const [listed, setListed] = useState<Listed | null>(null)
  const [filter, setFilter] = useState('')
  const [selected, setSelected] = useState(0)
  const search = useRef<HTMLInputElement>(null)
  const current = listed !== null && listed.asked === asked ? listed.result : null

  useEffect(() => { search.current?.focus() }, [])

  useEffect(() => {
    let live = true
    list(asked).then(
      (value) => {
        if (!live) return
        setListed({ asked, result: { state: 'ready', value } })
        if (value.exists) setTyped(value.path)
      },
      (error: unknown) => { if (live) setListed({ asked, result: { state: 'failed', why: errorMessage(error, `Could not list ${asked}`) } }) },
    )
    return () => { live = false }
  }, [list, asked])

  const listing = current?.state === 'ready' ? current.value : null
  const here = listing !== null && listing.exists ? listing.path : null
  const rows = useMemo(() => {
    if (listing === null) return []
    const needle = filter.trim().toLowerCase()
    return listing.entries.filter((entry) => needle === '' || entry.name.toLowerCase().includes(needle))
  }, [listing, filter])

  const open = (folder: string) => {
    setFilter('')
    setSelected(0)
    setAsked(folder)
  }
  const onSearchKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setSelected((at) => Math.min(rows.length - 1, at + 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setSelected((at) => Math.max(0, at - 1))
    } else if (event.key === 'Enter') {
      // Enter here opens the highlighted folder; it does not choose the one on show.
      event.preventDefault()
      const row = rows[selected]
      if (row !== undefined && here !== null) open(join(here, row.name))
    }
  }
  const onPathKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== 'Enter') return
    event.preventDefault()
    const path = typed.trim()
    if (path !== '') open(path)
  }

  return (
    <Dialog open onOpenChange={(isOpen) => { if (!isOpen) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
        <form
          aria-label={`Choose a folder on ${machine}`}
          className="mx-rename mx-getfile"
          onSubmit={(event) => {
            event.preventDefault()
            if (here !== null) onPick(here)
          }}
          onKeyDown={(event) => {
            // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
            if (event.key === 'Escape') event.stopPropagation()
          }}
        >
          <DialogTitle asChild><h2>Choose a folder on {machine}</h2></DialogTitle>
          <div className="mx-getfile-where">
            <button type="button" className="mx-text-button" onClick={() => { if (here !== null) open(parentOf(here)) }} disabled={here === null || here === '/'} aria-label="Go up one folder">Up</button>
            <code title={here ?? asked}>{here ?? asked}</code>
          </div>
          <input
            ref={search}
            aria-label="Narrow the folders"
            placeholder="Type a few letters of a folder name"
            value={filter}
            spellCheck={false}
            onChange={(event) => { setFilter(event.target.value); setSelected(0) }}
            onKeyDown={onSearchKey}
          />
          <ResultsList aria-label={`Folders in ${here ?? asked}`} className="mx-getfile-list">
            {current === null && <p role="status">Loading…</p>}
            {current?.state === 'failed' && <p role="alert">{current.why}. You can still type a path below.</p>}
            {listing !== null && !listing.exists && <p role="alert">There is no folder {listing.path} on {machine}, or it cannot be read.</p>}
            {here !== null && rows.map((entry, at) => (
              <ResultsOption
                key={entry.name}
                selected={at === selected}
                aria-label={entry.git === null ? `${entry.name}, folder` : `${entry.name}, folder on ${entry.git}`}
                className={`mx-getfile-row${at === selected ? ' is-selected' : ''}`}
                onClick={() => { open(join(here, entry.name)) }}
                onMouseEnter={() => { setSelected(at) }}
              >
                <span aria-hidden="true">▸ {entry.name}/{entry.git === null ? '' : `  · ${entry.git}`}</span>
              </ResultsOption>
            ))}
            {here !== null && rows.length === 0 && <p role="status">{filter.trim() === '' ? 'No folders inside this one.' : 'No folder matches.'}</p>}
          </ResultsList>
          {listing?.truncated === true && <p role="status">Showing the first {listing.entries.length} folders only.</p>}
          <p>Or type a path on {machine} and press Enter to go there.</p>
          <input aria-label={`Path on ${machine}`} value={typed} maxLength={MOST_PATH_CHARS} spellCheck={false} onChange={(event) => { setTyped(event.target.value) }} onKeyDown={onPathKey} />
          <div className="mx-rename-actions">
            <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
            <button type="submit" className="mx-text-button is-primary" disabled={here === null}>Use this folder</button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
