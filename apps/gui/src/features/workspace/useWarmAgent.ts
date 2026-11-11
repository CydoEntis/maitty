import { useEffect } from 'react'
import { useLatest } from '@/lib/useLatest'
import type { NewChatRequest } from './components/NewChatPage'

/** What a chat would start as, without its first message: enough to start its agent ahead of it. */
export type WarmSpec = Pick<NewChatRequest, 'folder' | 'agent' | 'model' | 'reasoning' | 'mode'>

/** Asks for the agent kept warm, or with `null` says none is wanted any more. */
export type WarmAgent = (spec: WarmSpec | null) => void

/** A model and an effort are picked one after the other: the agent is started for where the picks came to rest. */
export const WARM_SETTLE_MS = 400
/** The daemon stops a warm agent ten minutes after it was last asked for; a page still open asks again well inside that. */
export const WARM_AGAIN_MS = 240_000

/**
 * Keeps an agent starting while a new chat is being written, so it is up by the time the first message is sent. `spec` is
 * what the chat would start as, or null when no agent can be started ahead (no folder yet, or a branch to make first).
 * While `paused` (the chat is being started) nothing is asked: the start takes the warm agent, and asking again would
 * make a second one. Leaving without starting says none is wanted.
 */
export function useWarmAgent(warm: WarmAgent | undefined, spec: WarmSpec | null, paused: boolean): void {
  const key = spec === null ? null : JSON.stringify(spec)
  const specNow = useLatest(() => spec)
  const warmNow = useLatest(() => warm)
  useEffect(() => {
    if (key === null) {
      warmNow()?.(null)
      return undefined
    }
    if (paused) return undefined
    const ask = () => { warmNow()?.(specNow()) }
    const first = window.setTimeout(ask, WARM_SETTLE_MS)
    const again = window.setInterval(ask, WARM_AGAIN_MS)
    return () => {
      window.clearTimeout(first)
      window.clearInterval(again)
    }
  }, [key, paused, specNow, warmNow])
  useEffect(() => () => { warmNow()?.(null) }, [warmNow])
}
