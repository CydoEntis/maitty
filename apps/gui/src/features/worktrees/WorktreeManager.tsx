import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import type { RepoWorktree } from '@/features/inspect/api'
import { messageOf } from '@/features/inspect/useRepoQuery'
import { mergeWorktree, removeWorktrees, worktreeStanding } from './api'
import { lossOfRemoving, mergeBlock, standingWords } from './worktreeAdvice'

/** How long the daemon is given to remove a worktree before the list is read again. */
const REMOVE_SETTLE_MS = 1500

type RowProps = { entry: RepoWorktree; machine: number; onChanged: () => void }

function WorktreeRow({ entry, machine, onChanged }: RowProps) {
  const standing = useQuery({ queryKey: ['worktree-standing', machine, entry.path], queryFn: () => worktreeStanding(entry.path, machine), retry: false })
  const client = useQueryClient()
  const [busy, setBusy] = useState(false)
  const [sure, setSure] = useState(false)
  const [note, setNote] = useState<string | null>(null)
  const refresh = () => {
    void client.invalidateQueries({ queryKey: ['worktree-standing', machine, entry.path] })
    onChanged()
  }
  const run = (work: () => Promise<string | null>) => {
    setBusy(true)
    setNote(null)
    work()
      .then((said) => { setNote(said) })
      .catch((error: unknown) => { setNote(messageOf(error)) })
      .finally(() => { setBusy(false); setSure(false); refresh() })
  }
  if (standing.isError) return <li><strong>{entry.branch}</strong> <span>{messageOf(standing.error)}</span></li>
  if (!standing.data) return <li><strong>{entry.branch}</strong> <span>Reading…</span></li>
  const data = standing.data
  const loss = lossOfRemoving(data)
  const blocked = mergeBlock(data)
  const remove = () => {
    run(async () => {
      await removeWorktrees([{ path: entry.path, force: data.dirty > 0 }], machine)
      // The daemon removes it in the background and leaves one something still runs in.
      await new Promise((settle) => { window.setTimeout(settle, REMOVE_SETTLE_MS) })
      return 'Asked the service to remove it. If it is still listed, something is running in it: close that first.'
    })
  }
  return (
    <li className="mx-worktree-row">
      <strong>{entry.branch}</strong> <span title={entry.path}>{standingWords(data)}</span>
      <span className="mx-worktree-row-actions">
        <button type="button" className="mx-text-button" disabled={busy || blocked !== null} title={blocked ?? `Merge ${entry.branch} into the main checkout's branch (no fast-forward)`} onClick={() => { run(() => mergeWorktree(entry.path, machine)) }}>Merge</button>
        {sure
          ? <button type="button" className="mx-text-button is-danger" disabled={busy} onClick={remove}>{`Lose ${loss ?? 'nothing'}? Remove`}</button>
          : <button type="button" className="mx-text-button" disabled={busy} title="Closes what runs in it and removes the folder; the branch stays unless it is merged" onClick={() => { if (loss === null) remove(); else setSure(true) }}>Remove</button>}
      </span>
      {note !== null && <small role="status">{note}</small>}
    </li>
  )
}

type Props = { worktrees: readonly RepoWorktree[]; machine: number; repoName: string; onChanged: () => void }

/** The project's worktrees with where each stands, and Merge and Remove for the ones beside the main checkout. */
export function WorktreeManager({ worktrees, machine, repoName, onChanged }: Props) {
  const linked = worktrees.filter((entry) => !entry.main)
  if (linked.length === 0) return null
  return (
    <div className="mx-worktree-list" aria-label="Existing worktrees">
      <span className="mx-menu-caps">Worktrees of {repoName}</span>
      <ul>
        {linked.map((entry) => <WorktreeRow key={entry.path} entry={entry} machine={machine} onChanged={onChanged} />)}
      </ul>
    </div>
  )
}
