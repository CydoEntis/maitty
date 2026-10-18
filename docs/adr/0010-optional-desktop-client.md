# 0010. Offer an optional desktop client over the shared session engine

- **Status:** Superseded by 0011
- **Date:** 2026-10-16

## Context

Seshi is used daily as a terminal UI. Some users also want a desktop GUI, while
keeping the existing TUI reliable and letting people choose which interface to
install. The daemon already owns sessions independently of its clients
(ADR-0001), and the versioned local protocol is the boundary between them
(ADR-0003).

## Decision

Build the desktop GUI as a separate client over the same daemon, protocol, and
session engine. Keep the TUI as a supported client with its single layout. Let
users install the TUI, the GUI, or both. A GUI-only installation includes the
runtime needed to operate the daemon without requiring the interactive TUI.
Session ownership and agent behavior stay in the shared engine; presentation
behavior belongs to each client.

## Alternatives considered

- **Replace the TUI:** breaks established daily use and removes user choice.
- **Duplicate session and agent logic in the GUI:** creates divergent status,
  persistence, and lifecycle behavior.
- **Require both interfaces in every installation:** forces an interface on users
  who do not want it.

## Consequences

Both clients must remain compatible with the daemon protocol and share behavior
through the existing engine. Packaging and CI must verify TUI-only, GUI-only,
and combined installs on Windows, macOS, and Linux. GUI work must not change the
TUI's interaction layout or require the GUI to keep sessions alive.
