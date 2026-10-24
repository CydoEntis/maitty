# Maitty — design handoff (v1)

"All agents on deck." Maitty is a terminal UI for running many coding agents side by side. This package covers the **Maitty theme**, the **inspector drawer**, the **terminal dock**, and **12 proposed features**. It's a menu to discuss with the dev, not a commitment to build everything.

## About the files
The `.dc.html` files are **design references built in HTML**: a browser cell grid that imitates a terminal. They are not production code. Build them in Maitty's real TUI renderer and patterns.
- `theme.js` is the source of truth for layout, strings, glyphs, colours, keys and animation timings. `render(s)` draws every cell; `act(s, a)` handles keys and clicks.
- `splash.js` is the same for the splash screen.

To view them, serve the folder (`npx serve`) and open:
- **`Maitty Inspector.dc.html`** — the main prototype. Everything below is here. The **Show** buttons jump to each feature.
- `Maitty App Theme.dc.html` — the theme on its own (Day / Night / Midnight × 5 layouts).
- `Maitty Splash.dc.html` — the splash screen. The chosen version is "AI on deck", block letters.

Click the grid first so keys go to it.

## Fidelity
High fidelity for layout, colour roles, copy and keys. All data is sample data.

---

## 1. Theme
`maitty-theme.toml` holds three palettes with the same roles:
- **Day:** light
- **Night:** navy
- **Midnight:** neutral near-black

Role rules:
- **Accent (teal):** focus, active tab, working.
- **Brass:** project markers and the cost bar.
- **Amber:** "needs you" only.
- **Red / green:** failures and passes.

Layouts (Settings › Appearance): **Square**, **Rounded**, **Rounded · pills** (default in the prototype), **Flat · pills**, **Flat**.
- Pills use the `◖ ◗` half-circle ends on chips.
- Rounded uses the `╭╮╰╯` corners.

## 2. Inspector drawer (core, recommended)
- **Open / close:** `Ctrl+Space` then `f` files · `d` changes · `g` branch. The same key again, or Esc, closes it. There's also an "Inspect f" chip in the pane header.
- **Push, not cover:** slides in from the right over 9 frames (~0.27 s at 30 ms, ease-out cubic), and fades in. It **pushes** the agent pane instead of covering it.
- **Sidebar folds while open:** the sessions sidebar folds to an 8-col rail. `s` or `»` keeps the full sidebar.
- **Width follows what's open:**
  - tree: 27% of W
  - file: 55%
  - changes: 50%
  - branch: 30%

  It eases between sizes. Dragging the left edge (`╏` grip) or pressing `[` `]` sets the width; it's remembered per kind of view, and `=` resets. Min 32 cols; the pane always keeps at least 52.
- **Files tab:**
  - tree with M / A markers, and a teal ● on files an agent is editing
  - clicking a file opens it **beside** the tree (split view, file marked `›`)
  - under 60 cols the file takes the whole drawer, with `‹` back
  - clicking a folder folds or unfolds it
- **Changes tab:** the list of changed files with +/− counts, and the selected file's diff below. Enter opens the file.
- **Branch tab:**
  - ahead/behind counts, commits, PR status, last check
  - checkpoints (feature 6)
- **Action bar:** the bottom row of the drawer, on the panel colour, shows the tab's actions as keycaps.

## 3. Terminal dock (core, recommended)
- **Where:** under the agent pane, like VS Code's panel. The sidebar and drawer stay full height.
- **Belongs to the session:** each session has its own terminals (dev server, tests, shell). Tabs show state: ● running · ✕ failed · ○ idle · ■ stopped.
- **Hiding never stops a process.** The dock folds to a one-row strip that keeps every terminal's chip and state visible; clicking a chip reopens the dock on it.
- **Keys:** `` ` `` show/hide (in the app, `Ctrl+``) · ←→ switch terminals · `x` stop (output kept) · `r` restart · Esc hide.
- **Size:** drag the top edge (╍ grip) to resize. Default is 36% of H; min 7 rows.

## 4. Features to discuss
| # | Feature | What it does | Effort |
|---|---|---|---|
| 1 | **Radar** | A context-left meter per session (▰▰▱▱▱ 41%; amber under 35%, red under 15%); a session that needs you pulses amber. On the folded rail it becomes one bar per session. | S |
| 2 | **While you were away** | A card above the prompt: files changed, test results, and the open question. Enter opens Changes, Esc dismisses. | M |
| 3 | **Jump to file** | File paths in agent output become teal links; a click opens the drawer at that line, highlighted. | M |
| 4 | **Command palette** | `/` searches pinned prompts, sessions, files, terminals and actions; matched letters in teal. | M |
| 5 | **Send failure to agent** | A failing terminal shows "↗ Send failure to agent" (`a`), which posts the failure into the agent's prompt as a message. | S |
| 6 | **Checkpoints** | Automatic snapshots before big edits, plus manual ones (`k`), listed in Branch. Restore asks first, and later changes go to a stash. | L |
| 7 | **Cost** | Spend per session on its row; today's total against a budget above New/Menu. | M (needs per-provider usage) |
| 8 | **Approval queue** | Every agent's permission request in one amber box; `1` allow once, `2` always, `3` deny, and the next moves up. | M |
| 9 | **Handoff notes** | A finished agent leaves a short note on its row (✎); the full note opens beside the row. | S–M |
| 10 | **Watch mode** | `w` turns the pane into a calm status view: step, progress, files touched live, cost, context. | M |
| 11 | **Agent → agent handoff** | `h` sends this session's findings to another session; the target row shows "↙ from …". | M |
| 12 | **Pinned prompts** | Saved prompts at the top of the palette; Enter puts one in the prompt. | S |

My recommended order: **8 Approvals, 1 Radar, 3 Jump to file, 5 Send failure**. These have the most impact for multi-agent work. Then 4, 12, 2, 9. 6, 7, 10 and 11 need more backend work.

## Keys (prototype bindings; in the app these sit behind Ctrl+Space where noted)
- **Drawer:** `f` `d` `g` open/close · ↑↓ move · ←→ tab · Enter open · `[` `]` `=` width · Esc close
- **Dock:** `` ` `` show/hide · ←→ terminal · `x` stop · `r` restart · `a` send failure
- **Global:** Tab cycles focus (sidebar → pane → dock → drawer) · `/` palette · `w` watch · `h` handoff · `s` pin sidebar
- **Approvals:** `1` `2` `3` · Esc later
- **Checkpoint restore:** `y` / `n`

**Check for clashes:** `w` `h` `s` `/` `f` `d` `g` against the existing leader map before shipping.

## Open questions for the dev
1. Can we read per-session context %, and token cost per provider, reliably? Features 1 and 7 depend on it.
2. Can we intercept permission prompts from every agent CLI, for the approval queue (feature 8)?
3. Checkpoints (feature 6): git stash/commit on the worktree, or a filesystem snapshot?
4. Are terminal processes owned by the daemon, so they survive the UI closing (section 3)?

## Files
- `Maitty Inspector.dc.html` · `Maitty App Theme.dc.html` · `Maitty Splash.dc.html`
- `theme.js` · `splash.js` · `seshi-grid.js` (cell-grid renderer) · `support.js` (prototype runtime)
- `maitty-theme.toml` (Day / Night / Midnight palettes)
