import type { UpdaterState } from './useUpdater'

export type AppBehindStep = {
  label: string
  /** What pressing the button does; null while there is nothing to press. */
  action: 'download' | 'install' | 'check' | null
}

/**
 * The one button on the card shown when the service is newer than this app: it walks the updater
 * from found, to downloaded, to installed, since reconnecting is impossible until the app is replaced.
 */
export function appBehindStep(state: UpdaterState): AppBehindStep {
  switch (state.status) {
    case 'idle':
    case 'checking': return { label: 'Checking for an update…', action: null }
    case 'available': return { label: `Download Maitty ${state.version}`, action: 'download' }
    case 'downloading': return { label: `Downloading Maitty ${state.version}…`, action: null }
    case 'ready': return { label: 'Install and restart', action: 'install' }
    case 'current': return { label: 'Check again', action: 'check' }
    case 'failed': return { label: 'Try again', action: 'check' }
  }
}
