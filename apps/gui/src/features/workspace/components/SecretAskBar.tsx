import { useRef, useState } from 'react'
import { answerSecret } from '@/api/daemon'
import { messageOf } from '@/features/inspect/useRepoQuery'
import type { SecretAsk } from '../model/session'

type Props = {
  ask: SecretAsk
  /** The machine the session is on; a secret is only taken for this computer's sessions. */
  machine: number
  online: boolean
}

/**
 * An agent asked for a secret (`maitty_request_secret`). Only the person answers, here and never in the chat
 * (ADR 0006): the value goes from this box to the daemon and nowhere else, is not kept in the window's state
 * once sent, and the agent gets a single-use reference, never the value.
 */
export function SecretAskBar({ ask, machine, online }: Props) {
  const [open, setOpen] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const field = useRef<HTMLInputElement>(null)
  const stop = (event: { stopPropagation: () => void }) => { event.stopPropagation() }
  const send = (value: string | null) => {
    setBusy(true)
    setFailure(null)
    answerSecret(ask.id, machine, value)
      .then(() => { setOpen(false) })
      .catch((error: unknown) => { setFailure(messageOf(error)) })
      .finally(() => {
        // The value never outlives the answer: the field is emptied whether it was taken or not.
        if (field.current) field.current.value = ''
        setBusy(false)
      })
  }
  return (
    <div className="mx-pane-ended is-ask" role="alert" aria-label="Wants a secret">
      <span className="mx-pane-ended-text">
        <strong>{`Wants a secret: ${ask.label}`}</strong>
        <small title={ask.reason}>{ask.reason === '' ? 'It will get a single-use reference, never the value.' : ask.reason}</small>
        {failure !== null && <small role="alert">{failure}</small>}
      </span>
      {open
        ? (
          <form
            className="mx-secret-form"
            onSubmit={(event) => { event.preventDefault(); const value = field.current?.value ?? ''; if (value !== '') send(value) }}
            onMouseDown={stop}
          >
            <input ref={field} type="password" autoComplete="off" spellCheck={false} aria-label={`Value for ${ask.label}`} autoFocus disabled={busy} />
            <button type="submit" className="mx-pane-ended-button is-primary" disabled={busy || !online}>Give it</button>
            <button type="button" className="mx-pane-ended-button" disabled={busy} onClick={() => { setOpen(false) }}>Cancel</button>
          </form>
        )
        : (
          <>
            <button type="button" className="mx-pane-ended-button is-primary" disabled={!online || machine !== 0} title={machine === 0 ? 'Type the value; it goes to the daemon only' : 'Secrets are only taken for this computer’s sessions'} onMouseDown={stop} onClick={() => { setOpen(true) }}>Provide…</button>
            <button type="button" className="mx-pane-ended-button" disabled={!online || machine !== 0} title="The agent is told no" onMouseDown={stop} onClick={() => { send(null) }}>Decline</button>
          </>
        )}
    </div>
  )
}
