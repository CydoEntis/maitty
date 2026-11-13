import { useEffect, useState } from 'react'
import { chooseProjectFolder } from '@/api/folderPicker'
import type { GitHubAccount } from '@/api/githubAccounts'
import { changeGitRule, loadGitIdentities, type GitIdentity, type GitIdentityReport, type GitIdentityRule } from '@/api/gitIdentities'
import { LOCAL_MACHINE } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import { useRemoteFolderPick } from '@/features/workspace/components/useRemoteFolderPick'
import { GitRuleDialog } from './GitRuleDialog'

type Loaded = { status: 'loading' } | { status: 'ready'; report: GitIdentityReport } | { status: 'failed'; message: string }
type MaittyRule = { folder: string; identity: GitIdentity }
/** The rule dialog: adding one (no rule), or changing the one given. */
type Editing = { rule?: MaittyRule | undefined } | null
/** Removing a rule: asked once, then sent. */
type Removing = { folder: string; sending: boolean } | null

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

type RuleActions = {
  removing: Removing
  onChange: (rule: MaittyRule) => void
  onRemove: (folder: string) => void
  onConfirmRemove: () => void
  onKeep: () => void
}

function RuleRow({ rule, actions }: { rule: GitIdentityRule; actions: RuleActions | undefined }) {
  const { identity, folder } = rule
  const asking = actions?.removing?.folder === folder
  const sending = actions?.removing?.sending === true
  return (
    <li>
      <span>{folder}</span>
      <small>{ruleWords(rule)}</small>
      {actions !== undefined && identity !== null && !asking && (
        <>
          <button type="button" className="mx-text-button" aria-label={`Change the rule for ${folder}`} onClick={() => { actions.onChange({ folder, identity }) }}>Change</button>
          <button type="button" className="mx-text-button" aria-label={`Remove the rule for ${folder}`} onClick={() => { actions.onRemove(folder) }}>Remove</button>
        </>
      )}
      {actions !== undefined && asking && (
        <>
          <button type="button" className="mx-text-button" disabled={sending} onClick={actions.onConfirmRemove}>{sending ? 'Removing…' : 'Remove this rule'}</button>
          <button type="button" className="mx-text-button" disabled={sending} onClick={actions.onKeep}>Keep</button>
        </>
      )}
    </li>
  )
}

function Identities({ state, active, actions }: { state: Loaded; active: string | undefined; actions: RuleActions | undefined }) {
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
            {report.rules.map((rule) => <RuleRow key={rule.folder} rule={rule} actions={actions} />)}
          </ul>
        )}
    </>
  )
}

/**
 * Under a machine's row on the GitHub page: every account signed in there, the name and email its
 * commits carry, and the folders where a rule gives another account and identity. A rule Maitty
 * wrote can be changed or removed here, and another added; that machine's daemon writes git's config.
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
  const [changed, setChanged] = useState(0)
  const [editing, setEditing] = useState<Editing>(null)
  const [removing, setRemoving] = useState<Removing>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const remotePick = useRemoteFolderPick(machine, label)
  useEffect(() => {
    let current = true
    loadGitIdentities(machine).then(
      (report) => { if (current) setState({ status: 'ready', report }) },
      (error: unknown) => { if (current) setState({ status: 'failed', message: errorMessage(error, 'the machine did not answer') }) },
    )
    return () => { current = false }
  }, [machine, asked, changed])
  const active = accounts.find((account) => account.active)?.login ?? accounts[0]?.login
  const ready = state.status === 'ready' && state.report.installed && state.report.problem === null
  const reread = () => { setChanged((count) => count + 1) }
  const confirmRemove = () => {
    if (removing === null) return
    const { folder } = removing
    setRemoving({ folder, sending: true })
    setFailure(null)
    changeGitRule(machine, { kind: 'remove', folder }).then(
      () => { setRemoving(null); reread() },
      (error: unknown) => {
        setFailure(errorMessage(error, 'The machine did not remove the rule'))
        setRemoving(null)
      },
    )
  }
  const actions: RuleActions | undefined = ready
    ? {
        removing,
        onChange: (rule) => { setFailure(null); setEditing({ rule }) },
        onRemove: (folder) => { setFailure(null); setRemoving({ folder, sending: false }) },
        onConfirmRemove: confirmRemove,
        onKeep: () => { setRemoving(null) },
      }
    : undefined
  return (
    <div className="mx-github-details" role="group" aria-label={`GitHub details for ${label}`}>
      <span className="mx-menu-caps">Accounts</span>
      <ul aria-label="Accounts">
        {accounts.map((account) => <li key={account.login}><span>{account.login}</span>{account.login === active && <small>active</small>}</li>)}
      </ul>
      {onAddAccount !== undefined && <button type="button" className="mx-scard-button" onClick={onAddAccount}>Add another account</button>}
      <span className="mx-menu-caps">Commits</span>
      <Identities state={state} active={active} actions={actions} />
      {failure !== null && <p role="alert">{failure}</p>}
      {ready && <button type="button" className="mx-scard-button" onClick={() => { setFailure(null); setEditing({}) }}>Add a folder rule</button>}
      {editing !== null && (
        <GitRuleDialog
          machine={machine}
          where={label}
          logins={accounts.map((account) => account.login)}
          rule={editing.rule}
          chooseFolder={machine === LOCAL_MACHINE ? chooseProjectFolder : remotePick.browse}
          onWritten={() => { setEditing(null); reread() }}
          onCancel={() => { setEditing(null) }}
        />
      )}
      {remotePick.dialog}
    </div>
  )
}
