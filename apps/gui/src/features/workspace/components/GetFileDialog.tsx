import { machineLabel } from '@/data/machineNicknames'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { ResultsList, ResultsOption } from '@/components/ui/results-list'
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import type { RemoteIndex, RemoteListing } from '@/api/fileGets'
import { errorMessage } from '@/lib/error-message'
import { fuzzyFind } from '@/lib/fuzzy'
import type { SessionView } from '../model/session'

/** Longest path the host accepts. */
const MOST_PATH_CHARS = 4096

/** Most search matches shown at once. */
const MOST_MATCHES = 50

/** What the dialog can ask of the session's machine; without it, only a typed path works. */
export type Browse = {
  list: (path: string) => Promise<RemoteListing>
  index: (folder: string) => Promise<RemoteIndex>
}

type Row = {
  kind: 'dir' | 'file' | 'match'
  /** What the row shows: a name, or a path below the searched folder. */
  label: string
  /** The path on the machine: for a file, what is got; for a folder, what is opened. */
  full: string
  /** Characters of `label` the typed letters landed on. */
  marks: readonly number[]
}

type Settled<T> = { state: 'ready'; value: T } | { state: 'failed'; why: string }
type Loaded<T> = { state: 'loading' } | Settled<T>

/** One value for "not answered yet", so reading it does not change from one render to the next. */
const LOADING = { state: 'loading' } as const

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

/** The text with the characters at `marks` wrapped in <mark>, runs together. */
function Marked({ text, marks }: { text: string; marks: readonly number[] }) {
  const parts: { text: string; marked: boolean }[] = []
  for (let at = 0; at < text.length; at += 1) {
    const marked = marks.includes(at)
    const last = parts[parts.length - 1]
    if (last !== undefined && last.marked === marked) last.text += text.charAt(at)
    else parts.push({ text: text.charAt(at), marked })
  }
  return <>{parts.map((part, i) => (part.marked ? <mark key={i}>{part.text}</mark> : <span key={i}>{part.text}</span>))}</>
}

/**
 * Asks for a file on a session's machine. A person can click through the machine's folders, search
 * by typing a few letters of a name (inside the session's folder and everything below it), or type
 * a whole path. Everything shown came from the machine and was cleaned by the host.
 */
