# 0001. The daemon owns every session and conversation

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

Maitty exists so agents keep running when the person leaves. A client that owned a PTY would
take the agent down with it when it closed or crashed.

## Decision

One background daemon owns every PTY, terminal emulator, agent process and conversation. A
client only draws and sends input. The daemon starts on demand and outlives its clients.
Closing, crashing or detaching a client never kills a pane.

Exactly one runtime owns each conversation at a time. Ownership moves only by an explicit,
safe-idle handoff that keeps the provider's session identity; an ambiguous transfer is shown
and never falls back to a new or most-recent conversation.

Only one daemon runs per socket. A daemon takes an exclusive lock (`ipc::lock_daemon`)
before it restores anything, so two clients starting at once cannot each restore the saved
session and spawn every pane.

## Alternatives considered

- **Each client owns its panes** (tmux-less): rejected, it loses sessions on close.
- **Daemon-to-daemon hub for several machines**: rejected, see 0007.

## Consequences

- Anything that touches PTYs or process lifetime lives in `crates/maitty-core/src/daemon/`.
- A client may create a session only after the daemon reports it has finished restoring.
