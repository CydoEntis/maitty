# 0003. Msgpack frames over local sockets, versioned

- **Status:** Accepted, amended 2026-10-24
- **Date:** 2026-10-03

## Context

The client and daemon exchange snapshots and a high volume of pane output. The
transport must work on Windows (named pipes) and Unix (domain sockets), and tunnel
through SSH for remote machines.

## Decision

`interprocess` local sockets carrying length-delimited frames of msgpack
(`rmp-serde`) `ClientMsg` / `ServerMsg` enums. `PROTOCOL_VERSION` is checked in
the first exchange; mismatches are refused. For remotes, the same frames go
through `ssh host hydra proxy`.

## Alternatives considered

- **JSON lines:** readable, but slower and larger for pane output.
- **gRPC / HTTP:** network-oriented, more dependencies, no gain for a local tool.

## Consequences

Fast, compact and the same everywhere. Every message change needs a version bump,
and old clients must be restarted after an upgrade. Revisit if a stable public
API for third-party clients is wanted.

Amended 2026-10-24 (the user's decision): messages are msgpack named maps, so a field added
with `#[serde(default)]` is read by older and newer peers alike and needs no bump, provided a
test decodes the old shape. A removed, renamed or retyped field, or a new enum variant, still
needs one. The rule lives in [API_PATTERNS](../API_PATTERNS.md) (*Project-specific: API style*).
