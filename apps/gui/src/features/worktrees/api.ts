import { invoke, isTauri } from '@tauri-apps/api/core'

/**
 * Make a worktree for `branch` next to the repository `term` is in and open a session in it. A
 * branch that already exists is checked out; a new one starts from `base` (null: the current commit).
 */
export async function newWorktreeSession(term: number, branch: string, base: string | null): Promise<void> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  await invoke('new_worktree_session', { term, branch, base })
}
