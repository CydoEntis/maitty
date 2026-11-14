import { useCallback, useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { toast } from '@/lib/toast'
import { errorMessage } from '@/lib/error-message'
import { KEY_DOWN, KEY_ENTER, KEY_ESCAPE, KEY_LEFT, KEY_RIGHT, KEY_UP, isSelected, partsOfPrompt, promptOfScreen } from '../terminalKeys'

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
 * The question as the terminal shows it: what is asked stays in view, and the options scroll under it with
 * the selected one kept in sight, so a menu taller than the card loses neither.
 */
function ShownPrompt({ prompt }: { prompt: string }) {
  const { question, options } = partsOfPrompt(prompt)
  const box = useRef<HTMLPreElement>(null)
  const selected = useRef<HTMLSpanElement>(null)

  // Only when the menu itself changes, so a person who scrolled to read an option is not pulled back every second.
  useEffect(() => {
    const view = box.current
    const line = selected.current
    if (view === null || line === null) return
    const bottom = line.offsetTop + line.offsetHeight
    if (line.offsetTop < view.scrollTop) view.scrollTop = line.offsetTop
    else if (bottom > view.scrollTop + view.clientHeight) view.scrollTop = bottom - view.clientHeight
  }, [options])

  return (
    <div className="mx-terminal-prompt" role="group" aria-label="What the agent shows in its terminal">
      {question !== '' && <pre className="mx-terminal-question">{question}</pre>}
      <pre ref={box} className="mx-terminal-options">
        {options.split('\n').map((line, at) => (
          isSelected(line)
            ? <span key={at} ref={selected} className="mx-terminal-selected">{`${line}\n`}</span>
            : `${line}\n`
        ))}
      </pre>
    </div>
  )
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
        ? <ShownPrompt prompt={prompt} />
        : <p className="mx-waiting-note is-padded">The question is in the terminal.</p>}
      <div className="mx-waiting-foot is-keys" role="group" aria-label="Keys for the agent's terminal">
        {KEYS.map((key) => (
          <Button key={key.name} variant="soft" size="sm" className="mx-terminal-key" aria-label={key.name} disabled={busy} onClick={() => { press(key.keys) }}>{key.label}</Button>
        ))}
      </div>
    </>
  )
}
