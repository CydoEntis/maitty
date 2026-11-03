import { useRef, useState, type KeyboardEvent } from 'react'
import { searchPanes, type SearchHit } from './api'
import './find.css'

type Scope = 'pane' | 'all'

type Props = {
  /** Selects the next/previous match in the focused pane; false when there is none. */
  findInPane: (query: string, direction: 'next' | 'previous') => boolean
  clearPane: () => void
  /** What a session is called, by the daemon's pane number; null when it is not one of ours. */
  nameOf: (term: number) => string | null
  onOpenSession: (term: number) => void
  onClose: () => void
  online: boolean
}

/** ⌘F: look for text in the focused pane, or in every session's recent output. */
export function FindBar({ findInPane, clearPane, nameOf, onOpenSession, onClose, online }: Props) {
  const [query, setQuery] = useState('')
  const [scope, setScope] = useState<Scope>('pane')
  const [missed, setMissed] = useState(false)
  const [hits, setHits] = useState<SearchHit[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  const close = () => {
    clearPane()
    onClose()
  }

  const run = (direction: 'next' | 'previous') => {
    const text = query.trim()
    if (text === '') return
    if (scope === 'pane') {
      setMissed(!findInPane(text, direction))
      return
    }
    const mine = ++latest.current
    setError(null)
    void searchPanes(text)
      .then((found) => { if (mine === latest.current) setHits(found) })
      .catch((failure: unknown) => { if (mine === latest.current) setError(failure instanceof Error ? failure.message : 'The search failed') })
  }

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape') {
      event.stopPropagation()
      close()
    } else if (event.key === 'Enter') {
      event.preventDefault()
      run(event.shiftKey ? 'previous' : 'next')
    }
  }

  return (
    <div className="mx-find" role="search" aria-label="Find">
      <div className="mx-find-row">
        <input
          autoFocus
          className="mx-find-input"
          type="text"
          value={query}
          placeholder={scope === 'pane' ? 'Find in this pane' : 'Find in every session'}
          aria-label="Search text"
          onChange={(event) => { setQuery(event.target.value); setMissed(false); setHits(null) }}
          onKeyDown={onKeyDown}
        />
        <div className="mx-find-scope" role="group" aria-label="Where to look">
          <button type="button" aria-pressed={scope === 'pane'} onClick={() => { setScope('pane'); setHits(null) }}>Pane</button>
          <button type="button" aria-pressed={scope === 'all'} disabled={!online} onClick={() => { clearPane(); setScope('all'); setMissed(false) }}>All sessions</button>
        </div>
        {scope === 'pane' && (
          <>
            <button type="button" className="mx-find-step" aria-label="Previous match" title="Previous (Shift+Enter)" onClick={() => { run('previous') }}>↑</button>
            <button type="button" className="mx-find-step" aria-label="Next match" title="Next (Enter)" onClick={() => { run('next') }}>↓</button>
          </>
        )}
        <button type="button" className="mx-find-step" aria-label="Close find" title="Close (Esc)" onClick={close}>×</button>
      </div>
      {scope === 'pane' && missed && <p className="mx-find-note" role="status">No match in this pane</p>}
      {error && <p className="mx-find-note is-error" role="alert">{error}</p>}
      {scope === 'all' && hits !== null && (
        hits.length === 0
          ? <p className="mx-find-note" role="status">No session printed that recently</p>
          : (
            <ul className="mx-find-hits" aria-label="Matches">
              {hits.map((hit) => (
                <li key={`${String(hit.term)}:${String(hit.line)}`}>
                  <button type="button" onClick={() => { onOpenSession(hit.term); close() }}>
                    <span className="mx-find-session">{nameOf(hit.term) ?? `Session ${String(hit.term)}`}</span>
                    <span className="mx-find-text">{hit.text}</span>
                  </button>
                </li>
              ))}
            </ul>
          )
      )}
    </div>
  )
}
