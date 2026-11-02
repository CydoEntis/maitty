# 0043. A full-access setting for structured conversations

- **Status:** Accepted
- **Date:** 2026-11-02
- **Amends:** [0041](0041-chat-gui-shared-conversations.md)

## Context

Structured conversations could only start read-only or workspace-write, with
approvals on request. Codex started read-only from the desktop client, so an
agent asked to write a file needed an approval for every step and could not run
unattended. People who trust a project asked to start a session with no
sandbox and no prompts.

## Decision

Add `Sandbox::FullAccess`. A client asks for it explicitly when it creates a
conversation, always together with `ApprovalPolicy::Never`; nothing selects it
by default. The adapters map it to each provider's own switch: Codex
`danger-full-access`, and Claude `bypassPermissions` with
`--allow-dangerously-skip-permissions`. Maitty only chooses the setting. The
provider enforces it, and Maitty still never widens a session's permissions
after it starts.

A new enum variant is an incompatible message shape, so `PROTOCOL_VERSION`
moves from 62 to 63. A client and daemon from different sides of that line
refuse to talk instead of misreading each other.

## Consequences

The New conversation dialog offers Read only, Workspace write (the default) and
Full access, with Full access marked as unsandboxed. The GUI remembers what each
session was started with, because the daemon summary does not report it.
A running conversation's permissions are not changed in place, as in the
terminal: End session stops the agent (`ConversationEnd`, protocol 64), keeps its
history, and the user starts a new session with the permissions they want.
