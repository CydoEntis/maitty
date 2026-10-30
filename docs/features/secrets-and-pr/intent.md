# Intent: secrets and PR tracking

Type: feature
Author: the user · Status: accepted
Date: 2026-10-29 · Plan: [phase 33](../../ROADMAP.md#33--secrets-and-pr-tracking--proposed-2026-10-29)

## Problem

In the user's words: "If I need an API key or other secret, I ask for it through a secure
prompt, not in chat. I can link a pull request to this thread and watch its checks and
reviews."

Today an agent that needs a key asks for it in chat, so the key lands in the transcript.
Maitty also knows nothing about a pane's pull request, so a failing check or a new review goes
unseen until someone looks. The `#PR` tag in the status line shipped as "no data yet"
([phase 14](../../ROADMAP.md)).

## Proposed outcome

An agent asks for a secret and the user types it into a masked prompt in Maitty. The agent gets
a file path or a one-command wrapper, never the value in chat. Each pane's pull request, found
from its branch or linked by hand, shows its checks and review state. When checks fail, a
review arrives or a merge conflict appears, the agent gets a short message once its turn is
over.

## Affected users and systems

Anyone running agents that need credentials or that open pull requests. The existing MCP
server (`src/mcp.rs`), the daemon (a new `src/daemon/secrets.rs` and a PR watcher), the
protocol, the client prompt, the status line and sidebar, and the `maitty` CLI.

## Constraints

- Builds on the existing MCP server and `MAITTY_PANE_TOKEN` trust; there is no new server
  (AGENTS.md non-negotiable 5).
- Only the user can answer a secret prompt. Like `AnswerHelpers`, the answer is refused from
  inside a pane.
- A secret is held in the daemon's memory only, expires after 24 hours, and a ref works once.
  Nothing at rest ([ADR-0036](../../adr/0036-secrets-never-enter-chat.md)).
- A secret's value never appears in the state snapshot, logs, tool results or pane output.
- `gh` and every other slow call run in `spawn_bg`, never on the UI thread (non-negotiable 6).
- Wire changes bump `PROTOCOL_VERSION` (non-negotiable 2); both platforms build
  (non-negotiable 3).
- The wake-up uses the existing follow-up queue, so it never goes into a question the agent is
  asking.
- Logic lives in the daemon and the protocol; the terminal is one view of it.

## Out of scope

- Delegating a task to another agent or provider, in its own thread or worktree, with the
  result returned. Deferred; see [Later](../../ROADMAP.md#later).
- A GUI or chat view, and structured agent protocols (the Codex work in #197–#207 is separate).
- Secrets at rest, or syncing them between machines.
- Posting to, approving or merging a pull request.
- Hosts other than GitHub.

## Open questions

None.

## Tickets

Secrets: #218, then #219, #220 and #221 (each after #218).
PR tracking: #222, then #223, #224 and #225 (each after #222).
