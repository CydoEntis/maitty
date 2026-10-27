# 0028. A tab row in both looks, and every popup drawn like a pane

- **Status:** Accepted
- **Date:** 2026-10-27
- **Amends:** [ADR 0013](0013-v7-frame.md) (tabs in the pane title) and
  [ADR 0027](0027-two-looks-and-tabs-in-the-sidebar.md)

## Context

With two looks left (ADR 0027), the user asked on 2026-10-27 that notifications, modals and
toasts match the look they are in, showing a Hairline toast whose coloured bar hung outside a
frame. Floating still drew the session's tabs in the top-left pane's title, while Hairline had
moved them to a row of their own; the two looks did tabs two ways and Floating's narrow-title
rules were the hardest code in the frame. Popovers and the toast were frameless filled blocks in
Floating (softened at the corners when rounded), boxed in Hairline but for the toast.

## Decision

- **Tabs:** both looks give the tabs a row of their own over the panes. Floating's is the tab
  chips on one row right over the boxes (no gap), over the panes only (the side sheet is a card
  of its own, full height); Hairline's stays as ADR 0026 has it. Every pane title is then its
  pane's own: its number and name in a split, and for a tab's only pane `project · ⎇ branch`,
  the tab naming it. A tab is named the same everywhere: its own name, else its focused
  session's. Each tab shows its most urgent agent's state, on screen or not (a tab out of view
  showed none before), and carries the ✕: it closes the tab, or a lone tab's only pane; a split's
  panes keep their own.
- **Popups:** anything that floats over the screen is boxed as the look boxes a pane: in
  Floating a `line` box round `card`, corners by `ui.rounded`; in Hairline a square `line` box
  on the ground. Popovers and menus get theirs a cell outside their body (`popover_outline`);
  modals already had it.
- **Toast:** the same box, its left side a heavy `┃` in the notice's colour inside the box, so
  the colour can't stand apart from the frame.

## Alternatives considered

- **Keep Floating's tabs in the title:** two ways to do tabs, and the narrow-title rules stay.
- **Frameless filled popups in both looks:** fills are what Hairline has none of.
- **A tab row over the side sheet too in Floating:** the sheet is a card of its own, and its
  title would sit a row lower than the sidebar's for no reason.

## Consequences

The reference frames drew the tabs in the pane's top border and the why popover without a box;
their conformance tests leave the pane column's top rows and the popover's ring to the app
(`reference_frames_tests::expected`), and `frame_tests` holds the tab row and title instead.
`card::soften_block` is gone with the last frameless block.
