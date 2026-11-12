import { useMemo, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { switchBranch } from '@/api/daemon'
import { toast } from '@/lib/toast'
import { errorMessage } from '@/lib/error-message'
import { Icon } from '@/features/workspace/components/glyphs'
import { ChipMenu, type ChipItem } from '@/features/workspace/components/NewChatMenus'
import { useFolderBranches } from '@/features/workspace/model/newChatHost'

/** Where a running chat's folder can be moved to, and how. */
export type BranchMenuSpec = {
  /** The session's own folder (a worktree's, not the repository's). */
  folder: string
  /** Opens the dialog that makes a worktree for this session's repository; absent where it cannot. */
  onNewWorktree?: (() => void) | undefined
}

/**
 * The tray's branch button (layout 2g): it opens, above the box, **This chat works on** with the folder's
 * branches, the current one ticked. Picking another switches the folder's branch, so the agent's next edits go there;
 * work already done stays on its branch.
 */
export function BranchMenu({ spec, branch, worktree }: { spec: BranchMenuSpec; branch: string; worktree: boolean }) {
  const queryClient = useQueryClient()
  const [read, reload] = useFolderBranches(spec.folder)
  const [busy, setBusy] = useState(false)
  const inTree = useMemo(() => new Set((read?.worktrees ?? []).flatMap((tree) => (tree.branch === null ? [] : [tree.branch]))), [read])
  const move = (name: string) => {
    if (busy || name === branch) return
    setBusy(true)
    switchBranch(spec.folder, name, false)
      .then(() => { toast.success(`Now working on ${name}`); reload(); return queryClient.invalidateQueries({ queryKey: ['conversations'] }) })
      .catch((error: unknown) => { toast.error(errorMessage(error, `Could not switch to ${name}`)) })
      .finally(() => { setBusy(false) })
  }
  const rows: ChipItem[] = (read?.branches ?? [{ name: branch, when: null }]).map((entry) => ({
    key: entry.name,
    label: entry.name,
    ...(entry.name === branch ? { sub: 'This chat is on it' } : inTree.has(entry.name) ? { sub: 'worktree' } : {}),
    icon: <Icon name={inTree.has(entry.name) ? 'worktree' : 'branch'} size={13} />,
    checked: entry.name === branch,
    disabled: busy || (inTree.has(entry.name) && entry.name !== branch),
    onPick: () => { move(entry.name) },
  }))
  return (
    <ChipMenu
      title="This chat works on"
      summary={branch}
      side="top"
      width={300}
      label={<span className="mx-nc-branch">{branch}</span>}
      icon={<span className="mx-nc-branch"><Icon name={worktree ? 'worktree' : 'branch'} size={12} /></span>}
      sections={[{ heading: 'This chat works on', items: rows }]}
      footer={(
        <>
          {spec.onNewWorktree !== undefined && <button type="button" className="mx-chip-action" onClick={spec.onNewWorktree}><Icon name="worktree" size={12} />Move to a new worktree…</button>}
          <p className="mx-chip-note">Switching changes where the agent's next edits go. Work already done stays on its branch.</p>
        </>
      )}
    />
  )
}
