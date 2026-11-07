import { useEffect, useRef, useState } from 'react'
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

/** After every try failed: a centred red card with what to check and the two ways to try again. */
function ServiceDownCard({ detail, onStart, onRetry }: { detail: string | null; onStart: () => void; onRetry: () => void }) {
  const first = useRef<HTMLButtonElement>(null)
  useEffect(() => { first.current?.focus() }, [])
  return (
    <div className="mx-service-layer">
      <section className="mx-service-card is-down" role="alertdialog" aria-modal="false" aria-labelledby="mx-service-down-title">
        <h2 id="mx-service-down-title">maittyd isn't responding</h2>
        <p>Your sessions are probably still running; the app can't reach the service that owns them. Things to try:</p>
        <ol>
          <li>Press Start maittyd: the app starts the service itself on every try.</li>
          <li>Check that nothing else is using the Maitty socket, or that a security tool is not blocking it.</li>
          <li>If it keeps failing, run <code className="mono">maitty</code> in a terminal to see why the service stops.</li>
        </ol>
        {detail !== null && <details><summary>Details</summary><pre className="mono">{detail}</pre></details>}
        <div className="mx-service-actions">
          <button ref={first} type="button" className="mx-text-button is-primary" onClick={onStart}>Start maittyd</button>
          <button type="button" className="mx-text-button" onClick={onRetry}>Retry</button>
        </div>
      </section>
    </div>
  )
}

/** The service runs another version: an amber card says what a restart keeps and stops. Not now leaves a read-only bar. */
function ServiceUpdateCard({ detail, restart, onNotNow }: { detail: string | null; restart: RestartOffer | null; onNotNow: () => void }) {
  const [armed, setArmed] = useState(false)
  const first = useRef<HTMLButtonElement>(null)
  useEffect(() => { first.current?.focus() }, [])
  return (
    <div className="mx-service-layer">
      <section className="mx-service-card is-update" role="alertdialog" aria-modal="false" aria-labelledby="mx-service-update-title">
        <h2 id="mx-service-update-title">Restart the background service to finish updating</h2>
        <p>Your sessions are not lost, but this window can't talk to the older service until it restarts.</p>
        {detail !== null && <p>{detail}</p>}
        <div className="mx-service-actions">
          {restart && (
            <button
              ref={first}
              type="button"
              className={`mx-text-button is-primary${armed ? ' is-armed' : ''}`}
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
          <button type="button" className="mx-text-button" onClick={onNotNow}>Not now</button>
        </div>
      </section>
    </div>
  )
}

/**
 * What the window shows while maittyd cannot be reached: a reconnecting bar for the first tries, the
 * Service down card after the last one, or the update card (then a bar) when the service is another
 * version. Nothing when it is connected.
 */
export function ServiceStatus({ online, failures, mismatch, retryAt, detail, restart, onRetry }: {
  online: boolean
  failures: number
  mismatch: boolean
  retryAt: number | null
  detail: string | null
  restart: RestartOffer | null
  /** Tries to reach maittyd again now. The window's connection starts the service when it is not running, so this is also how it is started. */
  onRetry: () => void
}) {
  const [notNow, setNotNow] = useState(false)
  const state = serviceStateOf({ online, failures, mismatch })
  if (state === 'ok') return null
  if (state === 'reconnecting') return <ReconnectingBar failures={failures} retryAt={retryAt} onRetry={onRetry} />
  if (state === 'down') return <ServiceDownCard detail={detail} onStart={onRetry} onRetry={onRetry} />
  if (notNow) return <OfflineBanner retryAt={retryAt} detail={detail} restart={restart} onRetry={onRetry} />
  return <ServiceUpdateCard detail={detail} restart={restart} onNotNow={() => { setNotNow(true) }} />
}
