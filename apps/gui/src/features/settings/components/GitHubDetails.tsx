import { useEffect, useState } from 'react'
import type { GitHubAccount } from '@/api/githubAccounts'
import { loadGitIdentities, type GitIdentityReport, type GitIdentityRule } from '@/api/gitIdentities'
import { errorMessage } from '@/lib/error-message'

type Loaded = { status: 'loading' } | { status: 'ready'; report: GitIdentityReport } | { status: 'failed'; message: string }

/** What the row says about the name and email commits carry where no rule applies. */
function defaultWords(report: GitIdentityReport): string {
  if (report.name !== null && report.email !== null) return `Commits as ${report.name} <${report.email}>`
  if (report.name !== null) return `Commits as ${report.name}, with no email set`
  if (report.email !== null) return `Commits as <${report.email}>, with no name set`
  return 'No name or email is set for commits'
}

/** What a rule gives its folder, or that Maitty did not write it and so knows only the folder. */
function ruleWords(rule: GitIdentityRule): string {
  if (rule.identity === null) return 'set outside Maitty'
  const { login, name, email } = rule.identity
  return `${login} · ${name} <${email}>`
}

function Identities({ state, active }: { state: Loaded; active: string | undefined }) {
  if (state.status === 'loading') return <p>Reading git’s settings…</p>
  if (state.status === 'failed') return <p role="alert">Could not read git’s settings: {state.message}</p>
  const { report } = state
  if (!report.installed) return <p>git is not installed on this machine.</p>
  if (report.problem !== null) return <p role="alert">Could not read git’s settings: {report.problem}</p>
  return (
    <>
      <p>{defaultWords(report)}</p>
      <span className="mx-menu-caps">Folder rules</span>
      {report.rules.length === 0
        ? <p>{active === undefined ? 'No folder has a rule of its own' : `Every folder uses ${active}`}</p>
        : (
          <ul aria-label="Folder rules">
            {report.rules.map((rule) => <li key={rule.folder}><span>{rule.folder}</span><small>{ruleWords(rule)}</small></li>)}
          </ul>
        )}
    </>
  )
}

/**
 * Under a machine's row on the GitHub page: every account signed in there, the name and email its
 * commits carry, and the folders where a rule gives another account and identity. Read from that
 * machine's `gh` and git; nothing is changed from here but starting another sign-in.
 */
export function GitHubDetails({ machine, label, accounts, asked, onAddAccount }: {
  machine: number
  label: string
  accounts: readonly GitHubAccount[]
  /** Goes up when the page asks every machine again. */
  asked: number
  /** Starts GitHub's own sign-in on the machine; absent where the window cannot start one. */
  onAddAccount?: (() => void) | undefined
}) {
  const [state, setState] = useState<Loaded>({ status: 'loading' })
  useEffect(() => {
    let current = true
    loadGitIdentities(machine).then(
      (report) => { if (current) setState({ status: 'ready', report }) },
      (error: unknown) => { if (current) setState({ status: 'failed', message: errorMessage(error, 'the machine did not answer') }) },
    )
    return () => { current = false }
  }, [machine, asked])
  const active = accounts.find((account) => account.active)?.login ?? accounts[0]?.login
  return (
    <div className="mx-github-details" role="group" aria-label={`GitHub details for ${label}`}>
      <span className="mx-menu-caps">Accounts</span>
      <ul aria-label="Accounts">
        {accounts.map((account) => <li key={account.login}><span>{account.login}</span>{account.login === active && <small>active</small>}</li>)}
      </ul>
      {onAddAccount !== undefined && <button type="button" className="mx-scard-button" onClick={onAddAccount}>Add another account</button>}
      <span className="mx-menu-caps">Commits</span>
      <Identities state={state} active={active} />
    </div>
  )
}
