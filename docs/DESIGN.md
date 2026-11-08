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
- **Only what has the keys is lit.** The focused pane, the open sheet or the selected row;
  everything else is quieter.
- **Plain words.** Sentence case, no bracketed explanations, every key shown as a keycap.

## Themes and colour roles

Colours come only from the theme's roles, never a literal. Dark and light are two remembered
slots (Settings › Appearance) and a switch; **Night** is the default dark theme and **Day**
the default light one, and the browser lists each side on its own tab. **Night** is the sea
at night, a deep navy ground, sea-glass teal for focus and working, foam for names. **Day**
is the same sea by day: a pale grey-blue ground and a deep teal accent. The other built-in
themes fill the same roles; [theme sources](theme-credits.md) lists where they come from.

| Role | Used for | Night | Day |
| --- | --- | --- | --- |
| `bg` | the ground, and a terminal cell's default background | `#08111d` | `#f6f8fa` |
| `card`, `card2` | panels, header strips; fields and action rows inside modals | `#0f1b2b`, `#15253a` | `#ffffff`, `#e0edf2` |
| `btn`, `hov` | chips and buttons; the hovered row and the open session's row | `#15253a`, `#13233a` | `#e4ebf1`, `#e0edf2` |
| `line` | borders, dividers | `#1d2d42` | `#d3dce5` |
| `muted`, `text`, `strong` | quiet text, body text, names and titles | `#6f8399`, `#c9d6e3`, `#eef5fb` | `#61748a`, `#1f2f3f`, `#0b1724` |
| `accent`, `acc_ink` | focus, working, the current tab; ink on the accent | `#3dd6c6`, `#08111d` | `#0b7a71`, `#ffffff` |
| `blocked` | needs you, and only that | `#f5c542` | `#674100` |
| `done`, `err` | passed or finished; failed | `#7fd99a`, `#ff7b72` | `#2e8b57`, `#c93c37` |

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
no field; the folder button groups sessions by folder. A machine is a heading chip with its count (` This computer 5 `), an accent ` + `
chip (a shell there at once) and a green `●` at the right while it is up. While its app is missing
or on another version the ` + ` gives way to an accent ` install ` or ` update ` chip, the fix one
click from where it shows; the Machines dialog marks that row ` u install ` / ` u update ` beside its
state. A session is two lines:
the status mark and the name (bold and bright for an agent with something going on), then `■` in
the project's colour, the project and the branch in violet. No status words anywhere. Agents come
first, then shells, each in the order they started, so a row never moves when its status changes.
The open session has the row ground and an accent `▎`. An agent's subagents in its current turn
follow it: `└` and a mark each, or under the open session one row each with `├`/`└` and the name.
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

