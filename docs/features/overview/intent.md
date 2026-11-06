# Intent: one screen for every agent, and the key that takes you to the next one

Type: feature
Author: Cody · Status: proposed · Date: 2026-11-04
Plan: Plane epic to be filed (see [spec.md](spec.md) for the tickets)

## Problem

"See which ones need you, jump to them." The sidebar does this one project at a
time. With several projects and several machines there is no single place that
shows every agent at once, and no one key that goes to the next one waiting.
Workbench (Matt Birchler) ships exactly this as its Overview and ⌘J, and it is the
closest thing to maitty's mission statement as a screen.

## Proposed outcome

An **Overview** screen in the desktop app: every session, on every machine, in
one grid, led by the ones that need you. A **jump** key that goes to the next
session needing you from anywhere. A **New agent on a branch** button that does
in one click what the TUI's new-agent box does. Updates that install themselves
when the app has been idle.

## Affected users and systems

Desktop app users with more than one project or machine. `apps/gui` only, on top
of the daemon's existing status, machine and worktree APIs. The TUI already has
the equivalents and is unchanged.

## Constraints

- Status comes only from the daemon (ADR 0006). The grid never derives it.
- Sidebar order rules hold: rows keep creation order; groups never reorder on
  status changes (v16 §1). The grid is ordered by *need*, then creation.
- Sessions are never hidden automatically (guide: "Sessions stay visible").
- Updates never restart the app while any agent is working.
