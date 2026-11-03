import { useCallback, useEffect, useRef, useState } from 'react'
import { checkForUpdate, restartApp, type UpdateHandle } from './updater'

export type UpdaterState =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'current' }
  | { status: 'available'; version: string; notes: string }
  | { status: 'downloading'; version: string; progress: number | null }
  | { status: 'ready'; version: string }
  | { status: 'failed'; message: string }

function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message
  return typeof error === 'string' ? error : 'Could not reach the update server.'
}

/** Checks for a newer release once on launch, and again when asked. Nothing installs until `install` is called. */
export function useUpdater() {
  const [state, setState] = useState<UpdaterState>({ status: 'idle' })
  const [dismissed, setDismissed] = useState(false)
  const handle = useRef<UpdateHandle | null>(null)

  const settle = useCallback((pending: Promise<UpdateHandle | null>) => {
    pending
      .then((found) => {
        handle.current = found
        setState(found ? { status: 'available', ...found.info } : { status: 'current' })
      })
      .catch((error: unknown) => { setState({ status: 'failed', message: messageOf(error) }) })
  }, [])

  useEffect(() => { settle(checkForUpdate()) }, [settle])

  const check = useCallback(() => {
    setState({ status: 'checking' })
    setDismissed(false)
    settle(checkForUpdate())
  }, [settle])

  const install = useCallback(() => {
    const found = handle.current
    if (!found) return
    const { version } = found.info
    setState({ status: 'downloading', version, progress: null })
    found
      .install((progress) => { setState({ status: 'downloading', version, progress }) })
      .then(() => { setState({ status: 'ready', version }) })
      .catch((error: unknown) => { setState({ status: 'failed', message: messageOf(error) }) })
  }, [])

  const restart = useCallback(() => {
    restartApp().catch((error: unknown) => { setState({ status: 'failed', message: messageOf(error) }) })
  }, [])

  return { state, dismissed, check, install, restart, dismiss: () => { setDismissed(true) } }
}

/** One line for Settings. */
export function updaterSummary(state: UpdaterState): string {
  switch (state.status) {
    case 'idle': return 'Not checked yet.'
    case 'checking': return 'Checking…'
    case 'current': return 'You are on the latest version.'
    case 'available': return `Maitty ${state.version} is available.`
    case 'downloading': return `Downloading Maitty ${state.version}…`
    case 'ready': return `Maitty ${state.version} is installed. Restart to use it.`
    case 'failed': return `Could not check for updates: ${state.message}`
  }
}
