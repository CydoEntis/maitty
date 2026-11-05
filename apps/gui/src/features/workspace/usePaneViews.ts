import { useCallback, useState } from 'react'
import type { PaneView } from './model/chatView'

/**
 * The view each session was switched to by hand, by session id. A session never switched has no
 * entry and shows its agent's default (`effectiveView`). Kept per session, not per pane, so the same
 * session shown elsewhere agrees with itself; a window restart starts from the defaults again.
 */
export function usePaneViews() {
  const [choices, setChoices] = useState<ReadonlyMap<string, PaneView>>(new Map())
  const choose = useCallback((sessionId: string, view: PaneView) => {
    setChoices((current) => new Map(current).set(sessionId, view))
  }, [])
  return { choices, choose }
}
