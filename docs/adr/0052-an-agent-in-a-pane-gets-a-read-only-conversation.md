# 0052. An agent in a pane gets a read-only conversation that follows its own transcript

- **Status:** Accepted (the owner confirms the points under *To confirm*)
- **Date:** 2026-11-04
- **Relates to:** AGENTS.md non-negotiables 1, 4 and 5 and the chat GUI
  [spec](../features/chat-gui/spec.md) (Trust);
  [0045](0045-each-session-belongs-to-one-app.md) and
  [0046](0046-desktop-app-is-a-terminal-multiplexer.md) (the desktop lists terminal sessions;
  this gives one of them a chat); [0051](0051-the-daemon-keeps-provider-reported-usage.md)
  (provider files are read, never written)

## Context

The desktop chat view (Terminal | Chat switch, history, composer) links a pane to a daemon
conversation whose `native_term` is that pane. In real use no pane has one: the desktop starts
plain shells, the user types `claude` or `codex`, and the daemon only knows a process. The
conversations the daemon does hold are structured ones it starts itself (Claude stream-json,
Codex app-server) with no terminal, and the old native conversations were tied to a handoff that
ADR 0044 removed. Today nothing creates a `Native` conversation, and `Term.conversation_id`
marks a pane as owned by a conversation (not restored, not listed by the desktop), which is not
what a user's own `claude` is.

What the daemon already has for an agent in a pane, from its status hooks (verified by the pane's
process chain or `MAITTY_PANE_TOKEN`): the agent, its exact session id, and, for Claude, the path
of its transcript. Both providers write a JSON-lines file of the whole conversation as it
happens: Claude `~/.claude/projects/<folder>/<session>.jsonl`, Codex
`~/.codex/sessions/YYYY/MM/DD/rollout-<time>-<session>.jsonl`. The daemon already reads Codex's
files for usage.

## Decision

Stage 1, built now: **the daemon follows the transcript of an agent running in a pane and serves
it as a read-only `Native` conversation whose `native_term` is that pane.**

- A pane gets a conversation when it is an owned desktop pane, its agent is `claude` or `codex`,
  its status is hook-trusted and the hook gave a session id (and a transcript path, for Codex
  found by session id when the hook gave none). Nothing is guessed: no "most recent file".
- The conversation is **in memory only**: not in the session file, not in the history cache. It
  goes when the pane or the agent goes and is rebuilt from the file at the next hook. A new
  session id in the same pane (`/clear`, resume) keeps the conversation id and bumps its
  generation, so the chat starts over.
- History comes from the provider's own file, converted to the same history entries the
  structured runtimes produce (messages, tool calls with input, results, turn boundaries), by
  reusing the structured path's projection. A line that is not understood is skipped, never
  guessed.
- **History is display, not status.** The conversation's `status` is the pane's, from the owning
  runtime's hooks and process. Reading a file never changes it, and a file the daemon did not
  verify belongs to the pane's session (below) is not read.
- **Read-only.** `ConversationSend`, approve, answer, cancel and end refuse a followed
  conversation with a clear error. The agent keeps its native terminal; the chat view shows what
  it did, and the user types, and answers approvals, in the terminal (the owner decided that
  stays the fallback). The desktop disables the composer for it.
- **The terminal interface is untouched** (non-negotiable 4): the pane is still the user's own
  `claude` or `codex`; nothing is started, restarted or wrapped.

What is read, and the limits (SECURITY.md has the checklist): only a regular `.jsonl` file whose
real path is inside the user's home folder and whose name carries the hook-reported session id;
read-only, incrementally from a saved offset, at most 4 MiB per poll, a 16 MiB tail on first
read (the chat then says the start is missing), lines over 1 MiB dropped, every string cleaned
and bounded as the structured path does. Reading runs in a background task, never in the
daemon's event loop. The text of the file is not logged and is not written anywhere else.

## Alternatives considered

- **(a) The daemon starts the agent itself as a structured conversation** (Claude stream-json,
  Codex app-server) when a user starts claude/codex from the GUI. It gives the full chat
  (composer, approvals by id, cancel, queue) and exists today. Rejected as the first step: the
  agent then has **no terminal** (the structured runtimes are not PTYs), so non-negotiable 4 is
  kept only by running the same session in a pane *or* a structured runtime, never both
  (one writer per conversation, spec R5), and the handoff that did that was removed (ADR 0044).
  It also changes how sessions are created and listed in the desktop (ADR 0046), which is a
  design and GUI change much larger than a link, and it cannot help the `claude` a user already
  typed in a shell.
- **(c) Both, in stages** is the chosen shape: stage 1 now, (a) later if the owner wants a
  composer and approvals in the chat for new sessions.
- **Set `Term.conversation_id` on the pane.** Rejected: that marks the pane as owned by a
  structured runtime, so restore skips it, process detection stops renaming it and the desktop
  hides it. A followed pane must stay an ordinary pane.
- **Persist the followed conversation** like the structured ones. Rejected: the provider's file
  is the durable record; a copy would be a second private store (AGENTS.md privacy rule: no new
  caches of provider history) and a restore path to keep correct.
- **Type the composer's text into the pane's PTY.** Rejected for stage 1: while the agent shows a
  permission prompt, typed text answers the prompt; the chat cannot tell. It needs the pane's
  status as a gate and a decision on delivery receipts. Offered below.
- **Read the terminal's text.** Rejected: ADR 0036's rule is structured events, not scraping.

## Consequences

- No protocol change: `ConversationSummary` already carries `execution` and `native_term`, and
  `ConversationRead` already serves history. `PROTOCOL_VERSION` stays 70. A desktop client that
  does not know the new conversations still lists the pane.
- The TUI is unaffected: followed conversations exist only for desktop-owned panes, and the
  terminal app is never shown a conversation whose pane it does not own (ADR 0045).
- Codex's rollout format is read from its observed shape (response items and event messages) and
  is not versioned by Codex. A format change makes entries missing, not wrong; the parsers are
  covered by fixtures that must be re-checked against a real Codex when it changes.
- A Claude session seen only after a daemon restart appears at its next hook (the first prompt
  or stop), not at once.
- Edits Codex makes with `apply_patch` appear as file-change cards from the patch text.

## To confirm

1. Stage 1 is read-only. Does the owner want stage 2 (a composer for these panes by typing into
   the PTY when the agent is idle, or sessions started by the daemon as structured
   conversations) next, and which?
2. Following is on for every owned desktop pane running Claude or Codex, with no setting. The
   alternative is an opt-in per pane.
3. Only files under the user's home folder are read. An account whose config folder is elsewhere
   would need an allow-list entry.
