export type SpinnerStyle = 'braille' | 'snake' | 'bars'

// Frames mirror the TUI spinners in crates/maitty-settings/src/config.rs.
const SPINNERS: Record<SpinnerStyle, { frames: string[]; intervalMs: number }> =
  {
    braille: {
      frames: ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
      intervalMs: 80,
    },
    snake: {
      frames: ['⠇', '⠋', '⠙', '⠸', '⢰', '⣠', '⣄', '⡆'],
      intervalMs: 90,
    },
    bars: {
      frames: ['⣰⡄', '⣼⡀', '⣶⡆', '⣧⡇', '⣆⡆', '⣤⡄', '⣸⡆', '⣴⡇', '⣇⡄', '⣦⡀'],
      intervalMs: 85,
    },
  }

const listeners = new Set<() => void>()
let style: SpinnerStyle = 'braille'
let frame = 0
let timer: ReturnType<typeof setInterval> | undefined

function tick() {
  frame = (frame + 1) % SPINNERS[style].frames.length
  listeners.forEach((notify) => {
    notify()
  })
}

function startTimer() {
  // One shared clock keeps a dozen working rows from each owning a timer.
  timer = setInterval(tick, SPINNERS[style].intervalMs)
}

export function setSpinnerStyle(next: SpinnerStyle) {
  if (next === style) return
  style = next
  frame = 0
  if (timer !== undefined) {
    clearInterval(timer)
    startTimer()
  }
  listeners.forEach((notify) => {
    notify()
  })
}

export function subscribe(listener: () => void) {
  listeners.add(listener)
  if (timer === undefined) startTimer()
  return () => {
    listeners.delete(listener)
    if (listeners.size === 0 && timer !== undefined) {
      clearInterval(timer)
      timer = undefined
    }
  }
}

export function currentFrame() {
  return frame
}

export function spinnerFrames() {
  return SPINNERS[style].frames
}

/** One style's frames and speed, for previews that animate a style other than the chosen one. */
export function spinnerDefinition(spinner: SpinnerStyle): { frames: readonly string[]; intervalMs: number } {
  return SPINNERS[spinner]
}
