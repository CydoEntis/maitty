import { useEffect, useState } from 'react'
import { repoFile, repoTree } from './api'
import { joinPath, parentOf } from './format'
import { RepoError } from './RepoError'
import { messageOf, useRepoQuery } from './useRepoQuery'

export type LookAt = { path: string; line?: number; key: number }

/** The repository's files, read-only: folders to walk, and a text preview of the one you pick. */
export function FilesTab({ term, lookAt }: { term: number; lookAt: LookAt | null }) {
  const [folder, setFolder] = useState('')
  const [file, setFile] = useState<string | null>(null)
  const [seen, setSeen] = useState<number | null>(null)

  // A link clicked in the terminal opens its file: adjusting state while rendering, not in an effect.
  if (lookAt && lookAt.key !== seen) {
    setSeen(lookAt.key)
    setFolder(parentOf(lookAt.path))
    setFile(lookAt.path)
  }

  const tree = useRepoQuery(term, ['tree', folder], () => repoTree(term, folder || null))
  const preview = useRepoQuery(term, ['file', file], () => repoFile(term, file ?? ''), false, file !== null)

  useEffect(() => {
    if (lookAt?.line === undefined) return
    document.querySelector(`[data-line="${String(lookAt.line)}"]`)?.scrollIntoView({ block: 'center' })
  }, [lookAt, preview.data])

  const crumbs = folder ? folder.split('/') : []
  return (
    <div className="mx-inspect-split">
      <div>
        <nav className="mx-inspect-crumbs" aria-label="Folder">
          <button type="button" onClick={() => { setFolder('') }}>/</button>
          {crumbs.map((name, index) => (
            <button key={crumbs.slice(0, index + 1).join('/')} type="button" onClick={() => { setFolder(crumbs.slice(0, index + 1).join('/')) }}>{name}</button>
          ))}
        </nav>
        {tree.isPending && <p className="mx-inspect-note">Loading…</p>}
        {tree.isError && <RepoError error={tree.error} />}
        <ul className="mx-inspect-list" aria-label="Files">
          {tree.data?.map((entry) => {
            const path = joinPath(folder, entry.name)
            return (
              <li key={entry.name}>
                <button
                  type="button"
                  className={`mx-inspect-row${path === file ? ' is-current' : ''}`}
                  onClick={() => { if (entry.is_directory) setFolder(path); else setFile(path) }}
                >
                  <span className="mx-inspect-main">{entry.is_directory ? `${entry.name}/` : entry.name}</span>
                </button>
              </li>
            )
          })}
        </ul>
      </div>
      {file !== null && (
        <section aria-label={`Preview of ${file}`}>
          <h3 className="mx-inspect-file">{file}</h3>
          {preview.isPending && <p className="mx-inspect-note">Loading…</p>}
          {preview.isError && <p className="mx-inspect-note is-error" role="alert">{messageOf(preview.error)}</p>}
          {preview.data !== undefined && (
            <pre className="mx-preview">
              {preview.data.split('\n').map((line, index) => (
                <span key={index} data-line={index + 1} className={lookAt?.path === file && lookAt.line === index + 1 ? 'is-target' : undefined}>{line}{'\n'}</span>
              ))}
            </pre>
          )}
        </section>
      )}
    </div>
  )
}
