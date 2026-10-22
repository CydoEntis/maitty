# Handoff: Seshi — checkpoints Seshi checks itself (v1 design)

## Overview
Seshi is a terminal UI that runs coding-agent sessions (Codex, Claude) in a sidebar + pane layout. This design adds **one checkpoint: verify**. When a project has a check command set, Seshi runs it itself in the session's worktree after each agent turn ends. A session only reads **done** when the check passes. The result shows on the session row, explains itself, and failures reach the Inbox without burying questions.

A project with no check set must look exactly as Seshi does today.

Answers `seshi-design-brief-v6.md` (included). The earlier team / persona / specialist work is **dropped**; ignore it.

## About the design files
The files here are **design references built in HTML**: a browser cell-grid that imitates a terminal, so states can be clicked through. They are not production code. Recreate the design in Seshi's real TUI renderer and patterns (its existing draw helpers, sheets, popovers, Inbox, settings), not by porting the HTML/JS.

`seshi-checks/checks.js` is the readable source of truth for every layout, string, glyph and colour: each screen is a small `draw*` function writing cells to a grid, and `act()` is a single reducer for keys and clicks. Read it alongside this README.

## Fidelity
**High fidelity** for layout, copy, glyphs, colour roles and key bindings. Cell positions are exact at 160×45. Hex values are final for the Pixls theme; other themes reuse the same colour *roles*.

## How to view
Open `seshi-checks/Seshi Walkthrough.dc.html` in a browser (serve the folder; it loads `checks.js` as a module). A top bar steps through 18 states (← → or Next). Click the grid to focus it, then use the real keys. `Seshi Checkpoints Spec.dc.html` is the written spec.

---

## Layout (Pixls style, 160×45)
- **No outer frame, no gutters, no footer.** Panels share edges. Row 0 is one header strip (bg `card`) across the full width.
- **Sidebar**: x 0, width 34 cols (26 at 100 cols, 40 at ≥200). Header chip `SESSIONS 5`; right of header: counts `●1` (amber) and `✕1` (red) when present.
- **Pane**: starts at the sidebar's right edge (shared `│`). Header holds session tabs as chips: active `" 1 "` (accent bg) + `" fix-footer "`; inactive `" 2 "` (btn bg) + `" add-payments "`. Right of header: session state glyph + word, then `✕`.
- **Sheet** (Evidence / Inbox): docked right, **72 cols**, shares an edge with the pane. Below 140 cols it replaces the pane column; Esc returns.
- **Column divider**: single `│` in `line`; the focused panel's divider is `acc`.
- **Header chips**: `" TITLE "` on `btn` bg / `strong` text; focused panel's chip is `acc` bg / `accInk` text.
- **Modals**: centered, whole screen dimmed, 1-cell hard black shadow offset (+1,+1), `card` body, `line` hairline sides/bottom, full-width `acc` title strip with `Esc close` at right.
- **Buttons**: `" Label "` + `"key "` on one bg. Normal `btn`/`strong` with key in `acc`; primary `acc`/`accInk`; danger `btn` with label in `err`.

## Sidebar row (where the result lives)
Folder header: `▾` (acc) + folder name (bold `strong`, **never truncated**) + meta right-aligned in `dim` (`⎇ main · check: test`; meta truncates first, drops if under 6 cols).

Each session is **two rows**:
1. state glyph + name (bold) + age right-aligned (`dim`)
2. sub-line indented 4 cols: the check result

Selected: `▎` bar at left + tint (`bg` mixed 12% toward `text`) on both rows. Failed: red `▎` bar on both rows even when not selected.

