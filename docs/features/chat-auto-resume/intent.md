# Intent: resume chat conversations after a daemon restart

Type: feature
Author: Cody · Status: done
Date: 2026-11-07

## Problem

When the daemon restarts (an update, `kill-server`, or a crash), terminal agents come
back by themselves, but chat conversations land in *Recovery needed* until a person
presses Resume. After an update that means clicking through every chat on every
machine. Resume itself works and does not replay the finished prompt
(`docs/ROADMAP.md`, the 2026-10-29 entry on the isolated Codex app-server).

## Proposed outcome

After a restore, every chat conversation that was Running or Transferring when the
daemon went away resumes by itself, on the same provider session, with no window open.
A resume that fails leaves the conversation in *Recovery needed* and is not retried.

## Affected users and systems

People who run chat conversations and update or restart the daemon, locally or on a
remote machine. The daemon (`crates/maitty-core/src/daemon/restore.rs`,
`conversation/commands.rs`) and the `restore` settings. No protocol or window change.

## Constraints

- [ADR 0001](../../adr/0001-the-daemon-owns-every-session.md) and
  [ADR 0003](../../adr/0003-status-and-conversation-trust.md): the same provider
  session identity, never a new or most-recent conversation, one owner at a time.
- Assumption: resuming a stopped conversation under its own identity is not an
  ownership transfer.
- A resume is tried once per daemon start, so a runtime that fails again cannot loop.
- Needs `restore.enabled`.
- A setting, `restore.conversations`, on by default, like `restore.agents`.

## Out of scope

- A "Resuming…" state in the window.
- Terminal panes (already restored by `restore.agents`).
- Retrying a failed resume.
- Updating machines (`maitty machine update`).

## Open questions

None.

## Tickets

Filed in Plane, project MAITTY, module "Chat auto-resume", in this order:

1. MAITTY-223 note which conversations were running at restore
2. MAITTY-224 add the `restore.conversations` setting
3. MAITTY-225 separate who asked from how a conversation starts
4. MAITTY-226 resume running conversations after a restore (blocked by 1, 2 and 3)
