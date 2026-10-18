# 0011. Keep Seshi terminal-first

- **Status:** Accepted
- **Date:** 2026-10-17

## Context

Seshi's core job is running and monitoring coding-agent sessions in its terminal
interface. A chat-first desktop GUI would require a second provider integration
and session-presentation surface alongside the TUI. The user prefers to keep
Seshi focused and use T3 Code when a chat-oriented GUI is useful.

## Decision

Keep the Seshi TUI as its only user interface. Do not build a separate desktop
GUI client in this repository. The TUI continues to use the existing daemon,
protocol, and session engine.

## Alternatives considered

- **Build an optional desktop GUI:** duplicates a substantial provider and session
  integration surface and adds ongoing maintenance outside Seshi's terminal-first
  purpose.
- **Replace the TUI with a desktop app:** abandons the interface the user relies
  on daily.

## Consequences

Seshi remains focused on its terminal workflow. Users who want a chat-first GUI
can use a separate product such as T3 Code. Any future proposal to add a desktop
client must be reconsidered as new scope; this decision creates no dependency on
or integration with that product.