export function GetFileDialog({ session, onGet, onCancel, browse }: { session: SessionView; onGet: (path: string) => void; onCancel: () => void; browse?: Browse }) {
  const machine = machineLabel(session.machineName) ?? 'this computer'
  const start = session.cwd.endsWith('/') ? session.cwd : `${session.cwd}/`
  const [typed, setTyped] = useState(start)
  const [folder, setFolder] = useState(session.cwd)
  // Each answer is kept with the folder it is for, so one for a folder the person has left is never shown.
  const [listed, setListed] = useState<{ folder: string; result: Settled<RemoteListing> } | null>(null)
  const [indexed, setIndexed] = useState<Settled<RemoteIndex> | null>(null)
  const listing: Loaded<RemoteListing> = listed !== null && listed.folder === folder ? listed.result : LOADING
  const index: Loaded<RemoteIndex> = indexed ?? LOADING
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState(0)
  const search = useRef<HTMLInputElement>(null)
  const pathBox = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const focusField = () => {
      const field = browse === undefined ? pathBox.current : search.current
      if (field === null) return
      field.focus()
      field.setSelectionRange(field.value.length, field.value.length)
    }
    focusField()
    // The right-click menu that opened this is still closing, and its focus trap takes focus back as
    // it goes; claim it again once it has finished.
    const timer = window.setTimeout(() => {
      if (document.activeElement !== (browse === undefined ? pathBox.current : search.current)) focusField()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [browse])

  // The folder on show. A later answer for a folder the person has already left is ignored.
  useEffect(() => {
    if (browse === undefined) return
    let current = true
    browse.list(folder).then(
      (value) => { if (current) setListed({ folder, result: { state: 'ready', value } }) },
      (error: unknown) => { if (current) setListed({ folder, result: { state: 'failed', why: errorMessage(error, `Could not list ${folder}`) } }) },
    )
    return () => { current = false }
  }, [browse, folder])

  // The search index is made once, for the folder the session is in.
  useEffect(() => {
    if (browse === undefined) return
    let current = true
    browse.index(session.cwd).then(
      (value) => { if (current) setIndexed({ state: 'ready', value }) },
      (error: unknown) => { if (current) setIndexed({ state: 'failed', why: errorMessage(error, 'Could not search this folder') }) },
    )
    return () => { current = false }
  }, [browse, session.cwd])

  const searching = query.trim() !== ''
  const rows = useMemo<Row[]>(() => {
    if (searching) {
      if (index.state !== 'ready') return []
      return fuzzyFind(query, index.value.files, MOST_MATCHES).map((hit) => ({ kind: 'match', label: hit.path, full: join(index.value.folder, hit.path), marks: hit.positions }))
    }
    if (listing.state !== 'ready') return []
    const { value } = listing
    return [
      ...value.dirs.map((name): Row => ({ kind: 'dir', label: name, full: join(value.folder, name), marks: [] })),
      ...value.files.map((name): Row => ({ kind: 'file', label: name, full: join(value.folder, name), marks: [] })),
    ]
  }, [searching, query, index, listing])

  const choose = (row: Row | undefined) => {
    if (row === undefined) return
    if (row.kind === 'dir') {
      setQuery('')
      setSelected(0)
      setFolder(row.full)
      return
    }
    onGet(row.full)
  }
  const goUp = () => {
    if (listing.state !== 'ready') return
    setQuery('')
    setSelected(0)
    setFolder(parentOf(listing.value.folder))
  }
  const onSearchKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setSelected((at) => Math.min(rows.length - 1, at + 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setSelected((at) => Math.max(0, at - 1))
    } else if (event.key === 'Enter') {
      // Enter here chooses the highlighted row; it does not submit the typed path below.
      event.preventDefault()
      choose(rows[selected])
    }
  }

  const path = typed.trim()
  const ready = path !== '' && path !== start.trim()
  const shownFolder = listing.state === 'ready' ? listing.value.folder : folder
  const notices: string[] = []
  if (searching && index.state === 'ready') {
    if (index.value.truncated) notices.push(`Showing matches from the first ${String(index.value.files.length)} files only.`)
    if (index.value.hidden > 0) notices.push(`${String(index.value.hidden)} files with unusual names are left out.`)
  } else if (!searching && listing.state === 'ready') {
    if (listing.value.truncated) notices.push(`Showing the first ${String(listing.value.dirs.length + listing.value.files.length)} entries only.`)
    if (listing.value.hidden > 0) notices.push(`${String(listing.value.hidden)} entries with unusual names are left out.`)
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
      <form
        aria-label={`Get a file from ${machine}`}
        className="mx-rename mx-getfile"
        onSubmit={(event) => {
          event.preventDefault()
          if (ready) onGet(path)
        }}
        onKeyDown={(event) => {
          // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
          if (event.key === 'Escape') event.stopPropagation()
        }}
      >
        <DialogTitle asChild><h2>Get a file from {machine}</h2></DialogTitle>
        {browse !== undefined && (
          <>
            <div className="mx-getfile-where">
              <button type="button" className="mx-text-button" onClick={goUp} disabled={listing.state !== 'ready' || searching} aria-label="Go up one folder">Up</button>
              <code title={shownFolder}>{shownFolder}</code>
              <button type="button" className="mx-text-button" disabled={listing.state !== 'ready' || searching} onClick={() => { onGet(shownFolder) }} aria-label={`Get this folder, ${shownFolder}`}>Get this folder</button>
            </div>
            <input
              ref={search}
              aria-label={`Search in ${session.cwd}`}
              placeholder="Type a few letters of a file name"
              value={query}
              spellCheck={false}
              onChange={(event) => { setQuery(event.target.value); setSelected(0) }}
              onKeyDown={onSearchKey}
            />
            <ResultsList aria-label={searching ? 'Matches' : `Files in ${shownFolder}`} className="mx-getfile-list">
              {searching && index.state === 'loading' && <p role="status">Reading the folder to search it…</p>}
              {searching && index.state === 'failed' && <p role="alert">{index.why}</p>}
              {!searching && listing.state === 'loading' && <p role="status">Loading…</p>}
              {!searching && listing.state === 'failed' && <p role="alert">{listing.why}. You can still type a path below.</p>}
              {rows.map((row, at) => (
                <ResultsOption
                  key={`${row.kind}:${row.full}`}
                  selected={at === selected}
                  aria-label={row.kind === 'dir' ? `${row.label}, folder` : row.label}
                  className={`mx-getfile-row${at === selected ? ' is-selected' : ''}`}
                  onClick={() => { choose(row) }}
                  onMouseEnter={() => { setSelected(at) }}
                >
                  <span aria-hidden="true">{row.kind === 'dir' ? '▸ ' : ''}<Marked text={row.label} marks={row.marks} />{row.kind === 'dir' ? '/' : ''}</span>
                </ResultsOption>
              ))}
              {rows.length === 0 && ((searching && index.state === 'ready') || (!searching && listing.state === 'ready')) && <p role="status">{searching ? 'No file matches.' : 'This folder is empty.'}</p>}
            </ResultsList>
            {notices.map((notice) => <p key={notice} role="status">{notice}</p>)}
          </>
        )}
        <p>{browse === undefined ? `Type the file's path on ${machine}.` : 'Or type a path.'} It is saved in Downloads, in the maitty folder, up to 1 GB. A folder comes as a folder.</p>
        <input ref={pathBox} aria-label={`Path on ${machine}`} value={typed} maxLength={MOST_PATH_CHARS} spellCheck={false} onChange={(event) => { setTyped(event.target.value) }} />
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary" disabled={!ready}>Get file</button>
        </div>
      </form>
      </DialogContent>
    </Dialog>
  )
}
