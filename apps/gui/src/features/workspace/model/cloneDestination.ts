/**
 * The folder a person last cloned into on each machine, so the next clone starts there. A
 * convenience of this window only: a browser that keeps nothing simply starts from the default.
 */
const KEY_PREFIX = 'maitty.clone-into.'

const keyOf = (machine: string) => `${KEY_PREFIX}${machine.toLowerCase()}`

export function lastCloneDestination(machine: string): string | null {
  try {
    const kept = window.localStorage.getItem(keyOf(machine))
    return kept === null || kept.trim() === '' ? null : kept
  } catch {
    // Storage is blocked (a private window): there is nothing remembered.
    return null
  }
}

export function rememberCloneDestination(machine: string, folder: string): void {
  try {
    window.localStorage.setItem(keyOf(machine), folder)
  } catch {
    // Storage is blocked or full: the next clone starts from the default, which still works.
  }
}
