# Plan: the Terminal / Chat view switch

Status: proposed · Date: 2026-11-04 · Design: [v17 handoff §C.1, §C.2](design-handoff/v17/README.md) (refines v16 §B.3) · Spec: [spec.md](spec.md)

## What it is

One switch at the right of the tab bar. The focused pane's session is shown either
as its terminal or as a chat. Same process either way: switching never restarts
the agent, and what is sent from the chat shows in the terminal too. Both Claude
and Codex.

## What already exists

- The daemon owns structured conversations: `ConversationSend`, `ConversationRead`
  (ordered history with a cursor), approvals with ids, turn boundaries. See
  [spec.md](spec.md) and ADR 0036.
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
2. **CV-2 History.** Read the conversation on open and follow the cursor. Your
   messages as right-aligned bubbles; one agent message per turn with a lead
   sentence; turn dividers "Cooked for 7s · done 10:11 AM".
   - A history of 1,000 items renders without jank (virtualised list).
   - Reconnecting the daemon resumes from the last cursor, no duplicates.
3. **CV-3 Composer.** Enter sends, Shift+Enter newlines, Esc or **Stop**
   interrupts. While a turn runs the composer **queues** the message and says so
   ("Queue a message for when Claude finishes this step"); it is sent at the turn
   boundary. Disabled while an approval is pending.
   - Daemon: a queued send is held per conversation and delivered once, in order.
4. **CV-4 Tool steps.** Every tool call is a compact mono chip indented under the
   message: spinner while running, green check on success, terminal glyph otherwise.
   Click expands the input and result, bounded.
5. **CV-5 Edit blocks.** Claude `Edit` / `MultiEdit` / `Write` and Codex patch
   items become a diff card: "Edited `RecipeStore.swift` +5", unified diff with
   added and removed lines, "23 more lines" fold past 12 lines.
   - Paths are shown relative to the session's project root.
6. **CV-6 Command cards.** Claude `Bash` and Codex `exec_command` become a card
   with the command, **Running** with elapsed time, then the exit code; output is
   folded with the tail shown.
7. **CV-7 Approvals.** The amber card with the exact command and **Yes, once (y) /
   This session (a) / No (n)**, answered through the daemon's approval id. After an
   answer the card reads "You answered: …" and the buttons are gone in every view.
   - Keys y / a / n work while the card is focused.
8. **CV-8 Checklist and session panel (§C.2).** The docked panel (right by
   default, overlay under 1100px, `Ctrl/Cmd+\`), its header, the "Nothing to
   track yet" state, and the checklist card. Claude's `TodoWrite` and Codex's plan items feed a
   side panel: items with done / current / pending marks, "2 of 5" with a progress
   bar, elapsed time on the current item. The sidebar row shows the current item.
9. **CV-9 Helper agents.** Claude `Task` calls (the helpers maitty already tracks)
   become a "2 agents, 2 working" card inline and an **Agents** list in the side
   panel: title, phase ("Reading EmptyState.swift"), elapsed; done ones say
   "Done · 2 steps · 5m 30s".
10. **CV-10 Working line and shimmer.** "claude is working · 1:40 · 238k tokens"
    with the spinner under the last item; sidebar rows in *working* get a slow
    shimmer (CSS only, off under reduce motion). Token counts come from the
    provider's usage events when present, else hidden.
11. **CV-11 Line comments (§C.2 Changes).** The panel's Changes segment for
    this agent's worktree, the diff view, the per-line composer, the count
    marker, the comment bubble in chat, delivered as a structured comment
    `{file, line, text}` or queued while a step runs.
12. **CV-12 Files and Artifacts segments (§C.2).**
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

## Out of scope

The Inspect panel (§5b), attachments in the composer, model switching from the
composer (needs a provider API), history search. The session-switch rule (§C.0a)
and usage/cost (§C.7) are in the Overview spec.

## Verification

The gate in AGENTS.md, plus `pnpm test`, `pnpm typecheck`, `pnpm lint` in
`apps/gui`, plus a screenshot per ticket against the v16 prototype.
