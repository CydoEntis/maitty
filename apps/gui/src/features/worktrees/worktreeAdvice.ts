import type { WorktreeStanding } from './api'

/** What a worktree's standing says in a few words. */
export function standingWords(standing: WorktreeStanding): string {
  const ago = standing.last_commit === '' ? '' : ` · last commit ${standing.last_commit}`
  if (standing.dirty > 0) return `${String(standing.dirty)} uncommitted ${standing.dirty === 1 ? 'change' : 'changes'}${ago}`
  return `${standing.merged ? 'merged, nothing uncommitted' : 'not merged anywhere'}${ago}`
}

/** What pressing Remove would lose, or null when nothing: the button then asks twice only when it must. */
export function lossOfRemoving(standing: WorktreeStanding): string | null {
  if (standing.dirty > 0) return `its ${String(standing.dirty)} uncommitted ${standing.dirty === 1 ? 'change' : 'changes'}`
  if (!standing.merged) return 'commits that are on no other branch (the branch is kept, the folder goes)'
  return null
}

/** Why Merge cannot go ahead, or null when it can. */
export function mergeBlock(standing: WorktreeStanding): string | null {
  if (standing.branch === null) return 'It is on no branch.'
  if (standing.dirty > 0) return 'Commit its changes first.'
  if (standing.merged) return 'Its work is already on another branch.'
  return null
}
