# Intent: ask a side question about a session

Type: spike
Author: Cody · Status: proposed
Date: 2026-11-13 · Plan: Plane MAITTY-307, module *Daily loop 3, Throughput*

## Problem

While an agent works, you often want to ask something about what it is doing ("why
that file?", "what does this flag do?") without adding to its conversation. Today you
either interrupt it, start a new chat and re-explain, or type the agent's own `/btw` or
`/side` into its terminal. The first two cost the agent's context or yours; the third
only works for agents that have the command and only inside the terminal.

## Proposed outcome

From a session, you open a **side chat**: a separate conversation that knows what the
session is about, answers your question, and never touches the parent. When the answer
is worth keeping, **Bring back** sends it to the parent as one message you can edit
first. A side chat can be discarded with nothing left behind.

## Affected users and systems

People who run agents across projects and want a quick answer mid-task. Desktop chat
and session row menu, the daemon's session record (one new field), and the handoff
brief (`crates/maitty-shared/src/handoff.rs`). The TUI is unchanged for now (see Out
of scope).

## Constraints

- The daemon owns every session, and a side chat is a normal session
  ([ADR 0001](../../adr/0001-the-daemon-owns-every-session.md)). Closing the window
  never ends it.
- The parent is never written to without you pressing **Bring back**. A side chat
  cannot change the parent's status
  ([ADR 0003](../../adr/0003-status-and-conversation-trust.md)).
- The brief is text only: the side agent gets no conversation, memory or tool results
  from the parent, as the handoff brief already says.
- A new field on the session record is a protocol change, so it needs a
  `PROTOCOL_VERSION` decision when built (AGENTS.md, non-negotiable 2).

## Decisions

### Where the link to the parent is kept

On the daemon's session record, as a new optional field `side_of: Option<TermId>`,
`#[serde(default)]`, set when the side chat is started and restored with the session.
Not `started_by`: that field means "this agent started a helper for its parent and the
parent waits on it", and the sidebar, restore and the helper approval flow all act on
it. A side chat is started by you, and reusing the field would make the parent show
"wants to start N helpers" or wait on it.

Why the daemon and not the window: the link must survive closing the window and work
from the TUI and a second window. If the parent is closed, the side chat keeps running
and its **Bring back** is disabled with the reason "The session it came from is closed".
Not kept in the window's saved state, because that would lose it on another machine.

### What context the side chat starts with

The existing handoff brief for the parent (task, last prompt, branch, checkout, changed
files, recent output, capped at 4000 characters), plus one line saying this is a side
question and the agent should not change files. It starts in the parent's checkout,
read-only where the agent supports a read-only mode, and **without** its own worktree:
a side chat is a question, so it never needs a copy of the code.

It uses the same agent as the parent by default, with a choice on the start dialog.
The brief is refreshed once, at start; the side chat does not follow the parent
afterwards. A button, **Refresh context**, resends the brief when the parent has moved
on.

### What Bring back sends

One message, written into the parent's composer as a draft (never sent for you), in
this form:

```
Side question: <your question>
Answer: <the side agent's last reply>
```

It sends the question and the last reply only, not the whole side conversation, so the
parent's context grows by the answer and nothing else. You can edit it before sending.
If the parent is working, the draft joins its queue like any other message. After
sending, the side chat is left open and marked "Brought back"; it is not closed.

## Out of scope

- The agents' own `/btw` and `/side`, which stay as the agent defines them.
- A side chat from the TUI. The field and brief are shared, so a TUI command can follow.
- A side chat of a side chat. The menu item is hidden on a side chat.
- Merging a side chat's files into the parent (a side chat does not edit).

## Open questions

1. Is "read-only where the agent supports it" enough, or should a side chat be refused
   for agents with no read-only mode? (Proposed: allowed, with a note on the dialog.)
2. Should **Bring back** also offer the whole side conversation as a collapsed block?
   (Proposed: no; the last reply only, to keep the parent's context small.)

## Tickets to file

Not filed by this spike; each goes to Plane in module *Daily loop 3* once this is
accepted, in this order:

1. daemon: add `side_of` to the session record, set on start, restore it, and refuse a
   side chat of a side chat (protocol change; decide `COMPATIBLE_SINCE`)
2. shared: add a side-question line to the handoff brief and a refresh path
3. gui: **Ask a side question** in the session menu and palette; a start dialog
4. gui: **Bring back** writes the draft into the parent's composer; disabled with a
   reason when the parent is closed
5. docs: guide section for side chats
