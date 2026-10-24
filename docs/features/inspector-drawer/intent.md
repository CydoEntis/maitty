# Intent: the inspector drawer

Type: change + feature
Author: Cody · Status: accepted
Date: 2026-10-24 · Plan: [phase 26](../../ROADMAP.md#26--the-inspector-drawer--planned-2026-10-24)

## Problem

"I like the file tree, branches, git diff stuff that comes out from a native resizable drawer."
Files opens as a popup over the agent, the branch switcher is another popup, and the side panel
(Changes, Inbox, checks) is a fixed 72 columns.

## Proposed outcome

One right-hand panel holds Files, Changes and Branch next to the agent, pushing the panes aside
rather than covering them. You drag its edge or press `[` `]` to size it, and each view remembers
its width. Markdown files read pretty, with raw a key away. Ctrl+click on a `path:line` in agent
output lands in it, and a row's `+N −N` opens its Changes. Spec: [spec.md](spec.md).

## Affected users and systems

Anyone reviewing what an agent did. The client only: the side panel (`sheet.rs`), the screen
grid, Files, Changes, the branch view, saved UI state. No protocol change.

## Constraints

- It grows the existing side panel (decided 2026-10-24); no second right-hand surface. Not a
  dock layout (ROADMAP › Out, 2026-10-03).
- The sidebar keeps its own state when the panel opens (decided 2026-10-24).
- Keys: `f` Files, `d` Changes, `b` Branch as today; `[` `]` `=` only while the panel has the keys.
- Git and file reads run in background tasks (AGENTS non-negotiable 6).
- Markdown uses the pulldown-cmark crate (decided 2026-10-24).
- Every view works in Floating, Tiled, Compact and Grid at 80×24, 100×30 and 160×45.

## Out

- PR status in Branch (no PR data yet). A second drawer. Folding the sidebar automatically.
- Images shown in the terminal (terminal graphics were cut in phase 7).

## Open questions

None.

## Tickets

[#172](https://github.com/CydoEntis/maitty/issues/172), [#173](https://github.com/CydoEntis/maitty/issues/173), [#174](https://github.com/CydoEntis/maitty/issues/174), [#175](https://github.com/CydoEntis/maitty/issues/175), [#176](https://github.com/CydoEntis/maitty/issues/176)
