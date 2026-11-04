# Plan: the Terminal / Chat view switch

Status: proposed · Date: 2026-11-04 · Design: [v16 handoff §B.3](design-handoff/v16/README.md) · Spec: [spec.md](spec.md)

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

1. **CV-1 Switch and empty chat card.** The floating two-button switch (§B.3),
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
8. **CV-8 Checklist.** Claude's `TodoWrite` and Codex's plan items feed a
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
11. **CV-11 Claude parity check.** Confirm the Claude adapter delivers approvals
    with ids and turn boundaries the way Codex's does (the roadmap did Codex
    first). Fill any gap in the adapter, with fake-provider tests.

## Out of scope

The Inspect panel (§5b), attachments in the composer, model switching from the
composer (needs a provider API), history search.

## Verification

The gate in AGENTS.md, plus `pnpm test`, `pnpm typecheck`, `pnpm lint` in
`apps/gui`, plus a screenshot per ticket against the v16 prototype.
