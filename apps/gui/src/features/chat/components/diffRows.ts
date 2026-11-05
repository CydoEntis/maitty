import type { DiffHunk } from '../chatEvents'

export type DiffRow = { key: string; sign: string; text: string; tone: 'add' | 'remove' | 'context' | 'header' }

/** The lines a diff card draws: each hunk's header, then its lines with a sign. */
export function diffRows(hunks: readonly DiffHunk[]): DiffRow[] {
  return hunks.flatMap((hunk, hunkIndex) => [
    ...(hunk.header === '' ? [] : [{ key: `${String(hunkIndex)}h`, sign: '', text: hunk.header, tone: 'header' as const }]),
    ...hunk.lines.map((line, lineIndex): DiffRow => ({ key: `${String(hunkIndex)}:${String(lineIndex)}`, sign: line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' ', text: line.text, tone: line.kind })),
  ])
}

/** A line's tint is its theme colour at 10%, so it holds in the light themes too (v18 C.10). */
export const DIFF_TONE: Record<DiffRow['tone'], string> = {
  add: 'bg-[color-mix(in_srgb,var(--ok)_10%,transparent)]',
  remove: 'bg-[color-mix(in_srgb,var(--danger)_10%,transparent)]',
  context: '',
  header: 'text-dim',
}
