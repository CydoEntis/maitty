# Handoff: Seshi v5: cleanup pass

## Overview
This is a cleanup pass on the shipped Seshi UI. Structure, features and keys are unchanged from v4 (`design_handoff_seshi_v4`) and the flat layout (`design_handoff_seshi_flat`). v5 fixes repeated information, mixed background tones, colour roles, unclear focus and spacing. It applies to both the cards and flat layouts.

## About the design files
These are **HTML design references**: an exact cell grid. They are not production code.

**`seshi-app.js` is the spec.** v5 is a switch (`V5`, from `state.v5`); search for `V5` to find every change.

Open `Seshi v5.dc.html` in a browser, with all files in one folder:
- **v5: on / off (before)** compares v5 with the previous design.
- **Layout: cards / flat** switches the layout.
- **Size** switches between 100×30, 160×45 and 240×60.
- **Jump to** opens every state, including the new **Settings · General**.

## Fidelity
**High fidelity.** Match it cell for cell.

---

## 1. One background (the main fix)
- **The problem:** the gaps, card insides and the pane's program area were three slightly different greys. Windows Terminal acrylic or a background image showed through some of them and not others.
- **The rule:** every chrome cell uses the **terminal's default background**: gaps, card insides, sidebar and the pane area.
  - In the renderer, emit "default bg" (SGR 49), not a painted colour, so transparency and background images look the same everywhere.
  - Program output keeps its own colours; default-bg cells from programs stay default.
- **Raised tone:** only the sheet (`card`), modals and the status line are a painted, raised tone.
- **New setting:** Appearance › Background › **terminal** (the default, as above) / **solid**.
  - Solid paints everything `bg` and turns transparency off. Same layout, no mixed tones.

## 2. Spacing (cards)
| Row / column | Content |
|---|---|
| row 0 | blank margin |
| row 1 | sidebar top border; tab pills start at the pane column |
| row 2 to H−3 | pane cards (directly under the tabs, no blank row) |
| row H−2 | blank |
| row H−1 | status line |
| columns | 1-column outer margin, 1-column gap between cards (was 2 / 2) |

- **Sidebar card:** from row 1 to H−3. It has **no footer**; the status line holds those hints.
- **Sheet:** docks right of the panes with a 1-column gap, at the same height as the panes.
- **Flat:** unchanged from the flat handoff, apart from sections 3–6 below.

## 3. Each fact shows once
| Fact | Where it shows in v5 | Removed from |
|---|---|---|
| session name | sidebar row and pane title | branch text in the title |
| repo | pane title, in dim after the name | |
| agent | sidebar row meta | pane title |
| path, branch, PR, server, ⇆ tags | **status line** (selected session) | pane footer row (dropped), sidebar footer |
| model, ctx, cost | the agent's own status line inside the pane | Seshi chrome; don't repeat what Claude or Codex already print |

- **Pane content area:** gains the footer row back (content ends at h−2).
- **Pane title:** `name  repo`, then the right-aligned `state  ✕`.

## 4. Colour jobs (strict)
| Colour | Meaning |
|---|---|
| lime `acc` | focus: one lit edge and the focused title |
| amber `needs` | needs you, only |
| red `err` | failed, crashed, destructive |
| green `ok` | done, passing |
| sky | Ctrl+Space leader mode, only |

- **"Working":** a neutral `text` spinner. **Never amber.** The live app colours working sessions amber in the sidebar and the title tag today; change that.
- **CPU/RAM:** if shown, `dim` in the status line; it turns `err` above a threshold. Never amber.
- **Selected sidebar row:** a neutral tint, `mix(bg, text, 0.12)`, full width, plus a `▎` bar at its left edge.
  - The bar is lime when the sidebar has focus, and `dim` when the pane or a sheet has it.
  - It replaces the blue selection fill everywhere: sidebar, settings, lists.

## 5. Exactly one lit edge
- Only what has the keyboard gets the lime border (cards) or the lime bar (flat).
- When a sheet is open, the **sheet** is lit; the pane drops to a `line` border and its text fades 30% (was 42%).
- The sidebar selection bar goes dim at the same time.

## 6. Status line (now in both layouts)
- **Row:** H−1, on the `card` tone.
- **Left:**
  - in flat, a zone badge (`SIDEBAR` / `PANE`)
  - then the path `~\code\repo-worktrees\branch`, `⎇ branch`, `#PR ✓/✕`, the server tag and the `⇆` tag
- **Right:** `● n needs you` (amber, only when n > 0), then `j inbox   a actions   , settings   ? keys`.
- **Fitting:** the left side fills only the space the right side leaves.
  - Below 140 columns the path is dropped.
  - Wider than that, the path is truncated with `…` first; then the tags drop one at a time from the end.
  - Below 140 columns the right side becomes `● n   ? keys`.

## 7. Settings › General
- One version line: `seshi 0.34.0 · up to date · daemon and window match · u check now`. It replaces the 3-line version block and the separate button.
- Sections with dim caps headers and a rule:
  - **INPUT:** Leader key, Mouse, Keys after a pause
  - **LAYOUT:** Layout cards/flat, **Background terminal/solid**, Sidebar side, Splash screen
  - **SHELL:** Editor, Shell
- No empty sections. The help text for the selected row sits above the hint bar.
- One hint row inside the modal. The global status line is covered by the dim overlay, so hints are never doubled.

## Unchanged
Features, states, keys, copy, the text areas and the sheet/modal options from v4. The flat layout's focus zones and square ends.

## Files
- `seshi-app.js`: the spec
- `Seshi v5.dc.html`: the interactive viewer, with the before/after switch
- `seshi-grid.js`: the cell-grid renderer
- `seshi-tokens.js`: the Seshi Night colour roles
- `support.js`: runtime for the HTML viewer only