Reading while you work happens in a panel that slides in from the pane's right edge, inside its
box (about 220 ms, eased out): the panel ground, a line down its left edge, its title chip
(` files `, ` changes N `, ` branch `, ` references `, lit while it has the keys) and a ` » ` that
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
Midnight first). One model: **sessions live in the sidebar, an agent opens as a chat, a shell opens as a terminal,
and a terminal for an agent exists only when you ask** ("Open terminal here"). Every session is still a shell that
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
  that file and nowhere else. Every surface is a *panel* (sidebar, main card, Inbox, inspector), a *float* (menus,
  popovers, dialogs, the palette, find, toasts) or a *field* (the message box and its tray), and takes its fill,
  hairline, highlight, shadow and blur from that role's `--m-*` tokens. The `glass` finish is the base for **Glass**, which
  then lays its own translucent panels on top (below); the finish's own recipe is opaque `--main` cards on the window ground. A panel
  has a faint top highlight (`rgba(--hl, .035)` fading to `.012` over 120px on the sidebar, a flat `.018` on the
  workspace card), a `rgba(--hl, .09)` hairline, an inner top edge and a `0 10px 30px -12px` shadow; a float has a
  stronger highlight (`.07` to `.03`), a `.12` hairline and a `0 18px 44px -12px` shadow, with a 14px blur behind it; a
  field sits between (`.075` to `.035`). **Flat** (Mono Type, Console, Retro Pixel): solid fills, a hairline, no blur, no
  highlight (`--gk` is 0, which turns off every gradient and lift that multiplies by it; hover tints stay), one plain
  shadow on floats. A new surface is added to a role's list in `material.css`; it does not set a background of its
  own. A popover opens through `PopupContent` (`components/ui/popup.tsx`) or carries `mx-pop`, so it is portalled into
  the window root and takes the float role. **Inside a surface**, nothing reads the theme's solid colours directly: a
  sunken control is `--m-well`, a raised or current row `--m-raise`, a picked row `--m-sel`, a hovered one
  `--m-hover`, a dialog's own ground `--m-pane`. Flat maps them to the theme's `--well`/`--raise`/`--sel`/`--hover`/
  `--pane`; Glass maps them to tints of the light (`--hl`) and shade (`--sd`). Tints are written
  `rgba(var(--hl), calc(.06 * var(--hk)))`, never a fixed white. The hairline uses `--hl` (ink on a light theme).
  `src/material.test.ts` fails when a stylesheet reads a solid surface colour or a fixed white. Left on purpose:
  terminal grounds (`--term`, `--main` under a terminal), scrims, brand and status colours, the switch knob, and the
  legacy `App.css` shell outside the window root. **Background glow** (Settings › Appearance, `backgroundGlow`) is the
  only tint behind the cards, and Glass always has it, painted on the whole window so no seam shows under the title bar.
