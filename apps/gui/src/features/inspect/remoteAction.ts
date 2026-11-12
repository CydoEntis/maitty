import type { SyncAction } from './api'

/** The one thing the remote button does now, and why it cannot when it cannot. */
export type RemoteAction = { action: SyncAction; label: string; title: string; blocked: string | null }

/**
 * One button for the remote, by where the branch stands: behind it pulls, ahead it pushes, level
 * it fetches. Behind and ahead at once cannot be fast-forwarded, so it says so instead of
 * trying (merging is the person's call).
 */
export function remoteAction(ahead: number, behind: number): RemoteAction {
  if (ahead > 0 && behind > 0) {
    return {
      action: 'pull',
      label: `Pull ↓${String(behind)}`,
      title: 'Pull',
      blocked: `This branch has ${String(ahead)} commit${ahead === 1 ? '' : 's'} the remote lacks and the remote has ${String(behind)} it lacks, so it cannot be fast-forwarded. Rebase or merge in a terminal first.`,
    }
  }
  if (behind > 0) return { action: 'pull', label: `Pull ↓${String(behind)}`, title: 'Fast-forward this branch to its upstream', blocked: null }
  if (ahead > 0) return { action: 'push', label: `Push ↑${String(ahead)}`, title: 'Push this branch to its upstream', blocked: null }
  return { action: 'fetch', label: 'Fetch', title: 'Ask the remote what is new (changes no branch)', blocked: null }
}
