import { useEffect, useState } from 'react'

const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)'
const DARK_SCHEME_QUERY = '(prefers-color-scheme: dark)'

/** Whether the window is narrower than `px`. It changes only when the window crosses that width, so dragging the window's edge does not draw the app again at every pixel. */
export function useWindowNarrowerThan(px: number): boolean {
  const [narrower, setNarrower] = useState(() => window.innerWidth < px)
  useEffect(() => {
    const onResize = () => { setNarrower(window.innerWidth < px) }
    window.addEventListener('resize', onResize)
    return () => { window.removeEventListener('resize', onResize) }
  }, [px])
  return narrower
}

/** Whether the operating system asks for less motion, following it as it changes. */
export function useSystemReducedMotion(): boolean {
  const [query] = useState(() => (typeof window.matchMedia === 'function' ? window.matchMedia(REDUCED_MOTION_QUERY) : null))
  const [reduced, setReduced] = useState(() => query?.matches ?? false)
  useEffect(() => {
    if (!query) return
    const onChange = () => { setReduced(query.matches) }
    query.addEventListener('change', onChange)
    return () => { query.removeEventListener('change', onChange) }
  }, [query])
  return reduced
}

/** Whether the operating system is set to dark, following it as it changes (System mode needs it). */
export function useSystemDark(): boolean {
  const [query] = useState(() => (typeof window.matchMedia === 'function' ? window.matchMedia(DARK_SCHEME_QUERY) : null))
  const [dark, setDark] = useState(() => query?.matches ?? true)
  useEffect(() => {
    if (!query) return
    const onChange = () => { setDark(query.matches) }
    query.addEventListener('change', onChange)
    return () => { query.removeEventListener('change', onChange) }
  }, [query])
  return dark
}