- **Window style: four looks, one set of tokens** (Settings › Appearance › Window style, `style`; the root carries
  `data-style` and `data-finish`). **Glass** (the default: the redesign's Frosted look, clear frosted panels with 22px corners over a soft
  atmosphere, a rose accent, Inter and JetBrains Mono; the redesign's own opaque Glass was dropped, and a saved
  `frosted` reads as Glass), **Mono Type** (Space Mono everywhere,
  greyscale, square corners), **Console** (IBM Plex Sans and JetBrains Mono on true black, a teal marker on the
  selected row) and **Retro Pixel** (Pixelify Sans, square corners, 3px borders, hard offset shadows, scanlines). Each
  has a Midnight (dark) and a Day (light) side; layout, features and status colours never change. A style is data in
  `theme/windowStyles.ts` (palette overrides, fonts, the glass dials `--hk`/`--sk`); `withWindowStyle` lays it over the
  theme in use. The palette replaces the colours of Midnight, Paper and Day; any other theme keeps its own colours and
  takes the style's fonts, dials and shapes. **Components are built once and read tokens, never the style.** Besides the
  base tokens, the shared pieces read the component tokens declared in `material.css`: `--c-sel-*` (every selected
  state: a sidebar row, a settings nav item, a chosen segment, a palette row), `--c-field-*` (dropdown buttons, pickers,
  the new-session segments, inputs), `--c-track-*` (the tray behind a segmented control), `--c-sw-*` and `--c-knob-*`
  (the switch), `--c-tile-ring` (agent and shell tiles) and `--c-head-bg` (sticky headers). Glass uses the
  defaults; Mono, Console and Pixel override them. What tokens cannot say (square corners, Pixel's borders and
  scanlines, Glass's panels, no blur outside Glass) is the one `[data-style]` block at the end of
  `material.css`. `data-finish` is derived (`finishOf`): Glass is `glass`, the others `flat`. The style is
  the desktop app's own: it is not in the shared `window_style` setting, which stays Flat or Glass for the config file. A
  saved Flat or Tiled reads as Console. A new control gets tokens first and is checked in all four styles and both
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
  settings…. **Pane** (⋯ or a right-click in the pane): Split right, Split down, Maximize pane, Show chat or Show
  terminal and Open terminal here or Close its terminal (agents) or Clear and Copy folder path (shells), Files,
  Changes, Close pane.
- **Status bar.** A 28px line at the bottom of the main area under a hairline, 11.5px muted, one line.
  The left is empty, except "Reconnecting to maittyd…" in amber with a dashed dot when the daemon is
  unreachable (the sidebar foot says "Service stopped" or "Restart needed" beside a red dot). The right holds quick controls: a **theme** menu (Dark, Light or System, a few themes of
  the kind in use, and "All themes in Settings…"), the **window style** (the four styles), the terminal
  **text size** (−, the size which resets it to 14px, +; Ctrl+= and Ctrl+− do the same) and the shortcuts
  keycaps, which open the shortcut overlay. Hidden in Focus mode and by Settings › General › Window ›
  Status bar (off by default).
- **Sidebar** (`components/Sidebar.tsx`, `sidebar.css`). Agents only: no fleets, ships or tabs. The 48px header holds
  Collapse sidebar, then **New agent chat** (opens the New session dialog), **Add a remote machine** (Settings ›
  Machines) and the **Inbox** with its count (amber while something needs you). Under it the 32px sunken **Filter
  sessions** field (`/` focuses it; a raised `/` keycap; its border brightens on hover). Sessions are grouped by
  **machine**, this computer first: an uppercase 10.5px heading with a 12px caret (`--accent`; a project's caret is `--ansi-blue`), a laptop or server mark, the count, a
  group-by-project toggle, the line's 5px dot and a `+` (a click starts an agent chat on it, a right-click opens the full menu); clicking the heading's name folds the group. Within a
  machine the rows are **most urgent first**: needs you, an agent that stopped, working, done, idle, then a shell whose
  last command failed; creation order within each. While the new-chat page is open, a dashed **New chat** row ("waiting
  for your first message", three stepping dots) is listed first on this computer, so the draft is never missing from the
  sidebar; it goes when the chat starts and the chat's own row takes its place. Grouped by project, each project is a folding heading (its colour
  square, name, needs-you count, count). A **row** (40px, radius 8) is the 20px app tile with its status badge
  (spinner, amber `›`, green check, dashed square for unknown), the name (13px; 600 and `--fg` for an agent that is
  doing anything), and under it the project's 6px colour square, the project and `·` and the branch in mono
  `--ansi-mag`; a stopped agent says `stopped` and an exited shell `exited` at the right, in `--danger`, instead of a
  badge. A shell's tile is a flat `--raise` square with a teal `>_`. The selected row is a raised glass pill. The row
  is named by the name you gave it, else a shell's folder, else its agent. **Rename**: double-click, F2 or the menu
  turns the name into a field; Enter or leaving saves, Esc cancels, empty clears. Project colours are five tokens
  (`--proj-1` to `--proj-5`: teal, blue, pink, violet, green), chosen from the project's name, never amber or red.
  **Subagents** (the daemon's `subagent_activity`, `model/subagents.ts`): under a session that is not selected, an
  elbow line and one 14px tile per subagent with a corner mark (teal spinner working, amber square needs you, green
  check done), no text; under the selected session, a 28px row each on a guide line. Clicking one opens its own chat.
  The foot holds the update slot (or "Reconnecting to maittyd…") and the Settings gear. The edge drags (208 to 440px;
  double-click resets). The **rail** of a narrow window is the starters, then one tile per session.
- **Replies are typed in.** An agent's reply types toward the text that came in (`useTypedText`), so a chunk eases in instead
  of flashing. A reply that arrives after the history has loaded types from its first letter even when it arrives whole; what
  the history already held shows at once (`knownRows.ts`), and Reduce motion shows everything at once. A helper agent reporting back (the harness's `<task-notification>` block, which the history keeps as a
  user message) is one quiet line, "Agent "name" finished · 11s · 50.5k tokens", never the markup; once it has reported,
  its row leaves the sidebar (one that stopped or still needs something stays).
- **Panes swap with a glide.** Dropping a pane on another swaps them through a View Transition (`paneTransition.ts`): only the
  browser's pictures of the panes move, so text never reflows on the way. Without View Transitions, or with Reduce motion,
  the swap just happens.
- **Toasts are cards.** The notification card is the one design; a plain toast (Sonner) is laid out the same way: icon tile,
  title and detail, the close at the top right and the action on its own row under the text, 96px below the top edge and 36px
  in from the side.
- **A remote machine that needs the app** has no row of its own: its dot turns amber and an amber **↑ Update** (or Install)
  text button sits in its header before the folder button. While it updates, a mono percentage replaces the button and a
  2px accent line runs under the header (indented 26px left, 10px right); both go when it ends. The percentage is the stage
  the update has finished (probed 5, found 15, build fetched 55, sent and verified 85, reachable 95), not a byte count: the
  download and upload have none to report.
- **A chat that starts does not jump.** The message you send from the new chat page is drawn in the conversation at once,
  where it will stay, with "claude is starting…" under it; there is no queued row above the box and no bar that comes and goes.
  The needs-you card is raised by the window from the status change as well as by the daemon's own alert (one card per
  session, so they never stack). The theme's tokens are on the page root, so a menu or popover that renders outside the window
  reads the same colours and is never see-through.
- **Typing while an agent works.** A chat run by the daemon queues what you send, and the daemon sends it when the turn
  ends. A chat that follows an agent in a terminal does the same on this side: the message is held in the composer store
  (shown under the box in the queue, removable) and typed into the agent's prompt, one at a time, once it is ready for a
  message; nothing is typed while it is asking something.
- **Pane** (`components/PaneCard.tsx`, `pane.css`). Agents and shells share one 42px header: the 26px app icon with its
  status badge, the name (15px, 600) over the folder (11px `--dim`) and the branch (mono 10.5px `--ansi-mag`), then at
  the right the terminal | chat switch (agents with a chat), the **Files** and **Changes** tray, a hairline, split right,
  split down, maximize, ⋯ and close. The switch and the tray are one sunken tray look (`rgba(--sd,.3)`, a 1px
  `rgba(--hl,.1)` border, radius 9); the switch's thumb slides by `transform`. As the pane narrows the header gives up,
  in order (`model/paneHeader.ts`): the Files and Changes labels under 660px, the split buttons under 600px, Files and
  Changes under 520px, the switch under 420px; what is gone stays in ⋯ and the title keeps its room. A lone pane is no
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
- **Terminal text.** Settings › Text & cursor sets the size (12 to 16), the font family, the normal weight
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
  (the workspace card drops its own fill). A pane that is added, closed or resized takes its room at once, with no easing:
  easing a split made the terminal re-fit every frame and its text jumped. A terminal pane, header included, is one
  surface (`--main`), with no band where the header ends.
- **Close ends the session, asking first, with Undo.** The pane's **×**, the pane menu's Close session, a row's Close
  session and Ctrl+Shift+W (⌘W) first open the confirm dialog (`ConfirmCloseDialog`: "Close name?", Cancel and a red
  Close, or "Stop claude" for a running agent; Enter confirms, Esc cancels) unless every session in question has already
  ended. Confirming sends the daemon `CloseSession`: the session leaves the list at once and
  its process runs on for 6 seconds. A toast "Closed name" with **Undo** stays for the same 6 seconds;
  the button or ⌘Z (Ctrl+Shift+Z elsewhere; passed to the terminal when nothing was just closed, and left to a
  text field inside one) sends `UndoClose` and puts the pane back in its place. A chat session has no terminal to
  hide, so it ends at once with no Undo. Only closing the window leaves sessions running.
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
  Jump to the next session that needs you is ⌘J on macOS only: Ctrl+J is a terminal key.
- **Terminal | Chat switch.** An agent's pane header carries a two-button switch (terminal glyph, chat bubble). The
  chat covers the pane body while the terminal stays mounted underneath, hidden by `visibility`, so it keeps streaming
  and keeps its size: switching sends nothing to the daemon. **An agent opens as its chat**; a shell opens as a
  terminal, and a terminal for an agent exists only when you ask: **Open terminal here** starts a shell in the agent's
  folder beside it ("Close its terminal" ends it). Settings › General › Chat view picks the view each of Claude and
  Codex opens in (Chat by default). A shell, an unsupported agent or a session on another machine shows the chat
  card's empty state, "Chat view is for agent sessions", with the reason and the ways to start one.
  Changing the view, or switching between sessions, plays no animation.
- **Chat session ([ADR 0003](adr/0003-status-and-conversation-trust.md)).** A row and pane like any session, with the
  agent's mark and its branch line. Its pane has no terminal | chat switch; the pane is the chat. The approvals
  (y / a / n), user-input form, Stop and the daemon's queue are the chat's own.
- **Chat body** (`features/chat/chat.css`). A 760px centred column, 20px in, rows 14px apart, under a 56px fade at its
  top. Your messages are right-aligned glass bubbles (`rgba(--hl,.05)`, a `rgba(--hl,.09)` border, radius 16/16/5/16,
  14px). An agent's reply has no avatar: its first sentence in `--fg` at 500, the rest in `--fg2`, as markdown. Tool
  steps and finished subagents are 28px mono pills with a check ("Ran a subagent: Check migration order · 4m 2s").
  Commands and edits are glass cards (radius 12): a command shows the command, its time and `exit N` pill, the last
  lines of its output and "Show all N lines"; an edit shows "Edited `path` +14", its diff with the hunk header in
  `--ansi-blue`, added lines tinted with `--ok` at 7% and removed with `--danger`, syntax colours (keywords
  `--ansi-mag`, strings `--ansi-green`, comments `--dim`, numbers `--ansi-blue`), folded past 12 lines ("N more
  lines"). A request answered earlier stays in the list, "You answered: …". The working line is the spinner and "claude
  is working · m:ss". Problems with the history (offline, incomplete, a provider error) and Full access show in one
  slim strip above the list. **A subagent's chat** replaces the list with a bar ("Subagent · name · time", **Back to
  main session**), then what it was asked, what it is doing while it works and its report once done.
- **Message box** (`features/chat/components/Composer.tsx`), in three layers. **1. The question tray**, only while the
  agent asks: a slightly narrower amber tray sliding up from behind the box (`mxRise`): "Wants to run a command" with
  its pause tile, the command in a sunken mono box, and **Yes, once** `y` (amber, the same in every theme), **This
  session** `a`, **No** `n`. y, a and n answer from anywhere in the chat but a field. Answering removes it, the box
  unlocks and the agent is working again (its row, the Inbox and the badge follow the daemon). **2. The box**: glass
  (`--main` under `rgba(--hl,.075)` to `.035`, a `rgba(--hl,.12)` border, radius 16), a two-line text box on top, **+**
  (attach) at the left, and at the right the model picker ("Claude Sonnet 5.5 · Medium ▾": the agent's mark, the model
  picked or the one its own file says, the effort) and the round 32px send button, teal when it can send and grey while
  a question waits; with nothing typed while a turn runs it is a Stop square. Placeholders: "Message claude", "Queue a
  message for when claude finishes" while it works, "Answer the request above first" (and off) while a question
  waits. **3. The settings tray**: narrower and darker, under the box: the permission mode from the agent's own file
  ("Ask before running", "Plan", "Bypass permissions" in red, or "Permissions" before it has said), Subagents with their
  count, the project folder (opens Files), and the branch at the right. Queued messages are dashed bubbles above.
- **Esc ladder.** Esc closes the topmost open layer and reaches the terminal only when none is
  open (`model/escLadder.ts`): What's new, palette, inbox, Settings, the sidebar drawer,
  Focus mode. A new dialog adds its id there.
- **Jump to next (⌘⇧J, macOS only).** Opens the next session that needs you, in creation order across every machine
  (`features/needs-you/model/needsYou.ts`), with the toast "Needs you · name · 1 of 2"; with nothing needing you it
  only shows "Nothing needs you". Windows and Linux have no key for it yet.
- **Command palette (⌘K).** A 600px card 12% from the top over a dimmed scrim: type a few letters to jump to a
  session, project, action or theme (themes only once something is typed). Sessions come first, most urgent first,
  each named as its row is with its agent and project ("Needs you" in amber), then their projects, then the actions. A
  run of the typed text ranks above letters that merely appear in order; ↑ ↓ move, ↵ runs, Esc closes.
- **Inbox (⌘J).** The sidebar header's **Inbox** button (a count, amber when something needs you) opens a 440px
  panel from the right listing every session that needs you, finished or exited, in that order ("N across all
  projects", **Clear done**). A needs-you card shows what the agent asks in a well and has **Yes, once**, **This
  session** and **No**, which type `y`, `a` or `n` into that session; the window never decides for it. Done and error
  cards have **Clear** and **Open**. With nothing: "You're all caught up".
- **New session dialog** (⌘T, Ctrl+Shift+T, the sidebar's New agent chat, a machine's New agent chat here). A 560px
  dialog: **Agent chat | Shell**; AGENT (Claude Code, Codex as cards); PROJECT (a chip per project the machine has
  sessions in, the focused one first, with its colour; **Other folder…** opens the folder picker on this computer);
  WHERE IT WORKS (**New worktree**: the daemon starts an agent typed in a project's main checkout in a worktree of its
  own; **Current branch** shows disabled, since that is the daemon's `worktree.per_agent` setting); FIRST MESSAGE (held
  under the new session and sent when the agent takes input; its first three words name the session). **Cancel** and
  **Start agent** or **Open shell** with `Ctrl ↵`. On another machine the title says so and the agent is typed into a
  shell there.
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
- **Files and Changes** (⌘I, a pane's tray, a session's Show changes). The repository drawer: 680px over a scrim, 12px
  from the top and bottom and 24px from the right, radius 16. Its header is the project and branch; text tabs (Files,
  Changes with its count, Branches, History, Timeline, References) with a 2px accent underline; Changes lists the
  changed files (`+N −N`) and the picked file's diff (added lines tinted with `--ok`, removed with `--danger`, hunk
  headers in the accent). It reads the focused pane's repository and never changes it. **Restore** on a snapshot asks
  first and is disabled while the agent works; **Compare** picks two entries. A tab that cannot load shows one calm
  screen with Try again. A folder that is not a git project says so.
- **Quick-look links.** ⌘-click (Ctrl-click elsewhere) on a file name in a pane's output, with an
  optional `:line`, opens it in Inspect › Files at that line. A name that is not a file in the
  pane's repository says so in a toast.
- **Find (⌘F).** A bar over the top right of the panes. **Pane** looks in the focused pane's
  scrollback (Enter next, Shift+Enter previous); **All sessions** lists lines from every session's
  recent output, and a click shows that session.
- **Worktrees (⌘⇧N).** The New worktree dialog names a branch and what to start it from (an
  existing branch is checked out instead), then opens a session in the new folder in a new tab. A
  project's right-click menu has a **Worktrees** submenu that opens a shell in any of the repo's
  worktrees, and a **New worktree…** row. Worktrees group under their parent repo. A folder's row shows
  only its name: a branch belongs to a session, and a worktree's branch is not the folder's.
- **Ended panes.** A pane never disappears because a refresh missed its session or the session ended: it
  stays in its tab with its output, the tab keeps its name, and a card along the bottom says **Session
  ended** with **Restart shell** (a new shell in the same folder, in the same place) and **Close pane**
  (which stops nothing). Only the first list after launch drops panes of sessions that died while the app
  was closed. The sidebar still lists only the sessions the daemon lists, and an ended session's row goes.
- **Switching sessions.** A pane shows at once: the last screen it had (kept in memory per session,
  `features/terminal/screenCache.ts`, bounded in size and count) or the replay in one write per frame. A
  pane on this computer never shows a spinner, overlay or "Connecting" text. A pane on another machine
  that has not attached after 300 ms shows a 24 px strip under the pane header (spinner, **Catching up
  from <machine> · N ms**, a 2 px progress line on its top edge) that sits in the flow, never over the
  terminal, and goes when the stream is live. A real failure keeps its banner; it is not a loading state.
- **Sidebar footer and updates.** The footer's left slot shows the version, or what an update needs
  from you: a pill that opens a card upward with the matching actions. An update waiting to be downloaded or
  installed is a glass button, a sheen sweeping across it, a tile with an arrow falling into a tray, **Update** and the
  version in mono (**Restart to update** once downloaded); the download's percent and **Update failed** keep a tinted pill. The updater downloads first and installs only on
  **Restart now**, or by itself when the app is idle, so sessions keep running either way. Settings › General › Updates shows the state with
  one fitting button and two switches: download automatically (on) and **Install updates when idle** (on, helper
  "Never while an agent is working"). Idle means no agent working or waiting on an answer, no keyboard input
  for 5 minutes, and the window unfocused or minimized. After the relaunch a pill toast at bottom centre reads
  "Updated to X · What's new" for 9 s (dismissable), and What's new opens a 460 px dialog with the release's
  notes and "No session was interrupted."
- **Sidebar position.** Settings › Appearance › Sidebar › Sidebar position, or the palette, puts the sidebar
  on the left or right. On the right the window mirrors: the resize edge is on its left, the narrow-window
  overlay slides from the right, Inspect and the Inbox open from the left, and popovers open toward the middle.
- **Notifications.** Settings › Notifications has one switch each for needs you, done and error. With
  one on, a session that changes to that state while you are not looking at it (the window is in the
  background, or it is not the focused pane) sends a system notification, once per change. It names
  the session and where it is and never what the agent asked. Clicking it brings the window forward
  and shows that session. Opening the app, a session you are in, and a reconnect after the daemon
  was unreachable send nothing. The in-window notification card follows the state: green for done, amber
  for needs you, red for an error. Its position (any of the six corners and edges) is set in Settings ›
  Notifications, which can show a test card.
- **Interrupted agents.** When the daemon restarted while an agent was mid-turn in a desktop session,
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
- **Inbox detail.** A needs-you card shows what the agent is asking, the exact command or file and its
  choices as the agent printed them, instead of its last prompt.
- **Status is shape and word.** Working is the user's spinner, Needs you an amber tile with a dark `›`, Done a green
  check, Unknown a dashed square, and Idle nothing; a stopped agent or an exited shell says so in words on its row.
  Amber still means needs you and only that, in every theme.
- **Colours follow the theme.** Agent tiles, badges and the folder text are
  expressions over the active theme's tokens (`--accent`, `--ansi-*`, `--warn`, `--danger`,
  `--on-accent`), never fixed hex values, so switching theme recolours all of them.
- **Status marks, not effects.** Every state has its own shape on a row's or rail icon (`StatusBadge`), so colour
  is never the only signal: **working** is the user's spinner in `--accent`, on the icon's corner; **needs you** an amber tile with
  a dark `›`, popping once on arrival; **done** a dark green tile with a ✓; **exited** a dark red tile with a !
  (the daemon's `TermInfo.exit`, a non-zero exit code of the last command, shown while nothing is working or
  waiting); **idle** no badge at all; **unknown** (the daemon is gone) a dashed outlined square, with the
  row dimmed. The window only displays these; the daemon decides them. There are no waves, sweeps, shimmer or glow
  on rows and tabs, and no setting for them; the only looping motion is the spinner. Reduce motion (the setting or
  the system's) removes the pulses, slides and size animations, and the spinner holds one frame. The needs-you badge and the Inbox count pop once with a ring (.6s) and
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
- **Themes** are Midnight (default), Abyss, Paper and Day (a cool light theme), themes generated from six
  inputs (`bg fg accent mag blue green` and a light flag) by Oklab mixing, and **High Contrast** (pure
  black and white, lines `#5A5A5A`/`#8A8A8A`, amber and red kept). Every theme's `--pane` is its opaque
  `--main`. The foreground, terminal text (`--fg2`), muted and dim text keep 4.5:1 on both `--term` and the
  window `--bg`, and Midnight, Paper and Day also hold it for accent, amber, red, green and the ANSI blue,
  magenta and green; the theme tests check this. Every light theme holds it too, for accent, amber, red,
  green, blue, magenta and all 15 ANSI text colours but black (the generated ones darken a published hue in
  place until it passes). Paper and Day use a darker amber and red than the generated light themes. A pane's
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
