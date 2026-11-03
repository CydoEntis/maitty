import { repoReferences } from './api'
import { ago, formatBytes, MS_PER_SECOND } from './format'
import { RepoError } from './RepoError'
import { useRepoQuery } from './useRepoQuery'

export function ReferencesTab({ term }: { term: number }) {
  const query = useRepoQuery(term, ['references'], () => repoReferences(term))
  if (query.isPending) return <p className="mx-inspect-note">Loading…</p>
  if (query.isError) return <RepoError error={query.error} />
  // When the list was read, so the ages stay the same until it is read again.
  const now = query.dataUpdatedAt / MS_PER_SECOND
  return (
    <div className="mx-inspect-body">
      {query.data.length === 0 && (
        <p className="mx-inspect-note">No references yet. Files in this project&apos;s <code>.maitty/references</code> folder show here, and agents can list them.</p>
      )}
      <ul className="mx-inspect-list">
        {query.data.map((item) => (
          <li key={item.name} className="mx-inspect-row">
            <span className="mx-inspect-main">{item.name}</span>
            <span className="mx-inspect-meta">{item.kind} · {formatBytes(item.bytes)} · {ago(item.modified, now)}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
