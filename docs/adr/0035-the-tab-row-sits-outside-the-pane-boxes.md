# 0035. The tab row sits over the pane boxes, not inside them

- **Status:** Accepted
- **Date:** 2026-10-28
- **Amends:** [ADR 0028](0028-tab-rows-and-popups-drawn-like-panes.md) and
  [ADR 0031](0031-tabs-name-a-project-and-splits-have-no-title.md) (where Floating draws the row)

## Context

Floating drew the tab row inside the first pane's box: the first row inside the border for a
tab's only pane, a two-row plate over split panes. The user asked on 2026-10-28 for the tabs
to sit in the header, outside the pane's frame, as Hairline's already do. The tabs name the
whole tab, not the pane under them, so they should not read as part of one pane's box.

## Decision

- **Floating's tab row** is a plain row at the top of the pane column, level with the sidebar's
  top edge, with the pane boxes starting on the row under it. The same for a lone pane and for
  splits: no plate, and no tabs inside a border.
- **Contents are unchanged:** the chips, the `+`, `◫` and `⤢`, and the click and drag rules.
- **Hairline is unchanged:** its row over a rule stays as ADR 0026 has it.

## Alternatives considered

- **A full-width header over the sidebar too:** the sidebar's title and the session tabs would
  fight for one row, and a sheet docked at the right has its own header.
- **Keep the plate over splits only:** two ways to draw one row.

## Consequences

A split's panes lose the plate's extra row, so they gain a row; a lone pane has the same height
as before. A pane box's top edge is one row under the sidebar's. `frame_tests` and the
screen tests read the tabs on row 0 and the box's border on row 1.
