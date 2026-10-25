# Intent: checks that Maitty runs itself

Type: feature · Status: shipped in v0.37.0 · Date: 2026-10-22

## Problem

"An agent saying 'done' turns its session done, even when the project's tests fail. You find out
later, or not at all." (design brief v6)

## Goal

A project may name one **check** command. When an agent's turn ends, Maitty runs it in the
session's worktree and records the exit code, the time, and the tail of the output. The session
reads done only if the check passed. A failure reads `failed: <check>`, shows in the Inbox and
keeps the session visible. The user can re-run a check and override a failed one; the override
is recorded and shows. A project with no check looks exactly as Maitty does today.

## Scope

Anyone running agents in a project with tests. The daemon (runs the check, holds the result,
delays finished alerts and waiters), the protocol and saved session, the sidebar row, the
why popover, a new evidence sheet, the Inbox, Settings, and the row menu. The design is in
[DESIGN](../../DESIGN.md) and [UI](../../UI.md).

## Decisions

- The daemon runs the check, never the client (ADR-0001). Maitty observed the result; an agent's
  claim never counts. The check's process does not inherit `MAITTY_PANE_TOKEN`.
- Checks never stop an agent working. They change what Maitty says about it.
- The command comes from the user's own config. A command supplied by a repo is a later,
  separate feature with its own approval.
- Every screen works in all three layouts (Cards floating, Cards tiled, Flat) at 80×24, 100×30
  and 160×45, with an ASCII glyph fallback. Existing theme colours only; no new theme roles.
- The sidebar keeps its `+ new` / `actions` foot and the Appearance spinner choice. No new
  leader keys; re-run and override are row-menu items and keys inside the evidence sheet.
- This is not a workflow runner (ROADMAP › Out, 2026-10-18): Maitty runs one command the user
  named and reports the result. It starts no role and assigns no task.
- A waiter (`maitty wait`, MCP `maitty_wait`) on a session with a check returns after the check
  settles (decided 2026-10-22).

## Out of scope

Stale detection (a result older than the files), a command supplied by the repo
(`.maitty/checks.toml`) and its approval, further checkpoint kinds (`commits`, `approval`,
`review`), the design's working shimmer and 3-bar spinner, the design's sidebar foot. Several of
these came later, in phase 18.

## Links

- Plan: [phase 15](../../ROADMAP.md#15--checks-maitty-runs-itself--shipped-in-v0370)
- Tickets: #94–#103
