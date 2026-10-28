# 0032. A session owns its tabs, and picking it shows it

- **Status:** Accepted
- **Date:** 2026-10-28
- **Amends:** [ADR 0031](0031-tabs-name-a-project-and-splits-have-no-title.md) (the tab row)

## Context

ADR 0031 made the tab row list every tab of the current project. Sessions that are separate
rows in the sidebar then all appeared as tabs of one another, so a new session could not be told
from a tab, and the number keys walked across sessions. Picking a session's row also showed the
tab you were last on in it, often one of its shells, while the row said the session was open.

## Decision

- **The tab row lists the tabs of the session you are in**, numbered 1-n, and nothing else. The
  number keys and next/previous tab walk the same list. Another session in the project is
  reached from the sidebar, never from the tab row.
- **A new session is its own row and its own tab row.** A tab is made only by asking for one
  (leader `t`, the `+` chip), and it takes only the shell made for that request.
- **Picking a session shows the session**: its own tab, not the tab you were last on in it.
  Its other tabs are lines under its row, and a click on one shows that tab. A focus change
  that is not a pick (a notification, a finished agent) still returns to the tab you were last on.
- Everything else in ADR 0031 stands: a split has no title, `◫` and `⤢`, the bottom bar.

## Consequences

The tab row is a session's, as ADR 0027 had it. `App::session_tabs` is the one list the tab row,
the bottom bar and the number keys use.
