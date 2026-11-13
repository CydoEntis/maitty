import { useEffect, useState, type KeyboardEvent } from 'react'
import { repoFindFiles } from '@/features/inspect/api'
import { errorMessage } from '@/lib/error-message'
import { fileMention, pathFromFolder, withPath, type FileSearchPlace } from './fileMention'

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
}

/**
 * The daemon's files for `query`, asked a moment after the typing stops. An answer for a name typed earlier is
 * dropped, never shown over a later one; until the new one comes the last answer stays, so the list does not flicker.
 */
function useFoundFiles(place: FileSearchPlace | null, query: string | null): FoundFiles | null {
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
          setFound({ query, paths, truncated: reply.truncated, error: null })
        },
        (error: unknown) => {
          if (current) setFound({ query, paths: [], truncated: false, error: errorMessage(error, 'The search failed') })
        },
      )
    }, SEARCH_DELAY_MS)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [term, folder, checkout, query])
  return query === null ? null : found
}

export type FileMentionState = {
  readonly open: boolean
  readonly found: FoundFiles | null
  /** The answer is for what is typed now, not an earlier name. */
  readonly settled: boolean
  readonly active: number
  readonly pick: (path: string) => void
  readonly hover: (index: number) => void
  /** Handles a key the picker takes (arrows, Enter, Tab, Esc); false for one it leaves to the box. */
  readonly onKey: (event: KeyboardEvent<HTMLTextAreaElement>) => boolean
}

/**
 * The file picker an `@` opens in the message box: the typed name, the daemon's answer for it, the row the keys are
 * on, and what a pick writes. Esc puts it away until the text changes.
 */
export function useFileMention(place: FileSearchPlace | null, draft: string, setDraft: (text: string) => void, focus: () => void): FileMentionState {
  const mention = place === null ? null : fileMention(draft)
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
    const chosen = paths[active]
    if (chosen === undefined) return false
    if (event.key === 'ArrowDown') setCursor({ query, index: (active + 1) % paths.length })
    else if (event.key === 'ArrowUp') setCursor({ query, index: (active + paths.length - 1) % paths.length })
    else if ((event.key === 'Enter' || event.key === 'Tab') && !event.shiftKey) pick(chosen)
    else return false
    return true
  }
  return { open, found, settled: found?.query === query, active, pick, hover: (index) => { setCursor({ query, index }) }, onKey }
}
