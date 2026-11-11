/** The short tag before a file's name: its type in two to four letters. A type with no tag here has none. */
const TAGS: Readonly<Record<string, string>> = {
  ts: 'TS', tsx: 'TSX', js: 'JS', jsx: 'JSX', mjs: 'JS', cjs: 'JS', json: '{ }', md: 'MD', mdx: 'MD', rs: 'RS', py: 'PY', go: 'GO',
  css: 'CSS', scss: 'CSS', html: 'HTML', toml: 'TOML', yml: 'YML', yaml: 'YML', sh: 'SH', ps1: 'PS', sql: 'SQL', svg: 'SVG', png: 'IMG', jpg: 'IMG', jpeg: 'IMG', gif: 'IMG', webp: 'IMG',
}

export function fileTag(name: string): string | null {
  const dot = name.lastIndexOf('.')
  if (dot <= 0) return null
  return TAGS[name.slice(dot + 1).toLowerCase()] ?? null
}

/** Every folder that holds one of these paths, at any depth: the folders that get a dot in the tree. */
export function foldersHolding(paths: readonly string[]): Set<string> {
  const folders = new Set<string>()
  for (const path of paths) {
    const parts = path.split('/').slice(0, -1)
    parts.forEach((_, index) => folders.add(parts.slice(0, index + 1).join('/')))
  }
  return folders
}
