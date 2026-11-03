import { useState } from 'react'
import { Dialog as DialogPrimitive } from 'radix-ui'
import { Icon } from '@/features/workspace/components/glyphs'
import { hotkeyRows, isModifierSymbol } from '@/features/workspace/model/shortcuts'

/** The shortcuts worth knowing on day one, as the keymap names them, with a shorter label for the long ones. */
const FIRST_KEYS: ReadonlyArray<readonly [action: string, label: string]> = [
  ['New session', 'New session'],
  ['Split right', 'Split a pane'],
  ['Command palette', 'Command palette'],
  ['Attention inbox', 'Inbox'],
  ['Find in the pane or every session', 'Find'],
  ['Settings', 'Settings'],
]

const STEPS = ['welcome', 'set'] as const

type Props = {
  mac: boolean
  startFolder: string
  portalContainer: HTMLElement | null
  validateFolder: (path: string) => Promise<boolean>
  onStartFolder: (path: string) => void
  /** Skipped, finished or dismissed: the welcome is done. */
  onClose: () => void
  onStartSession: () => void
}

/** The first-run welcome: where sessions start, then the keys to know. */
export function WelcomeDialog({ mac, startFolder, portalContainer, validateFolder, onStartFolder, onClose, onStartSession }: Props) {
  const [step, setStep] = useState(0)
  const [draft, setDraft] = useState(startFolder)
  const [missing, setMissing] = useState(false)
  const [checking, setChecking] = useState(false)
  const last = step === STEPS.length - 1
  const rows = hotkeyRows(mac)

  const next = async () => {
    const typed = draft.trim()
    if (typed !== '') {
      setChecking(true)
      const exists = await validateFolder(typed).catch(() => false)
      setChecking(false)
      if (!exists) {
        setMissing(true)
        return
      }
    }
    onStartFolder(typed)
    setStep(1)
  }

  return (
    <DialogPrimitive.Root open onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogPrimitive.Portal container={portalContainer}>
        <DialogPrimitive.Overlay className="mx-wel-scrim" />
        <DialogPrimitive.Content className="mx-welcome">
          <div className="mx-wel-bars" aria-hidden="true">
            {STEPS.map((name, index) => <span key={name} className={index <= step ? 'is-done' : ''} />)}
          </div>
          <div className="mx-wel-step" key={STEPS[step]}>
            {step === 0 ? (
              <>
                <span className="mx-wel-icon"><Icon name="prompt" size={22} /></span>
                <DialogPrimitive.Title className="mx-wel-title">Welcome to Maitty</DialogPrimitive.Title>
                <DialogPrimitive.Description className="mx-wel-text">
                  Every session is a real shell, owned by the same daemon the Maitty TUI uses. Closing this window never stops an agent.
                </DialogPrimitive.Description>
                <label className="mx-wel-field">
                  <span>New sessions start in</span>
                  <span className={`mx-folder-field${missing ? ' is-invalid' : ''}`}>
                    <span className="mono" style={{ fontSize: 12, color: 'var(--dim)' }}>cd</span>
                    <input
                      value={draft}
                      placeholder="~"
                      aria-label="New sessions start in"
                      aria-invalid={missing}
                      spellCheck={false}
                      onChange={(event) => { setDraft(event.target.value); setMissing(false) }}
                      onKeyDown={(event) => { if (event.key === 'Enter') void next() }}
                    />
                  </span>
                  <small role={missing ? 'alert' : undefined} className={missing ? 'is-error' : undefined}>
                    {missing ? 'That folder was not found.' : 'Leave it empty for your home folder.'}
                  </small>
                </label>
              </>
            ) : (
              <>
                <DialogPrimitive.Title className="mx-wel-title">You&apos;re set</DialogPrimitive.Title>
                <DialogPrimitive.Description className="mx-wel-text">A few keys to start with. Everything is in the command palette too.</DialogPrimitive.Description>
                <ul className="mx-wel-keys">
                  {FIRST_KEYS.map(([action, label]) => {
                    const chord = rows.find((row) => row.action === action)?.chords[0] ?? []
                    return (
                      <li key={action}>
                        <span>{label}</span>
                        <span className="mx-wel-chord">{chord.map((key) => <kbd key={key} className={`mx-keycap${isModifierSymbol(key) ? ' is-symbol' : ''}`}>{key}</kbd>)}</span>
                      </li>
                    )
                  })}
                </ul>
              </>
            )}
          </div>
          <div className="mx-wel-actions">
            <button type="button" className="mx-confirm-button is-ghost" onClick={onClose}>Skip</button>
            <span className="mx-spacer" />
            {step > 0 && <button type="button" className="mx-confirm-button" onClick={() => { setStep(step - 1) }}>Back</button>}
            {last ? (
              <button type="button" className="mx-confirm-button is-primary" onClick={() => { onClose(); onStartSession() }}>Start a session</button>
            ) : (
              <button type="button" className="mx-confirm-button is-primary" disabled={checking} onClick={() => { void next() }}>Continue</button>
            )}
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
