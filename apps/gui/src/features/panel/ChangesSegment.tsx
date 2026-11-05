import { GitBranch } from 'lucide-react'
import type { ConversationDiff } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import type { SentComment } from './commentStore'
import { splitPath } from './patchLines'

type Query = { isPending: boolean; isError: boolean; error: unknown; data: ConversationDiff | undefined }

type Props = {
  query: Query
  /** The session's branch, from the session list; `null` when it is not in a repository. */
  branch: string | null
  comments: readonly SentComment[]
  onOpen: (path: string) => void
}

/**
 * This agent's worktree changes (v18 C.2). The daemon reports the working tree against its last
 * commit; it does not say which files are staged, how far the branch is ahead or behind, or whether it
 * was pushed, so none of that is shown.
 */
export function ChangesSegment({ query, branch, comments, onOpen }: Props) {
  if (query.isPending) return <p className="m-0 px-1 py-4 text-center text-[12.5px] text-dim">Reading the changes…</p>
  if (query.isError) return <p role="alert" className="m-0 px-1 py-4 text-center text-[12.5px] text-[var(--danger)]">{errorMessage(query.error, 'Could not read the changes')}</p>
  const diff = query.data
  if (diff === undefined) return null
  return (
    <div className="flex flex-col gap-2">
      {branch !== null && (
        <p className="m-0 flex items-center gap-1.5 rounded-lg border border-line2 bg-raise px-2.5 py-1.5 font-mono text-[12px] text-[var(--ansi-mag)]">
          <GitBranch className="size-3.5 shrink-0" aria-hidden="true" />
          <span className="truncate">{branch}</span>
        </p>
      )}
      {diff.files.length === 0 ? (
        <p className="m-0 px-1 py-4 text-center text-[12.5px] text-dim">No changes in this worktree.</p>
      ) : (
        <ul className="m-0 flex list-none flex-col gap-0.5 p-0" aria-label="Changed files">
          {diff.files.map((file) => {
            const { dir, name } = splitPath(file.path)
            const count = comments.filter((comment) => comment.file === file.path).length
            return (
              <li key={file.path}>
                <button type="button" className="flex min-h-7 w-full items-center gap-2 rounded-lg px-2 text-left text-[12.5px] hover:bg-hover" onClick={() => { onOpen(file.path) }}>
                  <span className="min-w-0 flex-1 truncate" title={file.path}><span className="text-dim">{dir}</span><span className="text-fg">{name}</span></span>
                  {count > 0 && <span className="shrink-0 rounded-full bg-brand px-1.5 text-[10.5px] leading-4 text-[var(--on-accent)]" role="img" aria-label={`${String(count)} comment${count === 1 ? '' : 's'}`}>{count}</span>}
                  <span className="shrink-0 font-mono text-[11.5px] text-[var(--ok)]">+{file.additions}</span>
                  <span className="shrink-0 font-mono text-[11.5px] text-[var(--danger)]">−{file.deletions}</span>
                </button>
              </li>
            )
          })}
        </ul>
      )}
      {diff.truncated && <p className="m-0 px-1 text-[11.5px] text-dim">Showing {diff.files.length} of {diff.total_files} changed files.</p>}
    </div>
  )
}
