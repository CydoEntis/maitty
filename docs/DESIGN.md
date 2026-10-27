# Design

Maitty's current look, in one page. It describes the visual language; the detailed rules
(sizes, what gives way, keys, every state's words) live in [UI](UI.md), and each section
below links to the part of UI.md that owns them. The code is the final word: the theme in
`src/theme.rs`, the screen in `src/client/screen/`, and the reference frames the tests compare
against (see [TESTING](TESTING.md#project-specific-layout-and-fixtures)).

## Principles

- **Status first.** Every session reads as working, needs you, done or idle at a glance, and
  that mark stays in view longest when space runs out.
- **Shape and word carry the meaning; colour backs them.** Every state reads in the ASCII
  glyph set and without colour.
- **Amber means needs you.** Nothing else is drawn in it: no heading, no decoration.
- **Only what has the keys is lit.** The focused pane, the open sheet or the selected row;
  everything else is quieter.
- **Plain words.** Sentence case, no bracketed explanations, every key shown as a keycap.

## Themes and colour roles

Colours come only from the theme's roles, never a literal. **Night** is the default: the sea
at night, a deep navy ground, sea-glass teal for focus and working, foam for names. **Day**
is the same sea by day: a pale grey-blue ground and a deep teal accent. The other built-in
themes fill the same roles; [theme sources](theme-credits.md) lists where they come from.

| Role | Used for | Night | Day |
| --- | --- | --- | --- |
| `bg` | the ground, and a terminal cell's default background | `#08111d` | `#f6f8fa` |
| `card`, `card2` | panels, header strips; fields and action rows inside modals | `#0f1b2b`, `#15253a` | `#ffffff`, `#e0edf2` |
| `btn`, `hov` | chips and buttons; the hovered row | `#1a2c44`, `#15253a` | `#e4ebf1`, `#e0edf2` |
| `line` | borders, dividers | `#1d2d42` | `#d3dce5` |
| `muted`, `text`, `strong` | quiet text, body text, names and titles | `#6f8399`, `#c9d6e3`, `#eef5fb` | `#61748a`, `#1f2f3f`, `#0b1724` |
| `accent`, `acc_ink` | focus, working, the current tab; ink on the accent | `#3dd6c6`, `#08111d` | `#0b7a71`, `#ffffff` |
| `blocked` | needs you, and only that | `#f5c542` | `#674100` |
| `done`, `err` | passed or finished; failed | `#7fd99a`, `#ff7b72` | `#2e8b57`, `#c93c37` |

`sky()` (checking, shortcut mode, copy selection) and `violet()` (could not run) are derived
from the theme, not roles of their own. Every built-in theme passes a contrast audit, and
needs you never looks like failed. See UI › *design system*.

## Type and glyphs

The terminal's own font. Bold marks names and states worth a look; italic only the splash
tagline and a toast's preview. Status glyphs come from Settings › Appearance › Status glyphs:
**Dots** (the default: a coloured `●` for what wants you, `○` at rest), or **Unicode**,
**ASCII** and **Nerd**, where each state has a glyph of its own (`✓` passed, `✕` failed,
`●` needs you, `◷` stale, `⊘` overridden, `?` could not run). Working is the spinner you
pick; checking is a sweeping ring. An agent's own mark is `✻` Claude, `◆` Codex, `✦` Gemini,
`◇` OpenCode. See UI › *sidebar check result* and *working spinner picker*.

## Layouts

One Layout setting picks the look ([ADR 0015](adr/0015-four-named-layouts-and-grid.md),
[ADR 0026](adr/0026-hairline-a-fifth-look.md)); every look has the same sessions, tabs, splits
and sheets.

- **Floating** (the default): each pane a box, with gaps between.
- **Tiled**: the same boxes, edge to edge, never merged.
- **Compact**: full-bleed zones, a header strip on row 0, zones split by one `│`.
- **Grid**: boxes with gaps, each pane in its own colour by its place in the tab.
- **Hairline**: no boxes at all, only `│` / `─` hairlines; always square
  (see *Hairline* below).

`ui.padding` leaves 0, 1 or 2 empty cells round the screen. See UI › *layout and navigation*
and *the frame*.

## Chips, rounding and padding

Anything you click or read as a control (a button, tab, tag or key) is a chip on `btn`, or on
the accent when it is current. `ui.rounded` is the one shape setting
([ADR 0021](adr/0021-one-line-rows-one-rounding-sticky-mode.md); Hairline ignores it and stays square): `off` is square, `corners`
rounds every box's corners (`╭╮╰╯`), and `full` also gives chips, buttons, selected rows and
the hover half-circle ends. Keys are keycaps: ` key ` on `btn`, `strong` and bold. The
pointer's hover is one quiet tint; selection is a neutral tint with an accent `▎` bar, never an
accent fill. See UI › *UI conventions*.

## Sidebar

`SESSIONS` and its count, with `●n` needs you and `✕n` failed at the right. Projects are folder
rows (a `−`/`+` fold button, the name, `⎇ branch` and check meta). A session is one line: its
mark, the name in its state's colour when it wants a look (shimmering while working, pulsing
when it needs you), then the agent's mark, its context past half full (`✻ 72%`, red from 85%),
uncommitted `+N −N` and the age. A second and third
line, always `muted`, appear only when they say more: a question, a check result, the first
failing line. The foot holds **+ New** and **Menu**. Collapsed, the sessions become chips along
row 0; hidden, the panes take everything. See UI › *sidebar check result* and *collapsed
sidebar*.

## Pane titles and tabs

The title sits on the pane's top border (a header strip in Compact). The top-left pane carries
the session's tabs as chips, the current one in the accent, then a **+** chip for a new tab;
other panes show their session's name. While a tab is split, every pane's number (its place in
reading order, from 1, so it changes when panes move, close or split) leads its name: `2 · claude`. On the right: the session's status mark (it
opens why) and a ✕; closing asks first, naming the pane (`Close pane 2 · claude`) and lighting
its lines in the red until you answer. In Grid a chip names each pane and its state is the glyph alone. Unfocused panes fade;
the focused border is the accent, or `sky()` in shortcut mode. See UI › *the frame* and *pane
focus*.

## Side panel

Reading while you work happens in a sheet docked right (it takes the pane column on a narrow
screen). **Files**, **Changes**, **Branch** and **References** share one sheet with a tab strip
(`Files f · Changes c · Branch b · References a`), each tab at its own remembered width: Files a
tree beside the open file (Markdown drawn), Changes the changed files and their diff, Branch the
branch, ahead/behind, recent commits, the last check and snapshots, References the project's
screenshots and design files, one line each (name, then kind, size and age muted at the right).
The lit tab names the open view, so the sheet's top border does not repeat it: Files and
Changes carry only the ✕, Branch and References their checkout's folder. The Inbox and the check evidence
sheet dock in the same place without the strip. See UI › *the frame* (Sheet), *Inbox check
groups* and *check evidence sheet*.

## Modals and popovers

Deciding is a modal: centred, framed like a pane (the title as an accent chip on the top border,
`Esc close` at the right), the screen behind dimmed, no shadow. Settings, override, command
approval and the key overlay are modals. Small explanations (why, a row's menu, the message and
helpers popovers) open beside their row with a `◂` pointer; a menu row ends in its key, the bound leader
key when there is one. Every panel ends in an action bar
of keycaps. See UI › *UI conventions*, *check modals and settings* and *why popover*.

## Toasts

One component for every notice: bottom right, above the agent's input area, a `card` with a
full-height `▌` bar in the notice's colour (`err`, `blocked` for needs you, `done`, else the
accent), a bold title, a detail line and an optional preview in italics. It slides in and fades
out with motion on. See UI › *the frame* (Toast).

