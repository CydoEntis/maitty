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

There is one look, **Floating** ([ADR 0039](adr/0039-floating-is-the-only-look.md)): each pane a
box, `ui.gap` between them. **Pane colours** (`ui.pane_colors`) gives each pane's border its own
colour by its place in the tab.

`ui.padding` is `small` by default: one empty cell all round the screen, so nothing touches the
terminal's own edge; `medium` is two, `large` three. `ui.gap` takes the same three sizes between
panes. See UI › *layout and navigation* and *the frame*.

## Chips, rounding and padding

Anything you click or read as a control (a button, tab, tag or key) is a chip on `btn`, or on
the accent when it is current. `ui.rounded` is the one shape setting
([ADR 0039](adr/0039-floating-is-the-only-look.md)): off is square, on rounds every box's corners (`╭╮╰╯`). Chips, buttons and rows have square ends either way. Keys are keycaps: ` key ` on `btn`, `strong` and bold. The
pointer's hover is one quiet tint; selection is a neutral tint with an accent `▎` bar, never an
accent fill. See UI › *UI conventions*.

## Sidebar

`SESSIONS` and its count, with `●n` needs you and `✕n` failed at the right. Projects are folder
rows (a `−`/`+` fold button, the name, `⎇ branch` and check meta). A session is one line: its
mark, the name in its state's colour when it wants a look (shimmering while working, pulsing
when it needs you), then the agent's mark, its context past half full (`✻ 72%`, red from 85%),
uncommitted `+N −N` and the age. A second and third
line, always `muted`, appear only when they say more: a question, a check result, the first
failing line. A session with two tabs or more lists them under it, one line each, flush with the
name and with no branch marks: the tab's number and name (the one on screen in the accent), and its most urgent agent's glyph and
word at the right; a click shows that tab. The foot holds **+ New** and **Menu**. Collapsed, the sessions become a narrow
column (each project its first letter, each session its mark and first letter); hidden, the panes take everything. See UI › *sidebar check result* and *collapsed
sidebar*.

## Pane titles and tabs

The session's tabs sit on the first
pane's top border, as the sidebar's `SESSIONS` chip sits on its own ([ADR 0038](adr/0038-tabs-sit-on-the-pane-border.md)),
the border running on between the chips (`ui.tab_gap`, on by default, leaves a cell of it between one tab and the next; off, they touch). A chip's ends follow `ui.rounded`: square, rounded corners, or a pill. Each chip is led by its state's glyph (on screen or
not) and ended by its ✕, the current one in the accent, then a **+** chip for a new tab. A tab
is named the same everywhere: its own name, else its focused session's. Panes are titled as an
editor's are ([ADR 0030](adr/0030-panes-titled-like-an-editor.md)): a tab's only pane has no
title of its own (its border carries the tabs), its tab naming it, showing its state (the glyph opens why) and
closing it. A split's panes have no title at all ([ADR 0031](adr/0031-tabs-name-a-project-and-splits-have-no-title.md)):
the tab names where you are, and a pane's top edge says its state, teal when focused and amber
when it needs you. The tabs list the session's tabs, numbered 1–n, each with
a ✕ on the open one only, and ends in `◫` (split) and `⤢` (zoom the
focused pane). A pane closes with Alt+x, leader `x` or its menu. Closing always
asks first (ADR 0029), naming the pane (`Close pane 2 · claude`) and lighting its lines in the
red until you answer. Unfocused panes fade;
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

The Menu and a row's context menu have no fill of their own: they are drawn on the ground like the panes. Everything else that floats over the screen is boxed as the look boxes a pane: a `line` box round
`card` with corners by `ui.rounded`
([ADR 0028](adr/0028-tab-rows-and-popups-drawn-like-panes.md)). Deciding is a modal: centred, framed like a pane (the title as an accent chip on the top border,
`Esc close` at the right), the screen behind dimmed, no shadow. Settings, override, command
approval and the key overlay are modals. Small explanations (why, a row's menu, the message and
helpers popovers) open beside their row with a `◂` pointer; a menu row ends in its key, the bound leader
key when there is one. Every panel ends in an action bar
of keycaps. See UI › *UI conventions*, *check modals and settings* and *why popover*.

## Toasts

One component for every notice: bottom right, above the agent's input area, its left side a
heavy `┃` in the notice's colour (`err`, `blocked` for needs you, `done`, else the accent), a
bold title, a detail line and an optional preview in italics. Floating draws it as a `line`
outline on the ground, always with round corners (ADR 0030). It slides in and fades
out with motion on. See UI › *the frame* (Toast).

## Mode bar

The one full-width row, along the bottom ([ADR 0024](adr/0024-the-mode-lives-in-a-bottom-bar.md)).
Typing: a quiet ` TYPING ` chip and the leader key. Shortcut mode: the whole row in `sky()`, a
` SHORTCUT ` chip and the next keys you can press; starting a pane ends it
([ADR 0025](adr/0025-a-new-pane-ends-shortcut-mode.md)). When something other than the panes
has the keys, the bar stays on `bg`, never sky: a calm chip (`strong` bold on `btn`) names the
holder, then its keys and the way out (`SIDEBAR · ↑↓ move · Enter open · Esc back`), with
`· shortcut mode waits` while the mode is held. Under a popup the bar is words only and takes
no click. **Update** or **Restart** sits at its right end when waiting, unless the sidebar shows it at its foot. See UI › *the frame*
(Mode bar).

## Splash

Centred on the ground: the name as a wordmark in 5-row block letters drawn as filled cells, its
"AI" in the accent; under it the swell, one rolling `▁▂▃▄▅▄▃▂` line fading into the ground at
each end; the tagline in italics; then, when a release is out (or installed and waiting for a restart), a
banner boxed as the look boxes a pane in the accent: `↑ maitty 0.47.0 is out` and `u update`,
which `u` or a click does; then the choice cards (Resume, New session, Open a project),
the chosen one with an accent `▌` edge. On the first start of a version the letters rise from
the deep colour one by one and the swell fades in. Night has its own wordmark, swell and deep
colours; other themes take them from their text and accent. See UI › *splash*.
