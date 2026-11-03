import { createContext, useContext } from 'react'
import type { SessionStatus } from '../model/session'

/** Whether motion is reduced (the setting or the system's). Spinners then hold one frame. */
export const ReducedMotionContext = createContext(false)

export function useReducedMotion(): boolean {
  return useContext(ReducedMotionContext)
}

export type EnabledEffects = { working: boolean; needs: boolean; done: boolean }

/** Which status effects run: each has its own switch, and all stop under reduced motion or offline. */
export const EffectsContext = createContext<EnabledEffects>({ working: false, needs: false, done: false })

/** The status-effect class for a row or tab, or none when that effect is off. */
export function useEffectClass(status: SessionStatus, strong: boolean): string {
  const effects = useContext(EffectsContext)
  const strength = strong ? ' mx-fx-strong' : ''
  if (status === 'working' && effects.working) return `mx-fx-working${strength}`
  if (status === 'needs' && effects.needs) return `mx-fx-needs${strength}`
  if (status === 'done' && effects.done) return `mx-fx-done${strength}`
  return ''
}
