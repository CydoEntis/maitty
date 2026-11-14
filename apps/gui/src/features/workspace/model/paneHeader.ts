/**
 * The pane width, in px, below which the header gives up the Chat · Terminal tabs. What is gone stays in the ⋯ menu;
 * the title always keeps its room.
 */
export const SWITCHER_MIN_PX = 360

export type HeaderFit = { switcher: boolean }

/** What the header shows at a pane width; everything while the width is not measured yet. */
export function headerFit(width: number | null): HeaderFit {
  return { switcher: width === null || width >= SWITCHER_MIN_PX }
}
