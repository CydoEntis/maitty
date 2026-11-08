import { useEffect, useRef, useState, type ReactNode } from 'react'
import { RECONNECT_TRIES, serviceStateOf, tryNumber } from '../model/serviceState'
import { OfflineBanner, type RestartOffer } from './states'

const SECOND_MS = 1000

/** The number of whole seconds left until `at`, never below one while a try is still to come. */
function secondsUntil(at: number | null, now: number): number | null {
  return at === null ? null : Math.max(1, Math.ceil((at - now) / SECOND_MS))
}

/** Re-renders each second so a countdown ticks. */
function useNow(): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = window.setInterval(() => { setNow(Date.now()) }, SECOND_MS)
    return () => { window.clearInterval(timer) }
  }, [])
  return now
}

/** Under the pane header while maittyd does not answer and more tries are coming. */
function ReconnectingBar({ failures, retryAt, onRetry }: { failures: number; retryAt: number | null; onRetry: () => void }) {
  const seconds = secondsUntil(retryAt, useNow())
  return (
    <div className="mx-offline" role="status">
      <span className="mx-offline-dot" aria-hidden="true" />
      <span className="mx-offline-text">
        <strong>Reconnecting to maittyd</strong> · your sessions are still running
      </span>
      <span className="mx-offline-count mono">
        Try {tryNumber(failures)} of {RECONNECT_TRIES}{seconds !== null && ` · next in ${String(seconds)}s`}
      </span>
      <button type="button" className="mx-offline-button" onClick={onRetry}>Retry now</button>
    </div>
  )
}

const POWER = <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v9" /><path d="M6.3 7.3a8 8 0 1 0 11.4 0" /></svg>
const RESTART = <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-3-6.7" /><path d="M21 4v5h-5" /></svg>
const CHEVRON = <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round"><path d="m9 6 6 6-6 6" /></svg>

/** The veil over the main card, and the glass card centred on it. The chat behind stays faintly visible but cannot be used. */
function ServiceLayer({ children }: { children: ReactNode }) {
  return (
    <>
      <div className="mx-service-veil" aria-hidden="true" />
      <div className="mx-service-layer">{children}</div>
    </>
  )
}

const plural = (count: number, one: string, many: string): string => `${String(count)} ${count === 1 ? one : many}`

/** After every try failed: the service stopped. A line on what is saved, the technical details tucked away, and how to start it again. */
function ServiceDownCard({ detail, sessions, onStart, onRetry }: { detail: string | null; sessions: number; onStart: () => void; onRetry: () => void }) {
  const first = useRef<HTMLButtonElement>(null)
  useEffect(() => { first.current?.focus() }, [])
  return (
    <ServiceLayer>
      <section className="mx-service-card" role="alertdialog" aria-modal="false" aria-labelledby="mx-service-down-title">
        <div className="mx-service-head">
          <span className="mx-service-tile is-danger" aria-hidden="true">{POWER}</span>
          <div className="mx-service-copy">
            <h2 id="mx-service-down-title">Maitty service stopped</h2>
            <p>{sessions === 1 ? 'Your session is saved. Start the service to pick up where it left off.' : `${sessions > 1 ? `Your ${String(sessions)} sessions are` : 'Your sessions are'} saved. Start the service to pick up where they left off.`}</p>
          </div>
        </div>
        <details className="mx-service-details">
          <summary>{CHEVRON}Details</summary>
          <div className="mx-service-detail mono">{RECONNECT_TRIES} tries over 30s · no answer from the service{detail !== null && <><br />{detail}</>}</div>
        </details>
        <div className="mx-service-actions">
          <button ref={first} type="button" className="mx-svc-primary" onClick={onStart}>Start service</button>
          <button type="button" className="mx-svc-secondary" onClick={onRetry}>Try again</button>
        </div>
      </section>
    </ServiceLayer>
  )
}

