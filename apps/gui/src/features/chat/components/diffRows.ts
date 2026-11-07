import type { DiffHunk } from '../chatEvents'

export type DiffRow = { key: string; sign: string; text: string; tone: 'add' | 'remove' | 'context' | 'header' }

/** The lines a diff card draws: each hunk's header, then its lines with a sign. */
export function diffRows(hunks: readonly DiffHunk[]): DiffRow[] {
  return hunks.flatMap((hunk, hunkIndex) => [
    ...(hunk.header === '' ? [] : [{ key: `${String(hunkIndex)}h`, sign: '', text: hunk.header, tone: 'header' as const }]),
    ...hunk.lines.map((line, lineIndex): DiffRow => ({ key: `${String(hunkIndex)}:${String(lineIndex)}`, sign: line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' ', text: line.text, tone: line.kind })),
  ])
}
