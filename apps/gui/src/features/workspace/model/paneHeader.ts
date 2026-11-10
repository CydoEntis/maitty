/**
 * The pane widths, in px, below which the header gives up a control. In order as the pane narrows:
 * Split right goes, then the terminal | chat switch. Whatever is gone stays in the ⋯ menu; the
 * side-panel button and the title always keep their room.
 */
export const SPLIT_MIN_PX = 440
export const SWITCHER_MIN_PX = 360

export type HeaderFit = { split: boolean; switcher: boolean }

/** What the header shows at a pane width; everything while the width is not measured yet. */
export function headerFit(width: number | null): HeaderFit {
  if (width === null) return { split: true, switcher: true }
  return { split: width >= SPLIT_MIN_PX, switcher: width >= SWITCHER_MIN_PX }
}
