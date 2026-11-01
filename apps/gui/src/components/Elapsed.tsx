import { useEffect, useState } from 'react'
import { formatElapsed } from '@/lib/working-since'

const TICK_MS = 1000

/** Ticks on its own so a running timer never re-renders the surrounding app. */
export function Elapsed({ since }: { since: number }) {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now())
    }, TICK_MS)
    return () => {
      clearInterval(timer)
    }
  }, [])
  return <span className="elapsed">{formatElapsed(now - since)}</span>
}
