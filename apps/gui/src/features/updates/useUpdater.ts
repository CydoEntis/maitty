import { useCallback, useEffect, useRef, useState } from 'react'
import { checkForUpdate, restartApp, type UpdateHandle } from './updater'

export type UpdaterState =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'current' }
  | { status: 'available'; version: string; notes: string }
  | { status: 'downloading'; version: string; progress: number | null }
  | { status: 'ready'; version: string }
  /** `during` says whether the check or the update itself failed: only the latter is something you started. */
  | { status: 'failed'; message: string; during: 'check' | 'update' }

function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message
  return typeof error === 'string' ? error : 'Could not reach the update server.'
}

/** Checks for a newer release once on launch, and again when asked. Nothing is installed until `restart` is called. */
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
    setState({ status: 'downloading', version, progress: null })
    found
      .download((progress) => { setState({ status: 'downloading', version, progress }) })
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

  return { state, check, download, restart }
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
