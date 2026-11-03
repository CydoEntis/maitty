# 0046. The desktop app is a terminal multiplexer: plain shells, no chat

- **Status:** Accepted
- **Date:** 2026-11-02
- **Supersedes:** [0044](0044-chat-and-terminal-are-separate-sessions.md) (its chat kind in the desktop app), and [0045](0045-each-session-belongs-to-one-app.md)'s open question about chats

## Context

The desktop app grew a chat screen beside its terminal view. Chat is a separate product surface that
needs its own maintenance and makes the terminal view harder to get right. Users want the desktop
app to work like the terminal app: a window of shells they move between.

## Decision

- The desktop app has **no chat**. Its sessions are shells that `maittyd` owns.
- A **new session is a plain shell** in the start folder. The user types what they want to run in it
  (`codex`, `claude`, `cargo test`).
- The **start folder** defaults to the user's home folder and is set in Settings → General.
- **Closing a session ends it.** Closing a pane alone is not the same action; the spec's pane
  close detaches and is decided with the layout work.
- The sidebar groups sessions by **git repository** by default. A session outside a repository is
  grouped by its own **folder**; there is no catch-all group. The user can group every session by
  folder in Settings.
- Sessions follow their shell: when a shell `cd`s, its group follows the current folder.
- Each desktop session stays owned by the desktop app (ADR 0045). Output reaches the window as a
  stream from the daemon, not by polling.

## Consequences

- Chat code in the desktop app is removed, and so are chat-only daemon calls the desktop app used.
  The daemon keeps its conversation support for now; removing it is a later step.
- Agents are started by typing them, so status must come from the daemon (foreground process and
  output), not from a chat protocol.
- The start folder and grouping need settings that persist per user.
