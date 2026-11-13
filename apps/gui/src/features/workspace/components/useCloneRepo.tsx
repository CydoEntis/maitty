import { useCallback, useState, type ReactNode } from 'react'
import { cloneRepository } from '@/api/cloneRepository'
import { loadGitHubOwners, loadGitHubRepos } from '@/api/githubRepos'
import { lastCloneDestination, rememberCloneDestination } from '../model/cloneDestination'
import { CloneRepoDialog } from './CloneRepoDialog'

type Target = {
  machine: number
  /** The machine as the dialog names it: "this computer" or its name. */
  where: string
  /** Where the destination starts when nothing is remembered for the machine. */
  startIn: string
  chooseFolder: (() => Promise<string | null>) | null
  /** Opens Settings on the GitHub page, for a machine that is not signed in. The dialog closes first. */
  onOpenGitHubSettings: () => void
}

/**
 * The new chat page's Clone a repository: `open` shows the dialog and answers with the cloned
 * folder, or null when it is cancelled, the way Browse answers with a chosen one.
 */
export function useCloneRepo({ machine, where, startIn, chooseFolder, onOpenGitHubSettings }: Target): { open: () => Promise<string | null>; dialog: ReactNode } {
  const [answer, setAnswer] = useState<((folder: string | null) => void) | null>(null)
  const open = useCallback(() => new Promise<string | null>((resolve) => { setAnswer(() => resolve) }), [])
  const clone = useCallback((address: string, parent: string) => cloneRepository(machine, address, parent), [machine])
  const loadOwners = useCallback(() => loadGitHubOwners(machine), [machine])
  const loadRepos = useCallback((owner: string | null, search: string) => loadGitHubRepos(machine, owner, search), [machine])
  const settle = (folder: string | null) => {
    answer?.(folder)
    setAnswer(null)
  }
  const dialog = answer === null ? null : (
    <CloneRepoDialog
      where={where}
      startIn={lastCloneDestination(where) ?? startIn}
      chooseFolder={chooseFolder}
      clone={clone}
      github={{ loadOwners, loadRepos, onOpenSettings: () => { settle(null); onOpenGitHubSettings() } }}
      onCloned={(folder, parent) => {
        rememberCloneDestination(where, parent)
        settle(folder)
      }}
      onCancel={() => { settle(null) }}
    />
  )
  return { open, dialog }
}
