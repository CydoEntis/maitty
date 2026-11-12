import { useState } from 'react'
import { commitAll, createPullRequest, syncRemote } from './api'
import { remoteAction } from './remoteAction'
import { messageOf } from './useRepoQuery'

/** What the Changes tab needs to commit and open a pull request: the checkout's folder on this computer, and a first message. */
export type ChangeActions = { folder: string; suggested: string; /** The machine the checkout is on (0: this computer); the host refuses any other. */ machine: number }

type Props = ChangeActions & {
  /** How many files the commit would take. */
  files: number
  /** Commits only here, and only on the upstream: what the remote button does depends on them. */
  ahead: number
  behind: number
  /** The checkout changed: read it again. */
  onChanged: () => void
  onNotice: (text: string) => void
}

/** The commit message with **Commit N files** and **Create PR**. Nothing runs until a button is pressed. */
export function CommitBar({ folder, machine, suggested, files, ahead, behind, onChanged, onNotice }: Props) {
  const [message, setMessage] = useState(suggested)
  const [busy, setBusy] = useState<'commit' | 'pr' | 'remote' | null>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const [pullRequest, setPullRequest] = useState<string | null>(null)

  const run = (what: 'commit' | 'pr' | 'remote', work: () => Promise<void>) => {
    setBusy(what)
    setFailure(null)
    work()
      .catch((error: unknown) => { setFailure(messageOf(error)) })
      .finally(() => { setBusy(null) })
  }
  const commit = () => {
    run('commit', async () => {
      const hash = await commitAll(folder, message, machine)
      onNotice(`Committed ${String(files)} ${files === 1 ? 'file' : 'files'} as ${hash}`)
      setMessage('')
      onChanged()
    })
  }
  const remote = remoteAction(ahead, behind)
  const syncWithRemote = () => {
    run('remote', async () => {
      onNotice(await syncRemote(folder, remote.action, machine))
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
      {files > 0 && <textarea
        className="mx-commit-message"
        aria-label="Commit message"
        placeholder="Commit message"
        rows={2}
        value={message}
        disabled={busy !== null}
        onChange={(event) => { setMessage(event.target.value) }}
      />}
      <div className="mx-commit-actions">
        {files > 0 && <button type="button" className="mx-text-button is-primary" disabled={busy !== null || message.trim() === ''} onClick={commit}>
          {busy === 'commit' ? 'Committing…' : `Commit ${String(files)} ${files === 1 ? 'file' : 'files'}`}
        </button>}
        <button type="button" className="mx-text-button" disabled={busy !== null || remote.blocked !== null} title={remote.blocked ?? remote.title} onClick={syncWithRemote}>
          {busy === 'remote' ? 'Working…' : remote.label}
        </button>
        <button type="button" className="mx-text-button" disabled={busy !== null} title="Pushes this branch and opens a pull request for it with GitHub's gh" onClick={openPullRequest}>
          {busy === 'pr' ? 'Opening…' : 'Create PR'}
        </button>
      </div>
      {failure !== null && <p className="mx-commit-error" role="alert">{failure}</p>}
      {pullRequest !== null && <p className="mx-commit-link">Pull request: <span>{pullRequest}</span></p>}
    </div>
  )
}
