import { flushSync } from 'react-dom'
import { isLinuxWebView } from '@/lib/gpu'

type Transitions = { startViewTransition?: (update: () => void) => { finished: Promise<unknown> } }

// Only the tab showing: a tab kept mounted behind it has panes too, and they do not move.
const NAMED = '.mx-panes:not(.is-kept) .mx-pane[data-session-id]'
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
 * Reduce motion on, the change just happens. It also just happens on the Linux web view: a swap by drag was
 * reported to take the whole window down there (0.72.0, Hyprland), and the glide is the one part of a swap
 * that asks the web view for something unusual.
 */
export function movePanes(update: () => void): void {
  const doc = document as Document & Transitions
  if (typeof doc.startViewTransition !== 'function' || isLinuxWebView() || reducedMotion() || panesNow().length < MIN_PANES) {
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
