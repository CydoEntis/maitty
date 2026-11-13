# Design

Maitty's current look, in one page. It describes the visual language; the detailed rules
(sizes, what gives way, keys, every state's words) live in [UI](UI.md), and each section
below links to the part of UI.md that owns them. The code is the final word: the theme in
`crates/maitty-terminal-client/src/theme.rs`, the screen in `crates/maitty-terminal-client/src/client/screen/`, and the reference frames the tests compare
against (see [TESTING](TESTING.md#project-specific-layout-and-fixtures)).

Most of this document describes the TUI. The desktop app's look is in
[Desktop app](#desktop-app) at the end; its code is `apps/gui/src/features/workspace/`
(tokens in `theme/themes.ts`, styles in `workspace.css`).

## Principles

- **Status first.** Every session reads as working, needs you, done or idle at a glance, and
  that mark stays in view longest when space runs out.
- **Shape and word carry the meaning; colour backs them.** Every state reads in the Unicode
  glyph set and without colour.
- **Amber means needs you.** Nothing else is drawn in it: no heading, no decoration.
- **Only what has the keys is lit.** The focused pane, the open sheet or the selected row (a selected row's ground is the theme's accent tinted into the surface it is on, `SELECTED_ACCENT`, so it is seen on dark and light alike; in the GUI a terminal selection is the solid accent);
  everything else is quieter.
- **Plain words.** Sentence case, no bracketed explanations, every key shown as a keycap.

## Themes and colour roles

Colours come only from the theme's roles, never a literal. Dark and light are two remembered
slots (Settings › Appearance) and a switch; **Maitty Night** (id `night`) is the default dark theme and
**Maitty Day** (id `day`) the default light one, and the browser lists each side on its own tab. **Maitty Dark** (`maitty-dark`) and **Maitty Light** (`maitty-light`) are the same accents on a standard black and white ground, from the file's `dark` and `light` sections. They are
Sea-glass and Brass, built from `crates/maitty-settings/maitty-tokens.json`, the one file the colours are
read from: brass is the accent, teal only ever means working, coral only ever means needs you, and the AI in
the wordmark is gold. Night is a deep teal-black ground; Day a pale sea-glass one. The other twelve built-in
themes (a dark and a light each of GitHub, One, Catppuccin, Solarized, Gruvbox and Tokyo Night, listed with Maitty's four in
`crates/maitty-settings/themes.json`) fill the same roles; [theme sources](theme-credits.md) lists where they come from.

| Role | Used for | Night | Day |
| --- | --- | --- | --- |
| `bg` | the ground, and a terminal cell's default background | `#05131a` | `#e9f0ed` |
| `card`, `card2` | panels, header strips; fields and action rows inside modals | `#0a2329`, `#092229` | `#f8fbfa`, `#d6e9e6` |
| `btn`, `hov` | chips and buttons; the hovered row and the open session's row | `#0b2a30`, `#092229` | `#d6e9e6`, `#c3dedb` |
| `line` | borders, dividers | `#17383c` | `#b9ccc5` |
| `muted`, `text`, `strong` | quiet text, body text, names and titles | `#8fa8a1`, `#c2d3ce`, `#eef5f2` | `#344845`, `#22393b`, `#0b1f22` |
| `accent`, `acc_ink` | focus, the current tab; ink on the accent (brass) | `#e3a72f`, `#07161a` | `#8a5a00`, `#f8fbfa` |
| `working` | working, and only that (teal) | `#3dd6c6` | `#09605a` |
| `blocked` | needs you, and only that (coral) | `#ff6a4a` | `#9e2e12` |
| `done`, `err` | passed or finished; failed | `#7fd99a`, `#e0245e` | `#439261`, `#c4003c` |

The day brass fill is too pale to read as text or carry an ink, so by day the accent is the file's `brassText`.
The error red is not in the file; it is kept apart from the coral of needs you. Day's `done` is the file's green
text lifted a third toward its green fill, because the file's own green and teal text colours are too close to
tell apart by colour alone.

`sky()` (checking, copy selection) and `violet()` (could not run) are derived
from the theme, not roles of their own. Every built-in theme passes a contrast audit, and
needs you never looks like failed. See UI › *design system*.

## Type and glyphs

The terminal's own font. Bold marks names and states worth a look; italic only the splash
tagline and a toast's preview. Status glyphs come from Settings › Appearance › Status glyphs:
**Dots** (the default: a `●` told apart by colour alone, amber needs you, green done, red problem,
`idle` colour at rest, `┆` for an unreachable machine) or **Unicode**, where each state has a
shape of its own (`●` needs you, `✓` done, `✕` problem, `○` idle). Working is the spinner you
pick; checking is a sweeping ring. An agent's own mark is `✻` Claude, `◆` Codex, `✦` Gemini,
`◇` OpenCode. See UI › *the sidebar* and *working spinner picker*.

## Layouts

There is one look, drawn to the TUI redesign: the sidebar and the workspace, `ui.gap` apart (one
column by default), and the status bar under them. Every pane is a box of its own with round
corners; a split is two to four boxes a column or row apart. Every label on a border is a title
chip. Exactly one box, the focused pane's, is in the accent.

`ui.padding` is the blank space inside the terminal round both frames: `small` none, `medium` (the
default) 1 row and 2 columns, `large` 2 rows and 4 columns. See UI › *layout and navigation* and *the frame*.

## Chips, rounding and padding

Anything you click or read as a control (a button, tab, tag or key) is a chip on `btn`, or on
the accent when it is current. `ui.rounded` is the one shape setting: `off` is square, `corners` (the default when on) rounds every box (`╭╮╰╯`) and keeps chips as square-ended full-background rectangles, and `full` also ends chips, tabs, buttons, titles, rows and the hover in half-circle pills (a Nerd Font glyph). Keys are keycaps: ` key ` on `btn`, `strong` and bold. The
pointer's hover is one quiet tint; selection is a neutral tint with an accent `▎` bar, never an
accent fill. See UI › *UI conventions*.

## Sidebar

Each machine with its sessions under it, on the panel ground, 34 columns by default (24 to 60
resizable). On the top border the ` maitty ` chip at the left and ` « ` (hide) at the right; hidden,
a ` » ` chip beside the panes brings it back. Filtering is a command (`/`) with its own modal,
no field; the folder button groups sessions by folder. In folder view each folder has a coloured bar and icon (the theme's pane colours in turn) and a line before the next folder. A machine is a heading chip with its count (` This computer 5 `), an accent ` + `
chip (a shell there at once) and a green `●` at the right while it is up. While its app is missing
or on another version the ` + ` gives way to an accent ` install ` or ` update ` chip, the fix one
click from where it shows; the Machines dialog marks that row ` u install ` / ` u update ` beside its
state. A session is two lines:
the status mark and the name (bold and bright for an agent with something going on), then `■` in
the project's colour, the project and the branch in violet. No status words anywhere, except
`limited 16:30` (`limited Sun 16:30` on another day) in amber on a session that hit its plan limit. Agents come
first, then shells, each in the order they started, so a row never moves when its status changes.
The open session has the row ground and an accent `▎`. An agent's subagents follow it: one still
running for as long as it runs (after its parent's turn too), one finished in the current turn for a
few seconds; `└` and a mark each, or under the open session one row each with `├`/`└` and the name,
and a quiet `└ 3 done · show` line that lists the finished ones (`· hide` folds them again). A subagent's row
opens its own steps in a modal ("subagent name": what it was asked, `▸` what it ran, what it said, the newest that
fit, asked again every 1.5 s while it works); Enter goes to its parent's pane. Another machine's opens the pane. Under the
open session, after its subagents, the workflow runs it reported that are still going (ADR 0010), one line
each: the step's mark (`·` waiting, the spinner working, `✓` passed, `✕` failed, `↺` sent back, `●` needs you
in amber, `–` skipped), the ticket and the step (`├ ● MAITTY-43 spec`), then `└ 2 runs done`. A run's line
opens its steps in a modal ("run MAITTY-42"): each step's mark, name, note and helper.
On the bottom border the release chip (` ↑ v0.15.0 ` when a release is out, else the version) and
` menu `, each opening a menu upward. See UI › *the sidebar*; the Machines dialog and its states
are in UI › *Machines dialog*.

## Pane titles

Every pane's top border carries its **title chip** at the left: the status mark, the name in bold,
(`mark name`), the accent with dark ink while the pane has the keys, and ` × ` at the right. Its
folder and branch are quiet chips on the **bottom** border (cut with `…`); a click on them opens the
Project menu, where Files and Changes are. Labels are capitalised. The program starts on the row under the border: an agent's pane is its own
terminal, and the TUI draws no chat of its own. With no pane open, a dashed box says `No pane open`
over an ` n new session ` chip. A pane closes with Alt+x, leader `x` or its `×`, with an undo, and a
running agent asks first, its box red until you answer. Unfocused panes fade; the focused pane's
box is the accent, or the KEYS yellow (`blocked`) in key mode. See UI › *the frame* and *pane focus*.

## Side panel

Reading while you work happens in a panel that slides in from the pane's right edge, over
its box, the pane's top line, its chips and its own ` ✕ ` included, so the panel's ` ✕ ` is the only close
at that corner (about 220 ms, eased out): the panel ground, a line down its left edge, its title chip
(` files `, ` changes N `, ` branch `, ` references `, lit while it has the keys) and a ` ✕ ` that
closes it (Esc too). Files starts 96 columns wide and at most 80% of the panes, a tree beside the
open file (Markdown drawn); Changes 54 and at most 70%, the changed files and their diff; Branch the
branch, ahead/behind, recent commits, the last check and snapshots; References the project's
screenshots and design files. The panes keep their size under it. The panels draw no key-hint row
of their own; the status bar shows their keys. The check evidence sheet docks beside the panes
(and over the sidebar below 100 columns). The Inbox is a panel 62 columns wide over the
workspace from the sidebar's side, with a card for each session that needs you, finished or exited. See UI › *the frame* (Sheet), *Inbox check
groups* and *check evidence sheet*.

## Modals and popovers

Everything that floats over the screen (menus, Settings, the keys help, confirms) is a box in the accent
on the panel ground, corners by `ui.rounded`; what sits on its top border stands on the ground
behind. Deciding is a modal: centred, its title as an accent chip on the top border (` settings `), a ` × `
chip on `btn` a cell in from the corner, no shadow. Settings has its sections down the left and the
chosen section's rows at the right (label, then the value bold in the accent with `‹ ›` and `2/3`), and a
footer with its keys and where it is saved. The keys help is three columns of keys. A menu has no title:
one row per item, the leader key at the right in the accent, the item that ends something last in red.
Small explanations (why, the message and helpers popovers) open beside their row with a `◂` pointer.
See UI › *UI conventions*, *Menus* and *check modals and settings*.

## Toasts

One component for every notice: bottom right, above the agent's input area, a small floating
pane: a box on the plain ground (no fill of its own) in the notice's colour (`err`, `blocked` for needs you, `done`, else the
accent) with a kind chip carrying the notice's mark on the top border (`▲ Needs you`,
`✕ Error`, `✓ Done`, `i Hint`, `✓ Notice`) and the modals' red × chip at its right, then a bold
title, a detail and an optional preview in italics. Every text row wraps at the toast's width
instead of being cut: the title and the detail onto up to three lines, the preview (an excerpt) onto
up to two, the last line ending in `…` only past that; the box grows with the lines. A question waiting on you also has its
two answers as buttons on the bottom border: a green `Answer` that goes to the session and a
red `Later esc` that dismisses it. Always round corners. It slides in and fades out
with motion on. See UI › *the frame* (Toast).

## Status bar

The one full-width row, along the bottom, on the panel ground. It is hidden while keys go to the pane
(no row, no gap; the focused frame turns the KEYS yellow in key mode instead) and shows for key mode,
for whatever holds the keys, for a flashed line and while Update or Restart waits. Typing with it up: a ` PANE ` chip in the accent,
and at the right `^space keys  alt+↓/alt+↑ sessions  ^space ? help` (a key that needs the leader is shown with it, since the bare key would be typed into the pane; the session keys need none).
Key mode: a yellow ` KEYS ` chip and `n new  s split  x close  tab pane  ? help  esc back`, each a click
that presses it; starting a pane ends it. When something other than the panes has the keys, a calm chip
(`strong` bold on `btn`) names the holder and its keys sit at the right with the way out last
(` SIDEBAR ` … `↑↓ move  enter open  n new  esc back`), with `shortcut mode waits` while the mode is held.
Keys are bold in the accent, their words muted. Under a popup the bar is words only and takes no click.
**Update** or **Restart** sits at its right end when waiting, unless the sidebar shows it at its foot.
See UI › *the frame* (Status bar).


## Splash

Centred on the ground: the name as a wordmark in 5-row block letters drawn as filled cells, its
"AI" in the accent; under it the swell, one rolling `▁▂▃▄▅▄▃▂` line fading into the ground at
each end; the tagline in italics; then, when a release is out (or installed and waiting for a restart), a
banner boxed as the look boxes a pane in the accent: `↑ maitty 0.47.0 is out` and `u update`,
which `u` or a click does; then the choice cards (Resume, New session, Open a project),
the chosen one with an accent `▌` edge. On the first start of a version the letters rise from
the deep colour one by one and the swell fades in. Night has its own wordmark, swell and deep
colours; other themes take them from their text and accent. See UI › *splash*.

The splash is the wordmark on the plain ground, with nothing round the screen, and under it the
START box (choices and recent projects) and the WHILE YOU WERE AWAY box (agents by urgency and the
machines) side by side. A smaller terminal shows less of the same splash: START alone with the
counts over it when there is no room for both boxes, then no wordmark when there are too few rows.
Only one too small for the START box draws the cards.

## Desktop app

The desktop app is drawn to the redesign handoff (`design_handoff_maitty_redesign`, "Maitty Desktop.dc.html",
Maitty Night first). One model: **sessions live in the sidebar, an agent opens as a chat, a shell opens as a terminal,
and a terminal for an agent exists only when you ask** ("New terminal"). Every session is still a shell that
maittyd owns; Claude Code and Codex sessions have a chat view ([chat view plan](features/chat-gui/chat-view-plan.md)),
every other agent and every plain shell stays terminal-only. The prototype is the reference for exact sizes, shadows
and timings; the feature stylesheets beside their components carry them (`sidebar.css`, `pane.css`, `menu.css`,
`dialogs.css`, `features/chat/chat.css`), and `workspace.css` the rest.

- **Glass cards on a flat ground** (the redesign handoff, `design_handoff_maitty_redesign`). The window is the theme's
  `--bg`, with two cards on it: the **sidebar** (264px, 12px from the window's edges) and the **workspace**, 12px
  beside it. Each is `--main` under a faint top-down highlight (`rgba(--hl, .035)` to `.012` over 120px), a 1px
  `rgba(--hl, .09)` border, radius 14, a 1px inner top highlight and a soft drop shadow (`0 10px 30px -12px`).
  `--hl`/`--hk` (the highlight colour and strength) and `--sd`/`--sk` (the shadow) are theme tokens: white and black at
  full strength on dark themes, the theme's own ink, fainter, on light ones (`themeStyle` in `theme/themes.ts`).
  Popovers, the palette, Settings and dialogs blur what is behind them and cast `--sh-pop`. **Background glow**
  (Settings › Appearance, `backgroundGlow`, `data-glow`) is off by default. A terminal sits on the card's own ground:
  xterm's background and ANSI black are `--main`, so a program that paints with black leaves no box. On a web view
  without WebGL 2 (`data-gpu="software"`) the same rules use cheaper values: no blur, a 1px ring for the popover
  shadow ([PERFORMANCE](PERFORMANCE.md)).
- **The notification card (`features/notifications/NotificationCard.tsx`).** 340px wide, radius 14 (18 in Glass), padding
  12/12/12/14. A 26px app icon with its 11px status badge, the session's name (13px, 600) with what happened beside it
  ("needs you" amber, "finished" green, "failed" red; 12px, 500), a 22px close, the body (12.5px, `--fg2`, code in 12px
  mono), then 28px buttons (the answer filled in the card's colour) and "just now" (11.5px, `--dim`). In Glass it has its
  own heavier glass: `.09` to `.035` highlight over `--main` at 88%, a `.13` hairline, a 22px blur and a
  `0 18px 48px -12px` shadow. Toasts sit 96px below the top edge and 36px in from the side, clear of the title bar's
  controls. The window ground is opaque on every platform (no Mica), as in the redesign.
- **One material, three roles (`apps/gui/src/material.css`).** The two finishes and the four styles are told apart in
  that file and nowhere else. Every surface is a *panel* (sidebar, main card, inspector), a *float* (menus,
  popovers, dialogs, the palette, find, toasts) or a *field* (the message box and its tray), and takes its fill,
  hairline, highlight, shadow and blur from that role's `--m-*` tokens. The `glass` finish is the base for **Glass**, which
  then lays its own translucent panels on top (below); the finish's own recipe is opaque `--main` cards on the window ground. A panel
  has a faint top highlight (`rgba(--hl, .035)` fading to `.012` over 120px on the sidebar, a flat `.018` on the
  workspace card), a `rgba(--hl, .09)` hairline, an inner top edge and a `0 10px 30px -12px` shadow; a float has a
  stronger highlight (`.07` to `.03`), a `.12` hairline and a `0 18px 44px -12px` shadow, with a 14px blur behind it; a
  field sits between (`.075` to `.035`). The **flat finish** (the Squared, Flat and Retro Pixel styles; not to be mixed up with the Flat style): solid fills, a hairline, no blur, no
  highlight (`--gk` is 0, which turns off every gradient and lift that multiplies by it; hover tints stay), one plain
  shadow on floats. A new surface is added to a role's list in `material.css`; it does not set a background of its
  own. A popover opens through `PopupContent` (`components/ui/popup.tsx`) or carries `mx-pop`, so it is portalled into
  the window root and takes the float role. **Inside a surface**, nothing reads the theme's solid colours directly: a
  sunken control is `--m-well`, a raised or current row `--m-raise`, a picked row `--m-sel`, a hovered one
  `--m-hover`, a dialog's own ground `--m-pane`. The flat finish maps them to the theme's `--well`/`--raise`/`--sel`/`--hover`/
  `--pane`; Glass maps them to tints of the light (`--hl`) and shade (`--sd`). Tints are written
  `rgba(var(--hl), calc(.06 * var(--hk)))`, never a fixed white. The hairline uses `--hl` (ink on a light theme).
  `src/material.test.ts` fails when a stylesheet reads a solid surface colour or a fixed white. Left on purpose:
  terminal grounds (`--term`, `--main` under a terminal), scrims, brand and status colours, the switch knob, and the
  legacy `App.css` shell outside the window root. **Background glow** (Settings › Appearance, `backgroundGlow`) is the
  only tint behind the cards, and Glass always has it, painted on the whole window so no seam shows under the title bar.
- **Window style: four shapes, any theme** (Settings › Appearance › Look › Window style, `style`; the root carries
  `data-style` and `data-finish`). A style is shape and type, never colour: whatever theme is in use is what every style
  wears. **Glass** (the default: frosted panels with 22px corners — mostly opaque, a milky sheen, a strong blur (28px, saturate 140%) and a bright edge, so what is behind them is only a soft wash of colour — over an atmosphere of three blobs (the theme's accent, blue and magenta) drawn from the theme; the terminal is see-through by default so its text sits on the frost,
  Inter and JetBrains Mono; a saved `frosted` reads as Glass), **Squared** (id `mono`: Space Mono everywhere, square corners),
  **Flat** (id `console`: IBM Plex Sans and JetBrains Mono, hairline borders, a marker on the selected row) and **Retro Pixel**
  (Pixelify Sans, square corners, 3px borders, hard offset shadows, scanlines). Layout and features never change. A style is data
  in `theme/windowStyles.ts` (fonts and the glass dials `--hk`/`--sk`, per dark and light side); `withWindowStyle` lays it over the
  theme in use and touches no colour. **Components are built once and read tokens, never the style.** Besides the
  base tokens, the shared pieces read the component tokens declared in `material.css`: `--c-sel-*` (every selected
  state: a sidebar row, a settings nav item, a chosen segment, a palette row), `--c-field-*` (dropdown buttons, pickers,
  the new-session segments, inputs), `--c-track-*` (the tray behind a segmented control), `--c-sw-*` and `--c-knob-*`
  (the switch), `--c-tile-ring` (agent and shell tiles) and `--c-head-bg` (sticky headers). Glass uses the
  defaults; Squared, Flat and Pixel override them. What tokens cannot say (square corners, Pixel's borders and
  scanlines, Glass's panels, no blur outside Glass) is the one `[data-style]` block at the end of
  `material.css`. `data-finish` is derived (`finishOf`): Glass is `glass`, the others `flat`. The style is
  the desktop app's own: it is not in the shared `window_style` setting, which stays Flat or Glass for the config file. A
  saved Flat or Tiled reads as the Flat style. A new control gets tokens first and is checked in all four styles and both
  themes (Settings › Appearance shows each style live). **Icons** (Settings › Appearance › Icons, `iconColor`,
  `data-icons`): `color` keeps app icons in their own colours; `grayscale` makes them neutral `--raise` tiles.
- **Title bar.** 32px with no background of its own, so it sits on the window ground (`--tbar` is `transparent`): the
  18px logo, "Maitty" in 12px `--muted`, and the platform's window buttons (46px captions on Windows).
  There is no activity bar, no middle column and no tab bar: the window is the sidebar and the workspace.
- **Small windows.** The window is never narrower than 720px (`minWidth`). Under **1100px** the sidebar is the
  60px rail, and its expand button opens the full sidebar as an overlay from the sidebar's side, over a scrim;
  picking a session, a click on the scrim or Esc closes it. Under **800px** the panes stack in one column. The
  widths are `COMPACT_WIDTH_PX` and `NARROW_WIDTH_PX` in `model/windowSize.ts`.
- **Settings dialog** (⌘,). A section list at the left and, at the right, a fixed header (the page title,
  a one-line description and a 30x30 **×** icon button, tooltip "Close (Esc)", no label) above a body that scrolls; the page
  itself never scrolls. The title and description slide with the page. Under Advanced, **Performance** shows live
  frame, drawing, output and process figures with a **Copy report** button, and a switch for a corner readout over
  the window ([PERFORMANCE](PERFORMANCE.md#measuring-in-the-app)).
- **Keyboard shortcuts** (⌘/ or Ctrl+/, also in the palette). An 860px dialog over a dimmed scrim: "Keyboard
  shortcuts", "Change them in Settings › Hotkeys" and a close ×, then an auto-fit grid (240px minimum) of the groups
  (Sessions, Panes, Find & jump, Window, and In chat: Enter, Shift+Enter, Y, A, N, Esc), each row 32px with its keys as
  small 22px keycaps. The window's rows come from the same table Settings › Hotkeys reads (`hotkeyRows`), so the two
  never differ. Other shortcuts are ignored while it is open; ⌘/ and Esc close it.
- **Menus** (`components/MenuPopup.tsx`, `menu.css`). One look for every right-click and ⋯ menu: 228px or wider,
  5px inside, radius 12, glass like the cards, a dim 11px heading that names what it is for, 30px rows of 13px text with
  their key at the right in mono, separators between groups. A danger row is red; one that stops something running
  is handled by the app, which asks in a dialog first (see Close ends the session). Arrow keys, Home and End move, Esc or a click outside closes and
  gives focus back. **Session**: Open, Open in split right, Open in split down (not for the one in front), Open
  terminal here (agents), Rename… (F2), Copy folder path, Get a file… (another machine's sessions), Show changes
  (agents), Stop session or Close shell.
  **Subagent**: Open, Open in split right, Copy summary, Stop subagent (disabled: the agent stops its own subagents).
  **Machine**: Install Maitty or Update Maitty first (a machine that needs it), New agent chat here, New shell here, Collapse or Expand, Group by project or Show as one list, Machine
  settings…. **Pane** (⋯ or a right-click in the pane): Split right, Split down, Changes and files, Pin terminals
  under chat or Move terminals to side panel (agents); then Rename…, Maximize pane, Show chat or Show terminal, and New
  terminal (agents) or Clear and Copy folder path (shells); then Close pane, only when there are several. **Close pane** on one pane of several closes only the pane when its
  session has work in it (an agent, a chat, a shell running a program): the session keeps running and stays in the
  sidebar (`useClosing`, `removePane`). A plain idle shell, or the only pane, ends its session as before, with Undo.
- **Status bar.** A 28px line at the bottom of the main area under a hairline, 11.5px muted, one line.
  The left is empty, except "Reconnecting to maittyd…" in amber with a dashed dot when the daemon is
  unreachable (the sidebar foot says "Service stopped" or "Restart needed" beside a red dot). The right holds quick controls: a **theme** menu (Dark, Light or System, a few themes of
  the kind in use, and "All themes in Settings…"), the **window style** (the four styles), the terminal
  **text size** (−, the size which resets it to 14px, +; Ctrl+= and Ctrl+− do the same) and the shortcuts
  keycaps, which open the shortcut overlay. Hidden in Focus mode and by Settings › General › Window ›
  Status bar (off by default).
- **Sidebar** (`components/Sidebar.tsx`, `sidebar.css`; layout 2e, `design-handoff/v19`). Agents only: no fleets, ships
  or tabs. At the top, two 32px rows: **New chat** (starts an agent chat on this computer; the arrow beside it offers
  New agent chat, New shell and Add a remote machine…, and Collapse sidebar sits at the end of the row) and **Search**
  with its keys (opens the command palette; there is no filter field). Under them two quiet 28px buttons, each opening a
  short list: **All projects ▾** narrows the list to one project (each with its count), and **Group by ▾** picks the
  headings, **Status** (the default), **Project** or **Machine** (`sidebarGroup`, also Settings › Sidebar ›
  Group sessions by). **By status** (`statusSections`): **Needs you** (amber, "waiting on you"; an agent that stopped
  is here too, and so is one held by its plan limit), **Working** ("running now"), **Finished** (green, "ready to review") and **Idle** ("nothing to do"; a
  session the daemon cannot read is here), each a 28px heading with its count, its few words
  and a caret that folds it; a section with nothing in it is not listed, and rows keep creation order within one. **Terminals** (`terminalSessions`) is pinned over the sidebar's foot in the status and the project list, outside the agents' scrolling list (a third of the sidebar's height at most, then it scrolls on its own): every session with no agent in it (a shell, or a program running in one), as created, each row saying its project, so agents are never sorted among dev servers. A terminal that needs you is listed under Needs you until it no longer does; the machine list keeps terminals under their machine. A row
  of another machine says `· machine` after its project (the nickname given to it, else its name; the tooltip has its
  name); a row of this computer names no machine. **By project** (`projectSections`): one heading per project
  by name, with its colour square, count and needs-you count; a project on another machine gets its own heading,
  `name · machine` with the line's dot. In both, a machine that needs the app is listed under the rows with its
  **↑ Update** (or Install) button or its percentage, since no machine heading carries it, and a draft chat is the
  first row of the list. **By machine**, this computer first: an uppercase 10.5px heading with a 12px caret (`--accent`; a project's caret is `--ansi-blue`), a laptop or server mark, the count, a
  group-by-project toggle, the line's 5px dot and a `+` (a click starts an agent chat on it, a right-click opens the full menu); clicking the heading's name folds the group. Within a
  machine the rows are in the **Session order** setting (`sidebarOrder`): as created, or with what needs you first.
  While the new-chat page is open, a draft **New chat** row (placeholders that pulse) is listed last on this computer,
  so the draft is never missing from the sidebar; it goes when the chat starts and the chat's own row takes its place.
  Grouped by project inside a machine, each project is a folding heading (its colour square, name, needs-you count,
  count). A row whose status changes glides to its new place over 420ms, and the headings and rows it passes glide with it, so
  nothing in the list snaps (`useFlip`); a new row slides in from the left. The moved row is tinted with the accent for a moment
  (`useStatusFlash`); Reduce motion turns both off.
  **Put away** (`splitPutAway`) is the last heading of the scrolling list in every grouping, folded until opened, with
  its count and "still running": the sessions put away from their menu, terminals included, as created. A row there has
  an arrow (**Bring back**) in place of the hover ×; a put-away session that needs you is listed under Needs you instead.
  The rail leaves put-away sessions out.
  A **row** (40px, radius 8) is the 20px app tile with its status badge
  A pointer resting on a row for 450ms shows a **hover card** beside it (`RowHoverCard`, 290px, level with the row and
  kept inside the window): the name, project and branch, what it is doing in the daemon's own words, its agent,
  changes, what set its status and how long ago (**Set by**: `hook: Stop, 3 s ago`, the daemon's `TermInfo.status_why`; no row
  when it gave no reason, the status is not known, the session stopped, it is limited or it waits on you to start helpers; a long reason is
  cut like the line above) and last activity. It takes no pointer input and goes on leave, click, scroll or a menu.
  (spinner, amber `!` square, solid green check, solid red cross, grey pause square for limited, grey square with a sleeping "zz" for idle, dashed square for unknown), the name (13px; 600 and `--fg` for an agent that is
  doing anything), and under it the project's 6px colour square, the project and `·` and the branch in mono
  `--ansi-mag`; a stopped agent says `stopped` and an exited shell `exited` at the right, in `--danger`, instead of a
  badge. An agent the daemon holds until its plan limit resets is **limited**: its row says `limited until 14:05` there in
  `--muted` (the daemon's `TermInfo.resume_at`, in this computer's time; `limited until Thu 14:05` when that is not today), and it goes back to the daemon's status when that
  time comes or the daemon clears it. Only that time makes a session limited, and only one the daemon calls idle or done: one that works, asks or failed keeps its status. A limited session is listed under the sidebar's Needs you and nowhere else that says needs you: the jump, the tray, the counts and the notices leave it out, because it waits on the limit and not on you, and its limit ending is not a new finish. Any other agent row has, at the right in 11px `--muted`, how long it has been in its state (`now`, `4m`, `3h`,
  `2d`; the daemon's `TermInfo.since`); it gives way to the close button on hover. A finished agent's menu has **Mark as
  done**, which files it under Idle without opening it (`MarkSeen`). A shell's tile is a flat `--raise` square with a teal `>_`. The selected row is a raised glass pill. The row
  is named by the name you gave it, else a shell's folder, else its agent. **Rename**: double-click, F2 or the menu
  turns the name into a field; Enter or leaving saves, Esc cancels, empty clears. Project colours are five tokens
  (`--proj-1` to `--proj-5`: teal, blue, pink, violet, green), chosen from the project's name, never amber or red.
  **Workflow runs** (the daemon's `workflows`, ADR 0010, `model/workflows.ts`, [brief](features/workflow-view/brief.md)): under
  the selected session, after its subagents, one row per run still going (its step mark, ticket key, and the step it is on in
  quiet text), then a quiet "N runs done · show" line that lists the finished ones. A run opens in the session's pane in
  place of the chat, as a subagent's chat does: the bar ("Run · MAITTY-42 · Add export", Back to main session) and its
  steps, each with a mark of its own shape (`·` waiting, spinner working, `✓` passed, `✕` failed, `↺` sent back, `●` needs
  you in amber, `–` skipped), its note, and the helper doing it, a link to that subagent's chat where one of the session's
  subagents has that name.
  **Subagents** (the daemon's `subagent_activity`, `model/subagents.ts`): under a session that is not selected, an
  elbow line and one 14px tile per subagent with a corner mark (teal spinner working, amber square needs you, green
  check done), no text; under the selected session, a 28px row each on a guide line. Clicking one opens its own chat.
  The foot holds the update slot (or "Reconnecting to maittyd…"), the Recent sessions clock and the Settings gear; the rail's foot has the same two buttons. The edge drags (208 to 440px;
  double-click resets). The **rail** of a narrow window is the starters, then one tile per session.
- **Replies are typed in.** An agent's reply types toward the text that came in (`useTypedText`), so a chunk eases in instead
  of flashing: a few whole words every 40ms at about 300 characters a second, faster while a lot is waiting, so a
  long reply that arrives whole takes a few seconds and a short one a moment (`nextShown`). A reply that arrives after the history has loaded types from its first letter even when it arrives whole; what
  the history already held shows at once (`knownRows.ts`), and Reduce motion shows everything at once. A helper agent reporting back (the harness's `<task-notification>` block, which the history keeps as a
  user message) is one quiet line, "Agent "name" finished · 11s · 50.5k tokens", never the markup; once it has reported,
  its row leaves the sidebar. One that still needs something stays; one that stopped without finishing stays while the
  agent works, for up to two minutes after its last report, and goes when its turn is over (`listedSubagents`). Under a session that is not selected they are a row of
  small icons; under the selected one, a row each. Both are kept mounted and folded with `Collapse`, so selecting
  another session eases one shut and the other open (0.2s) instead of the sidebar jumping by their height. The daemon
  takes a helper's own report that it finished as final, whatever second its other reports were stamped with, and only
  a start of its own brings a finished helper back, so one whose last news arrives late is not listed as running again
  and then as stopped (`apply_hook`).
- **Panes change places with a glide.** Every change to which panes are shown (a split, a pane closed, a session opened in a
  pane, a saved arrangement, maximize, a swap by drag) goes through a View Transition (`movePanes` in `paneTransition.ts`): only
  the browser's pictures of the panes move, cropped and never stretched, so text never reflows on the way; a pane that leaves
  fades out and one that arrives fades in. Without View Transitions, with Reduce motion, or on the Linux web view (where a swap
  by drag was reported to take the window down in 0.72.0), the change just happens.
- **Toasts are cards.** The notification card is the one design; a plain toast (Sonner) is laid out the same way: icon tile,
  title and detail, the close at the top right and the action on its own row under the text, 96px below the top edge and 36px
  in from the side.
- **A remote machine that needs the app** has no row of its own: its dot turns amber and an amber **↑ Update** (or Install)
  text button sits in its header before the folder button. While it updates, a mono percentage replaces the button and a
  2px accent line runs under the header (indented 26px left, 10px right); both go when it ends. The percentage is the stage
  the update has finished (probed 5, found 15, build fetched 55, sent and verified 85, reachable 95), not a byte count: the
  download and upload have none to report.
- **A chat that starts does not jump.** The message you send from the new chat page is drawn in the conversation at once,
  where it will stay, with "claude is starting…" under it; there is no queued row above the box and no bar that comes and goes. When the agent reports in and the real chat takes over, the message stays where it is while the history is first read (no loading skeleton in between), and the tray already says Worktree or Branch.
  The needs-you card is raised by the window from the status change as well as by the daemon's own alert (one card per
  session, so they never stack). The theme's tokens are on the page root, so a menu or popover that renders outside the window
  reads the same colours and is never see-through.
- **Typing while an agent works.** A chat run by the daemon queues what you send, and the daemon sends it when the turn
  ends. A chat that follows an agent in a terminal does the same: the message is left with the pane's daemon
  (`queue_follow_up`, the queue the terminal app uses; shown under the box in the queue, removable and editable through
  `drop_queued_follow_up`) and the daemon types it into the agent's prompt, one at a time, when a turn ends; nothing is
  typed while it is asking something. The window types none of them, so a queue goes on with the window closed, and
  the chat draws a message as sent when it leaves the front of the daemon's queue. While messages are still queued
  for a session (`SessionView.queued`), a turn that ends raises no "finished" notice or sound: the session has not
  finished. What a box holds (unsent text, files) is kept
  by conversation number and belongs to its session: when the daemon gives the session's chat a new number, it moves
  to that number (`followComposer`).
- **Settings pages.** ESSENTIALS: **General** (start and export folders, window, updates, sessions) and **Chat** (how a
  chat starts and is drawn: default agent, view, width, text size, full access, Yolo) and **Agents** (the account each
  of Claude and Codex is signed in to, as the agent's own command says, with **Sign in**, which opens the agent's own
  sign-in in a terminal, **Sign out**, and **Check again**). TERMINAL: **Terminal** (text,
  scrollback, and Panes: where an agent's terminals open, see-through), **Appearance** (look, surfaces, density and pane
  title text, motion) and **Sidebar** (grouping, order, position, icon and text size, width). WORKSPACE and ADVANCED
  are as they were. Each page has a letter that jumps to it (G, C, E, T, A, S, U, N, M, ?, D, P).
- **Chat size.** Settings › Chat › Chat view has **Chat width** (`chatWidth`: Auto, Narrow, Wide, Full) and **Chat
  text size** (`chatTextSize`: 90 to 140%). Auto is the 760px column on a small pane and grows with a large one, to 62%
  of the pane and 1100px at most, so the chat is not a strip on a big screen; the message box is as wide as the
  messages. The text size draws the whole chat larger or smaller (`zoom` on `.mx-chat`), cards and message box included.
- **Slash commands.** In a Claude chat, a slash as the first thing in the message box opens the agent's commands over
  it (`SlashPicker`, the shared `ResultsList`): its own, and the person's and the project's commands, skills and
  plugins, as Claude Code itself lists them for the session's folder (`agent_commands`, asked as the chat opens and
  again after five minutes), each with what it does and what it takes. Typing narrows them (the exact name, then names
  that start with it, then a plugin's command by its own name, then names that hold it); ↑ ↓ move, **Tab** completes
  the name, **Enter** or a click runs a command that takes nothing and completes one that takes something, **Esc**
  puts the list away until the text changes. The list goes once a space ends the name. A one-line message that
  starts with a slash is typed into a terminal agent instead of pasted, so it is read as a command. Claude Code leaves
  the commands that only its terminal runs out of that list (`/help`, `/permissions`, `/resume`, `/add-dir`…), so they
  are written by hand in `terminalCommands.ts` (from Claude Code 2.1.322) and offered, tagged `terminal`, to an agent
  that has a terminal: running one types it there and shows the terminal, since its menu is the terminal's to draw,
  and it is refused with a toast while the agent works. A listed command that opens a menu (`/model`, `/config`)
  still shows that menu in the terminal, not in the chat.
- **The tray under the message box** is one line (`SettingsTray`). At the left: the folder, the branch in mono in
  the theme's magenta with the branch mark (or the tree and the worktree's blue; the tooltip says which), then the
  permission mode. At the right, what the agent reported using (`UsageMeters`, `usageStrip.ts`): the context as a
  small ring with its percent, then each plan limit as a short bar with its percent and window (`49% week`,
  `12% 5h`). From 75% a meter is amber and from 90% red. When a limit starts over is in its tooltip. Only what the
  agent reported is drawn (ADR 0009); a chat session with no pane has none yet. Tokens and cost stay in Settings ›
  Usage & cost.
- **Codex's commands.** A Codex chat has the same picker. Codex cannot be asked what it takes, so its commands are a
  list the window keeps (`codexCommands.ts`: names from OpenAI's command reference, descriptions from Codex CLI
  0.167.0). The few whose result the conversation shows are sent from the chat; the rest open a picker or a page in
  Codex's terminal, are tagged `terminal`, and run there with the terminal shown. A Codex command is typed and Enter
  follows 400ms later (`typeMessage`), since Codex loses an Enter sent with it. Codex does list its **skills**
  (`agent_commands` asks its app server, `skills/list`), so those are offered too, tagged `skill` and written `$name`
  as Codex's own box takes them: beside the commands after a slash, alone after a `$`. Picking one leaves `$name ` in
  the box to write the message around; a skill is never sent by itself.
- **Queued messages.** Each waits as a dashed card over the box whose header reads **Queued** (and `1 of 3` in an accent
  pill when several wait; the tooltip says when it is sent) with Edit (a pencil) and Remove (×) at its right. Several
  stack with the next one to be sent in front, in the accent's line; the `1 of 3` is what says more wait behind it,
  and nothing sits beside the stack. A click anywhere on the card spreads them all out. For an agent in a terminal
  the header also has **Send now** (an up arrow): it types the message into the turn that is running, as typing in the
  terminal does, so the agent reads it at its next step; it is off while the agent asks something, since typed text
  would answer the question. A chat the daemon runs has no such button: it takes a message only when the turn ends.
- **Pane** (`components/PaneCard.tsx`, `pane.css`). Agents and shells share one 42px header: the 26px app icon with its
  status badge, the name (15px, 600) over the folder (11px `--dim`) and the branch (mono 10.5px `--branch`), then at
  the right, icons only (layout 2e): the terminal | chat switch (agents with a chat), split right, the terminals
  drawer (only while terminals are pinned under the chat), the **side panel** button (a green dot while the
  repository has changes, the counts in its tip), ⋯ and close. Split down and Maximize are in ⋯ and on their keys.
  The switch is a sunken tray (`rgba(--sd,.3)`, a 1px `rgba(--hl,.1)` border, radius 9) whose thumb slides by
  `transform`. As the pane narrows the header gives up, in order (`model/paneHeader.ts`): split right under 440px,
  the switch under 360px; what is gone stays in ⋯ and the title keeps its room. A lone pane is no
  card; with two or more each pane is a faint card (`rgba(--hl,.018)`, `rgba(--hl,.08)` border, radius 14) and the
  focused one gets a teal ring drawn above its content (`--accent-line` plus a 3px accent halo). A pane's status is the
  badge on its icon; there is no status chip in a corner. While broadcasting, the header says "◉ Broadcast".
- **First-run welcome.** On the first launch (no saved settings and no earlier welcome) a 560px dialog opens
  over a blurred scrim with a progress bar per step: **Welcome** (the same sessions as the TUI, closing never
  stops an agent, and the **New sessions start in** folder, checked before it continues) and **You're set**
  (six keycaps, then **Start a session**). Skip and Esc leave it; global shortcuts are ignored while it is
  open. A window that already has settings is never greeted. Settings › General › **Show the welcome again**
  replays it. The "Bring your TUI setup" step comes with the shared config.
- **Interface size.** Settings › Appearance › Interface size sets the sidebar icon size (Small 0.8, Medium 1,
  Large 1.2 times the session icons), the density (Compact 0.8,
  Comfortable 1, Spacious 1.25 times the row and pane spacing), the sidebar text size and the
  tab text size (12, 13.5, 15 px). They are CSS variables on the window root (`--pad`, `--sidebar-fs`,
  `--tab-fs`, `--icon-scale`), so every size rule that reads one follows it. The docked sidebar's right edge is a
  drag handle (`role=separator`, 208–440 px, default 264): the width follows the pointer and is saved
  on release; arrow keys step 16 px, Home or a double-click resets. The collapsed rail is fixed at 60px.
- **Terminal text.** Settings › Terminal sets the size (12 to 16), the font family, the normal weight
  (Light 300, Regular 400, Medium 500, Semibold 600) and the bold weight (Semibold, Bold 700, Extra bold 800).
  The family is one font name typed or picked from the monospace fonts this computer has (the
  webview cannot list fonts, so a short list of well-known ones is measured against generic families); empty is
  the bundled Geist Mono, which also stands behind a missing font. Names are limited to letters, numbers,
  spaces and `. _ + -`, and a stored value outside that, or a weight outside the lists, reads as the default.
  The settings are `look` fields on the terminal (`fontFamily`, `fontWeight`, `fontWeightBold`, `lib/terminalFont.ts`)
  and are applied to open terminals in place, followed by a refit, never by recreating them. Regular is 400 by
  default; Medium is the fix when Geist Mono looks thin in WebKitGTK on Linux.
- **What a session is using.** The agent the daemon detects (claude, codex, deepseek, ...), else the
  program running in it, else `shell` for a bare shell. Claude, Codex, DeepSeek, Gemini and opencode
  show their own mark on a tile in their own colours, the same in every theme; cargo and rustc have
  the Rust tile; any other name gets its first letter on a colour taken from the name. The marks are
  the LobeHub icon set's (MIT) and stay their owners' trademarks; they say which agent runs, nothing
  more.
- **Anything with a `title`, and every icon-only button, radio or switch**
  (by its `aria-label`), shows a themed tooltip (`HoverTips`), not the browser's: 420 ms after the
  pointer rests, 40 ms when another tip was on screen in the last 400 ms; hidden on a press, a key
  or the wheel; held back while a menu or a rich hover card is open. A trailing `(keys)` becomes
  keycaps and text after ` · ` a muted second line. It sits above the control, below it within 56px
  of the top, and beside it for a control marked `data-tip-side`. A control that opens its own hover
  card carries `data-hovercard` and gets no tip. A working row shows no run time, because the window only
  knows when it first saw the change.
- **Layouts are local, sessions are the daemon's.** A session's split layout (one to four panes in columns, each
  naming a session by id, each column with its relative width and each pane its relative height) is kept in this
  window only, in `localStorage` under `maitty.gui.layout.v3` (`model/layout.ts`, `model/layoutStorage.ts`). Choosing a
  session in the sidebar shows the layout it is in, else one of its own. Split right adds a column beside the focused
  pane, taking half its width; split down a pane under it, taking half its height; they mix. Right-click a pane for its
  menu.
  One pane fills the workspace card; two or more are **separate cards** on the window's ground with a gap between them
  (the workspace card drops its own fill). The layout itself never eases: a pane that is added, closed or resized has its final size at once, because
  easing a split made the terminal re-fit every frame and its text jumped. What the eye sees between the two layouts is the glide below,
  drawn from pictures; dragging a divider has no glide. A terminal pane, header included, is one
  surface (`--main`), with no band where the header ends.
- **Close ends the session, asking first when there is work to lose, with Undo.** The pane's **×**, the pane menu's
  Close session, a row's Close session and Ctrl+Shift+W (⌘W) first open the confirm dialog (`ConfirmCloseDialog`:
  "Close name?", Cancel and a red Close, or "Stop claude" for a running agent; Enter confirms, Esc cancels) unless every
  session in question has already ended or is a shell with nothing running in it, which Undo brings back as it was.
  Closing sends the daemon `CloseSession`: the session leaves the list at once and
  its process runs on for 6 seconds. A toast "Closed name" with **Undo** stays for the same 6 seconds;
  the button or ⌘Z (Ctrl+Shift+Z elsewhere; passed to the terminal when nothing was just closed, and left to a
  text field inside one) sends `UndoClose` and puts the pane back in its place. An agent's own terminals (its
  "New terminal" tabs) end with it and come back with it on Undo; the notice names the agent, not the count. A chat session closes the same way
  (`ConversationClose`): the daemon archives it at once and stops its agent when the 6 seconds pass; Undo
  (`ConversationUndoClose`) brings it back running. Only closing the window leaves sessions running.
- **An agent that ends by itself takes its pane with it.** A pane the desktop app started as an agent runs the agent
  inside a shell, which is left when the agent goes (`/exit`). The daemon ends that pane three seconds after the agent's
  own hook said its session was over (`close_ended_agents`, `AGENT_ENDED_GRACE`), never on a process scan that missed
  the agent, and keeps it when an agent is back by then (`/clear`, `/resume`) or the shell has been typed into since.
  The window takes the pane out of the layout instead of leaving it as an ended one, and ends the terminals the agent
  opened for itself (`useAgentExits`). A shell the person opened and typed an agent into stays a shell; so does a
  terminal app's pane; and a pane that dies with its agent still in it stays as an ended pane, since it may show why. Before
  it closes one, the daemon looks at what runs in the pane: anything but a shell keeps it.
- **An agent run by the pane's agent is not the pane's.** When an agent starts another of its kind as a command (a
  headless `claude -p`), that one's hooks report a session starting and ending. The daemon tells it by the processes
  above the reporter (`scan::reports_from_nested_agent`: two programs of the agent's name on the way up to the pane)
  and ignores its reports, so the pane keeps its own session, status and chat. An agent that runs under another
  program's name (`node`) cannot be told this way.
- **A dialog only the terminal shows still stops the chat.** Claude Code opens choosers no hook reports (its "We
  recommend compacting earlier" prompt). The daemon reads them off the screen (`crates/maitty-agent-detection/rules/
  claude.toml`, Maitty's own rules beside the vendored Herdr ones) and says the pane needs you, so the chat shows
  "claude is waiting for you", its box is off until it is answered in the terminal, and nothing queued is typed into it.
- **Hotkeys are keycaps.** Settings › Hotkeys draws each key as its own key-shaped cap (a thicker
  bottom edge, symbols ⌘⇧⌃⌥ in the interface font, other keys in Geist Mono), with a dim "or"
  between alternatives. Windows and Linux show Ctrl, Shift and Alt as words.
- **One key table.** `features/workspace/model/shortcuts.ts` holds every window shortcut once
  (`BINDINGS`); the key handler, tooltips, menu hints, palette, keycaps, Settings › Hotkeys and the
  overlay all read it through `formatShortcut(binding, platform)`. The glyphs are never rewritten in
  the DOM. On macOS the window uses ⌘ and leaves Ctrl to the terminal. On Windows and Linux a plain
  Ctrl+letter always goes to the terminal, so the window uses Ctrl+Shift (T, W, D, F, K, I, B, N,
  J ...); where two actions want the same Ctrl+Shift letter the second takes Ctrl+Alt+Shift (split
  down, broadcast). Ctrl with a non-letter (`,` `.` `/` `[` `]` `=` `1`-`9` Tab) is the window's
  too. A binding with `available: false` (Undo close) is registered but
  its key does nothing and the overlay hides it until the feature lands.
  Jump to the next session that needs you is ⌘J on macOS and Ctrl+Shift+J elsewhere: a bare Ctrl+J is a terminal key.
- **Terminal | Chat switch.** An agent's pane header carries a two-button switch (terminal glyph, chat bubble). The
  chat covers the pane body while the terminal stays mounted underneath, hidden by `visibility`, so it keeps streaming
  and keeps its size: switching sends nothing to the daemon, except that going to the terminal moves the text waiting
  in the chat box into the agent's prompt (one paste, no Enter; `model/draftCarry.ts`), under the message box's rule: only while
  the agent is idle or done, and not when files are attached. Text typed in the terminal's prompt does not come back to the box. **An agent opens as its chat**; a shell opens as a
  terminal, and stays one when an agent is typed into it (the view switch still shows its chat): only New chat opens a chat.
  Every session on this computer, agent or shell, has terminals of its own in its folder
  (`components/AgentTerminals.tsx`): a pill tab each with a status dot, **+** for another, × to end one. The first
  opens by itself the first time they are shown; closing the last leaves a note with **New terminal**.
  They are tabs of the agent, not rows of the sidebar; which shell is whose is the window's own memory
  (`data/agentTerminals.ts`), so the terminal app lists them as sessions, and one whose agent is gone is a row again.
  They show in the side panel's Terminal tab, or pinned under the agent's chat (Settings › Terminal › Panes ›
  Terminals open in, the layout buttons (the drawer icon pins them under, the side-panel icon moves them back), or
  the pane menu): a glass card of its own 8px under the pane, with a centred handle to size it (120 to 640px, at most
  60% of the two together; `model/terminalDrawer.ts`), folding to its 42px header (the **Terminals** button or ×),
  and starting folded in a window under 760px tall. The card's height eases over .28s; the terminal inside stays
  mounted at its open size and is only uncovered, so its text is never fitted again on the way. A shell's terminals
  stay in the side panel: a shell is never given a card under it.
  Settings › Chat › Chat view picks the view each of Claude and
  Codex opens in (Chat by default). A shell, an unsupported agent or a session on another machine shows the chat
  card's empty state, "Chat view is for agent sessions", with the reason and the ways to start one.
  Changing the view, or switching between sessions, plays no animation.
- **Chat session ([ADR 0003](adr/0003-status-and-conversation-trust.md)).** A row and pane like any session, with the
  agent's mark and its branch line. Its pane has no terminal | chat switch; the pane is the chat. The approvals
  (y / a / n), user-input form, Stop and the daemon's queue are the chat's own.
- **Chat body** (`features/chat/chat.css`). A 760px centred column, 20px in, rows 14px apart, under a 56px fade at its
  top. The scroller is as wide as the column, so its scrollbar sits at the column's edge and not the pane's, and the jump-to-latest button is centred over the column; the wheel still scrolls the chat from the empty room beside it. A table's cells break between words, never inside one, and its headings stay on one line (a wide table scrolls sideways). Your messages are right-aligned glass bubbles (a `rgba(--hl,.1)` to `.06` gradient, a `rgba(--hl,.1)` ring,
  radius 16/16/5/16, 14px, 82% wide at most). An agent's reply has no avatar: its first sentence in `--fg` at 15px, the
  rest in `--fg2` at 14/1.65 as rich text (headings 20 / 16.5 / 14, tables in a rounded scroller, inline code as a chip,
  a file's name as a **file badge**, a code block as a card with its language, **Copy** and "Show N more lines").
  **A step** is a 30px row: a 16px mark (the spinner in `--working` while it runs, a green check, a red × tile, a grey
  square when stopped), a verb at 13/600, what it acted on, and how it ended at the right in 12px; it opens to its
  detail, indented 33px. A file is a badge (type tag, folder in `--dim`, name in `--fg`, `:line` in the accent). A
  command opens to a glass card (radius 12) with the command, the last 3 lines of output on a darker well and "Show
  all N lines"; an edit opens to its diff with the hunk header in `--ansi-blue`, added lines tinted with `--ok` at 13%
  and removed with `--danger` at 12%, syntax colours (keywords `--ansi-mag`, strings `--ansi-green`, comments
  `--dim`, numbers `--ansi-blue`), folded past 12 lines. A Markdown file the agent wrote is a **document card** (title,
  first lines, **Open**, **Copy**), not a diff. **A run of steps** is one summary row over a **stack** of its latest
  commands and edits: up to 3 cards 80px tall, each one behind 9px lower, 5% smaller and fainter, showing only its
  edge; the front card has a `--working` glow while it runs and a `--danger` edge when it failed. A click opens every
  step on a left rail (0.3s `cubic-bezier(.2,.8,.2,1)`); a failed step sits on a `--danger-soft` ground. **The plan**
  is a card: "2 of 5 done", a 72px bar, done items struck through, the current one on `--accent-soft`. **Helper
  agents** are rows (the agent's tile with a status badge, name, type, what it was asked); several share a card, "3
  agents · 1 working · 2 done". A request answered earlier folds into one line, "Allowed", "Allowed for this session"
  or "Denied", and the step a pending request is about is the only step in amber, "waiting for you ↓". A turn ends
  with a divider ("Finished in 3m 40s · 10:46", red when it failed) and, when it edited files, the changed-files
  card. The working line is the spinner and "claude is working · m:ss". Problems with the history (offline, incomplete, a provider error) and Full access show in one
  slim strip above the list. **A subagent's chat** replaces the list with a bar ("Subagent · name · time", **Back to
  main session**), then the helper's own steps drawn as the chat's rows (read from the record its agent keeps,
  every 1.5 s while it works, "Its earlier steps are not shown." above them when the start was cut). Where there is
  no record (another machine, an older service, another agent) it shows what it was asked, what it is doing while
  it works and its report once done.
- **Message box** (`features/chat/components/Composer.tsx`), in three layers. **1. The question tray**, only while the
  agent asks: a slightly narrower tray rising from behind the box (`mxRise`, 0.28s). A request is tinted amber: "Needs
  you" and "Wants to run a command", the command in a sunken mono box, and **Yes, once** `y` (filled with the accent),
  **This session** `a`, **No** `n`, and for a command request **Always in this project** (the daemon then approves that exact command in that project without asking; see ADR 0005). Settings → Agents lists these under "Commands approved for a project" (`ApprovedCommands.tsx`), each with **Forget**. A question the agent asks is tinted with the accent instead, its choices as rows
  that light up when picked. y, a and n answer from anywhere in the chat but a field. Answering removes it, the box
  unlocks and the agent is working again (its row and the badge follow the daemon). **2. The box**: glass
  (`--main` under `rgba(--hl,.075)` to `.035`, a `rgba(--hl,.12)` border, radius 16), a two-line text box on top, **+**
  (attach) at the left, and at the right the model picker ("Claude Sonnet 5.5 · Medium ▾": the agent's mark, the model
  picked or the one its own file says, the effort) and the round 32px send button, teal when it can send and grey while
  a question waits. While the agent works and nothing is typed, the send button is the Stop square, still (the line over the
  box says the agent is at work, so the button does not spin); just after a send, before there is a turn to stop, the
  square is there and off.
  Claude's model chip and mode chip are always there, from the first frame of a new chat: while the agent starts or works they
  stay where they are, greyed out with the reason as their tooltip, so the box never gains or loses a control. A chat that
  follows another agent (Codex, Gemini) draws its model label from the start, with just the agent's name until it says its
  model. The "agent has not reported" hint of a starting chat floats over the conversation and moves nothing. Placeholders: "Message claude", "Queue a
  message for when claude finishes" while it works, "Answer the request above first" (and off) while a question
  waits. **3. The settings tray**: narrower and darker, under the box: the permission mode from the agent's own file
  ("Ask before edits", "Plan mode", "Bypass permissions" in red, or "Permissions" before it has said; its menu is headed PERMISSIONS with the agent's name and has a row a mode, each with a line saying what it does; a new Codex chat has Read only, Auto and Full access instead). The tray reads project folder (opens Files) first, then the mode, and at the right the word **Worktree** or **Branch** before the branch's name, so a chat in a worktree never reads as one on the main checkout. It does not list subagents: the sidebar does. Queued messages are dashed bubbles above (`QueuedMessages`): each has an **×** in its top right corner (remove) and **Edit**, which takes it out of the queue and puts its words back in the box, ahead of anything typed there, with its files back as attachments. A queued message shows its files as the chips a sent message has, never as the lines that name them to the agent. Two or more sit as a **stack**: the first one sent in front, showing its first two lines and "first of N", with the edges of up to two more under it. A click on a message spreads them all out, oldest first, each with its own × and Edit; a click again stacks them, and a queue down to one message starts stacked the next time it grows.
- **Esc ladder.** Esc closes the topmost open layer and reaches the terminal only when none is
  open (`model/escLadder.ts`): What's new, palette, Settings, the sidebar drawer,
  Focus mode. A new dialog adds its id there.
- **Jump to next (⌘J, Ctrl+Shift+J).** Opens the next session that needs you, in creation order across every machine
  (`features/needs-you/model/needsYou.ts`), with the toast "Needs you · name · 1 of 2"; with nothing needing you it
  only shows "Nothing needs you".
- **Command palette (⌘K).** A 600px card 12% from the top over a dimmed scrim: type a few letters to jump to a
  session, project, action or theme (themes only once something is typed). Sessions come first, most urgent first,
  each named as its row is with its agent and project ("Needs you" in amber), then their projects, then the actions. A
  run of the typed text ranks above letters that merely appear in order; ↑ ↓ move, ↵ runs, Esc closes. Three or more
  typed characters also search what this computer's sessions printed recently (`search_panes`, the same search as
  Find's *All sessions*; a session on another machine is not searched), 300 ms after typing pauses, one search at a
  time (a query typed while one runs waits, and only the newest is sent): up to 10 hits, one per session (its newest
  matching line), follow everything else in an IN SESSIONS group, each the session's name and the line in mono, cleaned
  of escape codes, control and bidirectional characters and cut to 60 cells with 16 before the match. Choosing one
  opens that session (it does not scroll to the line). An answer for an older query is dropped, and a failed search
  adds nothing.
- **No Inbox.** What needs you, what stopped and what finished are the first sections of the sidebar's status list, and
  the notification card and the tray menu say the same; the desktop app has no separate Inbox panel.
- **New chat page on another machine.** The same page as on this computer (layout 2f), titled by the machine's tray fact `On <nickname or name>`: its projects, a pasted path, no Browse…, no branch, worktree or attach controls, and a model button that holds only the agent switch. There is no New session dialog.
- **Focus mode (⌘.).** Hides the sidebar; a pill at the bottom says where you are and
  how many sessions need you elsewhere. Esc or the pill's Exit leaves it.
- **Broadcast (⌘⇧B).** Per layout. While on, every pane of the layout has a violet outline and a
  "Broadcast" pill, and what is typed in one pane is also typed into the layout's **idle** panes.
  A pane running something is skipped, so it never types into a busy agent.
- **Pane resize.** The gap between two columns drags (180px minimum, snaps to an even split,
  double-click resets). Widths are saved with the layout and reset when its columns change.
- **Drag a session into a pane.** Any sidebar row drags onto a pane: in the same layout it
  swaps, in another layout it moves here, otherwise it replaces the pane's session. The session that
  leaves keeps running.
- **Files dropped on a pane of another machine.** One sonner toast on the theme tokens (`bg-muted` track,
  `bg-primary` fill, `text-muted-foreground` percentage): the title `Sending <file> to <machine>`, with
  `(2 of 3)` for a drop of several, a thin `role="progressbar"` named `Sending <file>`, and sonner's own
  action button **Cancel**. It has no close mark while it runs. It is then replaced by a success, error or
  plain toast. See UI › *features that need a pane's files*.
- **A pasted image.** The same toast, titled `Sending the image to <machine>` (a progress bar named
  `Sending image`, **Cancel**), ending as `Sent the image to <machine>`. For a pane of this computer there is
  no progress: a short `Saving the image…` toast becomes `Pasted the image`.
- **Side panel** (⌘I, a pane's side-panel button, a session's Show changes; `features/inspect/InspectPanel.tsx`). It
  slides over a scrim at half of the window, never over the sidebar, 12px from the top and bottom and 24px
  from the right, radius 16; its edge drags and the width is remembered (a double-click goes back to following the
  window). Its header is one segmented control, **Changes** (with its count), **Files** and **Terminal**, each with
  its icon and the open one a raised pill, and ×. Esc closes it, except in a terminal, where Esc is the program's.
  Files reads the focused session's top folder before the panel opens; while a list or a file is still being read
  it shows still bars in its place, never the word "Loading". A pane's own menu has no heading.
  **Changes** is two columns: the branch, the changed files (a letter for added, modified or deleted, the name over
  its folder, the lines added and removed) and under them the commit message with Commit and Create PR; beside it
  the picked file's path, what happened to it, and its diff with both line numbers and without git's header lines.
  Every session on this computer has the Terminal tab: with an agent's terminals pinned under the chat the tab says
  so and offers to show them there. A chat session has no pane to ask about its repository, so Changes and
  Files appear for it once it has a terminal.
  **Checks** lists what the repository's checks said about the agent's last turns: each check's command, its state in
  words (Running, Passed, Failed, Accepted as it is, Could not run, Needs your approval, …) and the first failing
  line, with **Run again**, **Accept the failure…** (an optional reason, up to 280 characters; **Undo accept** takes it
  back), **Run once / Always run this / Never run** for a command the repository brought, and **Output** for the
  tail of its last run. The tab's count is the checks that want you (`ChecksTab.tsx`, `checks.ts`). Branches (with
  each branch's distance from its upstream), History (the commit log and the agent's snapshots),
  Timeline (what the pane reported lately) and References follow Terminal as tabs of their own; the tab strip scrolls
  when the panel is narrow. Changes lists the
  changed files (`+N −N`) and the picked file's diff (added lines tinted with `--ok`, removed with `--danger`, hunk
  headers in the accent), with line numbers drawn beside it. For an agent, a click on a code line opens a comment box
  under it with **Send to Claude** (the agent's name; Ctrl/⌘+Enter sends, Esc closes the box); the comment goes to the
  agent as a message naming the file, the line and its text (`model/agentMessage.ts`), and stays pinned under the line as an accent-tinted "You → Claude" box, also while other files are read; the file's header then counts the comments. The chat draws the comment at once, like a message typed in its box (`outsideSends.ts`). An
  agent that is asking something is not sent one. Under the file list, for a checkout on this computer, is the commit
  message (first the session's name, else the first line of its last prompt), **Commit N files** (every changed file has a tick,
  all ticked at first; only the ticked ones are committed, `commitText.ts`'s **Suggest** drafts a message from them) and **Create PR** (pushes the branch and runs `gh pr create --fill`; the address stays under the
  buttons). Between them is one **remote button** that follows where the branch stands against its upstream:
  **Pull ↓N** when it is behind (fast-forward only), **Push ↑N** when it is ahead, **Fetch** when it is level; when it
  is both ahead and behind the button is disabled and its tooltip says why (`remoteAction.ts`). What git or `gh` said
  is shown when any of them fails (`CommitBar.tsx`, `src-tauri/src/changes.rs`).
  A worktree whose folder was deleted by hand reads "its folder is gone" in the worktree manager with **Recreate**, which
  makes it again on its branch at the same place. Each row's standing comes from the daemon (`worktree_standing`), and one
  idle past `worktree.inactive_days` leads with "inactive N days". The same list opens with a line when two checkouts changed one file ("orders and rate-limit both changed checkout.ts"; the comparison is `maitty_shared::overlap`, shared with the terminal client). A pane's menu has **Hand off to <agent>** for an agent's pane: a new
  session of the other agent in the same folder starts with the text brief `maitty handoff` writes (`maitty_shared::handoff`; the
  desktop lists only uncommitted files and says so). The one other thing the panel
  changes is a file: **Edit** in the Files tab (this computer only, never in a diff) replaces an existing file whole;
  **Save** (Ctrl+S) first re-reads it and asks before overwriting a file that changed since the edit began. The daemon
  refuses links, `.git`, new files and anything over 256 KiB (`WriteFile`). In the History tab, **Restore** on a
  snapshot asks first and is disabled while the agent works, and **Compare** picks two entries. A tab that cannot load shows one calm
  screen with Try again. A folder that is not a git project says so.
  **Files** (`FilesTab.tsx`) lists the changed files first (**Changed**, each with its A, M or D mark), then the tree
  (**All files**): a file carries a type tag (`fileTag.ts`: TS, TSX, `{ }`, MD…) and its mark when changed, a folder a
  dot when it holds a changed file. It opens on the first changed file, else the README. A changed file shows its diff
  under a 44px bar with **Show the whole file**; a link to a line opens the whole file at that line.
- **Quick-look links.** ⌘-click (Ctrl-click elsewhere) on a file name in a pane's output, with an
  optional `:line`, opens it in Inspect › Files at that line. A name that is not a file in the
  pane's repository says so in a toast.
- **Find (⌘F).** A bar over the top right of the panes. **Pane** looks in the focused pane's
  scrollback (Enter next, Shift+Enter previous), or, when the pane shows its chat, in the conversation:
  the row that holds the text is scrolled to and marked, starting from the newest; **All sessions** lists lines from every session's
  recent output, and a click shows that session.
- **Worktrees (⌘⇧N).** The New worktree dialog names a branch and what to start it from (an
  existing branch is checked out instead), then opens a session in the new folder in a new tab. A
  project's right-click menu has a **Worktrees** submenu that opens a shell in any of the repo's
  worktrees, and a **New worktree…** row. The dialog also lists the project's other worktrees with where each stands
  ("merged, nothing uncommitted", "3 uncommitted changes", "not merged anywhere"), each with **Merge** (its committed
  work into the main checkout's branch, no fast-forward; refused while either side has uncommitted changes, and a
  conflict leaves the main checkout as it was) and **Remove** (the daemon closes what runs in it and removes the folder;
  it asks again first when that would lose changes or unmerged commits). This computer's worktrees only
  (`WorktreeManager.tsx`, `src-tauri/src/worktrees.rs`). Worktrees group under their parent repo. A folder's row shows
  only its name: a branch belongs to a session, and a worktree's branch is not the folder's.
- **Branch or worktree.** A session in the project's own folder shows its branch in `--branch` (violet: #A78BFA on a dark ground, #6B46C1 on a light one, set by `themeStyle`; every branch name and icon uses it) with the branch mark;
  one in a linked worktree (the daemon's `TermInfo.linked`, never worked out from the folder) shows it in `--branch`
  with a tree, on its sidebar row, its pane header and under the message box, where the word reads **Worktree** or
  **Branch**. The tooltip says which.
- **A session whose agent asks for a secret** (`maitty_request_secret`) shows a bar across the top of its pane with the
  label and the reason the agent gave, and **Provide…** (a masked box; Enter gives it) and **Decline**. The value goes
  from that box to the daemon and nowhere else: it is not kept in the window's state and not shown in the chat, and the
  agent gets a single-use reference (ADR 0006, `SecretAskBar.tsx`). This computer's sessions only.
- **A session that asks to start helpers** shows a bar across the top of its pane, in the needs-you colour: "Wants to start N helpers", the
  tasks, and **Allow** (this session, from now on), **Always in this project** and **No**. It is the only place the desktop app
  answers that request, and it stays until the daemon says the request is answered.
- **Ended panes.** A pane never disappears because a refresh missed its session or the session ended: it
  stays in its tab with its output, the tab keeps its name, and a card along the bottom says **Session
  ended** with **Restart shell** (a new shell in the same folder, in the same place) and **Close pane**
  (which stops nothing). Only the first list after launch drops panes of sessions that died while the app
  was closed. The sidebar lists the sessions the daemon lists. A row goes at once when the daemon says why it
  left (closed, exited, put away, moved, the daemon stopping, or a reason it did not record) or this window
  closed it; a row a list leaves out with no reason stays, shown as unknown, until a full list read from the
  daemon confirms it is gone, so a daemon restarting under the app makes no row flicker out.
- **Switching sessions.** A pane shows at once: the last screen it had (kept in memory per session,
  `features/terminal/screenCache.ts`, bounded in size and count) or the replay in one write per frame. A
  pane on this computer never shows a spinner, overlay or "Connecting" text. A pane on another machine
  that has not attached after 300 ms shows a 24 px strip under the pane header (spinner, **Catching up
  from <machine> · N ms**, a 2 px progress line on its top edge) that sits in the flow, never over the
  terminal, and goes when the stream is live. A real failure keeps its banner; it is not a loading state.
  The two tabs shown last stay mounted behind the one showing (`model/keptTabs.ts`, at most eight panes
  between them, since each terminal holds a GPU context): the same box, not painted, `inert` and hidden from
  screen readers, their terminals paused and their chats unmounted, so going back builds no terminal and a
  hidden chat reads nothing and answers no key. A paused terminal catches up from the kept screen when shown.
- **Cards in the chat** (a command and its output, a diff, a code block, the plan, helpers; `.mx-step-card`,
  `.mx-stack-card`). Each is a well sunk into the pane (`--m-well`, darker than the page), with a hairline at 14% of
  the theme's light, a faint light along its top edge and a soft drop shadow, and its header is a shade lighter than
  its body. The point is that a card reads as an object in the conversation, not as more of the page.
- **Jump to the latest message.** The round arrow over the chat glides to the newest row instead of cutting to it
  (0.18 to 0.42s by distance, easing in; `glidePlan`). From far up it starts two screens above the bottom, so a long
  chat is not drawn on the way. The reader's own wheel, key or touch ends the glide where it is, and with motion
  reduced it is a plain jump.
- **A session's row.** The agent's tile and status badge at the left. To its right, two lines. First, small and
  dim, where the session is: the project's colour square and folder at the left, and at the right the branch in mono
  in the theme's magenta with the branch mark, or the tree and the branch in blue when it runs in a worktree. Under
  it the title, with the time or the stop word at its right. Inside a project group the folder is in the heading, so
  the first line is the branch alone, and a session with neither has only its title (`SessionRow.tsx`, `PlaceLine`).
  Only a session on another machine, outside that machine's heading, has `· machine` after the folder, and only then is
  the branch held to 58% of the line; otherwise it takes all the folder leaves, cut with `…` and its full name in the
  tooltip.
  The close button shows on hover over the time.
- **Rows slide, folds fold, never both.** A row that changes place slides there (`useFlip`). A section or a
  session's subagents folding moves what is under it by itself, so while a fold runs nothing slides, and where things
  are when it ends is where the next slide starts from; the two together made the list bounce.
- **Sidebar footer and updates.** The footer's left slot shows the version, or what an update needs
  from you: a pill that opens a card upward with the matching actions. An update waiting to be downloaded or
  installed is a glass button, a sheen sweeping across it, a tile with an arrow falling into a tray, **Update** and the
  version in mono (**Restart to update** once downloaded); the download's percent and **Update failed** keep a tinted pill. Every card closes with an X in its corner; the download's card shows the bar, what has arrived of the whole (`34.9 MB of 102 MB`) and, on the right, the percent and the time left from the last ten seconds' speed (`about 12 min left`, `downloadEta.ts`). Settings shows the same bar and line under its update row while a download runs, with the percent on its button (`DownloadProgress`); a light passes along the bar, and with no size yet a short run slides across, so it never reads as stuck. The updater downloads first and installs only on
  **Restart now**, or by itself when the app is idle, so sessions keep running either way. Settings › General › Updates shows the state with
  one fitting button and two switches: download automatically (on) and **Install updates when idle** (on, helper
  "Never while an agent is working"). Idle means no agent working or waiting on an answer, no keyboard input
  for 5 minutes, and the window unfocused or minimized. After the relaunch a pill toast at bottom centre reads
  "Updated to X · What's new" for 9 s (dismissable), and What's new opens a 460 px dialog with the release's
  notes and "No session was interrupted."
- **Sidebar position.** Settings › Sidebar › Sidebar position, or the palette, puts the sidebar
  on the left or right. On the right the window mirrors: the resize edge is on its left, the narrow-window
  overlay slides from the right, Inspect opens from the left, and popovers open toward the middle.
- **Notifications.** Settings › Notifications has one switch each for needs you, done and error. With
  one on, a session that changes to that state while you are not looking at it (the window is in the
  background, or it is not the focused pane) sends a system notification, once per change. It names
  the session and where it is and never what the agent asked. Clicking it brings the window forward
  and shows that session. Opening the app, a session you are in, and a reconnect after the daemon
  was unreachable send nothing. The in-window notification card follows the state: green for done, amber
  for needs you, red for an error. Its position (any of the six corners and edges) is set in Settings ›
  Notifications, which can show a test card.
- **Interrupted agents.** When the daemon restarted while an agent was mid-turn,
  a banner above the panes says which agent and shows the command that brings it back
  (`codex resume --last`). **Resume** types it into that session; **Dismiss** hides the banner. Nothing
  resumes on its own.
- **Usage and cost (⌘⇧U / Ctrl+Shift+U).** Settings › Workspace › **Usage & cost** (key U), above Notifications,
  drawn as one Settings card; the shortcut and the palette open Settings on that page, and Esc closes Settings.
  There is no usage button in the sidebar footer and no full-screen view.
  Header: a **Day / Week / Month** switch and the window it means, named honestly because the daemon
  counts rolling UTC days that end today (Today (UTC), Last 7 days, Last 30 days), then tokens (in
  and out), cost, turns and time working. A figure no provider reported is a dash; with no reported
  cost the page shows no dollar total, and when any session reports none it shows **one** note, never
  one per row. A per-day bar chart (accent bars, no animation) has a hidden table as its text
  alternative. The table sorts on every header (`aria-sort`, buttons, cost descending by default;
  text columns start A to Z, numbers biggest first; unreported figures sort last), ends in a totals
  row, and its Machine column is always "This computer" until the report covers remote machines. A
  row opens its pane only while that pane is open in this window; a closed session's row is plain
  text. The **budget ring** (56px, accent, warn from 70%, danger from 90%) appears only when a monthly
  budget is set, the cost is known and the Month window is showing; otherwise "Set a monthly budget"
  (an inline `$` field; empty or "No budget" clears it). The budget is the `monthlyBudget`
  preference (client-side; the daemon serves none). States: skeleton while loading, plain words with
  Retry on failure (a failed refresh keeps the old figures and says they may be stale), "Nothing
  recorded yet" when the store is empty. Refreshes on open, when the period changes and every 60 s
  while open. Source: `features/usage/`, ADR 0009.
- **Turn timeline.** Inspect › Timeline lists the focused pane's latest events, newest first: the
  agent started working, stopped for you, finished (with how long the turn ran), and, in a shell
  that marks its prompts, each prompt and command with its exit code.
- **Status is shape and word.** Working is the user's spinner, Needs you an amber square with a `!`, a problem the same
  square in red, Done a green checked square, Unknown a dashed square, and Idle a sleeping `Zz`; a stopped agent or an
  exited shell also says so in words on its row.
  Amber still means needs you and only that, in every theme.
- **Colours follow the theme.** Agent tiles, badges and the folder text are
  expressions over the active theme's tokens (`--accent`, `--ansi-*`, `--warn`, `--danger`,
  `--on-accent`), never fixed hex values, so switching theme recolours all of them.
- **Status marks, not effects.** Every state has its own shape on a row's or rail icon (`StatusBadge`), so colour
  is never the only signal: **working** is the user's spinner in `--accent`, on the icon's corner; **needs you** Lucide's
  `SquareExclamationPoint` in `--warn`, with a ring that pulses while it waits; **done** Lucide's `SquareCheck` in `--ok`;
  **problem** (a stopped agent, or the daemon's `TermInfo.exit`, a non-zero exit code of the last command, shown while
  nothing is working or waiting) `SquareExclamationPoint` in `--danger`; **idle** a `Zz` in `--dim` with no square;
  **unknown** (the daemon is gone) a dashed outlined square, with the row dimmed. The three squares and the `Zz` sit on
  `--bg`, drawn as outlines (stroke 2.4, scaled so the square fills the badge). The window only displays these; the daemon decides them. There are no waves, sweeps, shimmer or glow
  on rows and tabs, and no setting for them; the only looping motion is the spinner. Reduce motion (the setting or
  the system's) removes the pulses, slides and size animations, and the spinner holds one frame. The needs-you badge pops once with a ring (.6s) and
  the update dot once (.7s), as `transform` and `opacity` on a pseudo-element. The terminal caret is steady
  (blinking is Settings › Text › Blink, xterm's own option, off by default), and nothing animates `box-shadow`,
  `filter` or `backdrop-filter` ([PERFORMANCE](PERFORMANCE.md)). The working spinner steps every 160 ms and
  holds while the window is in the background.
- **Working spinner** (Settings › Appearance › Theme, `spinner`) is a dropdown row under the dark and light theme:
  a 220x40 button with a live preview tile, the name, its source ("From the Maitty TUI" or "Web icon") and a
  chevron. The list is the Radix dropdown menu (`SpinnerPicker`, 260px wide, at most 340px tall, flipping upward
  when there is no room) in two groups, **Maitty TUI** (Braille, Snake, Bars) and **Web** (Circle, Dots,
  Equalizer, Spokes, Ping), each with a live preview and the picked one ticked. TUI spinners keep the TUI's exact
  frame strings, stepped by the shared clock. Web spinners are inline SVG in the text colour, 1em square
  (`WebSpinner`), each in its own `display: inline-block; contain: paint; will-change: transform` span, moved by
  CSS with `transform` and `opacity` only: Circle rotates .8s, Dots scale and fade 1s staggered .16s, Equalizer
  scales 3 bars .9s staggered .15s, Spokes step eight times in .9s, Ping scales a ring and fades it 1.3s. They
  hold still under Reduce motion and pause while the window is in the background. Text-only places (a hover
  card) show the Braille frames while a web spinner is picked.
- **Process icons** are generic tiles, never a product's mark: `>_` shell, `✻` codex, `rs` cargo.
- **Themes**: fifty, twenty-five dark and twenty-five light, the same in the GUI and the TUI (`crates/maitty-settings/themes.json` lists them; a TUI test holds its ground and text equal to it). Six are Maitty's own. Four are built from `crates/maitty-settings/maitty-tokens.json` (the TUI reads it too): **Maitty Night** and **Maitty Day** (sea-glass and brass, the defaults) and **Maitty Dark** and **Maitty Light** (a standard black and white ground, same accents); **Maitty Retro Dark** and **Maitty Retro Light** carry the Retro Pixel style's colours (navy ground, mint text and a lime accent by night; white, ink and a darker green by day) and are generated like the families below, so they work with any window style. In Maitty's first four, brass is the accent (`--accent` as a fill, `--accent-text` as text, darker by day), teal is only ever `--working`, coral is only ever needs-you (`--warn`). The other forty-four are twenty-two families, each with a dark and a light (GitHub, One, Catppuccin, Solarized, Gruvbox, Tokyo Night, Dracula, Nord, Ayu, Everforest, Rosé Pine, Kanagawa, Nightfox, Flexoki, Gruvbox Material, One Half, PaperColor, Tomorrow, Melange, Cyberdream, Doom One, Zenbones), generated from six inputs (`bg fg accent mag blue green` and a light flag) by Oklab mixing. A saved theme that is gone falls back to Maitty Night or Maitty Day (the TUI maps each removed id to its nearest survivor). The theme is stored as its id (`night`, `github-dark`, ...) in the shared settings. Every theme's `--pane` is its opaque
  `--main`. The foreground, terminal text (`--fg2`), muted and dim text keep 4.5:1 on both `--term` and the
  window `--bg`, and Maitty Night and Maitty Day also hold it for the accent text, working, needs-you, red, done and the ANSI blue,
  magenta and green; the theme tests check this. Every light theme holds it too, for accent, amber, red,
  green, blue, magenta and all 15 ANSI text colours but black (the generated ones darken a published hue in
  place until it passes). Maitty Day uses the file's darker *Text colours so those hold on the light ground. A pane's
  xterm palette comes from the same tokens, and on a light theme xterm also lifts any 256-colour or truecolour
  foreground a program picks to 4.5:1 (`minimumContrastRatio`; dark themes leave them as asked). The
  scroll viewport behind the rows is painted `--term`, never xterm's black. **Settings › Appearance › Theme** has an Appearance switch (System, Dark or Light:
  `themeMode`) and a searchable dropdown each for the dark and the light theme (`darkTheme`,
  `lightTheme`); the dropdown lists its theme in use first. System follows the OS's own dark or
  light setting as it changes. Hovering a theme in the slot that is in use previews it on the
  whole window; leaving the list or closing the menu restores it, and picking commits it. A
  single theme saved by an older version fills its own slot and pins the mode to it.
- **Type.** Geist for the UI, Geist Mono for terminals, paths, counts and keys, both bundled.
- **Machines.** With another machine, the sidebar's folders sit under a header per machine: `this
  computer` first, then the others in id order, in a small uppercase `--dim` label. A machine's state is a
  chip at the right of its header: a glyph and a word (`●` connected in `--done`; `○` connecting and `◐`
  reconnecting #n in `--dim`; `▲` needs login, not installed and needs update and `✕` unreachable and refused in
  `--danger`), so colour never carries it alone, and nothing animates. Amber stays for agents that need
  you. A machine that is down shows `disconnected since 2m ago` and its fix (wrapped, in `--dim`) under the
  header, and its folders are drawn at 60% opacity. A machine that needs the app has a line under its header,
  lined up with its mark: `▲ not installed` or `▲ needs update` in `--danger` and an **Install** or **Update**
  text button in the accent (11.5px), which opens the same question as the menu's Install Maitty or Update
  Maitty (`components/MachineUpdateDialog.tsx`, the Rename dialog's frame: what it needs, what it interrupts,
  Cancel, Update when idle, and the primary Install now or Update now); a machine's header shows even with no
  sessions anywhere. With one machine there is no header. **Settings › Machines** is a card per machine (chip
  in its tone, how long, what it needs in plain words or the fix in full, the primary **Install** or **Update**
  in the accent, then **Copy command** with the command under the text as the secondary way, **Retry**, a
  confirmed **Remove**) and an **Add a machine** card (a field, then the `~/.ssh/config` hosts as rows); its
  states are loading, empty, error and checking, and it works with the keyboard alone.

### Notification cards

Every toast is drawn by one component, `NotificationCard` (`apps/gui/src/features/notifications/`): the agent's icon with its status badge on the left, a bold title with a close button, a muted line saying what it wants (a command in mono), the buttons, and "just now" at the bottom right. It is a float, so it takes the material roles from `material.css` and sets only its tone colour (`needs`, `done`, `error`, `info`). `SessionNotice` (terminal sessions) and `NoticeToast` (conversation views) only map their data onto it; plain Sonner toasts (messages, errors) wear the same float material without the icon and buttons. Allow appears only when the session is stopped on a choice the GUI can answer.

- **Working badge.** A `--working` rounded square (3px radius) with the spinner in `--on-accent`; it is the same square at every size (sidebar row, pane header). The chat's scroll area fades to transparent over its top 36px.
