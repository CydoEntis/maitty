# Intent: Borrowed from TUIOS

Type: feature
Author: Cody · Status: draft
Date: 2026-10-23 · Plan: [Phase 20](../../ROADMAP.md#20--borrowed-from-tuios--built-on-dev-2026-10-23-live-checks-open)

## Problem

A read of TUIOS (tuios.dev, v0.10.1) turned up five places where Vibecasso can still lose
track of an agent or let one do something unsafe:

- An agent whose hooks said "working" and then hung shows a spinner forever. Nothing
  tells you it has stopped making progress.
- An agent answering another session's prompt with `vibecasso_answer` may be approving a
  different prompt from the one it read: the screen can change between its read and
  its answer.
- Text an agent sends to another agent can carry invisible characters that hide
  instructions from you.
- Each pane's recent output, secrets included, is saved to disk so it comes back after a
  restart. You can't turn that off without turning off restore. On Linux and macOS the
  files get the default permissions.
- Restoring a checkpoint while its agent is working only warns you. The agent keeps
  writing into files that are being rolled back.

## Proposed outcome

- A hooked agent that has said "working" but printed nothing and reported nothing for
  10 minutes is marked as quiet. It shows on its row, in the why popover, in the
  Inbox's NEEDS YOU (with no sound), and in `vibecasso ls` / `vibecasso_list` / `vibecasso wait`.
  Its status stays "working".
- `vibecasso_answer` answers only the prompt the caller saw, identified by an id from
  `vibecasso_list`. If the prompt has changed, it refuses and shows the current one.
- Text that `vibecasso send`, `vibecasso_send` and `vibecasso_start` would send to an agent is
  refused when it contains invisible characters. The error names each character and
  where it is.
- A Restore setting stops saving pane output and deletes what is already saved. Saved
  output files are readable only by you on Linux and macOS.
- Restoring a checkpoint is refused while its agent is working.

## Affected users and systems

Anyone running hooked agents (Claude, Codex, Pi, OpenCode and the others with
`vibecasso integrate`); agents that drive other agents over MCP or `vibecasso send`; anyone
who restores checkpoints.
Daemon status (`src/daemon/status.rs`), protocol (`TermInfo`), sidebar, why popover,
Inbox, `vibecasso ls`, MCP (`src/mcp.rs`), `cli::send`, persistence (`src/daemon/persist.rs`,
`src/daemon/restore.rs`), Settings › Sessions, the checkpoint menu.

## Constraints

- Status is never guessed from silence ([agent status](../../agent-status.md)). A quiet
  agent stays "working"; being quiet is shown alongside its status and never replaces it.
- An agent waiting on helpers it has reported is never marked quiet.
- One protocol bump for the phase (45 → 46), per AGENTS.md non-negotiable 2.
- Your own typing in a pane is never checked for hidden characters.
- The quiet mark in the Inbox makes no sound and sends no desktop or phone notification.

## Out of scope

- A fifth "unknown" status (declined 2026-10-23).
- A configurable quiet threshold. It is 10 minutes.
- Snooze or per-pane quiet mode (removed earlier; see the plan's Out list).
- Allowing zero-width joiners inside emoji.
- An override for restoring while an agent works.
- Turning off saving of check output tails (only pane output gets the setting).

## Open questions

## Tickets

#142, #143, #144, #145, #146, #147, #148, #149, #150
