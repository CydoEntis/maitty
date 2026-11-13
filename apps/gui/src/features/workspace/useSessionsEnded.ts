import { useCallback, useState } from 'react'

/**
 * Counts the watched sessions that have ended: listed once, then gone from the list. A session is
 * watched from the moment it is started, which can be before the list has it, so one that has not
 * been listed yet is not taken for one that ended.
 */
export function useSessionsEnded(listed: ReadonlyMap<string, unknown>): { watch: (id: string) => void; ended: number } {
  const [watched, setWatched] = useState<ReadonlyMap<string, boolean>>(new Map())
  const [ended, setEnded] = useState(0)
  const still = new Map<string, boolean>()
  let gone = 0
  let changed = false
  for (const [id, seen] of watched) {
    const here = listed.has(id)
    if (seen && !here) {
      gone += 1
      continue
    }
    if (here && !seen) changed = true
    still.set(id, seen || here)
  }
  if (gone > 0 || changed) {
    setWatched(still)
    if (gone > 0) setEnded(ended + gone)
  }
  const watch = useCallback((id: string) => {
    setWatched((now) => (now.has(id) ? now : new Map(now).set(id, false)))
  }, [])
  return { watch, ended }
}
