import type { DiffHunk } from '../chatEvents'

export type DiffRow = {
  key: string
  sign: string
  text: string
  tone: 'add' | 'remove' | 'context' | 'header'
  /** The line's number in the file (the old file for a removed line); null where the hunk header does not say. */
  number: number | null
}

const HUNK_START = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/

/** Where a hunk starts in the old and new file, read from its `@@ -a,b +c,d @@` header. */
function hunkStart(header: string): { old: number; new: number } | null {
  const match = HUNK_START.exec(header)
  return match === null ? null : { old: Number(match[1]), new: Number(match[2]) }
}

/** The lines a diff card draws: each hunk's header, then its lines with a sign and, where known, a line number. */
export function diffRows(hunks: readonly DiffHunk[]): DiffRow[] {
  return hunks.flatMap((hunk, hunkIndex) => {
    const start = hunkStart(hunk.header)
    let oldLine = start?.old ?? null
    let newLine = start?.new ?? null
    const lines = hunk.lines.map((line, lineIndex): DiffRow => {
      const number = line.kind === 'remove' ? oldLine : newLine
      if (line.kind !== 'add' && oldLine !== null) oldLine += 1
      if (line.kind !== 'remove' && newLine !== null) newLine += 1
      return { key: `${String(hunkIndex)}:${String(lineIndex)}`, sign: line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' ', text: line.text, tone: line.kind, number }
    })
    const header = hunk.header === '' ? [] : [{ key: `${String(hunkIndex)}h`, sign: '', text: hunk.header, tone: 'header' as const, number: null }]
    return [...header, ...lines]
  })
}
