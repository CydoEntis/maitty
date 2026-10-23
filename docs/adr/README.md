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
| [0003](0003-msgpack-over-local-sockets.md) | Msgpack frames over local sockets, versioned | Accepted |
| [0004](0004-one-layout.md) | One UI layout: the hydra layout | Superseded by 0007 |
| [0005](0005-herdr-compatible-keys.md) | Leader-key bindings follow herdr where they overlap | Superseded by 0006 |
| [0006](0006-floating-design-keys.md) | Leader keys follow the floating design's key map | Superseded by 0009 |
| [0007](0007-floating-and-tiled-styles.md) | One layout, drawn floating or tiled | Superseded by 0012 |
| [0008](0008-rename-to-seshi.md) | The app is called Seshi | Accepted |
| [0009](0009-leader-keys-after-the-cut.md) | Leader keys: a shell is one key, and only kept features have keys | Accepted |
| [0010](0010-optional-desktop-client.md) | Offer an optional desktop client over the shared session engine | Superseded by 0011 |
| [0011](0011-keep-seshi-terminal-first.md) | Keep Seshi terminal-first | Accepted |
| [0012](0012-flat-rendering-style.md) | Add a flat rendering style | Accepted |
| [0013](0013-v7-frame.md) | Draw the v7 frame: tabs in the pane title, no margin | Proposed |
| [0014](0014-v8-cleanup.md) | Bring over the v8 cleanup, keeping what the user already decided | Proposed |
