import { suggestProjectFolders } from '@/api/daemon'

const HOME_PREFIX = '~/'

/** Turns the start-folder setting into a path the daemon can open: empty or `~` is the home folder. */
export function resolveStartFolder(setting: string, home: string): string {
  const typed = setting.trim()
  if (!typed || typed === '~') return home
  if (typed.startsWith('~/') || typed.startsWith('~\\')) {
    const separator = home.includes('\\') && !home.includes('/') ? '\\' : '/'
    return `${home.replace(/[\\/]+$/, '')}${separator}${typed.slice(HOME_PREFIX.length)}`
  }
  return typed
}

/** Whether a typed folder exists, asked of the folder suggestions the desktop shell offers. */
export async function folderExists(path: string): Promise<boolean> {
  const typed = path.trim().replace(/(.)[\\/]+$/, '$1')
  if (!typed || typed === '~' || /^([A-Za-z]:)?[\\/]$/.test(typed)) return true
  // Suggestions end with the separator the typed path last used.
  const separator = /[\\/](?=[^\\/]*$)/.exec(typed)?.[0] ?? '/'
  const suggestions = await suggestProjectFolders(typed)
  const wanted = `${typed}${separator}`.toLowerCase()
  return suggestions.some((suggestion) => suggestion.toLowerCase() === wanted)
}
