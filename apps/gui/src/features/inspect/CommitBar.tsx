import { useState } from 'react'
import { commitAll, createPullRequest } from './api'
import { messageOf } from './useRepoQuery'

/** What the Changes tab needs to commit and open a pull request: the checkout's folder on this computer, and a first message. */
export type ChangeActions = { folder: string; suggested: string }

type Props = ChangeActions & {
  /** How many files the commit would take. */
  files: number
  /** The checkout changed: read it again. */
  onChanged: () => void
  onNotice: (text: string) => void
}

/** The commit message with **Commit N files** and **Create PR**. Nothing runs until a button is pressed. */
export function CommitBar({ folder, suggested, files, onChanged, onNotice }: Props) {
  const [message, setMessage] = useState(suggested)
  const [busy, setBusy] = useState<'commit' | 'pr' | null>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const [pullRequest, setPullRequest] = useState<string | null>(null)

  const run = (what: 'commit' | 'pr', work: () => Promise<void>) => {
    setBusy(what)
    setFailure(null)
    work()
      .catch((error: unknown) => { setFailure(messageOf(error)) })
      .finally(() => { setBusy(null) })
  }
  const commit = () => {
    run('commit', async () => {
      const hash = await commitAll(folder, message)
      onNotice(`Committed ${String(files)} ${files === 1 ? 'file' : 'files'} as ${hash}`)
      setMessage('')
      onChanged()
    })
  }
  const openPullRequest = () => {
    run('pr', async () => {
      const url = await createPullRequest(folder)
      setPullRequest(url)
      onNotice('Pull request opened')
    })
  }

  return (
    <div className="mx-commit">
      <textarea
        className="mx-commit-message"
        aria-label="Commit message"
        placeholder="Commit message"
        rows={2}
        value={message}
        disabled={busy !== null}
        onChange={(event) => { setMessage(event.target.value) }}
      />
      <div className="mx-commit-actions">
        <button type="button" className="mx-text-button is-primary" disabled={busy !== null || files === 0 || message.trim() === ''} onClick={commit}>
          {busy === 'commit' ? 'Committing…' : `Commit ${String(files)} ${files === 1 ? 'file' : 'files'}`}
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
