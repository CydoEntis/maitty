import { useState } from 'react'
import { checkAction, checkActions, checkOutput, checkStateWords, checkTone, type Check, type CheckAction } from './checks'
import { messageOf } from './useRepoQuery'

type Props = {
  term: number
  machine: number
  checks: readonly Check[]
  onNotice: (text: string) => void
}

const SECONDS = 1000

function took(check: Check): string {
  return check.duration_ms > 0 ? ` · ${(check.duration_ms / SECONDS).toFixed(1)}s` : ''
}

function CheckRow({ term, machine, check, onNotice }: { term: number; machine: number; check: Check; onNotice: (text: string) => void }) {
  const [busy, setBusy] = useState(false)
  const [reasoning, setReasoning] = useState<string | null>(null)
  const [output, setOutput] = useState<string | null>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const actions = checkActions(check)
  const send = (request: CheckAction, said: string) => {
    setBusy(true)
    setFailure(null)
    checkAction(term, machine, request)
      .then(() => { onNotice(said); setReasoning(null) })
      .catch((error: unknown) => { setFailure(messageOf(error)) })
      .finally(() => { setBusy(false) })
  }
  const showOutput = () => {
    if (output !== null) { setOutput(null); return }
    checkOutput(term, machine, check.name)
      .then(setOutput)
      .catch((error: unknown) => { setFailure(messageOf(error)) })
  }
  const repoFile = typeof check.source === 'object' ? check.source.Repo.file : null
  return (
    <li className={`mx-check is-${checkTone(check.state)}`}>
      <div className="mx-check-head">
        <strong>{check.name === '' ? 'check' : check.name}</strong>
        <span>{checkStateWords(check)}{took(check)}</span>
      </div>
      <code className="mx-check-command">{check.command}</code>
      {repoFile !== null && <small>Brought by the repository ({repoFile})</small>}
      {check.first_failing_line !== '' && <small className="mx-check-line">{check.first_failing_line}</small>}
      {check.state === 'Overridden' && <small>Accepted{check.overridden_by === '' ? '' : ` by ${check.overridden_by}`}{check.override_reason === '' ? '' : `: ${check.override_reason}`}</small>}
      {check.state === 'Stale' && check.changed_files.length > 0 && <small>Changed since: {check.changed_files.map((file) => file.path).join(', ')}</small>}
      {check.state === 'NeedsApproval' && (
        <small>This command came from the repository{check.previous_command !== null ? ' and is not the one you approved before' : ''}. Run it only if you trust it.</small>
      )}
      <div className="mx-check-actions">
        {actions.includes('rerun') && <button type="button" className="mx-text-button" disabled={busy} onClick={() => { send({ action: 'rerun', name: check.name }, `Running ${check.name} again`) }}>Run again</button>}
        {actions.includes('override') && reasoning === null && <button type="button" className="mx-text-button" disabled={busy} onClick={() => { setReasoning('') }}>Accept the failure…</button>}
        {actions.includes('undo') && <button type="button" className="mx-text-button" disabled={busy} onClick={() => { send({ action: 'undo', name: check.name }, `${check.name} reads as failed again`) }}>Undo accept</button>}
        {actions.includes('approve') && (['once', 'always', 'never'] as const).map((choice) => (
          <button key={choice} type="button" className="mx-text-button" disabled={busy} onClick={() => { send({ action: 'approve', name: check.name, choice, command: check.command }, `${check.name}: ${choice}`) }}>
            {choice === 'once' ? 'Run once' : choice === 'always' ? 'Always run this' : 'Never run'}
          </button>
        ))}
        {check.line_count > 0 && <button type="button" className="mx-text-button" onClick={showOutput}>{output === null ? 'Output' : 'Hide output'}</button>}
      </div>
      {reasoning !== null && (
        <form className="mx-check-reason" onSubmit={(event) => { event.preventDefault(); send({ action: 'override', name: check.name, reason: reasoning.trim() }, `${check.name} accepted as it is`) }}>
          <input aria-label="Why accept this failure" value={reasoning} placeholder="Why (optional)" maxLength={280} onChange={(event) => { setReasoning(event.target.value) }} />
          <button type="submit" className="mx-text-button is-primary" disabled={busy}>Accept</button>
          <button type="button" className="mx-text-button" onClick={() => { setReasoning(null) }}>Cancel</button>
        </form>
      )}
      {output !== null && <pre className="mx-check-output" aria-label={`Output of ${check.name}`}>{output}</pre>}
      {failure !== null && <p className="mx-commit-error" role="alert">{failure}</p>}
    </li>
  )
}

/** What the repository's checks said about the agent's last turn, with the choices the terminal app has: run again, accept a failure with a reason, approve a command the repository brought. */
export function ChecksTab({ term, machine, checks, onNotice }: Props) {
  if (checks.length === 0) {
    return <p className="mx-inspect-note">No checks have run for this session. A check is a command (tests, a build) that runs after each turn; set one in Settings or in the repository’s checks file.</p>
  }
  return (
    <ul className="mx-inspect-list mx-checks" aria-label="Checks">
      {checks.map((check) => <CheckRow key={`${check.name}-${check.turn}`} term={term} machine={machine} check={check} onNotice={onNotice} />)}
    </ul>
  )
}
