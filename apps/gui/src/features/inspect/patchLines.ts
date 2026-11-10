export type PatchLineKind = 'meta' | 'hunk' | 'add' | 'remove' | 'context'

/**
 * One line of a unified diff. `line` is its number in the new file, or in the old file for a removed
 * line; null for a line of the diff itself. `oldLine` and `newLine` are its place on each side, null
 * on the side it is not on.
 */
export type PatchLine = { kind: PatchLineKind; text: string; line: number | null; oldLine: number | null; newLine: number | null }

const HUNK_HEADER = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/
const NO_NEWLINE_NOTE = '\\'

/**
 * A file's unified diff as numbered lines. A hunk header says how many old and new lines follow, so a
 * removed line that happens to start with `-- ` is still read as code, not as a file header.
 */
export function patchLines(patch: string): PatchLine[] {
  let oldLine = 0
  let newLine = 0
  let oldLeft = 0
  let newLeft = 0
  return patch.split(/\r?\n/).map((text): PatchLine => {
    const header = HUNK_HEADER.exec(text)
    if (header) {
      oldLine = Number(header[1])
      oldLeft = Number(header[2] ?? 1)
      newLine = Number(header[3])
      newLeft = Number(header[4] ?? 1)
      return { kind: 'hunk', text, line: null, oldLine: null, newLine: null }
    }
    const inHunk = oldLeft > 0 || newLeft > 0
    if (!inHunk || text.startsWith(NO_NEWLINE_NOTE)) return { kind: text === '' ? 'context' : 'meta', text, line: null, oldLine: null, newLine: null }
    if (text.startsWith('+')) {
      newLeft -= 1
      newLine += 1
      return { kind: 'add', text, line: newLine - 1, oldLine: null, newLine: newLine - 1 }
    }
    if (text.startsWith('-')) {
      oldLeft -= 1
      oldLine += 1
      return { kind: 'remove', text, line: oldLine - 1, oldLine: oldLine - 1, newLine: null }
    }
    oldLeft -= 1
    newLeft -= 1
    oldLine += 1
    newLine += 1
    return { kind: 'context', text, line: newLine - 1, oldLine: oldLine - 1, newLine: newLine - 1 }
  })
}

/** What the agent is sent for a comment on a line: where it is, the line itself, then the comment. */
export function commentMessage(path: string, line: PatchLine, comment: string): string {
  const where = line.line === null ? path : `${path}, line ${String(line.line)}${line.kind === 'remove' ? ' (removed)' : ''}`
  return `About ${where}:\n> ${line.text.slice(1)}\n\n${comment.trim()}`
}
