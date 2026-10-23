# Handoff: Seshi v8 cleanup + message popover

## Overview
A cleanup pass on Seshi's TUI. It makes states readable at a glance, puts the hidden menus in plain sight, drops the "fleet" concept and merges three key-reference overlays into one. It also redesigns the copy feedback and toasts, and the message-agent popover.

Two prototypes:
- `cleanup/Seshi v8 Cleanup.dc.html` — 18 steps, the main cleanup
- `message-popover/Seshi Message Popover.dc.html` — 8 states × 3 sizes × 3 layouts × 2 glyph sets, with written rules on the page

## About the design files
These are **design references built in HTML**: a browser cell grid that imitates a terminal, so states can be clicked through. They are not production code. Build them in Seshi's real renderer using its existing draw helpers, sheets, popovers, Inbox and Settings.

The source of truth for every layout, string, glyph and colour role is the JS next to each page:
- `app8.js`: each screen is a `draw*` function writing cells, `act()` is one reducer for keys and clicks, and `PRESETS` lists the steps
- `msg.js`: the same structure for the popover

Serve each folder with a local web server (e.g. `npx serve`) and open the .dc.html. Click the grid, then use the real keys.

## Fidelity
High fidelity for copy, glyphs, colour roles, cell layout and keys. Colours are roles from the existing theme, so use the theme's values (hex values in `app8.js` `THEMES.seshi` match the current lavender theme).

---

## 1. Session rows (step 01)
Every state has its own glyph, word and colour. Colour is never the only signal.

| State | Glyph | Line 2 | Name |
|---|---|---|---|
| working | 3-bar Braille spinner, `work` (orange) | `<agent> · working` with shimmer, `· 4 agents` if reported | shimmer |
| needs you | `●` `needs` (amber) | the question, amber | strong bold |
| failed | `✕` `err` + red `▎` edge bar on all lines | `failed: test · exit 1` bold red | strong bold |
| done, not seen | `✓` `ok` | `done · not seen` bold green | strong bold |
| done | `○` `dim` | `done` dim | text |
| new | `○` `acc` | `<agent> · new · type to begin` | strong bold |

- Selected row: neutral tint plus a `▎` bar. The bar is accent only when the sidebar has focus.
- Below 140 cols, plain "done" rows collapse to one line. Rows that need attention keep line 2.
- Title row: `SESSIONS n` chip, with `●n` (amber) and `✕n` (red) counts at the right.
- **Footer:** lifted one row off the bottom, with a `line` divider above it. **No band background.** Two buttons: `new` (left, 1 col in) and `menu` (right). Each is a `btn` chip with the key letter in accent bold and the rest in `text`, with no brackets.
- **Amber is reserved for "needs you".** Don't use it for headings or anything else.

