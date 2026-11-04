import { createContext, useContext } from 'react'
import type { AnimationLevel, EffectStyle } from '@/lib/animation-level'
import type { WorkingScope } from '@/features/settings/preferences'
import type { SessionStatus } from '../model/session'

/** Whether motion is reduced (the setting or the system's). Spinners then hold one frame. */
export const ReducedMotionContext = createContext(false)

export function useReducedMotion(): boolean {
  return useContext(ReducedMotionContext)
}

/**
 * The style each status effect runs with, or null when it is off. Already capped by the animation
 * level, and null for all of them under reduced motion or while the daemon is disconnected. `cap` is
 * the level itself, for the Settings previews.
 */
export type EnabledEffects = { working: EffectStyle | null; needs: EffectStyle | null; done: EffectStyle | null; workingScope: WorkingScope; cap: AnimationLevel }

export const EffectsContext = createContext<EnabledEffects>({ working: null, needs: null, done: null, workingScope: 'all', cap: 'waves' })

export type StatusEffectKey = 'working' | 'needs' | 'done'

/** What a row or tab shows for its status: the host class and the layer's style, none when the effect is off. */
export type AppliedEffect = { className: string; status: StatusEffectKey | null; style: EffectStyle | null }

const NO_EFFECT: AppliedEffect = { className: '', status: null, style: null }

/** Pure so the scope rule is testable: Working can be limited to the row or tab you are in. */
export function appliedEffect(effects: EnabledEffects, status: SessionStatus, selected: boolean): AppliedEffect {
  const key: StatusEffectKey | null = status === 'working' || status === 'needs' || status === 'done' ? status : null
  if (key === null) return NO_EFFECT
  const style = effects[key]
  if (style === null) return NO_EFFECT
  if (key === 'working' && (effects.workingScope === 'off' || (effects.workingScope === 'selected' && !selected))) return NO_EFFECT
  return { className: `mx-fx-${key}${selected ? ' mx-fx-strong' : ''}`, status: key, style }
}

/** The status effect for a row or tab. */
export function useStatusEffect(status: SessionStatus, selected: boolean): AppliedEffect {
  return appliedEffect(useContext(EffectsContext), status, selected)
}
