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

/** Set just before restarting into an update, so the next launch can say sessions never stopped. */
export const markRestarting = (version: string) => { write(RESTARTED_KEY, version) }
export function takeRestarted(): string | null {
  const version = read(RESTARTED_KEY)
  if (version !== null) write(RESTARTED_KEY, null)
  return version
}
