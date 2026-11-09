import { flushSync } from 'react-dom'

type Transitions = { startViewTransition?: (update: () => void) => { finished: Promise<unknown> } }

const NAMED = '.mx-pane[data-session-id]'
/** Moving needs two panes to move between. */
const MIN_PANES = 2

const panesNow = (): HTMLElement[] => Array.from(document.querySelectorAll<HTMLElement>(NAMED))

/** Names each pane after its session, so the browser can tell which pane went where. */
function namePanes(): HTMLElement[] {
  const panes = panesNow()
  for (const pane of panes) pane.style.viewTransitionName = `pane-${pane.dataset.sessionId ?? ''}`
  return panes
}

const reducedMotion = (): boolean =>
  window.matchMedia('(prefers-reduced-motion: reduce)').matches || document.querySelector('.mx-app[data-motion="reduced"]') !== null

/**
 * Runs a change that moves panes (a swap) so they glide to their new places. The browser snapshots the panes before and after,
 * so the text inside never reflows in between: only pictures of the panes slide. Without View Transitions, or with
 * Reduce motion on, the change just happens.
 */
export function movePanes(update: () => void): void {
  const doc = document as Document & Transitions
  if (typeof doc.startViewTransition !== 'function' || reducedMotion() || panesNow().length < MIN_PANES) {
    update()
    return
  }
  namePanes()
  const transition = doc.startViewTransition(() => {
    flushSync(update)
    namePanes()
  })
  void transition.finished.finally(() => {
    for (const pane of panesNow()) pane.style.removeProperty('view-transition-name')
  })
}
