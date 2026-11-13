import { useState } from 'react'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { changeGitRule, type GitConfigWrite, type GitIdentity } from '@/api/gitIdentities'
import { errorMessage } from '@/lib/error-message'

/** Longest folder, name and email the daemon accepts. */
const MOST_PATH_CHARS = 4096
const MOST_TEXT_CHARS = 1024

type Props = {
  machine: number
  /** The machine as the title says it. */
  where: string
  /** The logins signed in there; the rule's account is one of them. */
  logins: readonly string[]
  /** The rule being changed; absent when one is being added. */
  rule?: { folder: string; identity: GitIdentity } | undefined
  /** A folder picker for that machine, whose answer is null when it is cancelled. */
  chooseFolder: () => Promise<string | null>
  onWritten: () => void
  onCancel: () => void
}

type Step = { at: 'editing' } | { at: 'asking' } | { at: 'previewed'; writes: GitConfigWrite[] } | { at: 'writing'; writes: GitConfigWrite[] }

function Preview({ writes }: { writes: readonly GitConfigWrite[] }) {
  return (
    <div className="mx-git-rule-preview" aria-label="Lines to be written" role="group">
      {writes.map((write) => (
        <div key={write.file}>
          <span>{write.file}</span>
          <pre>{write.lines.join('\n')}</pre>
        </div>
      ))}
    </div>
  )
}

/**
 * Gives a folder on a machine its own GitHub account and commit identity. The machine's daemon
 * first says which lines it would write to which git config files; nothing is written until
 * "Write these lines". Changing a field takes the preview away, so what is written is what was shown.
 */
export function GitRuleDialog({ machine, where, logins, rule, chooseFolder, onWritten, onCancel }: Props) {
  const [folder, setFolder] = useState(rule?.folder ?? '')
  const [login, setLogin] = useState(rule?.identity.login ?? logins[0] ?? '')
  const [name, setName] = useState(rule?.identity.name ?? '')
  const [email, setEmail] = useState(rule?.identity.email ?? '')
  const [step, setStep] = useState<Step>({ at: 'editing' })
  const [failure, setFailure] = useState<string | null>(null)
  const busy = step.at === 'asking' || step.at === 'writing'
  const filled = folder.trim() !== '' && login !== '' && name.trim() !== '' && email.trim() !== ''
  const identity: GitIdentity = { name: name.trim(), email: email.trim(), login }

  const edit = (set: (value: string) => void) => (value: string) => {
    set(value)
    setStep({ at: 'editing' })
    setFailure(null)
  }
  const fail = (error: unknown, back: Step) => {
    setFailure(errorMessage(error, 'The machine did not change the rule'))
    setStep(back)
  }
  const preview = () => {
    if (!filled || busy) return
    setStep({ at: 'asking' })
    setFailure(null)
    changeGitRule(machine, { kind: 'preview', folder: folder.trim(), identity }).then(
      (writes) => { setStep({ at: 'previewed', writes }) },
      (error: unknown) => { fail(error, { at: 'editing' }) },
    )
  }
  const write = () => {
    if (step.at !== 'previewed') return
    const { writes } = step
    setStep({ at: 'writing', writes })
    setFailure(null)
    changeGitRule(machine, { kind: 'apply', folder: folder.trim(), identity }).then(
      () => { onWritten() },
      (error: unknown) => { fail(error, { at: 'previewed', writes }) },
    )
  }
  const choose = () => {
    void chooseFolder().then((chosen) => { if (chosen !== null) edit(setFolder)(chosen) })
  }
  const title = rule === undefined ? `Add a folder rule on ${where}` : `Change the folder rule on ${where}`

  return (
    // A write that has started cannot be taken back, so the dialog stays until it has ended.
    <Dialog open onOpenChange={(isOpen) => { if (!isOpen && !busy) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
        <form
          aria-label={title}
          className="mx-rename mx-getfile"
          onSubmit={(event) => { event.preventDefault(); if (step.at === 'previewed') write(); else preview() }}
          onKeyDown={(event) => {
            // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
            if (event.key === 'Escape') event.stopPropagation()
          }}
        >
          <DialogTitle asChild><h2>{title}</h2></DialogTitle>
          <p>Repositories inside this folder use the account and identity below:</p>
          <div className="mx-clone-into">
            <input autoFocus={rule === undefined} aria-label={`Folder on ${where}`} value={folder} maxLength={MOST_PATH_CHARS} spellCheck={false} disabled={busy || rule !== undefined} onChange={(event) => { edit(setFolder)(event.target.value) }} />
            {rule === undefined && <button type="button" className="mx-text-button" disabled={busy} onClick={choose}>Choose…</button>}
          </div>
          <select aria-label="GitHub account" value={login} disabled={busy} onChange={(event) => { edit(setLogin)(event.target.value) }}>
            {logins.map((each) => <option key={each} value={each}>{each}</option>)}
          </select>
          <input aria-label="Commit name" placeholder="Name on commits" value={name} maxLength={MOST_TEXT_CHARS} disabled={busy} onChange={(event) => { edit(setName)(event.target.value) }} />
          <input aria-label="Commit email" placeholder="Email on commits" type="email" value={email} maxLength={MOST_TEXT_CHARS} spellCheck={false} disabled={busy} onChange={(event) => { edit(setEmail)(event.target.value) }} />
          {(step.at === 'previewed' || step.at === 'writing') && <Preview writes={step.writes} />}
          {step.at === 'asking' && <p role="status">Asking {where} what it would write…</p>}
          {failure !== null && <p role="alert">{failure}</p>}
          <div className="mx-rename-actions">
            <button type="button" className="mx-text-button" disabled={busy} onClick={onCancel}>Cancel</button>
            {step.at === 'previewed' || step.at === 'writing'
              ? <button type="submit" className="mx-text-button is-primary" disabled={busy}>{step.at === 'writing' ? 'Writing…' : 'Write these lines'}</button>
              : <button type="submit" className="mx-text-button is-primary" disabled={!filled || busy}>Show the lines</button>}
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
