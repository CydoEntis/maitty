import { repoBranches, repoStatus } from './api'
import { RepoError } from './RepoError'
import { useRepoQuery } from './useRepoQuery'

export function BranchesTab({ term }: { term: number }) {
  const status = useRepoQuery(term, ['status'], () => repoStatus(term))
  const branches = useRepoQuery(term, ['branches'], () => repoBranches(term))
  if (branches.isPending || status.isPending) return <p className="mx-inspect-note">Loading…</p>
  if (branches.isError) return <RepoError error={branches.error} tab="branches" onRetry={() => { void branches.refetch() }} retrying={branches.isFetching} />
  const here = status.data?.branch
  return (
    <div className="mx-inspect-body">
      {status.data && (
        <p className="mx-inspect-summary">
          {here ?? 'Detached HEAD'}
          {status.data.ahead > 0 && <span className="is-add"> ↑{status.data.ahead}</span>}
          {status.data.behind > 0 && <span className="is-remove"> ↓{status.data.behind}</span>}
        </p>
      )}
      <ul className="mx-inspect-list">
        {branches.data.map((branch) => (
          <li key={branch.name} className={`mx-inspect-row${branch.current ? ' is-current' : ''}`}>
            <span className="mx-inspect-main">{branch.name}</span>
            {branch.current && <span className="mx-inspect-meta">current</span>}
          </li>
        ))}
      </ul>
      <p className="mx-inspect-note">Switching branches happens in the terminal, so the agent sees it too.</p>
    </div>
  )
}
