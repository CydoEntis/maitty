# 0013. Draw the v7 frame: tabs in the pane title, no margin

- **Status:** Proposed
- **Date:** 2026-10-22

## Context

The v7 handoff (`docs/design/v7/`, brief v7) is a new layout for Seshi's checkpoints in
Cards floating, Cards tiled and Flat. Its frame differs from the one ADRs 0007 and 0012
left: no outer margin, no separate tab row, square or rounded borders instead of the
flush eighth-block edges, a 72-column side sheet, and modals and popovers with a hard
shadow. Every screen the other v7 packages build is drawn inside this frame, so it is
settled once, here.

## Decision

- **No outer margin.** The sidebar runs from row 0 to the last row at the screen's edge;
  the pane column starts at row 0. Floating leaves `ui.gap` between cards, tiled none (the
  two borders sit side by side, never merged into ┬/┴), Flat starts the panes on the
  sidebar's last column so one `│` parts them.
- **Tabs in the pane title.** The separate tab row is removed. The top-left pane of the tab
  carries the session's tabs as chips (number and name); other panes show one chip with
  their session's name. The right of every pane's title is the session's mark (glyph and
  word) and a ✕.
- **Corners are square or rounded.** `ui.corners` is `square` (the default; `flush` reads
  as square) or `rounded`; the flush eighth-block edges are no longer drawn.
- **The pane status line and the title's sub text are removed.** The branch is in the
  sidebar's folder header.
- **A sheet is 72 columns wide** beside the panes from 140 columns; below that it takes the
  pane column. The 96-column wide sheet is removed.
- **Modals and popovers cast a hard shadow.** A modal has an accent title row with
  `Esc close`; a popover has a raised title row and no border and sits beside the sidebar.
- **Hover is one post-pass.** The topmost clickable region under the pointer is tinted once
  after drawing; widgets no longer tint themselves.

## Alternatives considered

- **Keep the tab row above the panes:** costs a row at every size and repeats what the
  pane title can carry; the handoff has none.
- **Keep flush edges as an option:** a third border style to test in every layout, for a
  look the handoff dropped.
- **Keep the 96-column sheet at 200+ columns:** the handoff fixes the sheet at 72 so its
  content lays out the same at every width.

## Consequences

The frame is the same in all three layouts, tested against the handoff's reference
frames (`client/screen/v7_frame_tests.rs`). What the agent is on and where it runs are no
longer on the pane itself; the sidebar and the why popover carry them. Revisit if the
pane title needs more than tabs and the mark.
