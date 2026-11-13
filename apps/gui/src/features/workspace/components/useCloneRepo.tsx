import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { cloneRepository } from '@/api/cloneRepository'
import { loadGitHubOwners, loadGitHubRepos } from '@/api/githubRepos'
import { loadGitIdentities, ruleCovering, type GitIdentityRule } from '@/api/gitIdentities'
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

const NO_RULES: readonly GitIdentityRule[] = []

/**
 * The machine's folder rules, read each time the dialog opens. A machine that cannot say has none:
 * the dialog then names no account, as it does for a folder no rule covers.
 */
function useFolderRules(machine: number, open: boolean): readonly GitIdentityRule[] {
  const [rules, setRules] = useState(NO_RULES)
  useEffect(() => {
    if (!open) return undefined
    let current = true
    loadGitIdentities(machine).then(
      (report) => { if (current) setRules(report.rules) },
      // shortcut: the account line is a hint; a machine that could not be asked shows no line rather than an error in the clone dialog.
      () => { if (current) setRules(NO_RULES) },
    )
    return () => { current = false }
  }, [machine, open])
  return rules
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
  const rules = useFolderRules(machine, answer !== null)
  const accountFor = useCallback((parent: string) => ruleCovering(rules, parent)?.login ?? null, [rules])
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
      accountFor={accountFor}
      onCloned={(folder, parent) => {
        rememberCloneDestination(where, parent)
        settle(folder)
      }}
      onCancel={() => { settle(null) }}
    />
  )
  return { open, dialog }
}
