import { useEffect, useRef } from 'react'
import { ResultsList, ResultsOption } from '@/components/ui/results-list'
import { FILES_ASKED, type FoundFiles } from '../useFileMention'

type Props = {
  /** The answer for what is typed now; null while it is on its way. */
  found: FoundFiles | null
  active: number
  onPick: (path: string) => void
  onHover: (index: number) => void
}

/** What the picker says when it has no rows, or what its rows leave out. */
function noteOf(found: FoundFiles | null): string | null {
  if (found === null) return 'Looking for files…'
  if (found.offline) return 'Machine offline'
  if (found.error !== null) return `Could not search the files: ${found.error}`
  if (found.paths.length === 0) return 'No file matches. Keep typing, or Esc to close.'
  return null
}

/**
 * The session's files, over the message box while an `@` and a name are typed: the daemon's best matches, closest
 * first. The keys stay in the box (the composer moves `active`); a click picks one. It shares the command list's look.
 */
export function FilePicker({ found, active, onPick, onHover }: Props) {
  const list = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const row = list.current?.children.item(active)
    if (row instanceof HTMLElement && typeof row.scrollIntoView === 'function') row.scrollIntoView({ block: 'nearest' })
  }, [active])
  const paths = found?.paths ?? []
  const note = noteOf(found)
  return (
    <div className="mx-slash" role="presentation" onMouseDown={(event) => { event.preventDefault() }}>
      {note !== null && <p className="mx-slash-note" role="status">{note}</p>}
      {paths.length > 0 && (
        <ResultsList ref={list} id="mx-file-list" className="mx-slash-list" aria-label="Files">
          {paths.map((path, index) => (
            <ResultsOption
              key={path}
              id={`mx-file-${String(index)}`}
              selected={index === active}
              className={`mx-slash-row is-file${index === active ? ' is-active' : ''}`}
              title={path}
              onMouseMove={() => { if (index !== active) onHover(index) }}
              onClick={() => { onPick(path) }}
            >
              <span className="mx-slash-name">{path}</span>
            </ResultsOption>
          ))}
        </ResultsList>
      )}
      {paths.length >= FILES_ASKED && <p className="mx-slash-note">Only the best {FILES_ASKED} are listed: keep typing to narrow them down.</p>}
      {found?.truncated === true && <p className="mx-slash-note">This folder is too large to search whole, so some files may be missing.</p>}
      {paths.length > 0 && <p className="mx-slash-keys"><kbd>↑</kbd><kbd>↓</kbd> move · <kbd>Enter</kbd> insert · <kbd>Esc</kbd> close</p>}
    </div>
  )
}
