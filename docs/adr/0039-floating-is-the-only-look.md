# 0039. Floating is the only look

- **Status:** Accepted
- **Date:** 2026-10-31
- **Amends:** [ADR 0026](0026-hairline-a-fifth-look.md), [ADR 0027](0027-two-looks-and-tabs-in-the-sidebar.md)
  and [ADR 0021](0021-one-line-rows-one-rounding-sticky-mode.md)

## Context

Two looks (Floating and Hairline) still meant every change to tabs, titles, popups and toasts was
made and checked twice, and Hairline's chips and settings rows had their own rules. The user asked
on 2026-10-31 to remove Hairline entirely, and to simplify the settings that went with it.

## Decision

- Floating is the only look. There is no Layout setting. An older file that names `hairline`,
  `compact` or `flat` (or `tiled`, `grid`) is read as Floating, says so once in a toast and is
  written back as `layout = "floating"` (`Ui::fold_retired_layout`).
- `ui.rounded` is on or off. On rounds card corners (`╭╮╰╯`); chips, buttons and rows have square
  ends. An older `full` or `corners` reads as on, `off` as off. The half-circle pill ends (a Nerd
  Font option) are gone.
- `ui.padding` and `ui.gap` are `small`, `medium` or `large` (one, two or three cells); there is no
  "top" padding. `ui.dim` is `off`, `light`, `normal` or `heavy`. Older values read as the nearest
  name (`Ui::fold_legacy_sizes`); a gap or padding of `0` in an older file is kept.
- Status glyphs, the working spinner, the cursor and its blink sit with the Theme in Settings; the
  Theme row shows the chosen theme's ground and accent as two squares.

## Consequences

One drawing path for every part of the screen. The tab row's split and zoom buttons (Hairline's)
are gone with it; both stay available as keys and from the Menu. Users who chose Hairline or full
rounding see the nearest Floating look.
