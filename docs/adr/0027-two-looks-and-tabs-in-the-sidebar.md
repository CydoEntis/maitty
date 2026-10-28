# 0027. Two looks, pane colours, and a session's tabs in the sidebar

- **Status:** Accepted
- **Date:** 2026-10-27
- **Amends:** [ADR 0015](0015-four-named-layouts-and-grid.md) and
  [ADR 0026](0026-hairline-a-fifth-look.md)

## Context

Five looks (Floating, Tiled, Compact, Grid and Hairline) each had their own frame, title and
sidebar code, and every change to tabs, titles or toasts had to be made and checked five times.
The user found it too much to keep them linked up and asked on 2026-10-27 to cut them down. They
kept Grid's one idea, each pane in its own colour, as an option, and Floating's rounding choices.

They also found that a session's second tab was invisible from the sidebar: a session with tabs
was one row, showing only its most urgent tab's state, so a tab working out of view could not be
told from an idle one.

## Decision

Two looks: **Floating** (a box per pane) and **Hairline** (lines only). `ui.layout` takes
`floating` or `hairline`.

- **Pane colours** (`ui.pane_colors`, off by default) gives each pane in Floating a border in its
  own colour by its place in the tab, as Grid did (`Theme::grid_colors`; the focused one full and
  bold, the others faded 42% toward `bg`). The title stays Floating's.
- `ui.rounded` (off, corners, full) and `ui.gap` stay as they were for Floating.
- An older file's look is kept as near as the two allow, without rewriting the file
  (`Ui::fold_retired_layout`): Tiled (and Cards with `panes = "tiled"`) is Floating with
  `gap = 0`, Grid is Floating with pane colours, Compact (once Flat) is Hairline.

A session with two tabs or more lists them under its row in both sidebars, one line each,
flush with the name and with no branch marks: the tab's number and name (the tab on screen in the accent, or teal in Hairline), and
its most urgent agent's glyph and word at the right (`side_tabs.rs`). A click on a line shows
that tab. A session with one tab stays one row.

## Alternatives considered

- **Keep all five and only align them:** each change still lands five times.
- **Floating, Grid and Hairline:** Grid's titles were a third title style for one colour idea.
- **Hairline only:** the simplest code, but drops boxes for everyone who likes them.
- **One mark per tab on the session row:** no extra lines, but the marks say nothing about which
  tab is which.

## Consequences

Grid's title (the state glyph inside the name chip, no words) and Compact's header strip and
rules are gone, with their tests; the reference frames' Flat blocks are skipped. The website's
`layout-tiled`, `layout-compact` and `layout-grid` frames are replaced by `layout-hairline` and
`layout-colours`. The sidebar's tab lines are click targets for `TabPick`, so dragging one onto
another of the session's tab lines reorders them, and a middle click closes one, as on the tab
chips.