/** What a restart keeps and what it stops, one line each, from the daemon's own counts. A line with a count of 0 is left out. */
function restartFacts(restart: RestartOffer): Array<{ tone: 'ok' | 'warn' | 'dim'; text: string }> {
  if (restart.sessions === 0) return []
  if (!restart.canRestart) return [{ tone: 'warn', text: `The older service owns ${plural(restart.sessions, 'session', 'sessions')} but is not saving them, so Maitty leaves it running.` }]
  return [
    { tone: 'ok', text: `${plural(restart.sessions, 'session reopens', 'sessions reopen')} in the same folders` },
    ...(restart.working > 0 ? [{ tone: 'warn' as const, text: restart.working === 1 ? '1 agent was working. It will offer Resume.' : `${String(restart.working)} agents were working. They will offer Resume.` }] : []),
    { tone: 'dim', text: 'Running shells and commands stop' },
  ]
}

/** The service runs another version: what a restart keeps and stops. Later leaves a slim bar. */
function ServiceUpdateCard({ restart, onLater }: { restart: RestartOffer | null; onLater: () => void }) {
  const [armed, setArmed] = useState(false)
  const first = useRef<HTMLButtonElement>(null)
  useEffect(() => { first.current?.focus() }, [])
  const facts = restart ? restartFacts(restart) : []
  return (
    <ServiceLayer>
      <section className="mx-service-card" role="alertdialog" aria-modal="false" aria-labelledby="mx-service-update-title">
        <div className="mx-service-head">
          <span className="mx-service-tile is-accent" aria-hidden="true">{RESTART}</span>
          <div className="mx-service-copy">
            <h2 id="mx-service-update-title">Restart Maitty to finish updating</h2>
            <p>{restart ? `The app updated to v${String(restart.to)}, but the service is still on v${String(restart.from)}.` : 'The app updated, but the service is still on an older version.'}</p>
          </div>
        </div>
        {facts.length > 0 && (
          <ul className="mx-service-facts">
            {facts.map((fact) => <li key={fact.text}><span className={`mx-service-dot is-${fact.tone}`} aria-hidden="true" /><span>{fact.text}</span></li>)}
          </ul>
        )}
        <div className="mx-service-actions">
          {restart && (
            <button
              ref={first}
              type="button"
              className={`mx-svc-primary${armed ? ' is-armed' : ''}`}
              disabled={!restart.canRestart || restart.pending}
              onClick={() => {
                if (!armed) {
                  setArmed(true)
                  return
                }
                setArmed(false)
                restart.onRestart()
              }}
            >
              {restart.pending ? 'Restarting…' : armed ? 'Click again to restart' : restart.label}
            </button>
          )}
          <button type="button" className="mx-svc-secondary" onClick={onLater}>Later</button>
          {restart && <span className="mx-service-versions mono">v{restart.from} → v{restart.to}</span>}
        </div>
      </section>
    </ServiceLayer>
  )
}

/**
 * What the window shows while maittyd cannot be reached: a reconnecting bar for the first tries, the
 * Service down card after the last one, or the update card (then a bar) when the service is another
 * version. Nothing when it is connected.
 */
export function ServiceStatus({ online, failures, mismatch, retryAt, detail, restart, sessions, onRetry }: {
  online: boolean
  failures: number
  mismatch: boolean
  retryAt: number | null
  detail: string | null
  restart: RestartOffer | null
  /** The sessions this window last knew of, for "your N sessions are saved". */
  sessions: number
  /** Tries to reach maittyd again now. The window's connection starts the service when it is not running, so this is also how it is started. */
  onRetry: () => void
}) {
  const [notNow, setNotNow] = useState(false)
  const state = serviceStateOf({ online, failures, mismatch })
  if (state === 'ok') return null
  if (state === 'reconnecting') return <ReconnectingBar failures={failures} retryAt={retryAt} onRetry={onRetry} />
  if (state === 'down') return <ServiceDownCard detail={detail} sessions={sessions} onStart={onRetry} onRetry={onRetry} />
  if (notNow) return <OfflineBanner retryAt={retryAt} detail={detail} restart={restart} onRetry={onRetry} />
  return <ServiceUpdateCard restart={restart} onLater={() => { setNotNow(true) }} />
}
