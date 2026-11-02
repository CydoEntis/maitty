export type PaletteKind = 'Action' | 'Session' | 'Project' | 'Theme'

export type PaletteItem = {
  id: string
  label: string
  /** Dim text after the label, such as a session's project. */
  detail?: string
  kind: PaletteKind
  /** The keys that do the same thing, written as the platform does. */
  shortcut?: string
  /** A project colour dot, as a CSS colour. */
  dot?: string
  /** Shown instead of the kind, such as "needs you". */
  badge?: string
  /** Themes only appear once something is typed. */
  needsQuery?: boolean
  run: () => void
}

const NO_MATCH = 0
const SUBSTRING_SCORE = 100
const SUBSEQUENCE_SCORE = 40
const HALF = 2

/** How well `query` matches `text`: a run of it scores above letters that merely appear in order; 0 is no match. */
export function matchScore(query: string, text: string): number {
  const needle = query.trim().toLowerCase()
  if (needle === '') return SUBSEQUENCE_SCORE
  const haystack = text.toLowerCase()
  const at = haystack.indexOf(needle)
  if (at >= 0) return SUBSTRING_SCORE - Math.min(at, SUBSTRING_SCORE / HALF) + (at === 0 ? SUBSTRING_SCORE / HALF : 0)
  let position = 0
  for (const letter of needle) {
    position = haystack.indexOf(letter, position)
    if (position < 0) return NO_MATCH
    position += 1
  }
  return SUBSEQUENCE_SCORE
}

/** The most results the palette lists. */
export const PALETTE_LIMIT = 14

/** Items that match the query, best first (stable among equals), at most `PALETTE_LIMIT`. */
export function searchPalette(items: readonly PaletteItem[], query: string): PaletteItem[] {
  const typed = query.trim() !== ''
  return items
    .map((item, order) => ({ item, order, score: item.needsQuery && !typed ? NO_MATCH : Math.max(matchScore(query, item.label), matchScore(query, `${item.label} ${item.detail ?? ''}`) / HALF) }))
    .filter((entry) => entry.score > NO_MATCH)
    .sort((a, b) => b.score - a.score || a.order - b.order)
    .slice(0, PALETTE_LIMIT)
    .map((entry) => entry.item)
}
