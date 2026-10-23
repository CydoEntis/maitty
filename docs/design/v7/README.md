# Handoff: Seshi checkpoints — v7 (all three layouts)

## Overview
Seshi runs coding-agent sessions in a terminal UI. This design adds **checkpoints Seshi checks itself**: when a project has a check command, Seshi runs it in the session's worktree after each agent turn ends, and a session only reads **done** when it passes. v7 makes the design work identically in all three layouts (Cards floating, Cards tiled, Flat), at 160×45 / 100×30 / 80×24, and adds repo-command approval, stale results and several checkpoints.

A project with no check set looks exactly as Seshi does today. **No Team / specialist / profile / workflow screens** — that work is dropped.

## About the design files
These are **design references built in HTML**: a browser cell-grid imitating a terminal. Not production code. Recreate them with Seshi's real TUI renderer and patterns (sidebar rows, sheets, popovers, Inbox, Settings).

`checks7.js` is the source of truth for every layout, string, glyph, colour role and key: each screen is a `draw*` function writing cells; `act()` is one reducer for keys and clicks; `PRESETS` lists all 26 states.

## Fidelity
High fidelity: exact copy, glyphs, colour roles, cell layout and keys.

## How to view
Serve this folder (e.g. `npx serve`) and open `Seshi Checkpoints v7.dc.html`. Top bar: ← → or Next steps through 26 states; switch Layout (or "All three"), Corners, Size, Theme, Glyphs (incl. Plain text), Motion. Click the grid to use its keys. `Seshi Checkpoints v7 Notes.dc.html` holds the written answers.

## The one rule across layouts
Only the **frame** changes. Row content, line counts, words, sheet content, Inbox order and keys are identical in all three.
- **Cards floating** — bordered card per pane, 1-col gap. Title chip on the top border at x+2; right status on the top border. Content starts 2 cols in.
- **Cards tiled** — same cards, no gap (borders sit side by side `││`). Don't merge into ┬/┴.
- **Flat** — no borders; one header strip on row 0 holds every title chip; panes split by one `│`, accent when focused. Content starts 1 col in.
- **Corners** — square ┌┐└┘ or rounded ╭╮╰╯ on cards and modal bottoms.
- **Focus** — card layouts: whole border accent. Flat: title chip + left rule accent.
- **Failed edge bar** — red `▎` inside the card, 1 col in, on every line of the row; Flat: first column.

## Sidebar row
Folder header (name never truncated; meta truncates first). Each session: line 1 glyph + name + age; line 2 the check result (or question); line 3 at ≥140 cols only (first failing output line, or checkpoint names).

| State | Glyph | Line 2 |
|---|---|---|
| working | 3-bar spinner, orange + name shimmer | agent name · working |
| checking | ◜◝◞◟ sky | checking test · 6s |
| passed | ✓ ok | passed: test · 14.2s · 14:22 |
| failed | ✕ err + red edge bar | failed: test · exit 1 |
| overridden | ⊘ text | failed, overridden: test |
| could not run | ? rev | test could not run · not found |
| stale | ◷ text (not green) | stale pass: test · 3 files changed |
| never for repo | ? rev | not checked: never for this repo |
| needs you / approve | ● needs | the question / approve check: npm test |

Plain-text glyphs: `+ x / ? ~ !`. Every state has a word, so colour and glyph are never the only signal.

## Sizes (80×24: what drops first)
Sidebar 34 cols at 160, 26 at 100 and 80. Below 140 a row is one line **unless** line 2 is a check result or a question. Drop order: folder meta → age → line 3 → line 2 of sessions with no check and no question. Check lines put the word first and clip glyphs. Below 140 the sheet replaces the pane column (Esc returns). Approval modal uses short labels at narrow widths.

