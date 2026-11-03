const KIB = 1024
const MIB = KIB * KIB
const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60
const HOURS_PER_DAY = 24
const MINUTE = SECONDS_PER_MINUTE
const HOUR = MINUTES_PER_HOUR * MINUTE
const DAY = HOURS_PER_DAY * HOUR
export const MS_PER_SECOND = 1000

export function formatBytes(bytes: number): string {
  if (bytes >= MIB) return `${(bytes / MIB).toFixed(1)} MB`
  if (bytes >= KIB) return `${String(Math.round(bytes / KIB))} KB`
  return `${String(bytes)} B`
}

/** "5m ago" for a unix time in seconds, relative to `nowSeconds`. */
export function ago(unixSeconds: number, nowSeconds: number): string {
  const seconds = Math.max(0, nowSeconds - unixSeconds)
  if (seconds < MINUTE) return 'just now'
  if (seconds < HOUR) return `${String(Math.floor(seconds / MINUTE))}m ago`
  if (seconds < DAY) return `${String(Math.floor(seconds / HOUR))}h ago`
  return `${String(Math.floor(seconds / DAY))}d ago`
}

/** The folder part of a repo-relative path ('' for a file at the top). */
export function parentOf(path: string): string {
  const cut = path.lastIndexOf('/')
  return cut < 0 ? '' : path.slice(0, cut)
}

export function joinPath(folder: string, name: string): string {
  return folder ? `${folder}/${name}` : name
}