| State | Glyph (colour) | Sub-line |
|---|---|---|
| working (agent) | 3-bar spinner, **orange `acc`** | name + `working` in orange with a **shimmer** (see Motion) |
| checking | ring sweep `◜◝◞◟` (`sky` #5aa9ff) | `checking test · 6s` in sky |
| passed / done | `✓` (`ok`) | `test ✓ · 14.2s · 14:22` |
| failed | `✕` bold (`err`) + red edge bar | `failed: test · exit 1` bold `err` |
| overridden | `⊘` (`text`) | `test ✕ overridden by you` — never green |
| could not run | `?` (`rev` #b58cff) | `test could not run · not found` — not a failure, no edge bar |
| stale | `✓` (`ok`) | `test ✓ · stale: 3 files changed` with `◷` |
| needs you | `●` bold (`needs`) | the question text in amber, e.g. `asks: use pnpm or npm?` |
| approve command | `●` (`needs`) | `approve check: npm test` |
| idle / asleep | `○` / `z` (`dim`) | `finished` / `asleep zzZ` |

Rule: shape + word always carry meaning; colour only reinforces. Failed (red ✕) and needs-you (amber ●) differ in shape, word and colour.

Several checkpoints (room for later): `codex · build ✓ · test ✕ · review …`; if it doesn't fit: glyphs only + failing name (`✓✕… test`).

Sidebar bottom row: `+ new   j inbox   , settings` (keys in `acc`).

## Screens
1. **No check set** — today's Seshi. No check text anywhere.
2. **Settings › Check (empty)** — modal `PROJECT · affiliate-site`. Copy: "No check set. Seshi can run one command itself when an agent's turn ends; the session isn't done until it passes. Agents keep working." Command field placeholder `e.g. npm test`. "Projects without a check look exactly as they do today." Buttons: Save Ctrl+S, Cancel Esc.
3. **Settings (never run)** — rows: Command `npm test`, Runs `after each turn ends · in the session's worktree`, Timeout `10 min ‹ ›`, Status `… never run · runs after the next turn ends`, Source `you, in settings · no approval needed`, then `CHECKPOINTS 1 of 1 · room for more kinds later`. Buttons: Save, Run now r, Remove Del, Cancel.
4. **Approve repo command** — modal `RUN A CHECK FROM THIS REPO?`: "This command came from the repo, not from you:", field `$ npm test`, In (worktree path), From `.seshi/checks.toml · added in a1b2c3d`, When `after each agent turn ends, in that session's worktree`, then "Seshi never runs a repo's command without you." / "If the command changes, it asks again." Buttons: Allow this command a (primary), Run once 1, Don't run Esc. While waiting the session is **needs you**.
5. **Checking** — see row table.
6. **Passed**, 7. **Failed** — see row table.
8. **Why? (`i`)** — popover beside the sidebar, 60 cols, header `WHY · fix-footer reads failed`. Timeline: `14:21:51 agent's turn ended · it said "tests pass"` / `14:22:05 Seshi ran npm test in its worktree` / `14:22:19 ✕ exit 1 after 14.2s · 1 of 59 tests failed`. Hints ⏎ evidence, r re-run, O override, Esc.
9. **Evidence sheet** — header `CHECK · fix-footer › test`. Result chip (`✕ failed` on `err` bg) + `exit 1`. Rows (label col 9 wide, `dim`): Command, Folder, Ran (`14:22:05 · took 14.2s · after turn 3 ended 14:21:51`), Seen by (`Seshi ran this itself. The agent said "tests pass".`). `OUTPUT` chip + `all 13 lines`, then output block on a darker bg with 5-col line numbers; ✓ lines `ok`, ✗/error lines `err`. Buttons: Re-run r, Override… O, Open session o. Hints Esc back, r re-run, f full output, i why.
10. **Long output** — tail that fits; `last N of 1,284 lines   f full output`; `▲` at top right of the block.
11. **Printed nothing** — `(the command printed nothing)` / `exit 0 is what passed it.`
12. **Inbox sheet** — fixed group order: `NEEDS YOU` (amber chip) → `FAILED CHECKS` (red) → `COULD NOT RUN` (purple) → `FINISHED` (green); chip includes count. Item: glyph + name + ` · folder` + age; line 2 the result; failed adds the first failing output line in `dim`. Selected item shows its buttons (Output ⏎, Re-run r, Override… O).
13. **Override confirm** — `OVERRIDE A FAILED CHECK`: "fix-footer will read done (overridden) though test failed (exit 1 at 14:22)." / "This hides a failure. It stays visible:" / "⊘ on the row, in the Inbox history and in the evidence" / "for this result only; the next run replaces it". Reason (optional) field. Override O (danger), Cancel Esc.
14. **Overridden** — evidence banner `⊘ Overridden by you 14:30 · u undo` + reason + "next run replaces this"; failing output stays below.
15. **Could not run** — chip `? could not run` + `exit 127 command not found`; banner "Not a test failure. The work hasn't been checked." / "Fix the command in settings (,) or install pnpm, then r."
16. **Settings: missing command** — Status `? could not run on fix-footer 14:22 · pnpm: not found (exit 127)` + "Not on the worktree's PATH. Install it or change the command."
17. **Stale** — banner `◷ Stale: the agent changed 3 files after this ran (14:24)` / "This result is about older code. r re-runs; the row says stale until then."
18. **Room for more** — three checkpoints on a row and in the Inbox, as above.

## Interactions & keys
- Sidebar (focused): ↑↓/j k select · Enter → evidence (or focus pane if no result) · `i` why · `r` re-run · `O` override (failed only).
- Sheets: Esc close · `r` re-run · `O` override · `u` undo override · `f` full output (pager) · Inbox ↑↓ + Enter.
- Leader (existing): `Ctrl+Space j` Inbox, `Ctrl+Space ,` settings. **No new leader keys.** Confirm `r`/`O`/`u`/`f` against ADR 0009 before shipping.
- Mouse: every row, chip, button and `✕` is clickable, with a hover tint (cell bg mixed 10% toward `text`).
- Re-run flow: row → checking (ring) → result; toast at bottom-right on `btn` bg, auto-clears after 2.6 s.
- Approve flow: Allow → checking → result.
- Override: modal → row `⊘`, evidence keeps failure; `u` restores failed.

## Motion
- Frame tick 140 ms.
- Agent working spinner: 3-bar Braille equalizer, 2 cells, 10 frames (`checks.js` `BARS`).
- Checking: `◜◝◞◟`, advances every 2 frames.
- **Working shimmer**: text drawn in `acc` mixed 20% toward `bg`; a highlight (`acc` mixed 55% toward white) moves one character per frame across the text, with a half-strength neighbour on each side; loop length = text length + 8.
- Motion off: spinners freeze on a fixed frame, shimmer becomes plain orange. Nothing depends on motion.

## State (per session)
`checkState`: none | needs_approval | checking | passed | failed | overridden | could_not_run | stale. Plus per result: command, cwd, startedAt, durationMs, exitCode, afterTurnEndedAt, output (stored tail + total line count; cap ~1 MB), override {by, at, reason}, staleSince + changedFiles. Project: check command, source (user | repo file + commit), approved command hash, timeout.
Triggers: agent turn end → run (or ask approval if repo-sourced and unapproved/changed); file change after run → stale; re-run → checking; override → overridden (until next run).

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
- `seshi-checks/Seshi Walkthrough.dc.html` — clickable walkthrough (open this)
- `seshi-checks/Seshi Checkpoints Spec.dc.html` — written spec
- `seshi-checks/checks.js` — all screens, strings, glyphs, reducer
- `seshi-v6/seshi-grid.js` — cell-grid renderer (from Seshi's docs/design/v5)
- `seshi-v6/app.js` — only used for the Pixls/Night theme tables
- `seshi-checks/support.js` — prototype runtime
- `seshi-design-brief-v6.md` — the brief this answers

## Open decisions (recommendations)
- Re-run while the agent is working: queue until the turn ends; row says `queued`.
- Output storage: keep the latest result per session plus the overridden one, ~1 MB each.
- Override scope: this result only.
