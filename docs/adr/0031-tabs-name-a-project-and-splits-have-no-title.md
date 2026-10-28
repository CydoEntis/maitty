# 0031. The tab row lists the project's tabs, and a split has no title

- **Status:** Accepted, the tab row amended by 0032
- **Date:** 2026-10-28
- **Amends:** [ADR 0030](0030-panes-titled-like-an-editor.md) (pane titles),
  [ADR 0028](0028-tab-rows-and-popups-drawn-like-panes.md) (the tab row) and
  [ADR 0027](0027-two-looks-and-tabs-in-the-sidebar.md) (a session's tab lines)

## Context

A design handoff (projects → tabs → splits) found each terminal named three times: in the
sidebar, on the tab row and in its pane's title, with tab numbers that did not match pane
numbers. Its rule: a terminal is named once, a tab is listed in the sidebar and the tab row, and
a split has no title.

## Decision

- **Splits have no title** in either look. A split's top edge says its state: Hairline draws a
  `─` rule along its top, Floating its border; teal on the focused pane (sky in shortcut mode),
  amber on one that needs you, else `line`. Focus wins over amber. The load note moves to the
  bottom bar.
- **The tab row lists every tab of the current project**, numbered 1–n in the sidebar's order
  (each session's tabs together); the number keys (`Alt+1`–`9`, leader `1`–`9`) and next/previous
  tab walk the same list. Each tab shows its agent's state, its name and, on the open tab only,
  a ✕. (A split-shape icon per tab was tried and removed: it did nothing.)
- **`◫` and `⤢`** sit fixed at the row's right end: split the focused pane, and zoom it to fill
  the tab (and back). Zoom belongs to the tab and is not saved.
- **Sidebar tab lines** have no numbers and use a session row's look: state glyph, name, then the
  agent's age; the open one raised with a teal `▎`, one
  that needs you with an amber `▎` and name.
- **The bottom bar** after `TYPING` reads `tab 1/3 · 2 splits`, and at the right end shows
  `● N needs you` (the session's name when it is one), the tab's `⎇ branch` and its `▲ CPU · RAM`.
  Needs you keeps its place before the hints; the branch and load show only while the rest fits.

- **Floating: one top edge.** The sidebar's top edge and the pane box's are one row. A tab's
  only pane has the tab row as the first row inside its border; split panes get a plate (top
  edge and the tab row) above their boxes.
- **Hairline: where the keys are.** The line under the sidebar's header and the one under the
  tab row sit on one row; whichever has the keys is teal (sky in shortcut mode). A split's own
  rule says it for a pane in a split.

## Not taken from the handoff

- The sidebar keeps a session's second and third lines (a question, a check result), since a
  failed check shown there is shipped behaviour (phase 15).
- The sidebar stays 34 columns at 140 and wider, not 36.
- The handoff's Ctrl+1–9 is Alt+1–9 here (ADR 0029).

## Consequences

A pane has no ✕ or number on screen; a close confirm still names it. Tests that read a split's
title now read its edge. The sidebar's `ui.sidebar_tabs` setting still decides which of a
session's tabs get a line.
