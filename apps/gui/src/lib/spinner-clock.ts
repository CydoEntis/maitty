import { useSyncExternalStore } from 'react'
import { isWindowActive, subscribeWindowActivity } from './window-activity'

/** The Maitty TUI's spinners: a few Braille characters stepped by the shared clock. */
export type TuiSpinner = 'braille' | 'snake' | 'bars'
/** Inline SVG spinners drawn in the text colour and moved by CSS (`.mx-wspin`), so they need no clock. */
export type WebSpinner = 'circle' | 'dots' | 'equalizer' | 'spokes' | 'ping'
export type SpinnerStyle = TuiSpinner | WebSpinner

export const TUI_SPINNERS: ReadonlyArray<readonly [TuiSpinner, string]> = [['braille', 'Braille'], ['snake', 'Snake'], ['bars', 'Bars']]
export const WEB_SPINNERS: ReadonlyArray<readonly [WebSpinner, string]> = [['circle', 'Circle'], ['dots', 'Dots'], ['equalizer', 'Equalizer'], ['spokes', 'Spokes'], ['ping', 'Ping']]

export function isSpinnerStyle(value: unknown): value is SpinnerStyle {
  return [...TUI_SPINNERS, ...WEB_SPINNERS].some(([id]) => id === value)
}

export function isWebSpinner(style: SpinnerStyle): style is WebSpinner {
  return WEB_SPINNERS.some(([id]) => id === style)
}

/** The Braille spinner stands in wherever only text fits (a hover card) while a web spinner is chosen. */
const TEXT_FALLBACK: TuiSpinner = 'braille'

// Frames mirror the TUI spinners in crates/maitty-settings/src/config.rs.
const SPINNERS: Record<TuiSpinner, readonly string[]> = {
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
let style: TuiSpinner = TEXT_FALLBACK
let chosen: SpinnerStyle = TEXT_FALLBACK
const styleListeners = new Set<() => void>()
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
  if (next === chosen) return
  chosen = next
  for (const listener of styleListeners) listener()
  const textStyle = isWebSpinner(next) ? TEXT_FALLBACK : next
  if (textStyle === style) return
  style = textStyle
  frame = 0
  paint()
}

/** The spinner the user picked, for components that draw a web spinner or the text one. */
export function chosenSpinner(): SpinnerStyle {
  return chosen
}

export function subscribeSpinnerStyle(listener: () => void): () => void {
  styleListeners.add(listener)
  return () => { styleListeners.delete(listener) }
}

export function useSpinnerStyle(): SpinnerStyle {
  return useSyncExternalStore(subscribeSpinnerStyle, chosenSpinner)
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
export function spinnerDefinition(spinner: TuiSpinner): { frames: readonly string[]; intervalMs: number } {
  return { frames: SPINNERS[spinner], intervalMs: SPINNER_TICK_MS }
}
