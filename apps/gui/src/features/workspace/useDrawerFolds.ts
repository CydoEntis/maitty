import { useCallback, useState } from 'react'
import { startsFolded } from './model/terminalDrawer'

/**
 * Whether each agent's terminals drawer (pinned under its chat) is folded, by session id. Kept for the window, not each
 * pane, so the title bar's Terminal toggle and the pane's own button agree. A session never folded or opened by hand
 * starts folded in a window too short for a chat and a terminal (`startsFolded`).
 */
export function useDrawerFolds() {
  const [folds, setFolds] = useState<ReadonlyMap<string, boolean>>(new Map())
  const [short] = useState(startsFolded)
  const drawerFoldedOf = useCallback((sessionId: string) => folds.get(sessionId) ?? short, [folds, short])
  const setDrawerFolded = useCallback((sessionId: string, folded: boolean) => {
    setFolds((current) => new Map(current).set(sessionId, folded))
  }, [])
  return { drawerFoldedOf, setDrawerFolded }
}
