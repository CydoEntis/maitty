import { isWindowActive, subscribeWindowActivity } from './window-activity'

export type SpinnerStyle = 'braille' | 'snake' | 'bars'

// Frames mirror the TUI spinners in crates/maitty-settings/src/config.rs.
const SPINNERS: Record<SpinnerStyle, readonly string[]> = {
  braille: ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
  snake: ['⠇', '⠋', '⠙', '⠸', '⢰', '⣠', '⣄', '⡆'],
  bars: ['⣰⡄', '⣼⡀', '⣶⡆', '⣧⡇', '⣆⡆', '⣤⡄', '⣸⡆', '⣴⡇', '⣇⡄', '⣦⡀'],
}

/**
 * The TUI steps its spinners every 80-90 ms. Here every step repaints each mounted spinner in a
 * web view that may draw in software, so the frames go at half that rate (docs/PERFORMANCE.md).
 */
export const SPINNER_TICK_MS = 160

/** Elements showing the spinner; the clock writes each frame straight into them, outside React. */
const targets = new Set<Element>()
let style: SpinnerStyle = 'braille'
let frame = 0
let timer: ReturnType<typeof setInterval> | undefined
let unsubscribeWindow: (() => void) | undefined

export function currentGlyph(): string {
  return SPINNERS[style][frame] ?? ''
}

export function spinnerFrames(): readonly string[] {
  return SPINNERS[style]
}

/** A two-character frame needs a wider box (`is-wide`). */
export function isWideSpinner(): boolean {
  return (SPINNERS[style][0] ?? '').length > 1
}

function paint() {
  const glyph = currentGlyph()
  const wide = isWideSpinner()
  for (const target of targets) {
    target.textContent = glyph
    target.classList.toggle('is-wide', wide)
  }
}

function tick() {
  frame = (frame + 1) % SPINNERS[style].length
  paint()
}

/** One shared clock runs while something shows a spinner and the window is in front. */
function syncTimer() {
  const shouldRun = targets.size > 0 && isWindowActive()
  if (shouldRun && timer === undefined) timer = setInterval(tick, SPINNER_TICK_MS)
  if (!shouldRun && timer !== undefined) {
    clearInterval(timer)
    timer = undefined
  }
}

export function setSpinnerStyle(next: SpinnerStyle) {
  if (next === style) return
  style = next
  frame = 0
  paint()
}

/** Shows the spinner in `target` and keeps it stepping until the returned function is called. */
export function attachSpinner(target: Element): () => void {
  targets.add(target)
  paint()
  unsubscribeWindow ??= subscribeWindowActivity(syncTimer)
  syncTimer()
  return () => {
    targets.delete(target)
    syncTimer()
    if (targets.size === 0) {
      unsubscribeWindow?.()
      unsubscribeWindow = undefined
    }
  }
}

/** One style's frames and speed, for the Settings preview that animates each style. */
export function spinnerDefinition(spinner: SpinnerStyle): { frames: readonly string[]; intervalMs: number } {
  return { frames: SPINNERS[spinner], intervalMs: SPINNER_TICK_MS }
}
