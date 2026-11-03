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
