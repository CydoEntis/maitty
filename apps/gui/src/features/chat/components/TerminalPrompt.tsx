import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from '@/lib/toast'
import { errorMessage } from '@/lib/error-message'
import { KEY_DOWN, KEY_ENTER, KEY_ESCAPE, KEY_LEFT, KEY_RIGHT, KEY_UP, promptOfScreen } from '../terminalKeys'

/** How often the question is read again while the tray shows, so a menu that moves or changes is followed. */
const READ_EVERY_MS = 1000
/** After a key, the agent needs a moment to draw its answer before the screen is read. */
const REDRAW_MS = 150

const KEYS = [
  { label: '↑', name: 'Up', keys: KEY_UP },
  { label: '↓', name: 'Down', keys: KEY_DOWN },
  { label: '←', name: 'Left', keys: KEY_LEFT },
  { label: '→', name: 'Right', keys: KEY_RIGHT },
  { label: 'Enter', name: 'Enter', keys: KEY_ENTER },
  { label: 'Esc', name: 'Escape', keys: KEY_ESCAPE },
] as const

type Props = {
  /** Reads the agent's terminal screen; absent or failing where it cannot be read (another machine's). */
  readScreen: (() => Promise<string>) | undefined
  /** Types a key into the agent's terminal. */
  sendKeys: (keys: string) => Promise<void>
}

/**
 * A menu the agent draws in its own terminal that the chat has no buttons for (a size to pick, a setting to
 * confirm): the question as the terminal shows it, and the keys a person would press. They are the person's
 * own keystrokes, typed into the terminal the agent runs in, so the daemon still alone decides the status.
 */
export function TerminalPrompt({ readScreen, sendKeys }: Props) {
  const [prompt, setPrompt] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const alive = useRef(true)

  const read = useCallback(() => {
    if (readScreen === undefined) return
    readScreen()
      .then((screen) => { if (alive.current) setPrompt(promptOfScreen(screen)) })
      .catch(() => { if (alive.current) setPrompt(null) })
  }, [readScreen])

  useEffect(() => {
    alive.current = true
    read()
    const timer = window.setInterval(read, READ_EVERY_MS)
    return () => { alive.current = false; window.clearInterval(timer) }
  }, [read])

  const press = (keys: string) => {
    setBusy(true)
    sendKeys(keys)
      .then(() => { window.setTimeout(read, REDRAW_MS) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send the key')) })
      .finally(() => { setBusy(false) })
  }

  return (
    <>
      {prompt !== null && prompt.trim() !== ''
        ? <pre className="mx-terminal-prompt" aria-label="What the agent shows in its terminal">{prompt}</pre>
        : <p className="mx-question-note">The question is in the terminal.</p>}
      <div className="mx-question-answers is-keys" role="group" aria-label="Keys for the agent's terminal">
        {KEYS.map((key) => (
          <button key={key.name} type="button" className="mx-answer is-key" aria-label={key.name} disabled={busy} onClick={() => { press(key.keys) }}>{key.label}</button>
        ))}
      </div>
    </>
  )
}
