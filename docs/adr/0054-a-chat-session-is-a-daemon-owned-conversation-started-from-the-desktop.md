# 0054. A chat session is a daemon-owned structured conversation started from the desktop

- **Status:** Proposed (the owner confirms the points under *To confirm*). Points 3 and 5 were amended on 2026-11-05 by
  the owner's decisions below (default Chat; full access offered).
- **Date:** 2026-11-04
- **Relates to:** AGENTS.md non-negotiables 1, 4, 5 and 6;
  [0036](0036-secrets-never-enter-chat.md) and the chat GUI [spec](../features/chat-gui/spec.md)
  (trust, generations); [0045](0045-each-session-belongs-to-one-app.md) and
  [0046](0046-desktop-app-is-a-terminal-multiplexer.md) (the desktop lists terminal sessions; this
  adds a second kind); [0052](0052-an-agent-in-a-pane-gets-a-read-only-conversation.md) (stage 2,
  option (a)); [0053](0053-chat-attachments-are-files-named-in-the-message.md) (attachments)

## Context

ADR 0052 gave an agent the user typed in a shell pane a read-only chat, and left stage 2 to the
owner: a composer, approvals and Stop for sessions the daemon starts itself as structured
conversations (Claude stream-json, Codex app-server). The owner chose that (2026-11-04). The daemon
already does everything below the window: it starts and owns the provider process, serves ordered
history, holds a send queue (G24), answers approvals and user-input questions by request id, cancels
a turn and ends a conversation, all bound to a generation, and keeps the process when the client
goes. The Tauri commands for these exist. What is missing is the way a person asks for one and the
place it appears in a window that lists terminals only (ADR 0046).

A structured conversation has **no PTY**: the provider speaks JSON on pipes, so there is no native
terminal screen to show for it, and none is faked.

## Decision

1. **A chat session is a session of its own kind in the desktop.** The sidebar, tabs and panes list
   it beside terminal sessions, under the same project grouping (by its folder or worktree). It is
   the daemon's conversation, identified by the conversation id, not by a pane number.
2. **What the terminal side shows.** Nothing is attached to a terminal. The pane has the chat view
   only: its Terminal | Chat switch is absent, and where the terminal would be there is no
   hidden terminal, only the chat. A short line in the chat header says it is a chat session and
   that the agent has no terminal; to use the agent's own interface the user starts a terminal
   session instead. Nothing converts one into the other (one writer per conversation, spec R5).
3. **Choosing the kind.** *New agent on a branch* gets a **Chat | Terminal** choice beside the agent
   (Claude | Codex), **Chat by default**, a desktop Setting (Settings › General › Chat view › *New agent sessions start as*;
   Chat or Terminal). Terminal stays one click away in the dialog. The chat empty state of a shell pane keeps
   **Start claude here** and **Start codex here** (they type the command into the shell, as before)
   and gains **Start a Claude chat** and **Start a Codex chat**, which start a chat session in the
   pane's folder. A chat session has no `claude` or `codex` in a shell, so a plain shell, the TUI and
   the native agent terminal interface are untouched (non-negotiable 4).
