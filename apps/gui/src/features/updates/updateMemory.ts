const SKIPPED_KEY = 'maitty.gui.update.skipped'
const RESTARTED_KEY = 'maitty.gui.update.restarted'

function read(key: string): string | null {
  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}

function write(key: string, value: string | null) {
  try {
    if (value === null) window.localStorage.removeItem(key)
    else window.localStorage.setItem(key, value)
  } catch {
    // Without storage the choice only lasts this run.
  }
}

/** The release the user chose to skip. */
export const skippedUpdate = (): string | null => read(SKIPPED_KEY)
export const skipUpdate = (version: string) => { write(SKIPPED_KEY, version) }

/** What the next launch says about the update it was restarted into. */
export type RestartedUpdate = {
  version: string
  /** The version that was running before, if known. */
  previous: string | null
  notes: string
  /** The app installed it on its own while idle, rather than the user choosing Restart now. */
  automatic: boolean
}

/** Set just before restarting into an update, so the next launch can say what changed and that sessions never stopped. */
export const markRestarting = (update: RestartedUpdate) => { write(RESTARTED_KEY, JSON.stringify(update)) }

function parseRestarted(stored: string): RestartedUpdate {
  try {
    const parsed: unknown = JSON.parse(stored)
    if (typeof parsed === 'object' && parsed !== null && 'version' in parsed && typeof parsed.version === 'string') {
      const previous = 'previous' in parsed && typeof parsed.previous === 'string' ? parsed.previous : null
      const notes = 'notes' in parsed && typeof parsed.notes === 'string' ? parsed.notes : ''
      const automatic = 'automatic' in parsed && parsed.automatic === true
      return { version: parsed.version, previous, notes, automatic }
    }
  } catch {
    // A launch from before this held more than the version stored just the version.
  }
  return { version: stored, previous: null, notes: '', automatic: false }
}

export function takeRestarted(): RestartedUpdate | null {
  const stored = read(RESTARTED_KEY)
  if (stored === null) return null
  write(RESTARTED_KEY, null)
  return parseRestarted(stored)
}
