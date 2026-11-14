import type { ReactNode, RefObject } from 'react'
import { Popover } from 'radix-ui'
import { PopupContent } from '@/components/ui/popup'
import { Icon } from './glyphs'

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  folderName: string
  /** The project's colour square; null when the folder is no known project. */
  color: string | null
  /** The branch the chat starts on, or null outside a git repository. */
  branch: string | null
  /** The chat gets a worktree of its own: the branch is drawn in the worktree colour with its tree. */
  worktree: boolean
  /** The composer it opens over: it rises from the composer's top edge, not the pill's, so the message box stays in view. */
  anchor: RefObject<HTMLElement | null>
  children: ReactNode
}

/**
 * The new chat's "where": a pill at the start of the tray naming the project and branch, which opens upward over the
 * message box with the controls that choose them. Picking inside it keeps it open; Done, Escape or a click outside
 * closes it, and focus goes back to the pill.
 */
export function WherePopover({ open, onOpenChange, folderName, color, branch, worktree, anchor, children }: Props) {
  const place = branch === null ? folderName : `${folderName} · ${branch}`
  return (
    <Popover.Root open={open} onOpenChange={onOpenChange}>
      <Popover.Anchor virtualRef={anchor} />
      <Popover.Trigger asChild>
        <button type="button" className="mx-tray-fact is-button mx-where-pill" title="Where this chat runs" aria-label={`Where this chat runs: ${place}${worktree ? ', in a new worktree' : ''}`}>
          <span className="mx-ns-square" style={{ background: color ?? 'var(--dim)' }} aria-hidden="true" />
          <span className="mx-where-name">{folderName}</span>
          {branch !== null && (
            <>
              <span className="mx-where-dot" aria-hidden="true">·</span>
              <span className={`mx-where-branch${worktree ? ' is-worktree' : ''}`}>
                <Icon name={worktree ? 'worktree' : 'branch'} size={12} />
                <span>{branch}</span>
              </span>
            </>
          )}
          <Icon name="chevronDown" size={10} />
        </button>
      </Popover.Trigger>
      <PopupContent side="top" align="start" aria-label="Where this chat runs" className="mx-where">
        <div className="mx-where-body">{children}</div>
        <div className="mx-where-foot">
          <span className="mx-spacer" />
          <button type="button" className="mx-where-done" onClick={() => { onOpenChange(false) }}>Done</button>
        </div>
      </PopupContent>
    </Popover.Root>
  )
}
