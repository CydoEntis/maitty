/**
 * What Esc closes first, topmost to bottom (v18 C.9, B.8). Esc reaches the terminal only when none of
 * these is open, so closing an overlay never interrupts an agent. A new dialog adds its id here, at
 * the place it sits above the others, and reports whether it is open to `topmostOpen`.
 */
export const ESC_LADDER = [
  'comment-composer',
  'new-agent',
  'whats-new',
  'overview',
  'palette',
  'inbox',
  'settings',
  'drawer',
  'focus-mode',
] as const

export type EscLayer = (typeof ESC_LADDER)[number]

/** The topmost layer that is open, or null when Esc belongs to the terminal. */
export function topmostOpen(open: Readonly<Partial<Record<EscLayer, boolean>>>): EscLayer | null {
  return ESC_LADDER.find((layer) => open[layer] === true) ?? null
}
