# Spec: Overview, jump to next, new agent on a branch, idle updates, brain dump

Intent: [intent.md](intent.md) · Status: proposed · Date: 2026-11-04 · Design: [v17 handoff §C.0a, §C.3 to §C.8](../chat-gui/design-handoff/v17/README.md)

## Requirements

1. **Overview screen** (`⌘0`, sidebar entry with a badge of the needs-you count).
   One card per session across every machine: status shape and word (v16 §4.3),
   session name, project, machine chip when remote, elapsed time in the current
   state, and one line of context: the checklist's current item or the last
   message's lead sentence when structured events exist, else the pane title.
   Sections in order: **Needs you**, **Working**, **Done**, **Idle**, then
   **Unknown** (daemon disconnected). Within a section, creation order.
2. **Jump to next** (`⌘J` from anywhere, including a terminal pane): focus the
   next session in *Needs you* after the current one, wrapping; when none needs
   you, the next *Done*; otherwise a toast "Nothing needs you". The order is the
   Overview's, so the key and the screen never disagree.
3. **New agent on a branch**: a button in the sidebar's pinned area and in `⌘K`.
   One dialog: project, branch name (prefilled `agent/<slug>`), agent (claude,
   codex), optional first prompt. It creates the worktree through the daemon's
   existing worktree API (hooks run), opens a session there, starts the agent,
   and sends the prompt. The sidebar shows the branch name on the row.
5. **Brain dump** (`Ctrl/Cmd+Shift+N`): one text box for everything on your mind.
   An agent call splits it into proposed tasks, one row each: project, branch
   name, agent, prompt. Every row is editable or deletable before anything
   starts. **Start all** runs *New agent on a branch* once per row. A task the
   split could not place shows with an empty project for you to pick. Nothing
   starts without the click.
4. **Idle updates**: when an update is downloaded, the app is idle (§C.5: no
   keyboard input for 5 minutes, window unfocused or minimized) and **no agent is working**, install and
   restart; sessions survive because the daemon owns them. A setting turns it
   off. The toast "Updated to 0.65.0" shows after relaunch.

## Design

- One model module `features/overview/model/overview.ts`: pure functions from
  the session and machine state the app already holds to the ordered card list
  and the next-jump target. All ordering is tested here.
- The screen is a route like the existing settings view; cards reuse the sidebar
  row's status pieces so the shapes and words stay identical.
- Jump is a command in the existing shortcut table, with the pane-level rule from
  v16 §B.8 (a terminal pane passes `⌘J` to the app).
- New agent on a branch reuses the daemon calls the TUI's new-agent box uses; no
  new daemon API. The dialog's validation of the branch name matches git's.
- Brain dump is a dialog over the new-agent dialog's model: the split is one
  request to the daemon's conversation API with a fixed prompt and a typed JSON
  answer, validated at the boundary; the rows are the dialog's inputs.
- Idle updates extend `features/updates/updater.ts`: the download already runs
  in the background; this adds the idle timer and the working check.

## Tickets, in order

- **OV-1** (done) Overview model: ordering, sections, next-jump target. Tests only, no UI.
- **OV-2** (done) Overview screen and sidebar entry with badge.
- **OV-3** (done) Jump to next (`⌘J`) with the toast case. With nothing needing you the jump only toasts; it does not fall back to a Done session (v18 C.3).
- **OV-4** (partly done: the card shows what a stopped agent is asking, else the pane title; the last message's lead sentence has no GUI data yet) Context line from structured events (needs chat view CV-8 for the
  checklist item; ships with the last message's lead sentence first).
- **OV-5** New agent on a branch dialog and command.
- **OV-6** Idle updates with the working check and the setting.
- **OV-7** Brain dump (§C.6): the split call, the editable rows, Start all. After OV-5.
- **OV-8** No loading screen on session switch (§C.0a): last known screen kept
  per session, replay written in one batch, the "Catching up from <machine>"
  strip for a remote attach past 300 ms. Daemon unchanged.
- **OV-9** Usage and cost (§C.7): daemon stores provider-reported usage per
  session per day (never estimated); the page with its table, sorting, budget
  ring and the subscription note. Two tickets if the daemon part lands first.
- **OV-10** Performance page (§C.8): the live strip, the "what's costing" list
  with per-session switches, the renderer line, Copy report. With it, bring the
  perf harness up to the current transport (see `docs/PERFORMANCE.md`).
- **OV-11** Shortcuts and title bars (§C.0, §C.9): `formatShortcut`, the
  platform title bars, the Inbox move to Shift+J, the new bindings caught
  inside a pane.

## Verification and coverage

Vitest on the model for every ordering rule; a rendered check of the screen with
three machines and all six statuses; the gate in AGENTS.md. Jump is tested from a
terminal pane and from the Overview.
