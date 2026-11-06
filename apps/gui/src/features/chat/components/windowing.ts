/** The geometry of a virtualised list whose rows have different heights. Pure, so it is tested without a layout engine. */

export type ListLayout = {
  /** Where each row starts, from the top of the list. */
  readonly offsets: readonly number[]
  readonly heights: readonly number[]
  readonly total: number
}

/** A row not yet measured counts as `estimate` tall. */
export function buildLayout(keys: readonly string[], measured: ReadonlyMap<string, number>, estimate: number): ListLayout {
  const offsets: number[] = []
  const heights: number[] = []
  let total = 0
  for (const key of keys) {
    const height = measured.get(key) ?? estimate
    offsets.push(total)
    heights.push(height)
    total += height
  }
  return { offsets, heights, total }
}

/** The first row that ends below `position`. */
function firstEndingBelow(layout: ListLayout, position: number): number {
  let low = 0
  let high = layout.offsets.length
  while (low < high) {
    const middle = (low + high) >>> 1
    const end = (layout.offsets[middle] ?? 0) + (layout.heights[middle] ?? 0)
    if (end <= position) low = middle + 1
    else high = middle
  }
  return low
}

/** The rows to draw for a view: those that touch it, plus `overscan` rows each way. `end` is exclusive. */
export function visibleRange(layout: ListLayout, scrollTop: number, viewport: number, overscan: number): { start: number; end: number } {
  const count = layout.offsets.length
  if (count === 0) return { start: 0, end: 0 }
  const first = firstEndingBelow(layout, scrollTop)
  const last = firstEndingBelow(layout, scrollTop + viewport)
  return { start: Math.max(0, first - overscan), end: Math.min(count, last + 1 + overscan) }
}

/** Within this many pixels of the bottom, a list counts as following its newest row. */
export const FOLLOW_THRESHOLD_PX = 48

export function isAtBottom(scrollTop: number, viewport: number, total: number): boolean {
  return total - scrollTop - viewport <= FOLLOW_THRESHOLD_PX
}

/** Where the list was last seen, so the next scroll has a direction. */
export type FollowState = {
  readonly following: boolean
  readonly top: number
  readonly total: number
}

/**
 * Whether the list follows its newest row after a scroll. It goes by the direction the reader
 * moves, not by how far from the bottom they are: any move up stops following at once (a streaming
 * reply growing under a reader who nudged up would otherwise pull them back down), and only a move
 * down that ends within `FOLLOW_THRESHOLD_PX` of the bottom starts it again. A scroll the list
 * caused itself, and the browser pulling the view in because the list got shorter, say nothing
 * about what the reader wants.
 */
export function followAfterScroll(
  previous: FollowState,
  now: { readonly scrollTop: number; readonly viewport: number; readonly total: number },
  causedByList: boolean,
): FollowState {
  const next: FollowState = { following: previous.following, top: now.scrollTop, total: now.total }
  if (causedByList) return next
  const pulledIn = now.total < previous.total && now.scrollTop >= now.total - now.viewport
  if (pulledIn) return next
  if (now.scrollTop < previous.top) return { ...next, following: false }
  if (now.scrollTop > previous.top && isAtBottom(now.scrollTop, now.viewport, now.total)) return { ...next, following: true }
  return next
}
