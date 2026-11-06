import type { FolderListing } from '@/api/folders'

/** Where a session on another machine starts when nothing else is typed: its home folder. */
export const DEFAULT_REMOTE_FOLDER = '~'

export type FolderSuggestion = { kind: 'recent' | 'folder'; folder: string; label: string }

/** `~/a/b/` is `~/a/b`; the home folder and the root keep their one character. */
export function withoutTrailingSlash(path: string): string {
  const trimmed = path.trim()
  return trimmed.length > 1 ? trimmed.replace(/\/+$/, '') || '/' : trimmed
}

/** The folder one level up. The home folder and the root are their own parent. */
export function parentOf(path: string): string {
  const folder = withoutTrailingSlash(path)
  if (folder === '~' || folder === '/') return folder
  const cut = folder.lastIndexOf('/')
  if (cut < 0) return DEFAULT_REMOTE_FOLDER
  if (cut === 0) return '/'
  return folder.slice(0, cut)
}

/** The folder a typed path is inside (with its closing `/`) and the part still being typed. */
export function splitTyped(path: string): { folder: string; part: string } {
  const text = path.trim()
  const cut = text.lastIndexOf('/')
  if (cut < 0) return { folder: '~/', part: text === '~' ? '' : text }
  return { folder: text.slice(0, cut + 1), part: text.slice(cut + 1) }
}

/** Subfolders that continue what is typed; hidden ones only once a dot has been typed. */
export function subfolderSuggestions(listing: FolderListing | null, folder: string, part: string): FolderSuggestion[] {
  if (listing?.exists !== true) return []
  const wanted = part.toLowerCase()
  return listing.entries
    .filter((entry) => (part.startsWith('.') || !entry.name.startsWith('.')) && entry.name.toLowerCase().startsWith(wanted))
    .map((entry) => ({ kind: 'folder' as const, folder: `${folder}${entry.name}/`, label: entry.name }))
}
