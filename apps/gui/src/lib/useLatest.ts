import { useCallback, useEffect, useRef } from 'react'

/**
 * A stable function that calls whichever `fn` rendered last. For a callback that outlives its render,
 * such as the button of a toast, and so must not close over what it saw when it was made.
 */
export function useLatest<Args extends unknown[], Result>(fn: (...args: Args) => Result): (...args: Args) => Result {
  const latest = useRef(fn)
  useEffect(() => { latest.current = fn })
  return useCallback((...args: Args) => latest.current(...args), [])
}
