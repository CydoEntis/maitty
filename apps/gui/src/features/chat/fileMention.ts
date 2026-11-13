/** Where the message box searches for files: a pane of this computer's daemon, or the folder of a session on another machine. */
export type FileSearchPlace = PaneFileSearch | MachineFileSearch

/** This computer: the pane the daemon is asked through, and the folders paths are read from. */
export type PaneFileSearch = {
  readonly term: number
  /** The session's folder: an inserted path is relative to it. */
  readonly folder: string
  /** The top of the checkout the daemon answers from; null outside a repository, where it answers from the folder itself. */
  readonly checkout: string | null
}

/** Another machine: the host indexes the session's folder over the machine's ssh line, and the box ranks the index. */
export type MachineFileSearch = {
  /** The machine's name, as the host knows it. */
  readonly machine: string
  /** The session's folder there: the index's paths, and so an inserted path, are relative to it. */
  readonly folder: string
  /** The machine is not reachable now, so nothing is asked. */
  readonly offline: boolean
}

/** The file being named at the end of the message box: an `@` at the start or after a space, and what follows it. */
export type FileMention = { readonly start: number; readonly query: string }

/** The `@` being typed at the end of the draft; an `@` inside a word (an email address) is not one. */
export function fileMention(draft: string): FileMention | null {
  const found = /(^|\s)@(\S*)$/.exec(draft)
  if (found === null) return null
  const query = found[2] ?? ''
  return { start: draft.length - query.length - 1, query }
}

/**
 * The draft with the mention replaced by the path and a space. A path with whitespace or a double quote is quoted,
 * with its own quotes escaped, so it reads as one.
 */
export function withPath(draft: string, mention: FileMention, path: string): string {
  const written = /[\s"]/.test(path) ? `"${path.replaceAll('"', '\\"')}"` : path
  return `${draft.slice(0, mention.start)}${written} `
}

const slashes = (path: string): string => path.replaceAll('\\', '/').replace(/\/+$/, '')

const isDrivePath = (path: string): boolean => /^[A-Za-z]:\//.test(path)

/** Windows folders differ only in case, so a drive path is compared without it. */
const comparable = (path: string): string => (isDrivePath(path) ? path.toLowerCase() : path)

/**
 * A path the daemon gave relative to the checkout's top, made relative to the session's folder: `crates/a/x.rs` from
 * `crates/a` is `x.rs`, and from `crates/b` it is `../a/x.rs`. A folder not inside the checkout leaves it as it is.
 */
export function pathFromFolder(path: string, place: Pick<PaneFileSearch, 'folder' | 'checkout'>): string {
  if (place.checkout === null) return path
  const top = slashes(place.checkout)
  const folder = slashes(place.folder)
  if (!comparable(folder).startsWith(`${comparable(top)}/`)) return path
  const inside = folder.slice(top.length + 1).split('/')
  const target = path.split('/')
  const caseless = isDrivePath(folder)
  const same = (index: number) => {
    const a = inside[index] ?? ''
    const b = target[index] ?? ''
    return caseless ? a.toLowerCase() === b.toLowerCase() : a === b
  }
  let shared = 0
  while (shared < inside.length && shared < target.length - 1 && same(shared)) shared += 1
  return [...inside.slice(shared).map(() => '..'), ...target.slice(shared)].join('/')
}