## Screens (26 presets)
- **01–06 Rows:** no check, checking, passed, failed, overridden, could not run.
- **07–10 Evidence sheet** (72 cols, docked right): short output ("all 13 lines"), long ("last N of 1,284 lines · f full output"), no output on fail ("(no output) · It exited 1 without printing…"), printed nothing on pass ("exit 0 is what passed it.").
- **11 Inbox:** fixed order NEEDS YOU → FAILED CHECKS → COULD NOT RUN → FINISHED; selected item shows its buttons.
- **12 Override?** modal with reason; says what it hides and where the failure stays visible.
- **13 Why (i)** popover: agent said → Seshi ran → saw.
- **14–17 Settings › Check:** empty, set, never run, command missing.
- **A1–A5 Repo command approval:** waits in the Inbox as needs you (never pops over your work); modal shows command (diff if changed), folder, file + commit, when; "Read it. It can do anything you can in that folder." No default: **Enter / y / Space do nothing**. 1 Run once · 2 Always (press twice; first press arms it) · 3 Never · Esc Not now. A changed command resets Always. Never → session reads "not checked".
- **B1–B2 Stale:** row ◷ "stale pass"; sheet lists changed files, Re-run is primary; stays in FINISHED with Re-run first.
- **C1–C2 Several checkpoints:** word first (failed → needs you → next → all passed) + glyph strip; ≥140 a 3rd line names them; Inbox shows the full list if it fits, else the strip.

## Panels vs modals
Read-while-working → side sheet (Evidence, Inbox). Decide → modal (Approve, Override, Settings). Small explanations → popover by the row (Why, row menu `a`).

## Keys
Sidebar: ↑↓ select · Enter evidence · i why · r re-run · O override · a row menu. Sheets: Esc · r · O · u undo override · f full output. Approve modal: 1 / 2 (twice) / 3 / Esc. Leader: Ctrl+Space j Inbox, Ctrl+Space , settings. No new leader keys; confirm r/O/u/f/a against ADR 0009. Mouse: every row, chip, button and ✕ is clickable with a hover tint.

## Motion
140 ms frames. Working: 3-bar Braille spinner + shimmer band across orange text. Checking: ring sweep every 2 frames. Motion off freezes both; nothing depends on motion.

## State per session
checkState: none | needs_approval | checking | passed | failed | overridden | could_not_run | stale | never. Result: command, cwd, startedAt, durationMs, exitCode, afterTurnEndedAt, output tail + line count (~1 MB cap), override {by, at, reason}, staleSince + changedFiles. Project: command, source (user | repo file + commit), approval (once | always + command hash | never), timeout.

## Design tokens (Pixls theme)
| Role | Hex |
|---|---|
| bg | #141413 |
| card (header strip, modal) | #1c1c1a |
| card2 (fields) | #262624 |
| btn (chips, buttons) | #2e2d2a |
| line | #3a3934 |
| dim | #8a867c |
| text | #c9c5bb |
| strong | #ece9e1 |
| acc (focus, working) | #ff6a2f |
| accInk | #141413 |
| needs (amber, needs you only) | #f2c200 |
| ok (passed) | #3cc27e |
| err (failed) | #ff5a52 |
| rev (could not run) | #b58cff |
| sky (checking) | #5aa9ff |

Type: JetBrains Mono 12px / 16px line in the prototype (the real app uses the terminal font). Bold = weight 700. Glyph sets: Unicode (default), ASCII fallback (`- + x / ? ~`, `!` needs, `|` bar), Nerd Font (beta, opt-in; icons must be clipped to one cell).

## Files
- `Seshi Checkpoints v7.dc.html` — walkthrough (open this)
- `Seshi Checkpoints v7 Notes.dc.html` — written answers to brief v7
- `checks7.js` — all screens, strings, glyphs, reducer, presets
- `seshi-grid.js` — cell-grid renderer
- `support.js` — prototype runtime

## Open decisions (recommendations)
- Re-run while the agent works: queue until the turn ends; row says "queued".
- Output storage: latest result per session + the overridden one, ~1 MB each.
- Override scope: this result only.
