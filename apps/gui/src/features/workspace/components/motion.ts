import { createContext, useContext } from 'react'

/** Whether motion is reduced (the setting or the system's). Spinners then hold one frame. */
export const ReducedMotionContext = createContext(false)

export function useReducedMotion(): boolean {
  return useContext(ReducedMotionContext)
}
