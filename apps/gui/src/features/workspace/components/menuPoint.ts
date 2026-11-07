/** A menu's width, so it is kept inside the window. */
export const MENU_WIDTH_PX = 244
/** The gap a menu keeps from the window's edge and from the button it opens under. */
export const EDGE_PX = 8

/** Where a menu opens: at the pointer for a right-click, under the button for a ⋯ click. */
export function menuPoint(event: { clientX: number; clientY: number; currentTarget: Element }, under: boolean): { x: number; y: number } {
  if (!under) return { x: event.clientX, y: event.clientY }
  const box = event.currentTarget.getBoundingClientRect()
  return { x: box.right - MENU_WIDTH_PX, y: box.bottom + EDGE_PX }
}
