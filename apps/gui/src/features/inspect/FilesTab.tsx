import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { HighlightedLine } from '@/components/HighlightedLine'
import { Icon } from '@/features/workspace/components/glyphs'
import { type HighlightedLines, highlightFile, repoFile, repoTree } from './api'
import { joinPath } from './format'
import { RepoError } from './RepoError'
import { messageOf, useRepoQuery } from './useRepoQuery'

/** The file shown before one is picked: the README when the top folder has one, else its first file. */
function startingFile(entries: ReadonlyArray<{ name: string; is_directory: boolean }> | undefined): string | null {
  const files = (entries ?? []).filter((entry) => !entry.is_directory)
  return (files.find((entry) => /^readme(\.|$)/i.test(entry.name)) ?? files[0])?.name ?? null
}

export type LookAt = { path: string; line?: number; key: number }

/** A tree row's left padding: a base, then one step in for each folder level. */
const ROW_PAD_PX = 8
const LEVEL_PAD_PX = 14

/** Each folder on the way to a path, so opening a file from a link shows where it is. */
function ancestorsOf(path: string): string[] {
  const parts = path.split('/').slice(0, -1)
  return parts.map((_, index) => parts.slice(0, index + 1).join('/'))
}

/**
 * The repository's files, read-only: the tree on the left (folders open in place, each read when first opened) and the chosen
 * file beside it, highlighted for its type.
 */
export function FilesTab({ term, lookAt }: { term: number; lookAt: LookAt | null }) {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set())
  const [picked, setFile] = useState<string | null>(null)
  const [seen, setSeen] = useState<number | null>(null)

  // A link clicked in the terminal opens its file and the folders above it: adjusting state while rendering, not in an effect.
  if (lookAt && lookAt.key !== seen) {
    setSeen(lookAt.key)
    setOpen((current) => new Set([...current, ...ancestorsOf(lookAt.path)]))
    setFile(lookAt.path)
  }

  // The top folder is read by the tree too, under the same key, so this costs no second read.
  const top = useRepoQuery(term, ['tree', ''], () => repoTree(term, null))
  const file = picked ?? startingFile(top.data)
  const preview = useRepoQuery(term, ['file', file], () => repoFile(term, file ?? ''), false, file !== null)

  // Best effort: a failed or empty answer leaves the built-in per-line colouring in place.
  const text = preview.data
  const highlighted = useQuery({
    queryKey: ['highlight', term, file, text?.length],
    queryFn: () => highlightFile(file ?? '', text ?? ''),
    enabled: file !== null && text !== undefined,
    retry: false,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })
  const spans = highlighted.data ?? null

  useEffect(() => {
    if (lookAt?.line === undefined) return
    document.querySelector(`[data-line="${String(lookAt.line)}"]`)?.scrollIntoView({ block: 'center' })
  }, [lookAt, preview.data])

  const toggle = (folder: string) => {
    setOpen((current) => {
      const next = new Set(current)
      if (!next.delete(folder)) next.add(folder)
      return next
    })
  }

  return (
    <div className="mx-files">
      <nav className="mx-files-tree" aria-label="Files">
        <Folder term={term} folder="" depth={0} open={open} file={file} onToggle={toggle} onPick={setFile} />
      </nav>
      <section className="mx-files-view" aria-label={file === null ? 'File' : `Preview of ${file}`}>
        {file === null ? (
          <p className="mx-inspect-note">Pick a file to read it.</p>
        ) : (
          <>
            <h3 className="mx-inspect-file">{file}</h3>
            {preview.isPending && <p className="mx-inspect-note">Loading…</p>}
            {preview.isError && <p className="mx-inspect-note is-error" role="alert">{messageOf(preview.error)}</p>}
            {preview.data !== undefined && (
              <pre className="mx-preview">
                {preview.data.split('\n').map((line, index) => (
                  <span key={index} data-line={index + 1} className={lookAt?.path === file && lookAt.line === index + 1 ? 'is-target' : undefined}>
                    <i className="mx-preview-no" aria-hidden="true">{index + 1}</i>
                    {spans?.[index] ? <Spans spans={spans[index]} /> : <HighlightedLine line={line} path={file} />}
                    {'\n'}
                  </span>
                ))}
              </pre>
            )}
          </>
        )}
      </section>
    </div>
  )
}

function Spans({ spans }: { spans: NonNullable<HighlightedLines>[number] }) {
  return (
    <>
      {spans.map((span, index) => (span.kind === 'plain' ? span.text : <span key={index} className={`tok-${span.kind}`}>{span.text}</span>))}
    </>
  )
}

type FolderProps = { term: number; folder: string; depth: number; open: ReadonlySet<string>; file: string | null; onToggle: (folder: string) => void; onPick: (path: string) => void }

/** One folder's entries; a folder inside it that is open reads and shows its own. */
function Folder({ term, folder, depth, open, file, onToggle, onPick }: FolderProps) {
  const tree = useRepoQuery(term, ['tree', folder], () => repoTree(term, folder || null))
  if (tree.isPending) return depth === 0 ? <p className="mx-inspect-note">Loading…</p> : null
  if (tree.isError) return <RepoError error={tree.error} tab="files" onRetry={() => { void tree.refetch() }} retrying={tree.isFetching} />
  return (
    <ul className="mx-files-list">
      {tree.data.map((entry) => {
        const path = joinPath(folder, entry.name)
        const indent = { paddingLeft: `${String(ROW_PAD_PX + depth * LEVEL_PAD_PX)}px` }
        if (!entry.is_directory) {
          return (
            <li key={entry.name}>
              <button type="button" className={`mx-files-row${path === file ? ' is-current' : ''}`} style={indent} onClick={() => { onPick(path) }}>
                <span className="mx-files-twist" aria-hidden="true" />
                {entry.name}
              </button>
            </li>
          )
        }
        const isOpen = open.has(path)
        return (
          <li key={entry.name}>
            <button type="button" className="mx-files-row is-folder" style={indent} aria-expanded={isOpen} onClick={() => { onToggle(path) }}>
              <span className={`mx-files-twist${isOpen ? ' is-open' : ''}`} aria-hidden="true"><Icon name="chevronDown" size={11} /></span>
              {entry.name}
            </button>
            {isOpen && <Folder term={term} folder={path} depth={depth + 1} open={open} file={file} onToggle={onToggle} onPick={onPick} />}
          </li>
        )
      })}
    </ul>
  )
}
