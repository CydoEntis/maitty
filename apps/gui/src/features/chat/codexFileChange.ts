import type { FileChangeKind, FileDiff } from './chatEvents'
import { parseUnifiedDiff, wholeFileDiff } from './unifiedDiff'

// The daemon writes a Codex `fileChange` item as one text block (history_provider.rs):
// "<path> (<kind>)\n<diff>" per file, files joined by a newline, and a "diff cut off" note
// appended when the block passed its size limit. The label is "Edited <path>",
// "Edited N files" or "Edited files".
const CUT_MARKER = '\n... diff cut off'
const HEADER = /^(\S.*) \(([a-z_]+)\)$/
const MANY_FILES_LABEL = /^Edited (\d+) files$/
const NO_FILES_LABEL = 'Edited files'
const ONE_FILE_LABEL = /^Edited (.+)$/

export type CodexFileChange = {
  readonly file: string
  readonly change: FileChangeKind
  readonly diff: FileDiff
}

function expectedFileCount(label: string): number | null {
  if (label === NO_FILES_LABEL) return 0
  const many = MANY_FILES_LABEL.exec(label)
  if (many?.[1] !== undefined) return Number(many[1])
  return ONE_FILE_LABEL.test(label) ? 1 : null
}

export function changeKindOf(word: string): FileChangeKind {
  if (word === 'add' || word === 'update' || word === 'delete') return word
  return 'unknown'
}

export function diffForChange(change: FileChangeKind, body: string, truncated: boolean): FileDiff {
  // Codex sends a created file as its raw contents, not as a diff.
  if (change === 'add') return wholeFileDiff(body, 'add', truncated)
  if (change === 'delete' && !body.startsWith('@@')) return wholeFileDiff(body, 'remove', truncated)
  return parseUnifiedDiff(body, truncated)
}

/** The file named by a single-file label, for when the block itself cannot be read. */
export function fileNamedByLabel(label: string): string | null {
  if (expectedFileCount(label) !== 1) return null
  return ONE_FILE_LABEL.exec(label)?.[1] ?? null
}

/**
 * Splits a `fileChange` item into one entry per file. Returns an empty list when the block does
 * not start with a file header, so the caller can fall back to a card without a diff.
 *
 * The label's file count stops header detection once every file is found, so a line of a created
 * file's raw contents that happens to look like a header cannot start a phantom file.
 */
export function parseCodexFileChanges(label: string, output: string): CodexFileChange[] {
  const expected = expectedFileCount(label)
  if (expected === 0) return []
  const truncated = output.endsWith(CUT_MARKER)
  const text = truncated ? output.slice(0, output.length - CUT_MARKER.length) : output

  const sections: { file: string; kind: FileChangeKind; lines: string[] }[] = []
  for (const [index, line] of text.split('\n').entries()) {
    const header = HEADER.exec(line)
    const mayStartFile = index === 0 || expected === null || sections.length < expected
    const section = sections.at(-1)
    if (mayStartFile && header?.[1] !== undefined && header[2] !== undefined) {
      sections.push({ file: header[1], kind: changeKindOf(header[2]), lines: [] })
    } else if (section) {
      section.lines.push(line)
    } else {
      return []
    }
  }

  return sections.map((section, position) => {
    const isLast = position === sections.length - 1
    // The daemon joins files with a newline after each diff; that blank line is not part of the diff.
    if (!isLast && section.lines.at(-1) === '') section.lines.pop()
    return {
      file: section.file,
      change: section.kind,
      diff: diffForChange(section.kind, section.lines.join('\n'), truncated && isLast),
    }
  })
}
