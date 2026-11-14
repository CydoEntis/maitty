import { useCallback, useState } from 'react'
import type { PaneView } from './model/chatView'

/**
 * The view each session was switched to by hand, by session id. A session never switched has no
 * entry and shows its agent's default (`effectiveView`). Kept per session, not per pane, so the same
 * session shown elsewhere agrees with itself; a window restart starts from the defaults again.
 */
export function usePaneViews() {
  const [choices, setChoices] = useState<ReadonlyMap<string, PaneView>>(new Map())
  // The sessions showing their Work board over the view they were on; choosing a view leaves it.
  const [work, setWork] = useState<ReadonlySet<string>>(new Set())
  const choose = useCallback((sessionId: string, view: PaneView) => {
    setChoices((current) => new Map(current).set(sessionId, view))
    setWork((current) => {
      if (!current.has(sessionId)) return current
      const next = new Set(current)
      next.delete(sessionId)
      return next
    })
  }, [])
  const showWork = useCallback((sessionId: string) => {
    setWork((current) => new Set(current).add(sessionId))
  }, [])
  return { choices, choose, work, showWork }
}
