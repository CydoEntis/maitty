# 0002. Terminal substrate and wire protocol

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

The daemon must run the user's own programs in real terminals on Windows, macOS and Linux, and
every client must be able to talk to it, including across versions and over ssh.

## Decision

- `portable-pty` spawns processes in pseudo-terminals (ConPTY on Windows). A vendored `vt100`
  keeps each pane's screen and scrollback in the daemon.
- Clients and daemon talk over `interprocess` local sockets, with length-delimited frames of
  msgpack (`rmp-serde`) `ClientMsg` and `ServerMsg` enums.
- `PROTOCOL_VERSION` (`crates/maitty-shared/src/protocol.rs`) is checked in the first exchange.
  A mismatch is refused, never misparsed. It moves when a message shape changes incompatibly.
- A remote machine uses the same frames through `ssh host maitty proxy`.

## Alternatives considered

- **JSON over sockets**: larger and slower for terminal output.
- **A shared-memory or in-process emulator**: breaks the daemon/client split (0001).

## Consequences

- Adding a variant or retyping a field needs a version bump and a test that decodes the old shape.
- An added `#[serde(default)]` field needs no bump when a test decodes the old shape.
