# 0038. Floating's tabs sit on the first pane's top border

- **Status:** Proposed
- **Date:** 2026-10-30
- **Amends:** [ADR 0035](0035-the-tab-row-sits-outside-the-pane-boxes.md) (where Floating draws the
  tab row)

## Context

ADR 0035 moved Floating's tabs out of the pane's box into a row of their own over it. The sidebar's
`SESSIONS` chip sits on its box's top border; the tabs sitting above the pane box read as a second,
separate header. The user asked on 2026-10-30 for the tabs to sit on the pane's border the way the
sidebar's title does, keeping the solid chip design.

## Decision

Floating's tab chips are drawn on the top border of the first pane (the topmost, leftmost), a column in
from the corner like every title chip, and the tab row of their own is gone. The border runs on between
the chips and after them, and a cell of it is left clear between one tab and the next. The split and zoom tools sit at the right end of the pane column's top border.
The pane box now starts on the sidebar's top row, so the sidebar's title and the tabs share a row.

- **Contents are unchanged:** the chips, their marks, numbers and ✕, the `+`, `◫` and `⤢`, and every
  click and drag rule.
- **Splits:** the chips are on the first pane's border; the other panes' borders carry no words
  (ADR 0031).
- **The sheets' tabs** (Files, Changes, Branch, References) are chips on the sheet's top border the same way: a key chip then the name, the open one in the accent, a cell of border between them. A sheet's title, if it has room, goes to the right of the border; a context menu is the Menu's card, its title a chip on a lit border.
- **Hairline is unchanged:** its tab row over a rule stays as ADR 0026 has it.

- **Options:** `ui.tab_gap` (on by default; Settings > Appearance > Tab gap) puts a cell of border between one tab and the next, on the panes and on a sheet's tabs; off, they touch. The chips' shape follows `ui.rounded`: `off` square, `corners` rounded (square ends), `full` pill (caps). The red close button on a sheet's or modal's border follows it too, and is flush against the corner.

## Alternatives considered

- **Keep the row over the pane:** the user found it a second header beside the sidebar's.
- **A tab strip inside the box under its border:** it costs a row and reads as part of the terminal.

## Consequences

One row more for the panes. The tabs have `title_x(first pane)` to the tools to fit in, two columns
less than the row had, so a crowded tab row gives way sooner. Tests that put the tabs in a row over the
box now look for them on the border.