## Mode bar

The one full-width row, along the bottom ([ADR 0024](adr/0024-the-mode-lives-in-a-bottom-bar.md)).
Typing: a quiet ` TYPING ` chip and the leader key. Shortcut mode: the whole row in `sky()`, a
` SHORTCUT ` chip and the next keys you can press; starting a pane ends it
([ADR 0025](adr/0025-a-new-pane-ends-shortcut-mode.md)). When something other than the panes
has the keys, the bar stays on `bg`, never sky: a calm chip (`strong` bold on `btn`) names the
holder, then its keys and the way out (`SIDEBAR · ↑↓ move · Enter open · Esc back`), with
`· shortcut mode waits` while the mode is held. Under a popup the bar is words only and takes
no click. **Update** or **Restart** sits at its right end when waiting. See UI › *the frame*
(Mode bar).

## Hairline

Less frame, more work ([ADR 0026](adr/0026-hairline-a-fifth-look.md)). Every line is one
weight in `line`; colour is kept for state, and amber still means only needs you. It draws
from the theme's roles (`hairline::Ink`: `raised` is `card2`, `dim` is `muted`, `teal` the
accent, `amber` `blocked`, `green` `done`), so every theme has it. `ui.rounded` does not apply.

- **Sidebar:** 28 columns, then a `│`. `sessions` and the count, dim. A project is a dim header
  with `⎇ branch` dim at the right, a blank row above each but the first. A session is one row:
  the glyph, the name and its age at the right, or `needs you` in amber (a split whose own
  pane is fine but whose pane beside it asks names that pane instead: `▸ warm-finch`). A
  helper steps in under the session that started it, and a request to start helpers keeps its
  amber line under the row. The focused row is
  `raised` with a teal `▎` and a teal name; a row on screen without the keys is `raised`; one
  that needs you has an amber `▎` and an amber name. The foot, pinned to the bottom row, is
  two dim words, no keys: `new` at the left, `menu` at the right.
