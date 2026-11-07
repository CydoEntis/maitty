/** A path that matched, how well, and which of its characters the typed letters landed on. */
export type FuzzyHit = {
  path: string
  score: number
  /** Indexes into `path` of the matched characters, in order, for marking them. */
  positions: number[]
}

/** How far ahead of the next match a word start is preferred over the plain next occurrence. */
const BOUNDARY_LOOKAHEAD = 8

const BASE = 1
const BOUNDARY_BONUS = 16
const CONSECUTIVE_BONUS = 8
const IN_NAME_BONUS = 50
const NAME_START_BONUS = 30
/** Per character skipped between the first and the last match. A tight match beats a scattered one. */
const GAP_PENALTY = 4
const LENGTH_PENALTY = 0.1

const SEPARATORS = '/._- '

/** Whether a match at `index` starts a word: the start, after a separator, or a lower-to-upper step. */
function startsWord(path: string, index: number): boolean {
  if (index === 0) return true
  const before = path.charAt(index - 1)
  if (SEPARATORS.includes(before)) return true
  const here = path.charAt(index)
  return before === before.toLowerCase() && before !== before.toUpperCase() && here !== here.toLowerCase()
}

/** Whether every letter of `letters` appears in `text` in order, starting at `from`. */
function appearsInOrder(letters: string, text: string, from: number): boolean {
  let at = from
  for (const letter of letters) {
    at = text.indexOf(letter, at)
    if (at === -1) return false
    at += 1
  }
  return true
}

/** Where each typed letter lands in `path`, or null when they do not all appear in order. */
function place(query: string, path: string): number[] | null {
  const lower = path.toLowerCase()
  const positions: number[] = []
  let from = 0
  for (let index = 0; index < query.length; index += 1) {
    const letter = query.charAt(index)
    let at = lower.indexOf(letter, from)
    if (at === -1) return null
    // Prefer a word start a little further on over the first plain occurrence, but only when
    // the rest of what was typed still fits after it.
    const rest = query.slice(index + 1)
    const limit = Math.min(lower.length, at + BOUNDARY_LOOKAHEAD)
    for (let candidate = at + 1; candidate < limit && !startsWord(path, at); candidate += 1) {
      if (lower.charAt(candidate) === letter && startsWord(path, candidate) && appearsInOrder(rest, lower, candidate + 1)) {
        at = candidate
        break
      }
    }
    positions.push(at)
    from = at + 1
  }
  return positions
}

function scoreOf(path: string, positions: readonly number[]): number {
  const nameStart = path.lastIndexOf('/') + 1
  let score = 0
  positions.forEach((at, i) => {
    score += BASE
    if (startsWord(path, at)) score += BOUNDARY_BONUS
    const previous = positions[i - 1]
    if (previous !== undefined && at === previous + 1) score += CONSECUTIVE_BONUS
  })
  const first = positions[0] ?? 0
  const last = positions[positions.length - 1] ?? 0
  if (first >= nameStart) score += IN_NAME_BONUS
  if (first === nameStart) score += NAME_START_BONUS
  score -= GAP_PENALTY * (last - first + 1 - positions.length)
  return score - LENGTH_PENALTY * path.length
}

/**
 * The best `limit` of `paths` for what was typed, best first. A path matches when every typed
 * letter appears in it in order, whatever the case. A match in the file name beats one in the
 * folders, word starts and consecutive letters score higher, and a tight match beats a scattered
 * one. Ties keep the order of `paths`, so results do not jump about as more is typed.
 */
export function fuzzyFind(query: string, paths: readonly string[], limit: number): FuzzyHit[] {
  const typed = query.trim().toLowerCase()
  if (typed === '') return paths.slice(0, limit).map((path) => ({ path, score: 0, positions: [] }))
  const hits: (FuzzyHit & { order: number })[] = []
  paths.forEach((path, order) => {
    const positions = place(typed, path)
    if (positions !== null) hits.push({ path, positions, score: scoreOf(path, positions), order })
  })
  hits.sort((a, b) => b.score - a.score || a.order - b.order)
  return hits.slice(0, limit).map(({ path, score, positions }) => ({ path, score, positions }))
}
