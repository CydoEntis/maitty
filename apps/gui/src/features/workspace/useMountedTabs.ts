import { useMemo, useState } from 'react'
import { activeArrangement, type Layout } from './model/layout'
import { keptArrangements, rememberShown } from './model/keptTabs'

/** The tab in front and the ones kept mounted behind it. */
export function useMountedTabs({ layout }: {
  layout: Layout
}) {
  const tab = activeArrangement(layout)
  // The tabs shown last stay mounted behind the one showing, so going back to one does not build its terminals again.
  const [recentTabs, setRecentTabs] = useState<readonly string[]>([])
  const shownTabs = rememberShown(recentTabs, layout.activeId)
  if (shownTabs !== recentTabs) setRecentTabs(shownTabs)
  const keptTabs = useMemo(() => keptArrangements(layout, shownTabs), [layout, shownTabs])
  // In the layout's own order, so a tab coming to the front never moves in the page.
  const mountedTabs = tab ? layout.arrangements.filter((candidate) => candidate.id === tab.id || keptTabs.includes(candidate)) : []
  return { tab, mountedTabs }
}
