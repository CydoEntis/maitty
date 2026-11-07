/** The fleet that was selected last time, so the window opens where it was left. Only the choice is local; the fleets are the daemon's. */
export const FLEET_STORAGE_KEY = 'maitty.gui.fleet.v1'

export function readSelectedFleet(): string | null {
  try {
    return localStorage.getItem(FLEET_STORAGE_KEY)
  } catch (error) {
    console.warn('Ignoring the saved fleet', error)
    return null
  }
}

export function writeSelectedFleet(key: string | null): void {
  try {
    if (key === null) localStorage.removeItem(FLEET_STORAGE_KEY)
    else localStorage.setItem(FLEET_STORAGE_KEY, key)
  } catch (error) {
    console.warn('Could not save the selected fleet', error)
  }
}
