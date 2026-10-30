# 0036. Optional chat GUI over daemon-owned native conversations

- **Status:** Accepted
- **Date:** 2026-10-27
- **Supersedes:** [0011](0011-keep-seshi-terminal-first.md)

## Context

The user now wants chat as the main desktop experience, optional terminals, and
the existing TUI retained. GUI-created conversations must continue in the native
agent interface inside Maitty and vice versa. The existing daemon already owns
PTYs and persists native session identities; it does not yet carry structured chat.
[Research](../research/gui-agent-continuity.md) distinguishes native session resume
from full GUI history reconciliation.

## Decision

Allow an optional desktop chat client using the same daemon as the TUI. Add
daemon-owned structured provider adapters; start with installed Codex app-server.
Preserve native agent terminal interfaces and terminal-only installation.
Claude reaches equivalent continuity before the initial GUI release.

One runtime owns each conversation. Transfer only by explicit safe-idle handoff,
preserving exact provider identity, account/configuration, worktree and permission
posture. Stop the old runtime and reconcile native history before granting the
new runtime control. A failed or ambiguous transfer is visible and never falls
back to a new or most-recent conversation.

Tauri 2 hosts a React, TypeScript and Vite frontend styled with shadcn/ui and
Tailwind. It is an optional desktop client over the daemon's existing local IPC;
the TUI and terminal-only installation remain supported. Platform webviews are
used rather than bundled. The concrete GUI design remains in phases 34–36.

## Alternatives considered

- Keep a terminal-only product: does not serve the approved desktop chat audience.
- Replace native terminal interfaces with Maitty-rendered chat: rebuilds agent
  interaction and jeopardizes the workflow the user wants to preserve.
- Build Maitty's own model/tool runtime: diverges from native agent conversations.
- Permit concurrent native and structured writers: lacks verified provider support
  and risks inconsistent context or duplicated work.

## Consequences

Maitty gains provider adapters and transcript reconciliation to maintain, with
explicit supported-version checks. Existing TUI behavior remains supported.
Live roundtrip proof is required before claiming continuity for a provider.
This records a target architecture, not functionality already implemented.
