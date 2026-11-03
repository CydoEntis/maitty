import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import { repoWorktrees } from '@/features/inspect/api'
import { useRepoQuery } from '@/features/inspect/useRepoQuery'

type Props = {
  /** The daemon's pane number of any session in the project. */
  term: number
  portalContainer: HTMLElement | null
  canStart: boolean
  /** Starts a shell in the folder. */
  onOpenFolder: (folder: string) => void
  onNewWorktree: () => void
}

/** The project menu's worktrees: each opens a shell in it, and the last row makes another. Read only when the submenu opens. */
export function WorktreeMenu({ term, portalContainer, canStart, onOpenFolder, onNewWorktree }: Props) {
  return (
    <ContextMenuPrimitive.Sub>
      <ContextMenuPrimitive.SubTrigger className="mx-menu-item" disabled={!canStart}>Worktrees</ContextMenuPrimitive.SubTrigger>
      <ContextMenuPrimitive.Portal container={portalContainer}>
        <ContextMenuPrimitive.SubContent className="mx-menu" aria-label="Worktrees">
          <WorktreeItems term={term} onOpenFolder={onOpenFolder} />
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={onNewWorktree}>New worktree…</ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.SubContent>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Sub>
  )
}

function WorktreeItems({ term, onOpenFolder }: { term: number; onOpenFolder: (folder: string) => void }) {
  const query = useRepoQuery(term, ['worktrees'], () => repoWorktrees(term))
  if (query.isPending) return <ContextMenuPrimitive.Label className="mx-menu-caps">Loading…</ContextMenuPrimitive.Label>
  if (query.isError) return <ContextMenuPrimitive.Label className="mx-menu-caps">Could not read them</ContextMenuPrimitive.Label>
  return query.data.map((entry) => (
    <ContextMenuPrimitive.Item key={entry.path} className="mx-menu-item" title={entry.path} onSelect={() => { onOpenFolder(entry.path) }}>
      {entry.branch}{entry.main ? ' · main' : ''}
    </ContextMenuPrimitive.Item>
  ))
}
