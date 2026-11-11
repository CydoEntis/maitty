import { useMemo, useState } from 'react'
import type { PaneQueue } from './useChatComposer'

/**
 * For tests: a pane's queue that behaves as the daemon's does. Messages wait in order (one asked for `first`
 * goes ahead), one is taken back by its place and its words, and `sendNext` is the daemon typing the oldest
 * into the agent when a turn ends.
 */
export function useFakePaneQueue(): PaneQueue & { sendNext: () => void } {
  const [queued, setQueued] = useState<readonly string[]>([])
  return useMemo(() => ({
    queued,
    add: (text: string, first: boolean) => {
      setQueued((now) => (first ? [text, ...now] : [...now, text]))
      return Promise.resolve()
    },
    drop: (index: number, text: string) => {
      if (queued[index] !== text) return Promise.reject(new Error('that follow-up is no longer queued there'))
      setQueued((now) => now.filter((_, place) => place !== index))
      return Promise.resolve()
    },
    sendNext: () => { setQueued((now) => now.slice(1)) },
  }), [queued])
}
