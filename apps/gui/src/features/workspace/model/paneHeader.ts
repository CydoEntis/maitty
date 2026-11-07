/**
 * The pane widths, in px, below which the header gives up a control. In order as the pane narrows:
 * Files and Changes lose their labels, the split buttons go, Files and Changes go, then the
 * terminal | chat switch. Whatever is gone stays in the ⋯ menu, and the title always keeps its room.
 */
export const TOOL_LABELS_MIN_PX = 660
export const SPLITS_MIN_PX = 600
export const TOOLS_MIN_PX = 520
export const SWITCHER_MIN_PX = 420

export type HeaderFit = { toolLabels: boolean; splits: boolean; tools: boolean; switcher: boolean }

/** What the header shows at a pane width; everything while the width is not measured yet. */
export function headerFit(width: number | null): HeaderFit {
  if (width === null) return { toolLabels: true, splits: true, tools: true, switcher: true }
  return { toolLabels: width >= TOOL_LABELS_MIN_PX, splits: width >= SPLITS_MIN_PX, tools: width >= TOOLS_MIN_PX, switcher: width >= SWITCHER_MIN_PX }
}
