const STORAGE_KEY = 'maitty.gui.split-shells.v1'

/** The terminals this window opened by splitting a pane; the sidebar leaves them out while a pane shows them. */
export function readSplitShells(): ReadonlySet<string> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]')
    return new Set(Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [])
  } catch (error) {
    console.warn('Ignoring invalid Maitty split terminals', error)
    return new Set()
  }
}

/** `known` with `id` added, and the ids of sessions that no longer exist dropped so the list does not grow for ever. */
export function withSplitShell(known: ReadonlySet<string>, id: string, existing: ReadonlySet<string>): ReadonlySet<string> {
  return new Set([...known].filter((other) => existing.has(other)).concat(id))
}

export function writeSplitShells(ids: ReadonlySet<string>): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify([...ids]))
  } catch (error) {
    console.warn('Could not keep the split terminals', error)
  }
}
