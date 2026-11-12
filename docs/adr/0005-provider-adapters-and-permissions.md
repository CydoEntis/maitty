# 0005. Provider adapters sit behind the daemon, and Maitty never widens permissions

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

The daemon can start an agent as a structured conversation (no terminal) as well as watch one in
a pane (0003). Each provider speaks its own protocol.

## Decision

- Codex (`app-server`) and Claude Code (headless `stream-json`) each live in their own adapter
  crate behind the daemon's provider contract. Adapters are built with the daemon; there is no
  plugin ABI or runtime download.
- Maitty launches the official CLI with the user's own sign-in. It offers no login screen and
  does not handle OAuth tokens.
- Permission posture is the provider's. Maitty only chooses the setting and never widens a
  session's permissions after it starts. `Sandbox::FullAccess` is opt-in per conversation, always
  paired with `ApprovalPolicy::Never`, and nothing selects it by default.
- The one exception is a person's own, explicit **Always in this project** on a command request in
  a chat. The daemon then keeps that project and the hash of that exact command in
  `chat-allowed-commands.json` in the app's data folder (never in the repo) and approves the same
  command, written the same, in that project without asking. It covers command requests only: never
  a file change, a permission grant or a question. Only a human client can make it; a pane or
  structured client is refused. A rule widens nothing the person has not already said yes to.
- Report only provider capabilities that were verified against a pinned version.

## Alternatives considered

- **Bundle our own agent runtime**: rejected, the point is to run the user's agents.

## Consequences

- A new enum variant is an incompatible message shape and bumps `PROTOCOL_VERSION` (0002).
