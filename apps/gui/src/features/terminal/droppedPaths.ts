/** Characters a POSIX shell, and Claude Code reading a pasted path, treat as more than part of a name. */
const POSIX_SPECIAL = /[\s()[\]{}&;'"$`!#*?<>|\\]/g

function quoteForWindows(path: string): string {
  return /[\s&()^%!;,='[\]{}]/.test(path) ? `"${path}"` : path
}

function quoteForPosix(path: string): string {
  return path.replace(POSIX_SPECIAL, (character) => `\\${character}`)
}

/**
 * Files dropped on a terminal, as the text a terminal types for them: each path quoted for the shell,
 * separated by spaces, with a space after so more can be typed. Never ends in a newline, so nothing runs.
 */
export function formatDroppedPaths(paths: readonly string[], windows: boolean): string {
  const quote = windows ? quoteForWindows : quoteForPosix
  return paths.filter((path) => path !== '').map(quote).join(' ').concat(' ')
}
