import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { useEffect, useRef, useState } from 'react'
import { repoBranches, repoWorktrees } from '@/features/inspect/api'
import { messageOf, useRepoQuery } from '@/features/inspect/useRepoQuery'
import { branchNameProblem } from './branchName'

const CURRENT_COMMIT = ''

type Props = {
  /** The daemon's pane number of the session the worktree is made from. */
  term: number
  repoName: string
  onCreate: (branch: string, base: string | null) => Promise<void>
  onCancel: () => void
}

/** ⌘⇧N: a new branch in a folder of its own next to the repo, with a session open in it. */
export function NewWorktreeDialog({ term, repoName, onCreate, onCancel }: Props) {
  const [branch, setBranch] = useState('')
  const [base, setBase] = useState(CURRENT_COMMIT)
  const [error, setError] = useState<string | null>(null)
  const [working, setWorking] = useState(false)
  const input = useRef<HTMLInputElement>(null)
  const branches = useRepoQuery(term, ['branches'], () => repoBranches(term))
  const worktrees = useRepoQuery(term, ['worktrees'], () => repoWorktrees(term))

  useEffect(() => {
    input.current?.focus()
    // The menu or shortcut that opened this may still be giving focus back; claim it once more.
    const timer = window.setTimeout(() => { input.current?.focus() }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])

  const name = branch.trim()
  const existing = branches.data?.some((candidate) => candidate.name === name) ?? false
  const problem = name === '' ? null : branchNameProblem(name)

  const submit = () => {
    const found = branchNameProblem(name)
    if (found) {
      setError(found)
      return
    }
    setWorking(true)
    setError(null)
    void onCreate(name, existing || base === CURRENT_COMMIT ? null : base)
      .catch((failure: unknown) => { setError(messageOf(failure)) })
      .finally(() => { setWorking(false) })
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
      <form
        aria-label="New worktree"
        className="mx-rename mx-worktree"
        onSubmit={(event) => { event.preventDefault(); submit() }}
        onKeyDown={(event) => { if (event.key === 'Escape') event.stopPropagation() }}
      >
        <DialogTitle asChild><h2>New worktree</h2></DialogTitle>
        <p>A branch in its own folder next to {repoName}. A session opens there, and the other sessions keep working undisturbed.</p>
        <input ref={input} aria-label="Branch name" value={branch} placeholder="feat/my-change" onChange={(event) => { setBranch(event.target.value); setError(null) }} />
        {existing
          ? <p className="mx-worktree-note">That branch exists; the worktree checks it out.</p>
          : (
            <label className="mx-worktree-base">
              Start from
              <select value={base} onChange={(event) => { setBase(event.target.value) }}>
                <option value={CURRENT_COMMIT}>The current commit</option>
                {branches.data?.map((candidate) => <option key={candidate.name} value={candidate.name}>{candidate.name}</option>)}
              </select>
            </label>
          )}
        {(error ?? problem) && <p className="mx-worktree-error" role="alert">{error ?? problem}</p>}
        {worktrees.data && worktrees.data.length > 1 && (
          <div className="mx-worktree-list" aria-label="Existing worktrees">
            <span className="mx-menu-caps">Worktrees of {repoName}</span>
            <ul>
              {worktrees.data.map((entry) => <li key={entry.path}><strong>{entry.branch}</strong> <span>{entry.main ? 'main checkout' : entry.path}</span></li>)}
            </ul>
          </div>
        )}
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary" disabled={working || name === '' || problem !== null}>{working ? 'Creating…' : 'Create worktree'}</button>
        </div>
      </form>
      </DialogContent>
    </Dialog>
  )
}
