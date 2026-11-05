import { invoke, isTauri } from '@tauri-apps/api/core'

export type NewBranchAgent = { project: string; branch: string; agent: string; prompt: string }

/**
 * A branch in its own worktree of `project` (a local repository the app shows), with `agent` started
 * in it and `prompt` as its first prompt. The daemon checks every part and opens the session; the
 * session appears in the list under its project.
 */
export async function newBranchAgent(request: NewBranchAgent): Promise<void> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  await invoke('new_branch_agent', request)
}

/**
 * Make a worktree for `branch` next to the repository `term` is in and open a session in it. A
 * branch that already exists is checked out; a new one starts from `base` (null: the current commit).
 */
export async function newWorktreeSession(term: number, branch: string, base: string | null): Promise<void> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  await invoke('new_worktree_session', { term, branch, base })
}
