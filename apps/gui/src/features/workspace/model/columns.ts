import { MIN_COLUMN_PX } from './layout'

/** Within this share of an even split, a dragged pair of columns snaps to even. */
const SNAP_SHARE = 0.025
const HALF = 0.5

/** The widths after the handle at `index` moved by `delta` px: only its two neighbours change, never below the minimum. */
export function dragWidths(widths: readonly number[], index: number, delta: number): number[] {
  const left = widths[index]
  const right = widths[index + 1]
  if (left === undefined || right === undefined) return [...widths]
  const total = left + right
  let nextLeft = Math.min(total - MIN_COLUMN_PX, Math.max(MIN_COLUMN_PX, left + delta))
  if (Math.abs(nextLeft - total * HALF) < total * SNAP_SHARE) nextLeft = total * HALF
  return widths.map((width, position) => (position === index ? nextLeft : position === index + 1 ? total - nextLeft : width))
}
