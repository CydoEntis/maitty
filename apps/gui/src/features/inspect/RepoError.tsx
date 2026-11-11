import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { isNotARepository, messageOf } from './useRepoQuery'
import { copyText } from '@/lib/clipboard'

/** Git says this when another git command holds the repository's lock, which clears by itself. */
const GIT_BUSY_WORDS = ['index.lock', 'another git process']
/** What the operating systems say when a folder may not be read. */
const PERMISSION_WORDS = ['permission denied', 'operation not permitted', 'access is denied']
const GIT_BUSY_RETRY_SECONDS = 3
const SECOND_MS = 1000

type Tone = 'wait' | 'stop' | 'calm'
type Screen = { tone: Tone; title: string; body: string; autoRetrySeconds: number | null; reassure: boolean }

/** What each failure says to a person, in plain words: what happened, and that their work is fine. */
function screenFor(error: unknown, tab: string): Screen {
  const text = messageOf(error).toLowerCase()
  if (isNotARepository(error)) {
    return { tone: 'calm', title: 'This folder isn’t a git project yet', body: 'Changes, branches and history show up once a folder uses git. Run git init in a terminal here to start, or start a session inside a project that already is one.', autoRetrySeconds: null, reassure: false }
  }
  if (GIT_BUSY_WORDS.some((word) => text.includes(word))) {
    return { tone: 'wait', title: 'Git is finishing something', body: 'Another git command is working in this project. It usually wraps up in a few seconds.', autoRetrySeconds: GIT_BUSY_RETRY_SECONDS, reassure: true }
  }
  if (PERMISSION_WORDS.some((word) => text.includes(word))) {
    return { tone: 'stop', title: 'Maitty can’t read this folder', body: 'The folder’s permissions keep it private. Allow your user to read it, then try again.', autoRetrySeconds: null, reassure: true }
  }
  return { tone: 'wait', title: `Maitty couldn’t load your ${tab}`, body: 'Your terminals and agents keep running. Only this panel is affected.', autoRetrySeconds: null, reassure: true }
}

/** Counts down once a second to `onZero`, which then runs and the count starts over; holds still while `paused`. */
function useCountdown(seconds: number | null, paused: boolean, onZero: (() => void) | undefined): number | null {
  const [left, setLeft] = useState(seconds)
  const latest = useRef(onZero)
  useEffect(() => { latest.current = onZero })
  useEffect(() => {
    if (seconds === null || left === null || paused) return undefined
    const timer = window.setTimeout(() => {
      if (left <= 1) {
        setLeft(seconds)
        latest.current?.()
      } else {
        setLeft(left - 1)
      }
    }, SECOND_MS)
    return () => { window.clearTimeout(timer) }
  }, [left, seconds, paused])
  return seconds === null ? null : left
}

type Props = {
  error: unknown
  /** What the panel was loading, in a word: "changes", "files". */
  tab: string
  /** Asks again, read-only. Without it the screen has no retry. */
  onRetry?: (() => void) | undefined
  /** An answer is being fetched now. */
  retrying?: boolean
}

/**
 * What a repository tab says when its question failed: one calm screen in place of the tab's
 * content. Technical text lives only behind "Show details for support". The terminals are never
 * affected, and the screen says so.
 */
export function RepoError({ error, tab, onRetry, retrying = false }: Props) {
  const screen = screenFor(error, tab)
  const [detailsOpen, setDetailsOpen] = useState(false)
  const primary = useRef<HTMLButtonElement>(null)
  const left = useCountdown(onRetry === undefined ? null : screen.autoRetrySeconds, detailsOpen || retrying, onRetry)
  useEffect(() => { primary.current?.focus() }, [])
  const raw = messageOf(error)
  const details = `Tab: ${tab}\nError: ${raw}`
  return (
    <div className={`mx-inspect-fail is-${screen.tone}`} role={screen.tone === 'calm' ? 'status' : 'alert'}>
      <span className="mx-inspect-fail-halo" aria-hidden="true">
        <span className={`mx-inspect-fail-tile${retrying ? ' is-retrying' : ''}`} />
      </span>
      <h3 className="mx-inspect-fail-title">{screen.title}</h3>
      <p className="mx-inspect-fail-body">{screen.body}</p>
      {screen.reassure && <span className="mx-inspect-fail-pill">✓ Nothing in your project was changed</span>}
      {onRetry !== undefined && screen.tone !== 'calm' && (
        <Button ref={primary} type="button" size="sm" disabled={retrying} onClick={onRetry}>{retrying ? 'Checking…' : 'Try again'}</Button>
      )}
      {left !== null && !retrying && <p className="mx-inspect-fail-auto">Trying again automatically in {left}s</p>}
      <button type="button" className="mx-inspect-fail-more" aria-expanded={detailsOpen} onClick={() => { setDetailsOpen((open) => !open) }}>
        {detailsOpen ? 'Hide details' : 'Show details for support'}
      </button>
      {detailsOpen && (
        <div className="mx-inspect-fail-details">
          <pre>{details}</pre>
          <button type="button" onClick={() => { void copyText(details) }}>Copy details</button>
        </div>
      )}
    </div>
  )
}
