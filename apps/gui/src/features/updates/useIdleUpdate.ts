import { useEffect, useRef } from 'react'
import { isWindowActive, subscribeWindowActivity } from '@/lib/window-activity'
import { idleUpdateStep, isIdleForUpdate } from './idle'

/** How often idleness is looked at again; being idle is a matter of minutes, so this can be slow. */
const IDLE_CHECK_MS = 30_000

type Options = {
  /** The setting is on, the daemon is reachable and no restart is under way. */
  enabled: boolean
  agentBusy: boolean
  status: string
  offeredIsSkipped: boolean
  onDownload: () => void
  onInstall: () => void
  /** The clock; tests pass a fake one. */
  now?: () => number
}

/**
 * Downloads and installs an update while the app is idle (`isIdleForUpdate`). It keeps its inputs in
 * refs and holds no state, so watching the keyboard never re-renders the window.
 */
export function useIdleUpdate({ enabled, agentBusy, status, offeredIsSkipped, onDownload, onInstall, now = Date.now }: Options) {
  const lastInputAt = useRef(now())
  const latest = useRef({ agentBusy, status, offeredIsSkipped, onDownload, onInstall, now })
  useEffect(() => { latest.current = { agentBusy, status, offeredIsSkipped, onDownload, onInstall, now } })

  useEffect(() => {
    const onKeyDown = () => { lastInputAt.current = latest.current.now() }
    window.addEventListener('keydown', onKeyDown, true)
    return () => { window.removeEventListener('keydown', onKeyDown, true) }
  }, [])

  useEffect(() => {
    if (!enabled) return
    const evaluate = () => {
      const current = latest.current
      const idle = isIdleForUpdate({ now: current.now(), lastInputAt: lastInputAt.current, windowActive: isWindowActive(), agentBusy: current.agentBusy })
      const step = idleUpdateStep(idle, current.status, current.offeredIsSkipped)
      if (step === 'download') current.onDownload()
      if (step === 'install') current.onInstall()
    }
    evaluate()
    const timer = window.setInterval(evaluate, IDLE_CHECK_MS)
    const unsubscribe = subscribeWindowActivity(evaluate)
    return () => {
      window.clearInterval(timer)
      unsubscribe()
    }
  }, [enabled, agentBusy, status])
}
