import type { SearchHit } from '@/features/find/api'

export type PaletteKind = 'Action' | 'Session' | 'Project' | 'Theme' | 'Output'

export type PaletteItem = {
  id: string
  label: string
  /** Dim text after the label, such as a session's project. */
  detail?: string
  kind: PaletteKind
  /** The keys that do the same thing, written as the platform does. */
  shortcut?: string
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

/** The most "In sessions" hits the palette lists, after everything else. */
export const OUTPUT_HIT_LIMIT = 10
/** Shorter queries match too much output to be worth asking the service. */
export const OUTPUT_MIN_CHARS = 3

/** The first hit of each pane (the service lists a pane's newest line first), so one busy session cannot fill the list. */
export function firstHitPerPane(hits: readonly SearchHit[]): SearchHit[] {
  const seen = new Set<string>()
  return hits.filter((hit) => {
    const pane = `${String(hit.machine)}/${String(hit.term)}`
    if (seen.has(pane)) return false
    seen.add(pane)
    return true
  })
}

/** The palette's results with the session-output hits after every other kind. */
export function withOutputHits(results: readonly PaletteItem[], hits: readonly PaletteItem[]): PaletteItem[] {
  return [...results, ...hits.slice(0, OUTPUT_HIT_LIMIT)]
}

const SNIPPET_CELLS = 60
const CELLS_BEFORE_MATCH = 16
const ELLIPSIS = '…'
// eslint-disable-next-line no-control-regex -- terminal output is made of control characters; these strip them
const ESCAPE_SEQUENCE = /\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\x5c)?|.)?|\x9b[0-?]*[ -/]*[@-~]/gu
const CONTROL = /\p{Cc}+/gu
// A program could reverse the rest of the row with these, so a line would not read as what it printed.
const BIDI_CONTROL = /[\u{200e}\u{200f}\u{061c}\u{202a}-\u{202e}\u{2066}-\u{2069}]/gu
const WIDE = /[\u{1100}-\u{115f}\u{2e80}-\u{a4cf}\u{ac00}-\u{d7a3}\u{f900}-\u{faff}\u{fe30}-\u{fe4f}\u{ff00}-\u{ff60}\u{ffe0}-\u{ffe6}\u{1f300}-\u{1faff}\u{20000}-\u{3fffd}]/u
const PICTOGRAPH = /^\p{Extended_Pictographic}/u
const WIDE_CELLS = 2
const graphemeSegmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' })

function graphemes(text: string): string[] {
  return Array.from(graphemeSegmenter.segment(text), (part) => part.segment)
}

function graphemeCells(grapheme: string): number {
  return WIDE.test(grapheme) || PICTOGRAPH.test(grapheme) ? WIDE_CELLS : 1
}

/** How many terminal cells `text` takes: a CJK character or an emoji takes two. */
export function cellsOf(text: string): number {
  return graphemes(text).reduce((sum, grapheme) => sum + graphemeCells(grapheme), 0)
}

/** The first graphemes of `parts` that fit in `cells` terminal cells. */
function fitCells(parts: readonly string[], cells: number): string[] {
  const kept: string[] = []
  let used = 0
  for (const part of parts) {
    used += graphemeCells(part)
    if (used > cells) break
    kept.push(part)
  }
  return kept
}

/** A line of terminal output as one short line of plain text, cut by display width with the match in view. */
export function outputSnippet(line: string, query: string): string {
  const text = line.replace(ESCAPE_SEQUENCE, '').replace(BIDI_CONTROL, '').replace(CONTROL, ' ').replace(/\s+/gu, ' ').trim()
  const at = Math.max(0, text.toLowerCase().indexOf(query.trim().toLowerCase()))
  const before = graphemes(text.slice(0, at))
  const start = before.length - fitCells(before.toReversed(), CELLS_BEFORE_MATCH).length
  const lead = start > 0 ? ELLIPSIS : ''
  const rest = graphemes(text).slice(start)
  const whole = fitCells(rest, SNIPPET_CELLS - lead.length)
  if (whole.length === rest.length) return lead + whole.join('')
  return lead + fitCells(rest, SNIPPET_CELLS - lead.length - ELLIPSIS.length).join('') + ELLIPSIS
}
