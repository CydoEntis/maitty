# Spec: Overview, jump to next, new agent on a branch, idle updates

Intent: [intent.md](intent.md) · Status: proposed · Date: 2026-11-04

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
4. **Idle updates**: when an update is downloaded, the app is idle for 30 minutes
   (no input, window not focused) and **no agent is working**, install and
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
- Idle updates extend `features/updates/updater.ts`: the download already runs
  in the background; this adds the idle timer and the working check.

## Tickets, in order

- **OV-1** Overview model: ordering, sections, next-jump target. Tests only, no UI.
- **OV-2** Overview screen and sidebar entry with badge.
- **OV-3** Jump to next (`⌘J`) with the toast case.
- **OV-4** Context line from structured events (needs chat view CV-8 for the
  checklist item; ships with the last message's lead sentence first).
- **OV-5** New agent on a branch dialog and command.
- **OV-6** Idle updates with the working check and the setting.

## Verification and coverage

Vitest on the model for every ordering rule; a rendered check of the screen with
three machines and all six statuses; the gate in AGENTS.md. Jump is tested from a
terminal pane and from the Overview.
