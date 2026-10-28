# 0033. The collapsed sidebar is a narrow column, not a strip of chips

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Collapsing the sidebar (leader `z`) put every session as a chip along the top row, which cost
the panes a row and read as a second tab row above the real one, so sessions looked like tabs.

## Decision

- Collapsed, the sidebar stays a column at the side, 5 columns wide. Each project is its first
  letter in the project's colour, and under it each session is its state mark and the first letter
  of its name. The panes keep the top row.
- A `»` (`«`) chip at the top expands it; `+` and `≡` at the foot start a shell and open the Menu.
  Rows that don't fit scroll, with a `…` in the last place that opens the sidebar.
- Keys stay as they were, with `↑` `↓` added beside `←` `→` and `1`-`9`.
- Hiding the sidebar (`Z`) is unchanged: no column, the panes take everything.

## Consequences

`grid` reserves the column's width whenever the sidebar is not hidden, so collapsing no longer
moves the panes down a row. A session's full name is not shown while collapsed; expand to read it.
