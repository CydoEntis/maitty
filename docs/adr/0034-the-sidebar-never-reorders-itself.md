# 0034. The sidebar never reorders itself, and has no SSH group

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Sessions that needed you went to the top of their group, and their worktree and project went to
the top with them, so rows moved as agents asked or finished and were hard to find again. A
session that ssh'd into a machine moved out of its folder into a group of its own under an `SSH`
heading, the same kind of jump.

## Decision

- **A row stays where it is.** Sessions keep the order they were first listed in (or the order you
  dragged them to), worktrees go main first then by name, and projects keep the order they were
  first seen in. Needing you, working and finishing change a row's mark and colour, never its
  place. The Inbox still lists what needs you.
- **No SSH group.** A session on another machine stays with the folder it was started from and is
  named by the machine unless you named it.
- The `ui.attention_sort` setting goes (Settings › Sessions › Sort sidebar by attention). An older
  config that still has the key loads; the key does nothing.
- A helper always sits under the session that started it, whatever its state.

## Consequences

The reference frames keep showing the project that asks a question first; the fixture world lists
its projects in that order (`reference_fixture::world`) rather than the app sorting them.