## 2. Subagents
- **Remove the subagent tree from the sidebar.** The agent already lists its subagents in its own pane, so the tree duplicates it, and not every agent reports them (Codex doesn't).
- The session row shows a count, `· 4 agents`, **only when the agent reports subagents**.
- Setting: Settings › Agents › "Count on the session row" on/off.

## 3. Menu (step 02, key `m`)
A popover above the sidebar footer, grouped with headers, with each key right-aligned:
- **NEW:** Session `n` · Session with options… `c` · Shell here `s` · Pane beside this `p` · Tab `t`
- **GO TO:** Inbox `j` · Search commands `Space` · All keys `?`
- **VIEW:** Collapse sidebar `z` · Hide sidebar `Z`
- **SET UP:** Settings `,`
- Footer line: "Ctrl+Space then a key works anywhere"

Removed: "Start a fleet". "Set up agent status" moved to Settings › Agents.

## 4. New sessions, no "fleet" (steps 09–12)
- **`n` is instant** (`+` also works): it starts a session straight away with the defaults (agent: last used, current project, new worktree). There's no form; the task is typed in the session. The row reads "new · type to begin", and its tab opens and gets focus.
- **`c` is "New session…":** a small dialog with Agent / Where / Project rows (↑↓ choose a row, ←→ pick an option, project shown as `‹ seshi ›`), plus a one-line summary ("Opens claude in a new worktree of seshi. You type the task there.") and Start (Enter) / Cancel (Esc). There's no goal field.
- **Helpers are not a mode.** Any session may start helpers. The first time one tries, its row becomes **needs you**: "wants to start 4 helpers".
  - Enter opens a popover anchored to the row, with the screen dimmed except that row. It lists the helper tasks, then "Their permission prompts come to your Inbox. Nothing merges."
  - Choices: `1` Allow · `2` Always in this project · `3` No, do it alone · `Esc` later (stays on the row).
- **Settings › Sessions:**
  - "+ starts": instantly / with options
  - Agent: last used / claude / codex / gemini
  - Where: new worktree / this folder
  - "Sessions may start helpers": ask / always / never
  - "Helpers at once": 2 / 3 / 4

## 5. Collapse sidebar (steps 13–14)
- **`z` moves sessions into the top strip.** It does not shrink the sidebar into a column of cut-off names.
  - Row 0 starts with a `»` chip (expand), then one chip per session: glyph + full name (max 16 chars), `card` bg.
  - The selected chip uses `acc` bg with `accInk` text when the strip has focus, `btn` bg otherwise.
  - When they don't fit, a `+n more` chip opens the full sidebar.
- The pane gets the full width, and the pane's tabs move down to row 1 so they stay visible.
- Strip focused: ←→ or 1–9 switches sessions.
- **`Z`** (Shift+Z) hides the sessions completely. `z` or `»` restores them.

## 6. One key overlay (steps 07, 15, 16)
Replaces the Command palette, All shortcuts (261 lines) and the old Ctrl+Space overview.
- **Ctrl+Space → key map**
  - A banner: "Every key below goes after the leader: `[Ctrl]+[Space]` then `[p]` = split right".
  - Groups: Sessions (`n c r m j i`), Panes (`p - x ←↑↓→ z Z`), Tabs (`t ] [ R`), Project (`w› f d`), Seshi (`, ? q`).
  - Pressing a key runs it. Tab, or typing an unbound letter, switches to search.
- **Search (same box):** searches every command, including ones with no key.
  - Each result shows the label with the match in accent, the group in dim, and the key on the right.
  - With nothing typed it shows RECENT. Enter runs; Tab goes back to the map.
- **`?` → All keys:** the map plus "WITHOUT THE LEADER · press directly" (Alt+1–9, Alt+arrows) and "SEARCH ONLY" (commands with no key). Both headings are in dim.
- **Clean-up rules:**
  - one key per action (drop `v`, `|`, `b`)
  - sentence case everywhere
  - no bracketed explanations; the group gives the context
- **Keycaps everywhere keys appear:** each key is a raised chip, `" " + key + " "` on `btn` bg with `strong` bold text. Combos are split on `+` and joined with a dim `+`. This applies to the map, search, hints rows, Settings, the dialogs and the copy hint.
- Layout: 3 columns at ≥100 cols wide, 2 below. The key column is sized per group, not per column.

## 7. Settings (steps 05, 06, 12)
- Tabs: General, Sessions, Appearance, Agents, Check, Keys. **The Team tab is removed.**
- Labels never truncate (label column 33 cols).
- The selected row shows its explanation at the bottom, word-wrapped over up to two lines.
- Rename "Keys after a pause" to **"Leader waits for next key"**.
- Agents tab: agent status (Claude ✓ hooks installed, Codex ✓ read from the process, "Set up another agent…"), plus the Subagents count toggle.

## 8. Message a session (step 03, v8 version)
- The title names the session and agent once, e.g. `MESSAGE tall-crane · claude`. The quote is labelled "replying to".
- See section 11 for the full popover redesign.

## 9. Copy feedback and toasts (steps 17–18)
- **Selecting** (mouse drag, or `y` in keyboard select): a sky tint with `strong` text over the selection, and hints `[y] copy · [Esc] cancel` at the selection's end.
- **Copied:** the selection turns accent-tinted for the moment, and a **toast card** appears.
- **One toast component for every Seshi notice:**
  - bottom-right, above the agent's input area (`H - h - 5`)
  - `card` bg, `line` border, a full-height `▌` bar in the notice colour on the left
  - row 1: glyph + bold title, with `✕` at the right
  - row 2: detail in `text`, e.g. "214 characters · 5 lines from swift-lynx"
  - row 3 (optional): a preview in `dim` italic
  - clears after 2.4 s, or with ✕ / Esc
- Remove the faint one-line notice in the bottom-right corner.

## Action bars
Every panel's key hints sit in their own one-row **action bar** on a slightly lifted ground: `card` for sheets on the app background (Inbox), `card2` inside modals (Settings, key map, search). The bar runs the full width of its panel, with 1 col padding, and shows keycaps plus labels (`actionBar()` in `app8.js`). The bar is always the last row of its panel, so actions are in the same place everywhere.

## 10. Narrow (step 08, 100×30)
Sidebar is 28 cols (36 at ≥140). Same rules as above; footer entries drop from the end.

## 11. Message popover redesign (separate prototype)
Full rules are on the prototype page under "Rules". In short:
- **Size:**
  - width min(68, room right of the sidebar − 1)
  - starts with **1 input row** and grows a row per line up to 6 / 4 / 3 rows (160×45 / 100×30 / 80×24)
  - past that, the top input row reads "↑ n more lines"
- **Quote:** the tail of the agent's last message, wrapped, at most 4 / 3 / 2 lines.
  - Normal text with a dim `▏` gutter, not italic.
  - Label "codex asked" (blocked) or "claude said", plus "· last 3 of 5 lines" when cut.
- **Placement:** right of the sidebar with a `◂` pointer on the row's first line.
  - If there's no room below, it flips upward (bottom border on the row's last line).
  - If neither fits, it pins to the bottom of the screen.
