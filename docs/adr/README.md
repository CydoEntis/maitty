# Architecture decision records

An ADR records a decision that is expensive to reverse, along with the context
that made it the right call when it was made. Examples include the choice of
framework, persistence engine, auth model, architecture style, or module
boundary.

Don't write ADRs for ordinary implementation details. Those belong in code,
tests, and commit messages.

## How to add one

1. Copy [`0000-template.md`](0000-template.md) to `NNNN-short-title.md`, using
   the next number.
2. Fill it in. Set the status to `Proposed` until the user accepts it.
3. Add a row to the index below.
4. Update the project-specific section of the doc that owns the topic, with a
   one-line summary and a link.

Never rewrite an accepted ADR to change its decision. Write a new ADR that
supersedes it, and set the old one's status to `Superseded by NNNN`.

## Index

| # | Title | Status |
| --- | --- | --- |
| [0001](0001-daemon-owns-sessions.md) | Run sessions in a daemon, with a thin client | Accepted |
| [0002](0002-portable-pty-and-vt100.md) | Use portable-pty (ConPTY) and vt100 for terminals | Accepted |
| [0003](0003-msgpack-over-local-sockets.md) | Msgpack frames over local sockets, versioned | Accepted, amended 2026-10-24 |
| [0004](0004-one-layout.md) | One UI layout: the hydra layout | Superseded by 0007 |
| [0005](0005-herdr-compatible-keys.md) | Leader-key bindings follow herdr where they overlap | Superseded by 0006 |
| [0006](0006-floating-design-keys.md) | Leader keys follow the floating design's key map | Superseded by 0009 |
| [0007](0007-floating-and-tiled-styles.md) | One layout, drawn floating or tiled | Superseded by 0012 |
| [0008](0008-rename-to-seshi.md) | The app is called Seshi | Accepted |
| [0009](0009-leader-keys-after-the-cut.md) | Leader keys: a shell is one key, and only kept features have keys | Accepted |
| [0010](0010-optional-desktop-client.md) | Offer an optional desktop client over the shared session engine | Superseded by 0011 |
| [0011](0011-keep-seshi-terminal-first.md) | Keep Seshi terminal-first | Accepted |
| [0012](0012-flat-rendering-style.md) | Add a flat rendering style | Accepted |
| [0013](0013-v7-frame.md) | Draw the v7 frame: tabs in the pane title, no margin | Proposed, tabs in the title superseded by 0028 |
| [0014](0014-v8-cleanup.md) | Bring over the v8 cleanup, keeping what the user already decided | Accepted, its no-footer rule amended by 0024 |
| [0015](0015-four-named-layouts-and-grid.md) | Four named layouts, one setting, and the Grid style | Accepted, amended 2026-10-24, superseded in part by 0027 |
| [0016](0016-sticky-shortcut-mode.md) | The leader opens a shortcut mode that stays until you leave it | Accepted, amended by 0021, 0023 and 0025 |
| [0017](0017-keys-other-apps-use.md) | Default shortcuts: one key each, no Shift, keys other apps use | Accepted, amended by 0022 |
| [0018](0018-rename-to-vibecasso.md) | The app is called Vibecasso | Accepted |
| [0019](0019-the-name-lives-in-one-place.md) | The app's name lives in one place | Accepted |
| [0020](0020-rename-to-maitty.md) | The app is called Maitty | Accepted |
| [0021](0021-one-line-rows-one-rounding-sticky-mode.md) | One-line session rows, one rounding setting, shortcut mode that stays | Accepted, amended by 0023 and 0025 |
| [0022](0022-question-mark-all-keys-c-changes.md) | `?` lists all keys, `c` opens Changes, copy mode has no key | Accepted |
| [0023](0023-the-mode-is-always-shown.md) | The mode is always shown, the window is framed in shortcut mode, dead keys say so | Accepted, amended by 0024 |
| [0024](0024-the-mode-lives-in-a-bottom-bar.md) | The mode lives in a bar along the bottom, with key suggestions | Accepted |
| [0025](0025-a-new-pane-ends-shortcut-mode.md) | Starting a pane ends shortcut mode | Accepted |
| [0026](0026-hairline-a-fifth-look.md) | Hairline, a fifth look with no boxes | Accepted, amended by 0027 |
| [0027](0027-two-looks-and-tabs-in-the-sidebar.md) | Two looks, pane colours, and a session's tabs in the sidebar | Accepted, amended by 0028 and 0030 |
| [0028](0028-tab-rows-and-popups-drawn-like-panes.md) | A tab row in both looks, and every popup drawn like a pane | Accepted, amended by 0030 |
| [0029](0029-alt-letters-and-closing-without-asking.md) | Alt+letter shortcuts, and every close asks first | Accepted |
| [0030](0030-panes-titled-like-an-editor.md) | Panes titled like an editor's, and toasts drawn to suit each look | Accepted, amended by 0031 |
| [0031](0031-tabs-name-a-project-and-splits-have-no-title.md) | The tab row lists the project's tabs, and a split has no title | Accepted, tab row amended by 0032 |
| [0032](0032-a-session-owns-its-tabs.md) | A session owns its tabs, and picking it shows it | Accepted |
| [0033](0033-the-collapsed-sidebar-is-a-column.md) | The collapsed sidebar is a narrow column, not a strip of chips | Accepted |
| [0034](0034-the-sidebar-never-reorders-itself.md) | The sidebar never reorders itself, and has no SSH group | Accepted |
