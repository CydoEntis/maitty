# Plan: the Terminal / Chat view switch

Status: in progress (CV-0 to CV-7, CV-9, CV-10 and CV-13 done; CV-8 shell, checklist and Agents done; CV-11 and CV-12 done; the "out of scope" items have since been built) · Date: 2026-11-04 · Design: [v17 handoff §C.1, §C.2](design-handoff/v17/README.md) (refines v16 §B.3) · Spec: [spec.md](spec.md)

## What it is

One switch at the right of the tab bar. The focused pane's session is shown either
as its terminal or as a chat. Same process either way: switching never restarts
the agent, and what is sent from the chat shows in the terminal too. Both Claude
and Codex.

## What already exists

- The daemon owns structured conversations: `ConversationSend`, `ConversationRead`
  (ordered history with a cursor), approvals with ids, turn boundaries. See
  [spec.md](spec.md) and ADR 0006.
- The Claude adapter (`crates/adapters/maitty-provider-claude`) projects every
  tool call with its complete input JSON; the Codex adapter projects app-server
  items (`item/`, `turn/`). So edits, commands, to-do lists and helper agents are
  already in the history as typed items. Nothing is parsed from the terminal.
- The GUI has no chat view. All work below is in `apps/gui/`, plus small daemon
  additions where named.

## Tickets, in order

Each one is a vertical slice with its own tests (vitest for the model, a rendered
check for the view). Acceptance criteria are the bullets.

0. **CV-0 Event projection.** One GUI module that turns the daemon's history
   (messages, tool calls with input, approvals, turn boundaries) into the v17 §C.1
   event shapes: `message`, `edit{file, hunks}`, `command`, `approval`, `todo`,
   `subagent`, `turn`. Claude `Edit`/`MultiEdit`/`Write` and Codex patch items
   become `edit`; `Bash`/`exec_command` become `command`; `TodoWrite`/plan items
   become `todo`; `Task` becomes `subagent`. Pure functions, tested with recorded
   adapter output. Every later ticket renders from this and nothing else.
   **Done:** `apps/gui/src/features/chat/` (reducer `applyHistoryEntries` /
   `applyApprovals`, flatten with `selectChatEvents`; event types in `chatEvents.ts`).
   What the daemon's history carries limits the cards, and later tickets must
   not assume more (CV-13 below describes what it carries now):
   - A history cached by an older daemon has no `Tool.input`, so a Claude `Edit`,
     `MultiEdit`, `Write`, `TodoWrite` or `Task` from it still projects to a card with
     the file, diff, items or title marked unavailable.
   - Codex `fileChange` carries every file's diff, so edits are complete.
   - Timestamps exist only on live items (see CV-13); an item read back from a
     provider's saved history has none, and the GUI measures its elapsed time from
     when it first saw it.