- **Contrast:** the screen dims except the popover and its row. A full accent border in every layout (fixes Flat), `card` ground, input on `bg`, no shadow.
- **Queued vs sent:** the popover closes and the receipt is on the session row.
  - `◷ queued "…"` in dim while the agent is mid-turn, then `› sent "…"` in accent.
  - Enter queues when the agent is working; Ctrl+Enter sends now. Enter on an empty box just closes it.
- **Keys:** unchanged. Esc keeps the draft for that session.
- **Colours:** card, bg, accent, muted, blocked (amber only for the "● needs you" title chip), text/strong.

## Keys summary
- **Sidebar:** `↑↓` select · `Enter` open · `r` reply/message · `n` new · `c` new… · `m` menu · `j` inbox · `,` settings · `?` all keys · `Space` search · `z` collapse · `Z` hide · `y` select/copy
- **Leader:** `Ctrl+Space` opens the key map; every key above works after it.

## Open decisions
- `m` now opens the menu, so messaging moved to `r` (reply), Rename tab moved to `R`, and Shell here moved to `s`. Confirm these against the existing bindings before shipping.
- Number keys 1–3 answer the helpers question on the selected row. In the collapsed strip, 1–9 also jump between sessions, so pick one rule (suggest: 1–9 jump only while the strip has focus).
- Keep `c` for "New session…" (tried as you asked), or move it to Shift+`+`.
- The Sessions, Appearance, Check and Keys tabs in Settings are placeholders in the mock.

## Files
- `cleanup/Seshi v8 Cleanup.dc.html`, `cleanup/app8.js`, `cleanup/seshi-grid.js`
- `message-popover/Seshi Message Popover.dc.html`, `message-popover/msg.js`, `message-popover/checks7.js`, `message-popover/seshi-grid.js`
- `support.js` in each folder is the prototype runtime only
