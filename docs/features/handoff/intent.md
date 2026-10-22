# Intent: seshi handoff

Type: feature
Author: Cody · Status: accepted
Date: 2026-10-22 · Plan: [phase 13](../../ROADMAP.md#13--borrowed-from-t3-code-round-two--proposed-2026-10-19)

## Problem

An agent gets stuck, runs out of plan, or is the wrong tool for the next step. Switching to
another agent today means opening a pane in the same checkout and retyping what the first one was
doing: the task, the branch, which files it touched, where it stopped.

## Proposed outcome

`seshi handoff <pane> --to <agent>` opens a new pane beside the old one, in the same checkout, and
starts `<agent>` with a short text brief as its first prompt: the task, the last prompt, the branch
and checkout, the changed files, and the tail of the pane's screen. The brief says in its first line
that it is text only: the new agent does not have the old one's conversation, memory or tool results.
The old agent keeps running, untouched.

## Affected users and systems

Anyone running more than one agent kind. The CLI (`src/main.rs`, `src/cli.rs`, new
`src/cli/handoff.rs`) and the shared code that builds an agent's start command (moved out of the MCP
`seshi_start` tool). The daemon is unchanged: it already splits a pane with a command and a folder.

## Constraints

- No wire change: the command uses `Query::List`, `Query::Read` and `Command::Split` as they are.
- The daemon owns the new pane; the CLI only asks for it. Grants apply as for `seshi read` and
  `seshi split`.
- Agents start the way `seshi_start` starts them (`[quick]` commands with `{prompt}`, accounts, known
  plain-named agents), through one shared function.
- Works on Windows, macOS and Linux shells; under `cmd` the brief's line breaks become spaces, as any
  first prompt does today.

## Decisions

- **First prompt:** the brief uses `TermInfo.name` (the `/rename` name, else the first prompt) as the
  task and `TermInfo.summary` as the last prompt. A separate first-prompt field would need a wire
  change for little gain.
- **Delivery:** the brief is the new agent's first prompt on its command line, not typed in with
  `seshi send` after start-up, so there is no race with the agent's start screen. Briefs are capped
  at 4000 characters.
- **Changed files:** uncommitted paths plus files changed since the branch left the main branch. When
  git cannot answer (for example over `--remote`), the brief says so and the handoff still runs.
- **Main checkout:** a handoff from a pane in a repo's main checkout follows the per-agent worktree
  rule like any agent started there; the command prints where the new agent went.

## Out of scope

Carrying native context (transcripts, `--resume`), stopping or messaging the old agent, an MCP tool,
a key or menu item, choosing the new agent's model.

## Open questions

None for the first slice.

## Tickets

#116–#118, in order in [phase 13](../../ROADMAP.md#13--borrowed-from-t3-code-round-two--proposed-2026-10-19).
