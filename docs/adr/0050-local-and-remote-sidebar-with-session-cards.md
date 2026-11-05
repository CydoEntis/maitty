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
- **Projects stay.** Under each half (and under each machine's header in Remote) projects are rows
  that fold: `▾ maitty` with `⎇ branch` and a check mark (`✓`, or `✕` if a check failed) at the
  right. A folded project says how many sessions it hides, with an amber `●` when one needs you.
- **Two-row cards.** A session is its mark, name and age (`● you` when it needs you), then what it
  is doing. Its other tabs follow it as `○ shell  2`, grouped by a thin line.
- **Machines.** Local has no header. Each Remote machine has a header, a ` ⇄ build ` badge with its
  connection at the right. A machine that is not connected, or has no sessions, is that header and
  one line of state instead of its projects.
- **Removed from the row:** the agent's context percent, the clickable `+N −N` (Changes stays on
  leader `c`), the queued-follow-up line, the failing-check third line, and helper indentation.
- **Rounding has three values again.** `ui.rounded` is `full` (half-circle pills, a Nerd Font
  glyph), `corners` (rounded boxes with square, full-background chips) or `off`. An older `true`
  reads as `corners`; `full` is kept.
- **Close buttons** are the `✕` icon in the Unicode set (`x` in ASCII, the Nerd glyph in Nerd).
- **Toasts** are drawn on the plain ground with no fill of their own.
- **Pane and panels.** The focused session's machine is a badge on the first pane's top border.
  The Branch and References sheets no longer repeat the mode bar's key hints.

## Consequences

Sessions are found by machine, then project. The background count of uncommitted lines has no
reader left. The reference frames no longer
compare the sidebar's column; `screen::side` and `machine_group_tests` hold its tests. Half the
sidebar's height is held for Remote even when it is empty.
