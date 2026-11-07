# 0004. Clients are views over the daemon: TUI, CLI and an optional desktop app

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

Maitty has a terminal app and an optional desktop app. Both must work against the same daemon
without the desktop app dragging daemon code or a webview into the terminal-only install.

## Decision

- Three layers: a headless daemon, a TUI and CLI client, and an optional Tauri 2 desktop app
  (React and TypeScript front end, system webview). They share the wire protocol (0002) and
  nothing else.
- The desktop app must not link `maitty-core`, `maitty-terminal-client` or `maitty-settings`.
  `scripts/check-gui-deps.sh` enforces it.
- Both apps list and attach to every session, and either may type into one. The daemon records
  the app that created a session (`Surface`) as its size owner: only that app resizes the session,
  so one app never reflows a pane the other is drawing. The daemon still owns every session (0001).
  A window layout (which sessions sit in which panes) belongs to the app, not the daemon.
- Terminal output reaches the desktop window over a local WebSocket on `127.0.0.1`. The window
  gets a one-time token, sends it first, and trusts the socket only after the host answers
  `ready`; only the app's own origin may connect. The Tauri channel carries output until then.
- Terminal-only installation and the native agent terminal interfaces stay supported.

## Alternatives considered

- **One client that does both**: rejected, it ties the terminal install to a webview.
- **Plugin ABI for clients**: rejected, no current consumer.

## Consequences

- A desktop feature that needs daemon behaviour goes through the protocol, not a link.
- The loopback WebSocket is a security surface; its checks live in `docs/SECURITY.md`.
