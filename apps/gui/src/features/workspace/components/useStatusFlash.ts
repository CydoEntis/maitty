import { useLayoutEffect, useRef, type RefObject } from 'react'
import type { SessionStatus, SessionView } from '../model/session'

const FLASH_MS = 700

/**
 * A row whose status just changed is tinted with the accent for a moment, so the eye finds it where
 * it landed: in the status list it has just moved to another section. Rows are found by `data-flip`,
 * as `useFlip` finds them; the tint fades to whatever the row's own ground is, and nothing is tinted
 * when motion is reduced or on the first paint.
 */
export function useStatusFlash(container: RefObject<HTMLElement | null>, sessions: readonly SessionView[], reduced: boolean): void {
  const previous = useRef(new Map<string, SessionStatus>())
  useLayoutEffect(() => {
    const was = previous.current
    previous.current = new Map(sessions.map((session) => [session.id, session.status]))
    const root = container.current
    if (root === null || reduced || was.size === 0) return
    const changed = new Set(sessions.filter((session) => was.has(session.id) && was.get(session.id) !== session.status).map((session) => session.id))
    if (changed.size === 0) return
    for (const wrap of Array.from(root.querySelectorAll<HTMLElement>('[data-flip]'))) {
      if (!changed.has(wrap.dataset.flip ?? '')) continue
      const row = wrap.querySelector<HTMLElement>('.mx-row')
      if (typeof row?.animate !== 'function') continue
      row.animate([{ background: 'var(--accent-soft)', boxShadow: 'inset 0 0 0 1px var(--accent-line)' }, {}], { duration: FLASH_MS, easing: 'ease-out' })
    }
  }, [container, sessions, reduced])
}
