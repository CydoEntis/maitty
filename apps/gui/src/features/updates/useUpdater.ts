import { useCallback, useEffect, useRef, useState } from 'react'
import { checkForUpdate, restartApp, type UpdateHandle } from './updater'
import { downloadRate, keepSamples, type Sample, type Transfer } from './downloadEta'

export type UpdaterState =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'current' }
  | { status: 'available'; version: string; notes: string }
  /** `progress` is the share that arrived (null while the size is unknown); `transfer` the bytes behind it, null until the first arrive. */
  | { status: 'downloading'; version: string; progress: number | null; transfer: Transfer | null }
  | { status: 'ready'; version: string }
  /** `during` says whether the check or the update itself failed: only the latter is something you started. */
  | { status: 'failed'; message: string; during: 'check' | 'update' }

function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message
  return typeof error === 'string' ? error : 'Could not reach the update server.'
}

/** A running app is looked at again this often, so a release published after launch is still found. */
const MINUTES_PER_HOUR = 60
const SECONDS_PER_MINUTE = 60
const MS_PER_SECOND = 1000
const RECHECK_MS = MINUTES_PER_HOUR * SECONDS_PER_MINUTE * MS_PER_SECOND

/** Checks for a newer release on launch, hourly after that, and again when asked. Nothing is installed until `restart` is called. */
export function useUpdater() {
  const [state, setState] = useState<UpdaterState>({ status: 'idle' })
  const handle = useRef<UpdateHandle | null>(null)

  const settle = useCallback((pending: Promise<UpdateHandle | null>) => {
    pending
      .then((found) => {
        handle.current = found
        setState(found ? { status: 'available', ...found.info } : { status: 'current' })
      })
      .catch((error: unknown) => { setState({ status: 'failed', message: messageOf(error), during: 'check' }) })
  }, [])

  useEffect(() => { settle(checkForUpdate()) }, [settle])

  // Never while an update is being fetched or waits to be installed: a new check would replace its handle.
  const status = useRef(state.status)
  useEffect(() => { status.current = state.status })
  useEffect(() => {
    const timer = window.setInterval(() => {
      if (status.current === 'downloading' || status.current === 'ready') return
      settle(checkForUpdate())
    }, RECHECK_MS)
    return () => { window.clearInterval(timer) }
  }, [settle])

  const check = useCallback(() => {
    // Checking again would replace the handle of an update being fetched, or already fetched.
    if (state.status === 'downloading' || state.status === 'ready') return
    setState({ status: 'checking' })
    settle(checkForUpdate())
  }, [settle, state.status])

  const download = useCallback(() => {
    const found = handle.current
    if (!found) return
    const { version } = found.info
    setState({ status: 'downloading', version, progress: null, transfer: null })
    let samples: Sample[] = []
    found
      .download((received, total) => {
        samples = keepSamples(samples, { at: performance.now(), received })
        const progress = total === null ? null : Math.min(received / total, 1)
        setState({ status: 'downloading', version, progress, transfer: { received, total, bytesPerSecond: downloadRate(samples) } })
      })
      .then(() => { setState({ status: 'ready', version }) })
      .catch((error: unknown) => { setState({ status: 'failed', message: messageOf(error), during: 'update' }) })
  }, [])

  // Installing waits for this: the download alone changes nothing on disk that is running.
  const restart = useCallback((onFailed?: () => void) => {
    const found = handle.current
    if (!found) return
    found.install()
      .then(restartApp)
      .catch((error: unknown) => {
        setState({ status: 'failed', message: messageOf(error), during: 'update' })
        onFailed?.()
      })
  }, [])

  /** The release notes of the update that was found, for the dialog shown after it is installed. */
  const releaseNotes = useCallback(() => handle.current?.info.notes ?? '', [])

  return { state, check, download, restart, releaseNotes }
}

/** One line for Settings. */
export function updaterSummary(state: UpdaterState): string {
  switch (state.status) {
    case 'idle': return 'Not checked yet.'
    case 'checking': return 'Checking…'
    case 'current': return 'You are on the latest version.'
    case 'available': return `Maitty ${state.version} is available.`
    case 'downloading': return `Downloading Maitty ${state.version}…`
    case 'ready': return `Maitty ${state.version} is downloaded. Restart to use it.`
    case 'failed': return state.during === 'check' ? `Could not check for updates: ${state.message}` : `The update failed: ${state.message}`
  }
}
