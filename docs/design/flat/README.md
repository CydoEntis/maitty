# Handoff: Seshi flat layout (OpenCode-like)

## Overview
An alternative layout for Seshi that users can pick: **Settings › Appearance › Layout › cards / flat**. It carries the same features, states and keys as the v4 handoff (`design_handoff_seshi_v4`). Only the chrome changes: no cards, borders, gaps or rounded ends. The look is full-bleed tinted zones with square ends, in the spirit of OpenCode, using Seshi's own colour roles.

## About the design files
These are **HTML design references**: an exact cell grid, one character per cell, 24-bit fg/bg, bold/italic. They are not production code. Rebuild them in Seshi's TUI stack.

**`seshi-app.js` is the spec.** The flat layout is a switch (`FLAT`, set from `state.flat`). Search for `FLAT` to find every place it changes. The functions that matter are `frame`, `layout`, `sidebar`, `tabs`, `sheet`, `statusLine`, `pane` and `panes`.

Open `Seshi Flat.dc.html` in a browser, with all files in one folder:
- It opens in flat.
- Click the grid, then use the keyboard. Keys are in the v4 README (`j m T Q P S C i l u U O ,`); in addition, **Tab** moves the focus zone between the sidebar and the pane.
- **Size** switches between 100×30, 160×45 and 240×60.
- **Jump to** opens any state.
- **Layout: flat / cards** compares with the cards layout.

`Seshi Options.dc.html` (row 3) shows the flat screens next to the others.

## Fidelity
**High fidelity.** Glyphs, copy, spacing and colour roles are final.

---

## Layout (flat)
```
row 0           tab bar (full width of the pane column) · right end: leader pill
rows 1..H-2     sidebar slab │ pane(s)                    │ sheet (full height, when open)
row H-1         status line (full width)
```
- **No outer margin, no gaps.** Zones touch. Their different backgrounds separate them.
- **Sidebar slab:**
  - 34 columns wide (26 below 140 columns, 40 at 200 and up).
  - Full height, from row 0 to H−2.
  - Same background as pane cards in the cards layout (`PB`).
- **Terminal ground:** the darkest tone, `PANEBG = mix(bg, #000, 0.4)`.
  - The terminal is darkest so programs' own colours sit on a neutral, dark ground.
  - Program output is not tinted.
- **Pane title:** on the first row inside the pane, at column 3, with the state tag right-aligned (`⠹ working   ✕`). It has no border and no pill.
- **Splits:** only at 150+ columns in the pane column, and only when there is no sheet. The two panes touch; there is no gap or divider.
- **Sheet (Inbox, Changes, PR, Server, Tickets, Checkpoints):**
  - It docks on the right, from row 0 to H−2: a **full-height rail**, 72 columns at 160 and 96 at 240+. Below 140 columns it covers the pane column.
  - It sits on the `card` tone, its title on the first inside row, with a status-bar line at the bottom.
  - The bottom status bar is a square, full-width tinted line instead of a pill.
- **Modals (Ship, Roll back, Settings, key map):** unchanged: centred, over the dimmed screen. In flat they use square-ended tinted blocks with no outline.

## Focus: how you can tell where you are
The focus is a **zone** (`state.zone`): `side` or `pane`. Tab toggles it; Enter on a session moves to the pane; Esc returns to the sidebar; clicking a row or a pane sets it.
- **Focused zone:** a left-edge bar `▎` in `acc` (lime) down the whole zone.
  - When the leader is armed, the pane's bar turns sky.
  - When the pane needs you, its tag is amber, but the bar stays lime or sky.
- **Unfocused zone:** the bar is `line` grey. Programs don't dim in flat; the bar and the status line are what tell you.
- **Selected sidebar row:** a full-width `hov` block with a `▎` at its left edge.
  - Lime when the sidebar is focused, `dim` when the pane is.
  - Amber if that session needs you.
- **Status line, left end:** a square ` SIDEBAR ` or ` PANE ` badge in `acc` on `accInk`. This is the explicit answer to "where am I", and it works even when colours are muted.

## Tab bar
- It spans the pane column on the `card` tone, at row 0.
- Tabs are **square blocks with no gap between them**: the current tab is `acc` on `accInk` text, the others `PB` with `text`.
- The ends are `█` instead of the rounded Nerd Font caps. Use a plain space with the block background if `█` isn't wanted.
- Contents are the same as in cards: ` n name glyph `, then `+`.
- At the right end, the armed leader shows as a sky block: ` ⌨ CTRL+SPACE  press a key · ? keys · Esc ` (just the first part below 140 columns).

## Status line (new, flat only)
One row at the bottom, on the `card` tone:
- **Left:** the zone badge, then `~\code\folder   ⎇ branch` and `#PR ✓/✕` for the selected session. Just `~` on first run.
- **Right:** `● n needs you` in amber (only when n > 0), then `j inbox   ctrl+space leader   ? keys`. Keys are `text` bold, labels `dim`.
- This replaces the two things the cards layout shows on the sidebar footer (`a actions`, the cog). Settings (`,`) and Actions (`a`) stay on the keys and the key map.

## Rounded things in cards → square in flat
| Element | Cards | Flat |
|---|---|---|
| Card | `╭╮╰╯` rounded border | none; tinted zone |
| Pane title | in the border | first inside row |
| Tab | pill with `\ue0b6` / `\ue0b4` | block with `█` ends |
| Selected row | pill | square block + `▎` bar |
| Buttons | pills | square blocks |
| Answer bar, inputs | pills | square tinted blocks |
| Text area (follow-up) | rounded box | tinted block, bar on the left, `↑ n more` and word count on the top line |

## Unchanged from v4
The features, states, keys, copy, colour roles, text-area behaviour and everything else in `design_handoff_seshi_v4/README.md`, including the new-key list and the open sheet/modal and line/soft options. `soft` only applies to cards.

## Not covered
- The pull-up toast may still have slightly rounded ends; square it to match.
- Light theme and the 12 themes were not checked; run the contrast audit on `PANEBG` and the zone bar.
- Files, Find, Branches, the new-agent dialog and notification history keep their 0.15 design.

## Files
- `seshi-app.js`: the spec (state, actions, render, presets)
- `Seshi Flat.dc.html`: the interactive viewer, opening in flat
- `Seshi Options.dc.html`: side-by-side comparisons
- `seshi-grid.js`: the cell-grid renderer
- `seshi-tokens.js`: the Seshi Night colour roles
- `support.js`: runtime for the HTML viewer only
