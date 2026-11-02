# 0044. Chat and terminal are separate sessions, chosen when one starts

- **Status:** Accepted
- **Date:** 2026-11-01
- **Amends:** [0041](0041-chat-gui-shared-conversations.md) (its handoff rule)

## Context

[0041](0041-chat-gui-shared-conversations.md) let one conversation move between a
structured chat runtime and a native agent terminal through an explicit, safe-idle
handoff. In use that move was the least reliable part of the product. It stops one
runtime and resumes the same provider session in the other, so it needs a persisted
transfer fence, confirmed process cleanup and a history reconcile. A stall anywhere left
a session stuck mid-switch ("Returning…", "Recovery needed"). It also only worked for
Codex; Claude never had it. Each new provider would need its own handoff.

What users actually need is the same list of sessions in the desktop app and the terminal
app, with one name and one closed state, not a live switch.

## Decision

A session is one of two kinds, chosen when it starts and kept for its life:

- **Chat**: a daemon-owned structured conversation. It has no terminal.
- **Terminal session**: an agent running in a terminal pane the daemon owns. It is the
  same pane the terminal app shows, so both clients can open it.

Both kinds appear in both clients' session lists. The terminal app lists a chat but opens
it only in the desktop app; the desktop app lists and opens terminal sessions for Codex
and Claude agents.

A conversation's name and closed state are the daemon's (`ConversationRename`,
`ConversationArchive`), so every client shows the same ones. Closing a chat only puts it
away; closing a terminal session ends its program, as it does in the terminal app.

The handoff commands (`ConversationHandoffToNative`, `ConversationHandoffToChat`) and
their state machine are removed. A conversation already running in a native terminal when
this lands stays a terminal and does not return to chat.

## Alternatives considered

- **Keep the handoff and fix it**: every provider would need a safe stop-and-resume path,
  and the failure states are visible to the user. It stays the riskiest part of the daemon.
- **Show the lists in both apps but never open across**: leaves the desktop app unable to
  use the terminal sessions the terminal app already runs.

## Consequences

Fewer states and no transfer to get stuck in. Starting a session means choosing its kind
up front; a chat cannot be continued in a terminal. The daemon gains pane-keyed terminal
access (`TerminalInput`, `TerminalResize`, `TerminalScreen`, `TerminalReplay`). Revisit if
users need to move a live session between kinds often enough to justify a provider-by-
provider handoff again.
