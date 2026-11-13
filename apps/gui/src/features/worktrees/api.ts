import { z } from 'zod'
import { invoke, isTauri } from '@tauri-apps/api/core'

/**
 * Make a worktree for `branch` next to the repository `term` is in and open a session in it. A
 * branch that already exists is checked out; a new one starts from `base` (null: the current commit).
 */
export async function newWorktreeSession(term: number, branch: string, base: string | null): Promise<void> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  await invoke('new_worktree_session', { term, branch, base })
}

const StandingSchema = z.object({
  dirty: z.number().int().nonnegative(),
  branch: z.string().nullable(),
  merged: z.boolean(),
  last_commit: z.string(),
  /** Days with no commit and no file change, once past `worktree.inactive_days`; null otherwise. */
  inactive_days: z.number().int().nonnegative().nullable(),
  missing: z.boolean(),
})

/** Where a worktree stands, as the daemon read it: what removing it would lose. */
export type WorktreeStanding = z.infer<typeof StandingSchema>

/** Where the worktree at `folder` (on `machine`; only this computer's, 0, is answered) stands. */
export async function worktreeStanding(folder: string, machine: number): Promise<WorktreeStanding> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  return StandingSchema.parse(await invoke<unknown>('worktree_standing', { machine, folder }))
}

const OverlapSchema = z.array(z.object({ file: z.string(), checkouts: z.array(z.string()) }))

/** The files more than one of these checkouts (name, folder) changed since their branches parted, committed or not. */
export async function worktreeOverlaps(checkouts: readonly { name: string; path: string }[], machine: number): Promise<z.infer<typeof OverlapSchema>> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  return OverlapSchema.parse(await invoke<unknown>('worktree_overlaps', { machine, checkouts: checkouts.map((checkout) => [checkout.name, checkout.path]) }))
}

/** Makes a worktree whose folder was deleted by hand again, on its branch; resolves to a line to show. */
export async function recreateWorktree(main: string, folder: string, branch: string, machine: number): Promise<string> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  return z.string().parse(await invoke<unknown>('worktree_recreate', { machine, main, folder, branch }))
}

/** Merges the worktree's committed work into the main checkout's branch; resolves to a line to show. */
export async function mergeWorktree(folder: string, machine: number): Promise<string> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  return z.string().parse(await invoke<unknown>('worktree_merge', { machine, folder }))
}

/**
 * Removes worktrees with the daemon, which closes what runs in each first. `force` goes on when the worktree
 * holds uncommitted changes (they are lost). The daemon removes them in the background and leaves one something
 * still runs in, so the caller looks again afterwards.
 */
export async function removeWorktrees(items: readonly { path: string; force: boolean }[], machine: number): Promise<void> {
  if (!isTauri()) throw new Error('Worktrees are only available in the desktop app')
  await invoke('remove_worktrees', { machine, items: items.map((item) => [item.path, item.force]) })
}