1. **CV-1 Switch and empty chat card.** **Done:** the Terminal | Chat switch sits in each pane head (not a single
   switch at the tab bar's right: each pane has its own view), `Ctrl/Cmd+Shift+E` toggles the focused pane, the
   transition is transform/opacity only and off under Reduce motion, and Settings › General › Chat view sets the
   per-agent default. The chat card has no header of its own: the pane head already shows process, name, cwd,
   machine chip and status, so a second header would repeat it. Not built: the pane switcher and "the project's
   running agents to jump to" on the shell card. Original brief: The floating two-button switch (§B.3),
   the slide-and-fade transition (off under reduce motion), a `Settings → Claude /
   Codex` default of terminal or chat, and the chat card with its header (process,
   session name, cwd, machine chip, pane switcher, status pill with timer).
   - Switching a working agent changes nothing in the daemon (assert no command sent).
   - A shell tab shows "Chat view is for agent sessions" with **Start claude here**
     and **Back to terminal**.
2. **CV-2 History.** **Done:** `features/chat/` (`chatFeed.ts`, `useChatFeed.ts`, `chatRows.ts`, `components/`).
   A virtualised list (1,500 items draw under 40 rows), stable event ids as keys, the cursor followed as far as the
   daemon allows (see UI.md: the daemon makes a cursor stale on every change, so the feed re-reads from the start
   then and applies only entries that changed), a fresh projection on a new generation, a resume after a failed
   read without duplicates, pending approvals polled into `applyApprovals`. Cards are plain and complete; the
   polished chips, diffs and approval card are CV-4..7. Known limit: adjacent agent messages of a turn are joined,
   not all of a turn's messages (that would reorder text against its steps).
   Original brief:  Read the conversation on open and follow the cursor. Your
   messages as right-aligned bubbles; one agent message per turn with a lead
   sentence; turn dividers "Cooked for 7s · done 10:11 AM".
   - A history of 1,000 items renders without jank (virtualised list).
   - Reconnecting the daemon resumes from the last cursor, no duplicates.
3. **CV-3 Composer.** **Done in the client** (`useChatComposer.ts`, `components/Composer.tsx`): Enter sends,
   Shift+Enter adds a line, Esc or Stop interrupts, disabled while an approval is pending or the daemon is offline,
   and a minimal approval card answers by request id. **The queue is the daemon's** (G24): a message sent while a
   turn runs is held by the daemon (`ReadResult.queued`, removed with `ConversationDequeue`) and delivered when the
   turn ends, in order and once each, whether or not the window stays open; the chat shows it as a pending bubble
   with Remove. Delivery is at the end of the turn, not mid-turn. Original brief:  Enter sends, Shift+Enter newlines, Esc or **Stop**
   interrupts. While a turn runs the composer **queues** the message and says so
   ("Queue a message for when Claude finishes this step"); it is sent at the turn
   boundary. Disabled while an approval is pending.
   - Daemon: a queued send is held per conversation and delivered once, in order.
4. **CV-4 Tool steps.** **Done** (`StepCards.tsx`: name, target from the tool's input, status mark, expand to input and result). Every tool call is a compact mono chip indented under the
   message: spinner while running, green check on success, terminal glyph otherwise.
   Click expands the input and result, bounded.
5. **CV-5 Edit blocks.** **Done** (`EditCard`: relative path, counts, tinted hunks, fold at 12 lines; the header opens the panel's diff). Claude `Edit` / `MultiEdit` / `Write` and Codex patch
   items become a diff card: "Edited `RecipeStore.swift` +5", unified diff with
   added and removed lines, "23 more lines" fold past 12 lines.
   - Paths are shown relative to the session's project root.
6. **CV-6 Command cards.** **Done** (`CommandCard`: live Running time, elapsed from the provider's duration or the daemon's times, exit code for Codex and "unavailable" for Claude, last 3 lines). Claude `Bash` and Codex `exec_command` become a card
   with the command, **Running** with elapsed time, then the exit code; output is
   folded with the tail shown.
7. **CV-7 Approvals.** **Done** (answers by id, y/a/n on the focused card, `answered_approvals`, link to the tool step). The amber card with the exact command and **Yes, once (y) /
   This session (a) / No (n)**, answered through the daemon's approval id. After an
   answer the card reads "You answered: …" and the buttons are gone in every view.
   - Keys y / a / n work while the card is focused.
8. **CV-8 Checklist and session panel (§C.2).** **Removed by the revised v18 (D.6, ticket 48); the text below is history.** Was done (`features/panel/`, sidebar progress row for sessions this window has read: `Ctrl/Cmd+\` toggle, docked 320px or overlay under 1100px, checklist card, Agents card, empty states). The docked panel (right by
   default, overlay under 1100px, `Ctrl/Cmd+\`), its header, the "Nothing to
   track yet" state, and the checklist card. Claude's `TodoWrite` and Codex's plan items feed a
   side panel: items with done / current / pending marks, "2 of 5" with a progress
   bar, elapsed time on the current item. The sidebar row shows the current item.
9. **CV-9 Helper agents.** **Done in the chat** (`AgentsCard`; the panel's Agents list is the session panel work; the history carries no phase line or step count, so rows show title, type, elapsed). Claude `Task` calls (the helpers maitty already tracks)
   become a "2 agents, 2 working" card inline and an **Agents** list in the side
   panel: title, phase ("Reading EmptyState.swift"), elapsed; done ones say
   "Done · 2 steps · 5m 30s".
10. **CV-10 Working line and shimmer.** **Working line done** with live elapsed; **no token count** (the history has no usage events) and the sidebar shimmer is not built. "claude is working · 1:40 · 238k tokens"
    with the spinner under the last item; sidebar rows in *working* get a slow
    shimmer (CSS only, off under reduce motion). Token counts come from the
    provider's usage events when present, else hidden.
11. **CV-11 Line comments (§C.2 Changes).** **Done in the Inspect panel's Changes tab** (it was dropped with the session panel, v18 D.6, then came back there): a click on a diff line opens a comment box with **Send to <agent>**, and the comment goes to the agent as a message naming the file, line and text (`model/agentMessage.ts`, `features/chat/outsideSends.ts`, `features/inspect/PatchView.tsx`). See DESIGN.md › Side panel.
12. **CV-12 Files and Artifacts segments (§C.2).** **Done** (touched files with kind; artifacts are created documents, reports, data and pictures by extension, previewed as text through `conversation_file`; pictures are not previewable).
13. **CV-13 Claude parity check.** Confirm the Claude adapter delivers approvals
    with ids and turn boundaries the way Codex's does (the roadmap did Codex
    first). Fill any gap in the adapter, with fake-provider tests.
    **Done (daemon side, ticket G22):** what `read_conversation` now carries, with no
    `PROTOCOL_VERSION` bump (every new field is optional or defaults when absent):
    - `Tool.input`: the tool's own input JSON for a Claude call (edit file and text,
      `TodoWrite` items, `Task` title and prompt), and for a Codex helper-agent call the
      same `Task` shape (`description`, `prompt`, `tool`, `model`). Strings are cleaned of
      terminal escapes and cut with a `[truncated]` marker; an input too deep, too wide or
      too big is left out whole. The label is unchanged (`"Edit "` for Claude), so the tool
      name is still read from it.
    - `Tool.exit_code` and `Tool.duration_ms` for Codex commands. Claude reports neither,
      so they are absent for it.
    - `Entry.started_at_ms` and `completed_at_ms`: Codex's own times for a live item, the
      daemon's clock for Claude, turn times in whole seconds converted to milliseconds. An
      item hydrated from Codex's saved history has none (Codex reports none there).
    - Claude approvals: `item_id` is the tool-use id when Claude's request names it (the
      same id the tool step has), else the request id. An edit tool (`Edit`, `MultiEdit`,
      `Write`) is `kind: FileChange` with the diff in `changes`; any other tool is `Command`
      with `"Tool: target"` as its text.
    - `answered_approvals`: the last 64 approvals a person answered, with the decision and
      time, held in memory (they last as long as the daemon). User-input answers are never
      recorded.
    - Codex `plan` items keep their text (`Tool` of kind `plan`, label `Plan`); helper-agent
      items keep their title (`Tool` of kind `collabAgentToolCall`). Neither marks the history
      unsupported any more. A command label is cut at 2048 characters instead of 256.
    Not verified against a live Claude CLI: that its `can_use_tool` request carries
    `tool_use_id` (the code falls back to the request id when it does not).
    GUI follow-up: render `answered_approvals` and the timestamps (G17, G18).
    **Secrets (ticket 18, done):** the daemon masks token- and key-shaped strings in every history text
    (message text, tool input, command output, approval text) with `[redacted]`, and replaces the text of
    an edit or write to an obviously secret file (`.env`, keys, credential stores) with
    `[content of a secret file omitted]`, keeping the path, the kind and an `omitted_bytes` count in the
    input. A shell command that prints such a file has its output omitted the same way. The cards
    (`redaction.ts`, `StepCards.tsx`) show an omitted edit as a named file with a muted note, a masked
    value as text with a "the daemon hid it" note, never as an error or an empty diff. The rules and
    their limits are in [SECURITY](../../SECURITY.md#conversation-history-and-secrets).

## What is still missing (client and daemon)

- **A composer, approvals and a queue for a pane's chat.** A Claude or Codex agent the user typed in a shell pane has a
  **read-only** daemon conversation that follows the agent's own transcript (G23, [ADR 0003](../../adr/0003-status-and-conversation-trust.md)):
  sending, answering approvals and cancelling stay in the terminal. The composer, approvals, queue and Stop live in
  **chat sessions** instead (G25, [ADR 0003](../../adr/0003-status-and-conversation-trust.md)):
  Claude or Codex started from the desktop as a structured conversation the daemon owns, chosen with **Session: Chat**
  in New agent on a branch or **Start a Claude chat** on a shell's empty chat card (Terminal is the default). Done:
  start (in a folder or a new branch worktree), list, composer, Stop, daemon queue, approvals by id, user-input form,
  attachments, close (ends the agent). Left: resuming a chat session's agent after a daemon restart from the window,
  a chat session on another machine, and a try against a live Claude and Codex (the daemon is tested against the
  fake providers only; Codex's rollout shape is fixtures only).
- History is still polled (1 s while working), but a poll is now one small read: every page carries a `resume`
  cursor that returns only the entries changed since (G24 part 2). A push of events is not built.
- Left: token counts (no usage events in the history), a helper agent's phase line and step count (not in the
  history), the sidebar working shimmer, an external-link route (agent links copy their address, see UI.md), a
  Claude exit code (the provider reports none), and the session panel (CV-8, removed by v18 D.6).

## Out of scope

Nothing from this plan is still out of scope. Attachments in the composer, the model and
reasoning picker (Claude; Codex has no model pick yet), the Inspect panel (§5b) and find in the
conversation (plus find across sessions' terminal output) were first listed here and are now built;
DESIGN.md describes them. Still not built: search across old conversations' text
(spec.md leaves arbitrary history browsing out of scope). The session-switch rule (§C.0a)
and usage/cost (§C.7) are in the Overview spec.

## Verification

The gate in AGENTS.md, plus `pnpm test`, `pnpm typecheck`, `pnpm lint` in
`apps/gui`, plus a screenshot per ticket against the v16 prototype.
