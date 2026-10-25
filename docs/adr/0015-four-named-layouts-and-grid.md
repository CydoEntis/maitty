# 0015. Four named layouts, one setting, and the Grid style

- **Status:** Accepted (amended 2026-10-24, see *Amendments*)
- **Date:** 2026-10-23

## Context

The look was chosen by two settings with names that no longer said what they do:
`ui.layout` (`cards` / `flat`) and, for Cards only, `ui.panes` (`floating` / `tiled`).
The user supplied a third design, "Windows" (`design_handoff_seshi_windows`): each pane a
coloured, numbered window. Built in full it would also bring a right-hand sidebar of
terminals, files and agents, a top bar of workspace chips, a fixed 2×2 grid, zoom, a
scratch shell and bare-letter keys, which the plan's Out list (other structural layouts,
2026-10-03) and ADR 0014 (no bare letter acts) rule out. The user chose the window look only,
and asked for clearer names that also fit it.

## Decision

One setting, `ui.layout`, picks one of four looks over the same session layout (ADR 0004):
**Floating** (cards with gaps; Cards floating before), **Tiled** (cards packed edge to edge;
Cards tiled), **Compact** (full-bleed zones, no boxes; Flat, ADR 0012) and **Grid** (new).
`ui.corners` (square or rounded) applies to every look that draws boxes: Floating, Tiled and
Grid. Floating stays the default. Older values keep loading: `cards` with `panes = "tiled"`
reads as Tiled, other `cards` as Floating, and `flat` as Compact. `ui.panes` is read only for
that.

Grid is a rendering style, like Compact: each pane is a box in its own colour, chosen by its
position in the tab (green, pink, violet, cyan, then again) from colours every theme has
(`Theme::grid_colors`: `done`, `accent`, `violet()`, `cyan()`, skipping any too like one
already taken, then the theme's other colours). The focused pane's border is that colour, bold; the
others' is the colour faded toward the ground. The top border carries the pane's number and
its state on the left and its name as a tag on the right. The sidebar, tabs, splits, keys
and sheets are unchanged.

## Alternatives considered

- **The full Windows design:** a second session model (sidebar, top bar, grid, zoom,
  scratch) and keys taken from the agent pane; it reverses two recorded decisions for a look.
- **Keep two settings and add a third value to `ui.layout`:** the names stay unclear, and
  `panes` would mean nothing in two of four looks.
- **The design's fixed colours:** clash with light themes and skip the contrast audit.

## Consequences

Settings shows one Layout choice. Internally the code keeps `Look::flat` for Compact.
A pane's colour in Grid follows where it sits, so it changes when a split closes. The
scratch shell, zoom and the right-hand sidebar stay out; they would come back only through a
new proposal. Supersedes the names in ADR 0007 and ADR 0012, not their rules.

## Amendments

- **2026-10-24:** a Grid pane's top border no longer shows its state's words or a name tag on
  the right; the user found both repeated what the title already said. The state shows only as
  its glyph inside the chip that names the pane (the current tab on the top-left pane). The
  rules live in [`docs/UI.md`](../UI.md) (the Grid paragraph and *Pane title*).
