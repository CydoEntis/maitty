# 0056. An agent session is one terminal session with a chat view

- **Status:** Proposed (the owner decided the product shape on 2026-11-05; the points under *To confirm*
  are open)
- **Date:** 2026-11-05
- **Supersedes in part:** [0054](0054-a-chat-session-is-a-daemon-owned-conversation-started-from-the-desktop.md)
  (point 2: a chat session has no terminal) and [0052](0052-an-agent-in-a-pane-gets-a-read-only-conversation.md)
  (the chat of an agent in a pane is read-only)
- **Relates to:** AGENTS.md non-negotiables 1, 4 and 5;
  [0036](0036-secrets-never-enter-chat.md); [0046](0046-desktop-app-is-a-terminal-multiplexer.md);
  the v18 handoff, README B.3 ("Chat shows the focused pane's session. Switching never restarts the agent.")

## Context

The owner's model for the desktop app: chat is the default way to work, a default agent (Claude or
Codex) is chosen once, and every chat is a session of its own. The same session can be switched
between its terminal view and its chat view at any time. A plain shell, with no agent in it, has only
the terminal view. The v18 design draws exactly this: one session, two views, "Same session as the
terminal · switching views never restarts claude", and a shell tab that says "Chat view is for agent
sessions".

The code has two kinds of agent session that do not match it. ADR 0052 gives an agent typed into a
shell a chat that follows the provider's transcript but cannot send, and ADR 0054 gives a session the
daemon starts itself a full chat (composer, approvals, Stop) that has no terminal because the
provider speaks JSON on pipes. A person gets one or the other, and cannot switch.

## Decision

1. **One kind of agent session: the agent runs in a PTY**, as the user would run it, so its native
   terminal is always there (non-negotiable 4). A new session starts the default agent in a shell the
   daemon owns, in the chosen folder.
2. **The chat view is a view of that session.** History comes from the provider's own transcript
   (ADR 0052, stage 1, unchanged). Status comes from the owning runtime's hooks (non-negotiable 5).
3. **The composer writes to the PTY.** A message is typed into the agent as the user would type it.
   Because typed text answers a permission prompt, the composer is on only while the pane's trusted
   status says the agent is waiting for a prompt; while the status is *needs you* the chat shows what
   is waiting and says to answer it, and the answer is a key sent to the PTY by the same gate.
4. **The structured chat sessions of ADR 0054 stay** until this is built and proven, then are retired
   by a later ADR. Nothing is removed by this one.
5. **A plain shell has no chat view**, and the window offers a separate New terminal for it.

## Alternatives considered

- **Keep both kinds and let the person pick** (today). Rejected by the owner: the choice is made
  before the work starts and cannot be undone, which is what the switch is for.
- **Make the structured session also run a PTY.** Not possible: Claude's stream-json and Codex's
  app-server are not terminal interfaces.
- **Read the terminal's text for the chat.** Rejected by ADR 0036: structured events, not scraping.

## To confirm

- Whether the composer typing into a PTY is acceptable for a message with attachments (ADR 0053 names
  files in the message text, which a PTY can carry) and for a long, multi-line message (bracketed
  paste).
- What the chat shows for a permission prompt, since the transcript has no entry for it until it is
  answered: the hook's notification text, or only "waiting for you in the terminal".
- Stop: an interrupt key to the PTY, and whether that is reliable for both agents.
- The daemon work is large (a PTY input path with a status gate, delivery receipts, restore of an
  agent session after a daemon restart) and touches the protocol; it needs its own tickets and a
  `PROTOCOL_VERSION` bump if a message changes.

## Consequences

- The desktop window can offer chat-first sessions without losing the native terminal.
- Until it is built, the window starts a chat session of ADR 0054 for a new session when Settings says
  Chat, and that session has no terminal view.