4. **Starting.** One new field on `ConversationStart`, `branch: Option<String>`, defaulted. Without
   it the daemon starts the conversation in the given folder. With it the daemon, after the same
   checks as *New agent on a branch* (a valid branch name, a project the app already shows, a git
   repository), makes the worktree with the existing machinery (`create_trusted_worktree`, the
   repo's setup hook, the trust copy, the made-worktrees list for clean-up) and starts the
   conversation in it. The first prompt, if any, is an ordinary `ConversationSend` once the start
   replied, so it is idempotent like every send. The folder, never a path from the window, decides
   what the provider can touch; the window passes the project the sidebar already shows.
5. **Permissions.** A GUI-started chat uses the existing posture **workspace write, ask on request**
   (Claude `--permission-mode default` with `--permission-prompt-tool=stdio`; Codex `workspace-write`
   with `on-request`). Every tool use the provider does not allow by itself becomes an approval card
   in the chat, answered by request id with Yes once (y), This session (a) or No (n). **There is no
   auto-approve.** The owner uses the agent's full-access mode for everything, so a chat can instead start in
   it, per chat: the Tauri command takes an optional `full_access` flag, and the daemon starts the provider with
   `Sandbox::FullAccess` and `ApprovalPolicy::Never` (ADR 0043: Claude `bypassPermissions`, Codex
   `danger-full-access`). It is a labelled checkbox, "Full access (skip permission prompts)", with a warning that
   there is no sandbox and no prompts; the Setting "Full access in new chats" only sets its starting value and is
   **off** by default. A chat started this way shows a red Full access tag in its header. The posture is fixed
   for the session and never widened in place. It needs no wire change: the choice is already in
   `LaunchSettings`. The older `create_conversation` command (which
   takes one) is left in the host for the preview screens in `App.tsx`; the desktop window does not call it. A sandbox
   is as strong as the provider's own; the agent runs with the user's permissions and the card is the
   user's gate (SECURITY.md).
6. **Status** comes only from the daemon's structured events for the current generation (ADR 0036).
   The sidebar row shows the conversation summary's status (working, needs you, done, idle). A
   command from a superseded generation is refused and changes nothing. Nothing is read from output.
7. **The composer is on.** Enter sends, Shift+Enter adds a line, Esc or **Stop** cancels the turn, a
   message sent while a turn runs is queued by the daemon (and shown with Remove), approvals and
   user-input questions are answered in the chat by id, and attachments (ADR 0053) work because the
   agent runs on the same computer. The composer is off, with its reason, for a followed (read-only)
   chat, while an approval waits, and while the daemon is offline.
8. **Surviving and ending.** The daemon owns the provider process, so closing the window, or the
   window crashing, changes nothing; the session is back in the sidebar at the next launch. The
   conversation is also saved by the daemon's existing record, so after a daemon restart it is
   listed with its history readable and no agent running. Starting the agent again is the existing
   resume path; the window does not offer it for a chat session yet (a follow-up). **Closing a chat
   session in the window ends the agent** (`ConversationEnd`: the process stops, the record stays so
   its history is readable) and puts it away (`ConversationArchive`), the same meaning as closing
   a terminal session ("closing a session ends it", ADR 0046).
9. **Local only.** A chat session runs on this computer. Starting one for another machine is refused
   in the window and the daemon never forwards it. Windows: the provider is started without a
   console window by the existing process code; paths are never built by the window.
10. **Secret masking.** Redaction (ticket 18) happens where history is stored or served, in the
   daemon. A chat session needs nothing of its own: it reads through the same choke point. The
   window adds no copy of the text.

## Alternatives considered

- **A pane whose terminal view shows the agent's own TUI**, with the composer typing into it.
  Impossible for a structured conversation (no PTY) and rejected for a followed pane in ADR 0052.
- **Make the Chat | Terminal choice only a setting.** Rejected: the choice is per start and has
  different consequences (no terminal), so it is asked where it is made.
- **A new `Command` for a branch chat.** Rejected: a defaulted field on `ConversationStart` is the
  same message with one more input, and the start still returns `ConversationStarted`.
- **Create the worktree from the window**, then start in its folder. Rejected: the window would have
  to run git (non-negotiable 6), skip the trust copy and hook, and the daemon could not tell the folder
  is one it should clean up.
- **Offer full access** as an advanced option. First rejected (nothing asked for it), then accepted on
  2026-11-05 on the owner's decision, with the visible warning and tag described in point 5.
- **Convert a running terminal session into a chat session.** Rejected: ADR 0044 removed the handoff;
  the agent's process cannot change its transport.

## Consequences

- `ConversationStart` gains a defaulted `branch`. A new client talking to an older daemon of the same
  version would drop it and start in the project's main checkout instead of a worktree, which is
  unsafe, so `PROTOCOL_VERSION` is bumped to 72. The old shape still decodes (the test pattern in
  API_PATTERNS.md).
- The desktop's session list is no longer terminals only: `kind` tells them apart, and anything that
  needs a pane (find, restart, usage per pane, terminal input, paste of an image) is off for a chat
  session or uses the conversation's own path.
- The session panel (Changes, Files) already reads a conversation's folder and works unchanged.
- A chat session started in a branch worktree adds a worktree the user may later tidy; it is listed
  by the existing worktree tidy-up like any other.

## To confirm

1. Decided 2026-11-05: the default is **Chat** (a Setting), Terminal stays reachable in both places.
2. Closing a chat session ends its agent (not only hides it).
3. Decided 2026-11-05: full access is offered per chat, off by default, with a warning and a visible tag.
4. A chat session has no terminal at all; the way to see the agent's own interface is a terminal
   session. Say if you want a button that starts one in the same folder.
5. The branch chat runs the repository's setup hook (the same as a branch terminal session).
