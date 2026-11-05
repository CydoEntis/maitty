# 0050. A Local and Remote sidebar of session cards, with real pills as an option

- **Status:** Accepted
- **Date:** 2026-11-04
- **Amends:** [ADR 0039](0039-floating-is-the-only-look.md) (rounding) and
  [ADR 0021](0021-one-line-rows-one-rounding-sticky-mode.md) (one-line rows)

## Context

The user supplied a design handoff for the terminal UI on 2026-11-04. The sidebar was one list of
project folders, each with one-line session rows, and a machine header whenever a second machine
was known. The handoff splits it by where a session runs and drops the folders. It also asks for
notifications without a fill, and for pills to stay an option beside plain rounded boxes.

## Decision

- **Local and Remote.** The sidebar's top half is `LOCAL` (this computer) and its bottom half is
  `REMOTE n` (other machines), on a `├ … ┤` rule that joins the card's sides, with a `+ add` chip
  that opens the Machines dialog. Each half scrolls on its own. With no other machine, Remote says
  so and offers to add one.
- **Cards, not folders.** Every session is a three-row card: its mark, name and age (`● you` when
  it needs you); a badge naming its machine (`⌂ local`, `⇄ build`) with its folder and branch; and
  what it is doing. Its other tabs sit under it as `├ shell 2`. There are no folder rows, so
  nothing folds. A machine that is not connected, or has no sessions, is one two-row item and its
  cards are not drawn.
- **Removed from the row:** the agent's context percent, the clickable `+N −N` (Changes stays on
  leader `c`), the queued-follow-up line, the failing-check third line, and helper indentation.
- **Rounding has three values again.** `ui.rounded` is `full` (half-circle pills, a Nerd Font
  glyph), `corners` (rounded boxes with square, full-background chips; the handoff's look) or
  `off`. An older `true` reads as `corners`; `full` is kept.
- **Toasts** are drawn on the plain ground with no fill of their own.
- **Pane and panels.** The focused session's machine is a badge on the first pane's top border.
  The Branch and References sheets no longer repeat their keys; the mode bar shows them.

## Consequences

Sessions are found by machine and card, not by project. A project's sessions are no longer grouped,
and the background count of uncommitted lines has no reader left. The reference frames no longer
compare the sidebar's column; `screen::side` and `machine_group_tests` hold its tests. Half the
sidebar's height is held for Remote even when it is empty.
