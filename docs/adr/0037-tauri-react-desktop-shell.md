# 0037. Tauri 2 desktop shell with React and shadcn/ui

- **Status:** Accepted
- **Date:** 2026-10-28
- **Supersedes:** none

## Context

The optional chat GUI needs a polished cross-platform desktop interface while
Maitty keeps its Rust daemon, terminal client and terminal-only installation.
The GUI must share daemon-owned conversations and sessions.

## Decision

Use Tauri 2 as the optional desktop host, with a React and TypeScript frontend
bundled by Vite and styled with shadcn/ui and Tailwind. The frontend connects to
the existing daemon through a narrow, typed Tauri command boundary and the
daemon's authenticated local IPC. The OS webview is used on each platform.

## Alternatives considered

- Native Rust-only GUI: does not provide the requested React component ecosystem.
- Electron: duplicates more runtime and process responsibilities than this
  Rust application needs.
- Browser-only frontend: does not provide the requested desktop application.

## Consequences

The project maintains frontend tooling and platform webview prerequisites.
Native GUI packaging is separate from the Rust CLI/TUI install and bundles the
standalone CLI as a sidecar so the GUI can start the daemon without owning its
process lifetime. The desktop GUI must be exercised on Windows, macOS and Linux
before release.
