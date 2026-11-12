import { changeKind } from './changeKind'

/** The most file names a suggested message lists before it counts the rest. */
const NAMED_FILES = 3
const VERB = { A: 'Add', M: 'Update', D: 'Remove' } as const

type Changed = { path: string; patch: string }

function nameOf(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1)
}

/** The folder most of the files share, or null when they do not share one. */
function commonFolder(paths: readonly string[]): string | null {
  const folders = new Set(paths.map((path) => path.slice(0, Math.max(0, path.lastIndexOf('/')))))
  const [only] = folders
  return folders.size === 1 && only !== undefined && only !== '' ? only : null
}

/** A first draft of a commit message from what changed; a person still edits it. */
export function commitTextFor(files: readonly Changed[]): string {
  const [first] = files
  if (first === undefined) return ''
  const kinds = new Set(files.map((file) => changeKind(file.patch).letter))
  const [kind] = kinds
  const verb = kinds.size === 1 && kind !== undefined ? VERB[kind] : 'Update'
  if (files.length <= NAMED_FILES) return `${verb} ${files.map((file) => nameOf(file.path)).join(', ')}`
  const folder = commonFolder(files.map((file) => file.path))
  return `${verb} ${String(files.length)} files${folder === null ? '' : ` in ${folder}`}`
}
