# 0042. Decouple desktop clients and provider adapters

- **Status:** Accepted
- **Date:** 2026-10-30
- **Amends:** [0041](0041-chat-gui-shared-conversations.md), [0037](0037-tauri-react-desktop-shell.md)

## Context

The optional Tauri host currently depends on the main `maitty` crate to send
requests to the daemon. The daemon, TUI and CLI also live in one Cargo package.
The product must keep the known terminal app working while letting the GUI and
provider integrations evolve independently.

## Decision

Separate the workspace into a headless core daemon, TUI/CLI client, optional
desktop GUI client, shared wire protocol, and provider adapter packages. The TUI
and GUI connect to the daemon through the versioned, authenticated local
protocol. Neither client links to the daemon implementation. The daemon remains
the only owner of sessions, conversations, persistence and agent processes.

Put Codex, Claude Code and later provider-specific transport code in separate
packages behind the daemon's provider contract. Adapters are built with the
daemon; this decision does not add a dynamic plugin ABI or runtime downloads.
The GUI remains optional, and the existing `maitty` command and terminal-only
installation remain supported.

## Consequences

The Tauri host can depend only on the shared protocol/client package, not the
main runtime crate. Each provider adapter can be checked independently against
the same contract. Extract the packages in small compiling steps and preserve
the existing daemon protocol and TUI behavior throughout. Keep the current
integrated GUI branch as the comparison point until the separated build passes
the Windows, macOS and Linux gates.