- **Tabs:** a row of their own across the top of the pane column, then a `─` rule in `line`
  (`├` where the sidebar's `│` meets it). Each tab is plain words: its most urgent agent's mark
  (so a tab out of view still shows it works or asks), its number while the session has
  several, and its name; the current one teal bold, the others dim. A dim `+` after them opens
  a tab, there even with one tab.
- **Panes:** row 1 is the pane's number while the tab is split (bold: teal focused, amber when
  it needs you, else dim), the glyph, the name in `strong` bold, then `project · ⎇ branch` dim;
  no tabs, so the title names only its pane; at the
  right the state (`needs you · 2m` or `needs you · approve check` amber bold, `working` teal,
  `done` green, `idle` dim) and
  the ✕. Row 2 is a `─` rule the pane's width: teal focused (`sky()` in shortcut mode), amber
  when it needs you and is not focused, else `line`. Panes side by side or stacked are parted
  by one `│` or `─`, never boxed; the terminal sits two columns in.
- **Mode bar:** under a full-width `─` rule in `line` (`┴` where the sidebar's `│` meets it).
  `TYPING` is a teal bold word, then `Hairline · 2 panes` dim, then the hint;
  whatever has the keys is named in `strong` bold, no chip. At the right `● N needs you` in
  amber bold, or `all clear` dim, just before Update or Restart. Shortcut mode keeps its sky row.
- **Menus, popups and panels:** the same lines. A modal (Settings, a confirm, override, approval,
  the key map, new session, themes) is a solid `bg` box in a square `line` frame over the
  dimmed screen, its title plain words on the top border, `Esc close` dim at its right. A
  popover (why, a row's `⋯`, the message and helpers asks) is framed the same, with the `◂`
  in the accent. The side sheet is a zone like a pane: its tabs (or title) on the first row,
  a `─` rule under it lit teal under the open tab while it has the keys, and a `│` to its left.
  The toast keeps its `▌` bar and gets a hairline frame. No chip is filled: a key and what it
  does are words (the key `strong` bold), the chosen option or primary button is the accent's
  words, a danger button the red's. A selected row is `raised` with an accent marker. The
  collapsed sidebar's chips and the splash's keys are words too; the splash's wordmark stays.
- **Motion off:** working is a still `⠿`, not a frozen spinner frame.

See UI › *Hairline* for sizes, what gives way and what takes a click.

## Splash

Centred on the ground: the name as a wordmark in 5-row block letters drawn as filled cells, its
"AI" in the accent; under it the swell, one rolling `▁▂▃▄▅▄▃▂` line fading into the ground at
each end; the tagline in italics; then the choice cards (Resume, New session, Open a project),
the chosen one with an accent `▌` edge. On the first start of a version the letters rise from
the deep colour one by one and the swell fades in. Night has its own wordmark, swell and deep
colours; other themes take them from their text and accent. See UI › *splash*.
