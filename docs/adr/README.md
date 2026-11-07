# Architecture decision records

An ADR records a decision that is expensive to reverse, along with the context
that made it the right call when it was made. Examples include the choice of
framework, persistence engine, auth model, architecture style, or module
boundary.

Don't write ADRs for ordinary implementation details, and not for how the UI looks or which
key does what. Those belong in code, tests, `docs/DESIGN.md`, `docs/shortcuts.md`, and commit
messages.

## How to add one

1. Copy [`0000-template.md`](0000-template.md) to `NNNN-short-title.md`, using
   the next number.
2. Fill it in. Set the status to `Proposed` until the user accepts it.
3. Add a row to the index below.
4. Update the project-specific section of the doc that owns the topic, with a
   one-line summary and a link.

To change a decision, edit its ADR in the same change and say why in the commit message. Git
holds the history; an ADR states only what is true now.

## Index

| # | Decision |
| --- | --- |
| [0001](0001-the-daemon-owns-every-session.md) | The daemon owns every session and conversation, and only one daemon runs |
| [0002](0002-terminal-substrate-and-wire-protocol.md) | portable-pty and vt100; msgpack over local sockets, versioned |
| [0003](0003-status-and-conversation-trust.md) | Status comes only from the owning runtime, and one writer decides it |
| [0004](0004-client-split-and-desktop-host.md) | Clients are views: TUI, CLI and an optional desktop app, sharing one installed daemon |
| [0005](0005-provider-adapters-and-permissions.md) | Provider adapters sit behind the daemon; Maitty never widens permissions |
| [0006](0006-secrets-never-enter-chat.md) | Secrets never enter chat |
| [0007](0007-multiple-machines.md) | One connection per machine, held by the client |
| [0008](0008-windows-first-and-one-product-name.md) | Windows is first-class; the product name lives in one place |
| [0009](0009-usage-is-provider-reported-never-estimated.md) | Usage is what the provider reported, never estimated |
