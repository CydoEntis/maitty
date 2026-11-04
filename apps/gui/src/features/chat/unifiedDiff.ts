import type { DiffHunk, DiffLine, FileDiff } from './chatEvents'

const HUNK_MARKER = '@@'
// Lines a git or Codex diff puts before its first hunk; they name the file, not a change.
const FILE_HEADER_PREFIXES = ['--- ', '+++ ', 'diff ', 'index ', 'new file', 'deleted file', 'similarity', 'rename '] as const
const NO_NEWLINE_MARKER = '\\'

export function splitLines(text: string): string[] {
  if (text === '') return []
  const lines = text.split('\n')
  if (lines.at(-1) === '') lines.pop()
  return lines
}

export function diffFromHunks(hunks: readonly DiffHunk[], truncated = false): FileDiff {
  let added = 0
  let removed = 0
  for (const hunk of hunks) {
    for (const line of hunk.lines) {
      if (line.kind === 'add') added += 1
      if (line.kind === 'remove') removed += 1
    }
  }
  return { status: 'available', hunks, added, removed, truncated }
}

/** Reads a unified diff. Text that is not a diff yields no hunks rather than an error. */
export function parseUnifiedDiff(text: string, truncated = false): FileDiff {
  const hunks: { header: string; lines: DiffLine[] }[] = []
  let current: { header: string; lines: DiffLine[] } | null = null
  for (const line of splitLines(text)) {
    if (line.startsWith(HUNK_MARKER)) {
      current = { header: line, lines: [] }
      hunks.push(current)
      continue
    }
    if (current === null && FILE_HEADER_PREFIXES.some((prefix) => line.startsWith(prefix))) continue
    if (line.startsWith(NO_NEWLINE_MARKER)) continue
    // A diff with no @@ line still has changes; keep them in one headerless hunk.
    if (current === null) {
      current = { header: '', lines: [] }
      hunks.push(current)
    }
    const sign = line.charAt(0)
    const body = line.slice(1)
    if (sign === '+') current.lines.push({ kind: 'add', text: body })
    else if (sign === '-') current.lines.push({ kind: 'remove', text: body })
    else current.lines.push({ kind: 'context', text: body })
  }
  return diffFromHunks(hunks, truncated)
}

function hunkHeader(oldStart: number, oldCount: number, newStart: number, newCount: number): string {
  return `${HUNK_MARKER} -${String(oldStart)},${String(oldCount)} +${String(newStart)},${String(newCount)} ${HUNK_MARKER}`
}

/** A hunk where every line is added or every line is removed (a created or deleted file). */
export function wholeFileDiff(text: string, kind: 'add' | 'remove', truncated = false): FileDiff {
  const lines = splitLines(text).map((line): DiffLine => ({ kind, text: line }))
  if (lines.length === 0) return diffFromHunks([], truncated)
  const header = kind === 'add' ? hunkHeader(0, 0, 1, lines.length) : hunkHeader(1, lines.length, 0, 0)
  return diffFromHunks([{ header, lines }], truncated)
}

/** One replacement as a hunk: the old text removed, the new text added. */
export function replacementHunk(oldText: string, newText: string): DiffHunk {
  const removed = splitLines(oldText)
  const added = splitLines(newText)
  return {
    header: hunkHeader(removed.length > 0 ? 1 : 0, removed.length, added.length > 0 ? 1 : 0, added.length),
    lines: [
      ...removed.map((line): DiffLine => ({ kind: 'remove', text: line })),
      ...added.map((line): DiffLine => ({ kind: 'add', text: line })),
    ],
  }
}
