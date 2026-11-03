import { useState } from 'react'
import { repoDiff } from './api'
import { PatchView } from './PatchView'
import { messageOf, useRepoQuery } from './useRepoQuery'

export const CHANGES_REFRESH_MS = 4000

export function ChangesTab({ term }: { term: number }) {
  const query = useRepoQuery(term, ['diff'], () => repoDiff(term), CHANGES_REFRESH_MS)
  const [picked, setPicked] = useState<string | null>(null)
  if (query.isPending) return <p className="mx-inspect-note">Loading…</p>
  if (query.isError) return <p className="mx-inspect-note is-error" role="alert">{messageOf(query.error)}</p>
  const { files } = query.data
  const selected = files.find((file) => file.path === picked) ?? files[0]
  if (!selected) return <p className="mx-inspect-note">No changes. The working tree matches the last commit.</p>
  return (
    <div className="mx-inspect-split">
      <ul className="mx-inspect-list" aria-label="Changed files">
        {files.map((file) => (
          <li key={file.path}>
            <button type="button" className={`mx-inspect-row${file.path === selected.path ? ' is-current' : ''}`} onClick={() => { setPicked(file.path) }}>
              <span className="mx-inspect-main">{file.path}</span>
              <span className="mx-inspect-meta"><span className="is-add">+{file.additions}</span> <span className="is-remove">−{file.deletions}</span></span>
            </button>
          </li>
        ))}
        {query.data.truncated && <li className="mx-inspect-note">Showing {files.length} of {query.data.total_files} files.</li>}
      </ul>
      {selected.binary ? <p className="mx-inspect-note">Binary file; no diff to show.</p> : <PatchView patch={selected.patch} />}
    </div>
  )
}
