import { useNow } from '@/lib/clock'
import { formatAgo, formatElapsed } from '../model/tracking'

/** How long ago `at` was ("now", "4m", "3h"); re-renders on its own each second. */
export function Ago({ at, prefix = '' }: { at: number; prefix?: string }) {
  const now = useNow()
  return <>{prefix}{formatAgo(now - at)}</>
}

/** A running "0:42" since `since`; re-renders on its own each second. */
export function ElapsedSince({ since, prefix = '' }: { since: number; prefix?: string }) {
  const now = useNow()
  return <>{prefix}{formatElapsed(now - since)}</>
}

/** "disconnected since 2m ago", or "disconnected just now" in its first minute; re-renders on its own. */
export function DisconnectedSince({ at }: { at: number | null }) {
  const now = useNow()
  if (at === null) return <>disconnected</>
  const ago = formatAgo(now - at)
  return <>{ago === 'now' ? 'disconnected just now' : `disconnected since ${ago} ago`}</>
}
