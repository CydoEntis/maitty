/**
 * A file's unified diff (the `patch` of `conversation_diff`) as numbered lines, for the panel's diff
 * view. Pure. A line comment needs the line's number in the file as it is now, so each line carries
 * its old and new number from the hunk headers; a removed line has no new number and cannot be
 * commented on.
 */

export type PatchLine =
  | { readonly kind: 'hunk'; readonly key: string; readonly text: string }
  | { readonly kind: 'add' | 'remove' | 'context'; readonly key: string; readonly text: string; readonly oldNo: number | null; readonly newNo: number | null }

const HUNK_HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/
const FILE_HEADER = 'diff '
const NO_NEWLINE_MARKER = '\\'

export function parsePatch(patch: string): PatchLine[] {
  const lines: PatchLine[] = []
  let oldNo = 0
  let newNo = 0
  let inHunk = false
  for (const [index, raw] of patch.split(/\r?\n/).entries()) {
    const header = HUNK_HEADER.exec(raw)
    if (header) {
      oldNo = Number(header[1])
      newNo = Number(header[2])
      inHunk = true
      lines.push({ kind: 'hunk', key: String(index), text: raw })
      continue
    }
    // Everything before the first hunk names the file, and a marker after a line is not a line.
    if (!inHunk || raw.startsWith(NO_NEWLINE_MARKER)) continue
    // A hunk line starts with a sign or a space, so a `diff ` line is the next file's header and ends the hunk.
    if (raw.startsWith(FILE_HEADER)) {
      inHunk = false
      continue
    }
    const sign = raw.charAt(0)
    const text = raw.slice(1)
    if (sign === '+') {
      lines.push({ kind: 'add', key: String(index), text, oldNo: null, newNo })
      newNo += 1
    } else if (sign === '-') {
      lines.push({ kind: 'remove', key: String(index), text, oldNo, newNo: null })
      oldNo += 1
    } else if (sign === ' ') {
      lines.push({ kind: 'context', key: String(index), text, oldNo, newNo })
      oldNo += 1
      newNo += 1
    }
  }
  return lines
}

/** The directory part of a path (with its trailing slash) and the file name, so the directory can be dimmed. */
export function splitPath(path: string): { dir: string; name: string } {
  const cut = path.lastIndexOf('/')
  return cut === -1 ? { dir: '', name: path } : { dir: path.slice(0, cut + 1), name: path.slice(cut + 1) }
}
