export type ChangeKind = { letter: 'A' | 'M' | 'D'; word: string; tone: 'add' | 'edit' | 'remove' }

const ADDED: ChangeKind = { letter: 'A', word: 'Added', tone: 'add' }
const DELETED: ChangeKind = { letter: 'D', word: 'Deleted', tone: 'remove' }
const MODIFIED: ChangeKind = { letter: 'M', word: 'Modified', tone: 'edit' }
const FIRST_HUNK = '@@'

/** What happened to the file, read from the header git writes above its diff. */
export function changeKind(patch: string): ChangeKind {
  const hunk = patch.indexOf(FIRST_HUNK)
  const header = hunk === -1 ? patch : patch.slice(0, hunk)
  if (/^new file mode /m.test(header)) return ADDED
  if (/^deleted file mode /m.test(header)) return DELETED
  return MODIFIED
}
