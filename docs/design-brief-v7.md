# Seshi — design brief v7: checkpoints, in all three layouts

Written 2026-10-22, for the designer. It follows [v6](design-brief-v6.md): your v6 handoff
(`docs/design/v6/`, `seshi-checks/checks.js`) is built and released in Seshi 0.37.1. This brief asks
for two things: (1) confirm or redraw how it looks in the layouts you did not draw, and (2) design the
screens that are still missing.

Same format as before: an interactive cell-grid HTML prototype with the state machine as the spec.
Draw each screen at **160×45, 100×30 and 80×24**.

## Please read first: what this is not

The Team screen (lead, workers Forge / Canvas / Lens, tasks waiting on other tasks) is the old v5
idea. It was cut and is **not** part of this. Do not draw it. Checkpoints are one thing only: Seshi
runs the project's check command after an agent's turn ends and tells the truth about the result.

## What Seshi is

A terminal app where one person runs many AI coding agents side by side, each in its own git
worktree, and only looks up when one needs them.

## The three layouts (all stay)

Seshi has three layouts a user picks in Settings. **None is being replaced.** Checkpoints appear
in all three.

| Layout | What it is |
| --- | --- |
| **Cards, floating** | Each pane is a card with its own border, with a gap between cards. |
| **Cards, tiled** | Cards sit edge to edge with no gap. |
| **Flat** | No card borders. Panes are separated by thin rules. This is the layout your v6 handoff drew. |

Corners are square or rounded, a separate setting. Every theme must keep working.

## What is built today

A project may name one check command (Settings › Check, or `[[check]]` in the config file). With none
set, nothing below appears and Seshi looks exactly as before.

When an agent's turn ends, Seshi runs the command in that session's worktree. The session reads done
only if it passes.

- **Sidebar row.** A check line under the session: *checking*, *passed*, *failed (exit 1)*,
  *failed, overridden*, *could not run*. Failed shows the first failing line of output.
- **Why popover** (`i`). Says why the session is in its state, including the check.
- **Evidence sheet.** Opens from the row. Command, exit code, when it ran, duration, the tail of the
  output. Keys inside: re-run, override (asks for a reason), undo override.
- **Inbox.** A failed check is listed beside questions and finished work.
- **Settings › Check.** The command and timeout for the project you are in.
- **Row menu.** Re-run check, override, undo override.
- **Command line.** `seshi ls` shows the check next to the status; `seshi wait` exits 4 on a failed
  check.

## Part 1 — confirm or redraw (the three layouts)

Your v6 handoff was drawn for Flat. I adapted it to the two card layouts myself, without your input.
Please draw each of these in **Cards floating, Cards tiled and Flat**, at all three sizes, and tell
me where my adaptation is wrong:

1. A session row in each state: checking, passed, failed, failed-and-overridden, could not run.
2. The evidence sheet: short output, long output, no output, and a command that printed nothing.
3. The Inbox with one failed check among questions and finished work.
4. Settings › Check: empty, set, set but never run, and a command that does not exist.
5. The why popover for a failed check.

At **80×24** the sidebar is narrow. Show what drops first. Today the second line of a row is dropped
below 140 columns; say if the check line should survive instead.

Also confirm the **plain-text fallback** (no Nerd Font): pass and fail must not depend on colour or
on a special glyph.

## Part 2 — screens still missing

Design each fully: where it lives, what is on it, its states, keyboard and mouse, and how it looks
at 100×30 and 80×24, in all three layouts.

### A. Approving a command a repo supplies

Today the command only comes from the user's own config. A later feature lets a repo ship its own
(`.seshi/checks.toml`). A command from a cloned repo must never run without approval. Draw the
moment: what is about to run, in which folder, from which file, and the choices (run once, always
for this repo, never). Make it hard to approve by reflex.

### B. A stale result

The agent changed files after the check ran, so the old result may no longer be true. Show how a
row and the sheet say so, and how to re-run from there. A stale pass must not look like a fresh one.

### C. Several checkpoints in a list

Not built yet, but the row and Inbox must have room. Draw a project with three or four, for example
*build ✓ · test ✓ · review …*, and how it narrows at 100×30 and 80×24. Kinds under consideration
after verify: **commits** (the branch has new commits), **approval** (you approve a plan), **review**
(a fresh session reads the diff). Design the shape; do not build screens around the details.

## Not asked for

- The Team / lead / worker screens.
- The working shimmer and the 3-bar spinner from v6 (not built; say if you still want them).
- The v6 sidebar foot. The sidebar keeps its `+ new` / `actions` foot and the spinner choice in
  Settings › Appearance.

## Constraints (unchanged from v6)

- A terminal grid: one character per cell, 24-bit colour, bold / italic / underline / dim only.
  Existing theme colours only; no new theme roles.
- Everything works by keyboard (leader `Ctrl+Space` + a key; bare keys when the sidebar has focus;
  Alt+arrows move between panes) and by mouse (every chip clickable, hover states). No new leader
  keys: free keys are listed in `docs/adr/0009-leader-keys-after-the-cut.md`.
- Motion is short (90–140 ms), can be turned off, and nothing depends on it.
- Nothing may hide an agent that needs you. The Inbox stays the one place that lists them.
- Model, context and cost stay out of Seshi's own chrome.
- Windows Terminal, Ghostty, Alacritty, kitty, iTerm2, on Windows, macOS and Linux.
- The contrast audit keeps passing.
