import { useCallback, useEffect, useRef, useState } from 'react'

const ARM_MS = 4000

/**
 * A two-step confirm in place. When `needsArm` is set, the first press only arms (and disarms itself
 * after a few seconds); the second runs. `press` returns true when the action should run now.
 */
export function useArmed(needsArm: boolean) {
  const [armed, setArmed] = useState(false)
  const timer = useRef<number | undefined>(undefined)
  useEffect(() => () => { window.clearTimeout(timer.current) }, [])
  const disarm = useCallback(() => {
    window.clearTimeout(timer.current)
    setArmed(false)
  }, [])
  const press = useCallback((): boolean => {
    if (!needsArm || armed) {
      disarm()
      return true
    }
    setArmed(true)
    timer.current = window.setTimeout(() => { setArmed(false) }, ARM_MS)
    return false
  }, [needsArm, armed, disarm])
  return { armed, press, disarm }
}

/** Whether a program other than the plain shell runs in the session, so closing it would stop work. */
export const runsAgent = (process: string): boolean => process !== 'shell'
