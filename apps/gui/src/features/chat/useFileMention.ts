import { useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import { indexRemoteFiles, type RemoteIndex } from '@/api/fileGets'
import { repoFindFiles } from '@/features/inspect/api'
import { errorMessage } from '@/lib/error-message'
import { fuzzyFind } from '@/lib/fuzzy'
import { fileMention, pathFromFolder, withPath, type FileSearchPlace, type MachineFileSearch, type PaneFileSearch } from './fileMention'

/** The most paths asked for and listed; past this a name is typed further, not scrolled to. */
export const FILES_ASKED = 50
/** Typing pauses this long before the daemon is asked, so a burst of keys is one question. */
const SEARCH_DELAY_MS = 120

/** What the daemon answered for one typed name. */
export type FoundFiles = {
  readonly query: string
  /** Relative to the session's folder, best first. */
  readonly paths: readonly string[]
  readonly truncated: boolean
  readonly error: string | null
  /** The session's machine is not reachable, so nothing was asked. */
  readonly offline: boolean
}

/**
 * The daemon's files for `query`, asked a moment after the typing stops. An answer for a name typed earlier is
 * dropped, and the last answer is let go as the name, the pane or the picker changes, so none outlives its name.
 */
function usePaneFiles(place: PaneFileSearch | null, query: string | null): FoundFiles | null {
  const [found, setFound] = useState<FoundFiles | null>(null)
  const term = place?.term ?? null
  const folder = place?.folder ?? ''
  const checkout = place?.checkout ?? null
  useEffect(() => {
    if (term === null || query === null) return undefined
    let current = true
    const timer = window.setTimeout(() => {
      repoFindFiles(term, query, FILES_ASKED).then(
        (reply) => {
          if (!current) return
          const paths = reply.paths.map((path) => pathFromFolder(path, { folder, checkout }))
          setFound({ query, paths, truncated: reply.truncated, error: null, offline: false })
        },
        (error: unknown) => {
          if (current) setFound({ query, paths: [], truncated: false, error: errorMessage(error, 'The search failed'), offline: false })
        },
      )
    }, SEARCH_DELAY_MS)
    return () => {
      current = false
      window.clearTimeout(timer)
      setFound(null)
    }
  }, [term, folder, checkout, query])
  return query === null || found?.query !== query ? null : found
}

/** How long a machine's finished index is reused: each index is an ssh round trip, but new files should soon show. */
export const INDEX_KEPT_MS = 30_000

type KeptIndex = { readonly index: Promise<RemoteIndex>; settledAt: number | null }

const keptIndexes = new Map<string, KeptIndex>()

/** Lets go of every kept index, so one test does not see another's. */
export function forgetMachineIndexes(): void {
  keptIndexes.clear()
}

/**
 * The index of `folder` on `machine`: the one running or finished in the last `INDEX_KEPT_MS` if there is one, else a
 * new one. A failed index is not kept, so the next `@` asks again.
 */
function machineIndex(machine: string, folder: string): Promise<RemoteIndex> {
  const key = `${machine}\n${folder}`
  const kept = keptIndexes.get(key)
  if (kept !== undefined && (kept.settledAt === null || Date.now() - kept.settledAt < INDEX_KEPT_MS)) return kept.index
  const entry: KeptIndex = { index: indexRemoteFiles(machine, folder), settledAt: null }
  keptIndexes.set(key, entry)
  entry.index.then(
    () => { entry.settledAt = Date.now() },
    () => { if (keptIndexes.get(key) === entry) keptIndexes.delete(key) },
  )
  return entry.index
}

type IndexAnswer = { readonly machine: string; readonly folder: string } & ({ readonly index: RemoteIndex } | { readonly error: string })

/**
 * The machine's index of the session's folder, asked as the picker opens and let go as it closes, so the keys typed
 * after an `@` ask nothing more. An answer for a picker since closed, or for another place, is never returned.
 */
function useMachineIndex(place: MachineFileSearch | null, open: boolean): IndexAnswer | null {
  const [answer, setAnswer] = useState<IndexAnswer | null>(null)
  const machine = place === null || place.offline ? null : place.machine
  const folder = place?.folder ?? ''
  useEffect(() => {
    if (machine === null || !open) return undefined
    let current = true
    machineIndex(machine, folder).then(
      (index) => { if (current) setAnswer({ machine, folder, index }) },
      (error: unknown) => { if (current) setAnswer({ machine, folder, error: errorMessage(error, 'The search failed') }) },
    )
    return () => {
      current = false
      setAnswer(null)
    }
  }, [machine, folder, open])
  return answer?.machine === machine && answer.folder === folder ? answer : null
}

/** The files of another machine's folder for `query`: its index, ranked here as the Get a file dialog ranks it. */
function useMachineFiles(place: MachineFileSearch | null, query: string | null): FoundFiles | null {
  const answer = useMachineIndex(place, query !== null)
  const offline = place?.offline === true
  return useMemo(() => {
    if (query === null) return null
    if (offline) return { query, paths: [], truncated: false, error: null, offline: true }
    if (answer === null) return null
    if ('error' in answer) return { query, paths: [], truncated: false, error: answer.error, offline: false }
    const paths = fuzzyFind(query, answer.index.files, FILES_ASKED).map((hit) => hit.path)
    return { query, paths, truncated: answer.index.truncated, error: null, offline: false }
  }, [query, offline, answer])
}

const isPane = (place: FileSearchPlace | null): place is PaneFileSearch => place !== null && 'term' in place
const isMachine = (place: FileSearchPlace | null): place is MachineFileSearch => place !== null && 'machine' in place

function useFoundFiles(place: FileSearchPlace | null, query: string | null): FoundFiles | null {
  const pane = usePaneFiles(isPane(place) ? place : null, query)
  const machine = useMachineFiles(isMachine(place) ? place : null, query)
  return isMachine(place) ? machine : pane
}

export type FileMentionState = {
  readonly open: boolean
  /** Null while the answer for what is typed now is on its way. */
  readonly found: FoundFiles | null
  readonly active: number
  readonly pick: (path: string) => void
  readonly hover: (index: number) => void
  /** Handles a key the picker takes (arrows, Enter, Tab, Esc); false for one it leaves to the box. */
  readonly onKey: (event: KeyboardEvent<HTMLTextAreaElement>) => boolean
}

/**
 * The file picker an `@` opens in the message box: the typed name, the daemon's answer for it, the row the keys are
 * on, and what a pick writes. It follows the caret, so it is open only while the caret sits at the end of the text.
 * Esc puts it away until the text changes.
 */
export function useFileMention(place: FileSearchPlace | null, draft: string, caretAtEnd: boolean, setDraft: (text: string) => void, focus: () => void): FileMentionState {
  const mention = place === null || !caretAtEnd ? null : fileMention(draft)
  const [closedFor, setClosedFor] = useState<string | null>(null)
  const open = mention !== null && closedFor !== draft
  const found = useFoundFiles(place, open ? mention.query : null)
  const paths = found?.paths ?? []
  const [cursor, setCursor] = useState({ query: '', index: 0 })
  const query = mention?.query ?? ''
  const active = cursor.query === query ? Math.min(cursor.index, Math.max(0, paths.length - 1)) : 0
  const pick = (path: string) => {
    if (mention === null) return
    setDraft(withPath(draft, mention, path))
    focus()
  }
  const onKey = (event: KeyboardEvent<HTMLTextAreaElement>): boolean => {
    if (!open) return false
    if (event.key === 'Escape') {
      setClosedFor(draft)
      return true
    }
    const choosing = event.key === 'ArrowDown' || event.key === 'ArrowUp' || ((event.key === 'Enter' || event.key === 'Tab') && !event.shiftKey)
    // Until the answer for what is typed now comes there is nothing to choose, and Enter must not send the half-typed name.
    // A machine's index can take as long as its ssh line's timeout, though, and holding Enter that long reads as a hang.
    if (found === null) return event.key === 'Enter' && isMachine(place) ? false : choosing
    const chosen = paths[active]
    if (chosen === undefined) return false
    if (event.key === 'ArrowDown') setCursor({ query, index: (active + 1) % paths.length })
    else if (event.key === 'ArrowUp') setCursor({ query, index: (active + paths.length - 1) % paths.length })
    else if ((event.key === 'Enter' || event.key === 'Tab') && !event.shiftKey) pick(chosen)
    else return false
    return true
  }
  return { open, found, active, pick, hover: (index) => { setCursor({ query, index }) }, onKey }
}
