import { useState } from 'react'
import { LOCAL_MACHINE } from '@/api/machines'
import { commitAll, createPullRequest, syncRemote } from './api'
import { remoteAction } from './remoteAction'
import { commitTextFor } from './commitText'
import { messageOf } from './useRepoQuery'

/** What the Changes tab needs to commit and open a pull request: the checkout's folder on this computer, and a first message. */
export type ChangeActions = {
  folder: string
  suggested: string
  /** The machine the checkout is on (0: this computer). */
  machine: number
  /** On another machine, the pane there whose checkout is meant: its daemon is asked, and `folder` is not used. */
  term?: number
}

type Props = ChangeActions & {
  /** How many files the commit would take. */
  files: number
  /** The files ticked for the commit. */
  chosen: readonly { path: string; patch: string }[]
  /** Every changed file is ticked, so the commit takes all of them, including any the list was too long to show. */
  everything: boolean
  /** Commits only here, and only on the upstream: what the remote button does depends on them. */
  ahead: number
  behind: number
  /** The checkout changed: read it again. */
  onChanged: () => void
  onNotice: (text: string) => void
}

/**
 * The commit message with **Commit N files**, the remote button and **Create PR**. Nothing runs
 * until a button is pressed. A checkout on another machine has the remote button only: committing
 * and pull requests are this computer's.
 */
export function CommitBar({ folder, machine, term, suggested, files, chosen, everything, ahead, behind, onChanged, onNotice }: Props) {
  const [message, setMessage] = useState(suggested)
  const [busy, setBusy] = useState<'commit' | 'pr' | 'remote' | null>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const [pullRequest, setPullRequest] = useState<string | null>(null)
  const here = machine === LOCAL_MACHINE

  const run = (what: 'commit' | 'pr' | 'remote', work: () => Promise<void>) => {
    setBusy(what)
    setFailure(null)
    work()
      .catch((error: unknown) => { setFailure(messageOf(error)) })
      .finally(() => { setBusy(null) })
  }
  const commit = () => {
    run('commit', async () => {
      const hash = await commitAll(folder, message, machine, everything ? undefined : chosen.map((file) => file.path))
      const count = everything ? files : chosen.length
      onNotice(`Committed ${String(count)} ${count === 1 ? 'file' : 'files'} as ${hash}`)
      setMessage('')
      onChanged()
    })
  }
  const remote = remoteAction(ahead, behind)
  const syncWithRemote = () => {
    run('remote', async () => {
      onNotice(await syncRemote(folder, remote.action, machine, term))
      onChanged()
    })
  }
  const openPullRequest = () => {
    run('pr', async () => {
      const url = await createPullRequest(folder, machine)
      setPullRequest(url)
      onNotice('Pull request opened')
    })
  }

  return (
    <div className="mx-commit">
      {/* With nothing changed there is nothing to commit; the branch can still go up as a pull request. */}
      {here && files > 0 && <textarea
        className="mx-commit-message"
        aria-label="Commit message"
        placeholder="Commit message"
        rows={2}
        value={message}
        disabled={busy !== null}
        onChange={(event) => { setMessage(event.target.value) }}
      />}
      <div className="mx-commit-actions">
        {here && files > 0 && <button type="button" className="mx-text-button" disabled={busy !== null || chosen.length === 0} title="Writes a first message from the files that will be committed" onClick={() => { setMessage(commitTextFor(chosen)) }}>Suggest</button>}
        {here && files > 0 && <button type="button" className="mx-text-button is-primary" disabled={busy !== null || message.trim() === '' || chosen.length === 0} onClick={commit}>
          {busy === 'commit' ? 'Committing…' : `Commit ${String(everything ? files : chosen.length)} ${(everything ? files : chosen.length) === 1 ? 'file' : 'files'}`}
        </button>}
        <button type="button" className="mx-text-button" disabled={busy !== null || remote.blocked !== null} title={remote.blocked ?? remote.title} onClick={syncWithRemote}>
          {busy === 'remote' ? 'Working…' : remote.label}
        </button>
        {here && <button type="button" className="mx-text-button" disabled={busy !== null} title="Pushes this branch and opens a pull request for it with GitHub's gh" onClick={openPullRequest}>
          {busy === 'pr' ? 'Opening…' : 'Create PR'}
        </button>}
      </div>
      {failure !== null && <p className="mx-commit-error" role="alert">{failure}</p>}
      {pullRequest !== null && <p className="mx-commit-link">Pull request: <span>{pullRequest}</span></p>}
    </div>
  )
}
