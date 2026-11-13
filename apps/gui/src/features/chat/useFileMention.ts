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

type IndexAnswer = { readonly index: RemoteIndex } | { readonly error: string }

/**
 * The machine's index of the session's folder, asked once as the picker opens and let go as it closes, so each `@`
 * sees the files as they are then while the keys typed after it ask nothing more. An answer for a picker since closed,
 * or for another folder, is dropped.
 */
function useMachineIndex(place: MachineFileSearch | null, open: boolean): IndexAnswer | null {
  const [answer, setAnswer] = useState<IndexAnswer | null>(null)
  const machine = place === null || place.offline ? null : place.machine
  const folder = place?.folder ?? ''
  useEffect(() => {
    if (machine === null || !open) return undefined
    let current = true
    indexRemoteFiles(machine, folder).then(
      (index) => { if (current) setAnswer({ index }) },
      (error: unknown) => { if (current) setAnswer({ error: errorMessage(error, 'The search failed') }) },
    )
    return () => {
      current = false
      setAnswer(null)
    }
  }, [machine, folder, open])
  return answer
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
    if (found === null) return choosing
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
