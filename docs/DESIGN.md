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
- **Shape and word carry the meaning; colour backs them.** Every state reads in the ASCII
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

**Machines.** With one machine the sidebar is exactly as above. Once the window knows another
machine, every machine's folders sit under a header, this computer first and the others in their
stable id order (the sidebar never reorders itself, ADR 0034): `this computer` in `strong` bold,
then `⇄ build` with its connection state at the right, a glyph and a word in a theme role
(`connected` in `done`, `connecting` and `reconnecting #n` in `muted`, `needs login`, `needs
update` and `unreachable` in `err`; amber stays for agents that need you). ASCII marks read
`<> build` and `+`, `.`, `~`, `x`. The name is cut with a `…` before the word gives way to the
glyph. A machine that dropped keeps its folders and sessions, drawn again in `muted`: no
bold, no amber, the working mark held still. Under its header two `muted` lines say
`disconnected since 2m ago` and why (`text`, the exact fix the link failure carries, wrapped onto
up to three rows at the sidebar's width, the last ending in `…` only past three). Its needs-you and failed marks leave the title's counts and the Inbox
until the line is back, and everything returns to normal on its own. A connected machine's header
may carry up to two `muted` lines of what its own daemon reports, never added to another's: its
plan limits (`claude 5h 23% · week 41%`) and `$9.00 today`, with `daemon 0.50.0` when that is not
this window's version. Collapsed, a machine other than this one is a row `●B` (its state glyph,
coloured as above, and its first letter) over its folders.

The **Machines dialog** (leader `w`) is a standard modal: the same state chips and words as the header,
the fix of a down machine written in full under it in `text`, the Add field on `card2`, and a dim line
for Windows. Its behaviour is in [UI](UI.md#project-specific-machines-dialog).

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
The lit tab names the open view, so the sheet's top border does not repeat it: Every tab
carries only the ✕. The Inbox and the check evidence
sheet dock in the same place without the strip. See UI › *the frame* (Sheet), *Inbox check
groups* and *check evidence sheet*.

## Modals and popovers

The Menu and a row's context menu have no fill of their own: they are drawn on the ground like the panes. Everything else that floats over the screen is boxed as the look boxes a pane: a `line` box round
`card` with corners by `ui.rounded`
([ADR 0028](adr/0028-tab-rows-and-popups-drawn-like-panes.md)). Deciding is a modal: centred, framed like a pane (the title as an accent chip on the top border,
a red ✕ chip at the right), its border in the accent because it holds the keys, the screen behind
dimmed in text and ground alike so the card stands out, no shadow. Settings, override, command
approval and the key overlay are modals. Small explanations (why, a row's menu, the message and
helpers popovers) open beside their row with a `◂` pointer; a menu row ends in its key, the bound leader
key when there is one. Every panel ends in an action bar
of keycaps. See UI › *UI conventions*, *check modals and settings* and *why popover*.

## Toasts

One component for every notice: bottom right, above the agent's input area, a small floating
pane: a `card` boxed in the notice's colour (`err`, `blocked` for needs you, `done`, else the
accent) with a kind chip carrying the notice's mark on the top border (`▲ Needs you`,
`✕ Error`, `✓ Done`, `i Hint`, `✓ Notice`) and the modals' red ✕ chip at its right, then a bold
title, a detail and an optional preview in italics. Every text row wraps at the toast's width
instead of being cut: the title and the detail onto up to three lines, the preview (an excerpt) onto
up to two, the last line ending in `…` only past that; the box grows with the lines. A question waiting on you also has its
two answers as buttons on the bottom border: a green `Answer` that goes to the session and a
red `Later esc` that dismisses it. Always round corners (ADR 0030). It slides in and fades out
with motion on. See UI › *the frame* (Toast).

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

## Desktop app

The desktop app is a terminal multiplexer window ([ADR 0046](adr/0046-desktop-app-is-a-terminal-multiplexer.md)):
no chat, every session a shell that maittyd owns. The design handoff (v10, terminal-first) is the
reference for exact sizes, shadows and timings; `workspace.css` carries them.

- **Floating cards on a flat ground.** The window is the theme's `--bg`. The sidebar, the tab tray, each pane
  and the big panels (Inbox, Inspect, Find) are cards: opaque `--card` (`--main`), a 1px `--pane-line` hairline,
  radius 14 (Settings 16), 8px outer margin and 12px between panes. Dark themes add a 1px top highlight
  (`--sh`, an inset line with no blur radius); light themes add nothing. Cards have no resting drop shadow and
  no gradient, so a spinner ticking inside one repaints only its glyph. Only small, short-lived surfaces blur
  what is behind them and cast a soft shadow: menus, popovers, the palette, Settings and other centred dialogs
  with their scrims, and notification cards (`--blur-pop`, `--sh-pop`). The sidebar, panes, tab tray and
  drawers never blur. **Background glow** (Settings › Appearance, `backgroundGlow`, set on the root as
  `data-glow`) is off by default; on, the ground is `--bg` under two faint tints, one of the accent and one of the
  magenta, painted once behind the opaque cards. A pane's terminal sits on the theme's opaque `--term`
  ground, and ANSI black is that ground, so a program that paints with it (Claude Code under its prompt and status
  line) leaves no box. Settings › Appearance › See-through terminal lets the card show behind the text instead, only
  when xterm draws with WebGL (the DOM renderer repaints rows, and each would blend through the card); such programs
  then show as boxes on it. On a web view without WebGL 2 (`data-gpu="software"` on the root) Glass uses the same
  rules with cheaper token values: no blur, a 1px ring instead of the soft popover shadow, and always a flat
  ground; the colours, radii, layout, focus ring and status colours stay ([PERFORMANCE](PERFORMANCE.md)). The
  focused pane has one even `--accent-line` border all the way round, flashed once when focus arrives, and no
  extra line on its header.
- **Window style: Flat, Tiled or Glass** (Settings › Appearance › Window style, `style`, stored with the other
  preferences; the root carries `data-style` and `data-finish`). The finish and the layout only; features and
  status colours do not change. The text above is **Glass**, the default for a fresh install: floating opaque cards, colour icons and the status sweeps.
  **Flat** makes the window `--bg` with no glow, cards flat `--main` with
  radius 8 (Settings 10), no blur or resting shadow, flat top-bar buttons and status pills (no gradient, lift or
  entrance), no outline on the selected row or open tab, and the session tray flush under its main row with no
  box. **Tiled** is Flat's finish as a hairline grid: no gap between panes or at the window's edge, every region
  divided by a 1px `--pane-line`, radius 0 and no shadows. The sidebar is flush with a hairline; the title bar is
  `--main` with a hairline under it; the 40px tab bar is a flush strip (tabs, **+**, Repository and Inbox divided
  by hairlines, one line under the bar); each pane draws its own right and bottom line and a line under its
  header; and a static 9px `+` tick (`--tick`, `--line2`) marks each of the four corners of the pane area.
  Popovers, menus and dialogs are hairline-only (`0 0 0 1px var(--line2)`), square and with no blur. Chrome type
  (tab titles, pane process names, project names, the status bar) is Geist Mono, uppercase, letter-spacing
  .1em, 11 to 11.5px, through `--ch-ff`, `--ch-tt`, `--ch-ls`, `--ch-fs-sm` and `--ch-fs-md`, which only Tiled
  sets (the rules fall back to the old values elsewhere); terminal text and session names are unchanged.
  Flat and Tiled share `data-finish="flat"`;
  The styles differ only in CSS variables set on the app root (`--rk` radius multiplier, `--r14`/`--r16`, `--pg` pane gap (0 in Tiled), `--ptop`, `--tbh`/`--tbp`/`--tbg`/`--tbb` for the tab bar, `--tabh`/`--tabbr` for tabs, and a few more beside them in `workspace.css`); the layout rules read them, so a style is a block of values, not a second set of rules. The focused pane's ring and the needs-you amber
  stay in all three. Glass starts with the three status sweeps on and Flat and Tiled with them off; a choice made
  in Settings wins. Settings saved as `minimal` read as Flat and `expressive` as Glass. **Icons** (Settings › Appearance › Icons, `iconColor`, set on the root as
  `data-icons`) is separate from the style: `color` keeps app icons in their own colours in every
  style; `grayscale` makes them neutral `--raise` tiles with the glyph in `--fg2`. Project tiles are always neutral.
- **Settings dialog** (⌘,). A section list at the left and, at the right, a fixed header (the page title,
  a one-line description and a 30x30 **×** icon button, tooltip "Close (Esc)", no label) above a body that scrolls; the page
  itself never scrolls. The title and description slide with the page. Under Advanced, **Performance** shows live
  frame, drawing, output and process figures with a **Copy report** button, and a switch for a corner readout over
  the window ([PERFORMANCE](PERFORMANCE.md#measuring-in-the-app)).
- **Shortcut overlay** (⌘/ or Ctrl+/, also in the palette; the table above lists the keys). An 820px dialog over a blurred scrim, titled
  "Keyboard shortcuts" with a Close `esc` button, holding an auto-fill grid (230px minimum) of Sessions,
  Panes, Find & jump and Window with each action and its keycaps (several chords stack). It is generated
  from the same table (`hotkeyRows`, each row has a group) that Settings › Hotkeys reads, so the two never
  differ. Other shortcuts are ignored while it is open; ⌘/ and Esc close it.
- **Session menu: name, pin, close.** Right-click a sidebar row, a tray item or a rail icon for a 260px
  menu: a **Name** field (the folder is its placeholder; Enter saves and an empty name clears it), **Pin to
  top** or **Unpin**, **Open** and **Close session** (ends it). A name replaces the folder title everywhere.
  A pinned session shows a pin on its main row, leads its project when no focused session is in it, and
  sorts first in the tray. There is no Archive; a session archived by an older version shows as an ordinary
  row. The daemon stores the name and pinned per session (`RenamePane`, `PinPane`; they survive a daemon
  restart), so the TUI shows the names too and ignores the pin.
- **Status bar.** A 28px line at the bottom of the main area under a hairline, 11.5px muted, one line.
  The left is empty, except "Reconnecting to maittyd…" in amber with a dashed dot when the daemon is
  unreachable. The right holds quick controls: a **theme** menu (Dark, Light or System, a few themes of
  the kind in use, and "All themes in Settings…"), the **layout style** (Flat, Tiled or Glass), the terminal
  **text size** (−, the size which resets it to 14px, +; Ctrl+= and Ctrl+− do the same) and the shortcuts
  keycaps, which open the shortcut overlay. Hidden in Focus mode and by Settings › General › Window ›
  Status bar (on by default).
- **Sidebar.** A `Projects` title with a **+** button (a bordered square that lights up in the accent on hover) and the collapse button at the right; groups by git repo (or by folder, in Settings), sorted by
  name. A shell outside any repo is grouped by its own folder, so there is no catch-all group.
  A group's tile carries a ring when it holds the focused session. Each group is one **main row**
  with a **tray** of its other sessions hanging off its bottom edge. The main row is the process
  icon with its status badge, then the program and where it works (`claude · dev`: the branch at the
  checkout's root, the folder within the repo deeper in, the `~` path outside a repo), then a status
  phrase when the session is not just working or idle. The lead is the session you are in, else the last one picked in that project, else
  the most urgent; Settings › Appearance › Sidebar can make it always the one that most needs you.
  Urgency runs needs you, error, done, working, idle, then most recently changed. The tray comes in
  three styles: **With names** (icon and process; two fit, more go behind **+N**), **Icons** (six
  fit) and **Stacked** (one line each with folder and state; three fit; the default). **+N** turns amber when a
  hidden session needs you and opens a side menu listing every session of the project. Clicking a
  session focuses its pane where it is open, else opens it in a **new tab**; it never replaces
  another pane. When the lead changes the new main row fades in. ⌘B folds
  it to a 60px rail. Every project tile is the same neutral (`--raise` with the glyph in `--fg2`); a project has no colour of its own.
  In the rail each project is a 48px group (radius 14, square in Tiled) tinted in `--fg2` (6% collapsed,
  9% open, 4% more when it holds the focused session) with an inset ring (20%, 34% focused); the focused
  project also has a 3px bar at the rail's left edge. Its tile is an outlined folder filled 14% (26%
  focused) with two letters inside: the initials of hyphenated parts (`maitty-docs` is `MD`), else the first
  two letters (`maitty` is `Ma`). Badges on the tile: an amber square when something needs you; when the
  group is folded, a count pill and a spinner if anything in it is working. An open group has a 20px
  hairline under the tile, then its session icons. **Rail popovers** open beside the item on hover or keyboard focus after 280ms
  (instantly once one is open), 10px to its right with an arrow pointing at it, and close 160ms after the
  pointer leaves both the item and the popover, so the popover can be used. A project's (260px) shows its
  colour, name, path and branch, then every session in priority order (icon, name, process, state word),
  each one clickable, and the hint "Click the folder to collapse · right-click for more". A session's
  (240px) shows its project and name, its state, process and path, a mono well with the pending approval
  command (amber) or the last prompt (none for a plain shell), and "In tab N" when it is open in one.
  They replace the browser's `title` tooltips in the rail. Under 1100px the sidebar is not on screen: a **Sessions** button at the start of the tab bar (and ⌘B) slides it in over the panes on a scrim, and picking a session, starting one or opening Settings, a click on the scrim or Esc closes it.
- **First-run welcome.** On the first launch (no saved settings and no earlier welcome) a 560px dialog opens
  over a blurred scrim with a progress bar per step: **Welcome** (the same sessions as the TUI, closing never
  stops an agent, and the **New sessions start in** folder, checked before it continues) and **You're set**
  (six keycaps, then **Start a session**). Skip and Esc leave it; global shortcuts are ignored while it is
  open. A window that already has settings is never greeted. Settings › General › **Show the welcome again**
  replays it. The "Bring your TUI setup" step comes with the shared config.
- **Interface size.** Settings › Appearance › Interface size sets the sidebar icon size (Small 0.8, Medium 1,
  Large 1.2 times the project tiles, session icons, tray chips and the rail, whose width follows), the density (Compact 0.8,
  Comfortable 1, Spacious 1.25 times the row, tab and pane spacing), the sidebar text size and the
  tab text size (12, 13.5, 15 px). They are CSS variables on the window root (`--pad`, `--sidebar-fs`,
  `--tab-fs`, `--icon-scale`), so every size rule that reads one follows it. The docked sidebar's right edge is a
  drag handle (`role=separator`, 200–420 px, default 264): the width follows the pointer and is saved
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
- **The tab bar shows one project.** Only the tabs of the project the active tab belongs to are on
  the bar (a tab's project is that of its focused pane's session, so it moves when the shell cds
  into another repo). Picking a session of another project in the sidebar switches the bar to that
  project. ⌃Tab and ⌃⇧Tab cycle only these tabs, and closing a tab goes to the next one of the same
  project. The **+** after the tabs starts a shell in the project's folder and says so ("New
  session in maitty"); ⌘T and the sidebar's **New** use the start folder.
- **Tabs that do not fit scroll.** When the tabs run off the bar, a scroll button appears at each
  end that has more tabs beyond it, the ends fade, and the mouse wheel scrolls the tabs sideways.
  The active tab is always scrolled into view (measured against the bar, 30px of room), so
  switching tabs never lands off-screen; the new-tab button stays put at the end. An **All tabs**
  button (a count and a chevron, with an amber diamond when a tab needs you) opens a menu that
  lists every tab of the project; Esc or a click outside closes it.
- **A tab says what is in it.** A tab shows a small icon for each pane it holds (up to three,
  overlapped) and `+N` for the panes beyond the first; hovering it opens a list of every pane with
  its agent, folder and state. A collapsed session's popover says "Open in tab N"; the expanded rows
  carry no tab number. A working row shows its spinner and no run time, because the window only
  knows when it first saw the change. Anything with a `title`, and every icon-only button, radio or switch
  (by its `aria-label`), shows a themed tooltip (`HoverTips`), not the browser's: 420 ms after the
  pointer rests, 40 ms when another tip was on screen in the last 400 ms; hidden on a press, a key
  or the wheel; held back while a menu or a rich hover card is open. A trailing `(keys)` becomes
  keycaps and text after ` · ` a muted second line. It sits above the control, below it within 56px
  of the top, and beside it for a control marked `data-tip-side`. A control that opens its own hover
  card carries `data-hovercard` and gets no tip.
- **Tabs hold layouts, panes hold sessions.** A tab is one to four panes in columns; closing a
  pane or tab detaches and the session stays in the sidebar. Ending a session is its own action
  on the sidebar row, a pane's menu or a row's menu. Right-click opens a menu on a pane (split,
  maximize, rename, copy folder path, close pane, close session), a tab (new tab, close tab) and a
  session in the sidebar, expanded or collapsed (show in pane, rename, close session). Rename opens a
  small dialog; the name is the daemon's, so every app shows it, and it replaces the folder on the
  row, the tab and the pane header. An empty name goes back to the folder.
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
  too. A binding with `available: false` (Session panel, Usage, Terminal/Chat, Undo
  close) is registered but its key does nothing and the overlay hides it until the feature lands.
  Jump to the next session that needs you is ⌘J on macOS only: Ctrl+J is a terminal key.
- **Esc ladder.** Esc closes the topmost open layer and reaches the terminal only when none is
  open (`model/escLadder.ts`): comment composer, New agent and What's new, Overview, palette,
  inbox, Settings, the sidebar drawer, Focus mode. A new dialog adds its id there.
- **Overview (⌘0 / Ctrl+0).** The sidebar's pinned **Overview** row (a 40 by 36 icon in the
  rail) carries an amber badge with the number of sessions that need you, hidden at 0. The screen
  sits over the panes (they stay mounted, so no terminal is resized or replayed) under the tab bar;
  a tab or a card leaves it, Esc closes it. One card per session across every machine, in the
  sections Needs you (always shown, with a dashed "Nothing needs you" box when empty), Working,
  Done (exited sessions included, keeping their red shape), Idle and Unknown; empty sections are
  hidden. Cards keep creation order inside a section (`features/overview/model/overview.ts`), so a
  status change moves a card between sections and never reorders the others. A card shows the
  status shape (still, never animated) and name, the project, the branch, the machine chip when
  remote, one context line (what a stopped agent is asking, else the process and folder) and the
  state with the time in it. **Jump to next** (⌘J, macOS only) moves a ring and a one-shot pulse
  to the next Needs-you card on the Overview; elsewhere it opens that session with the toast
  "Needs you · name · 1 of 2"; with nothing needing you it only shows "Nothing needs you".
- **Command palette (⌘K).** A 600px card over a dimmed scrim: type a few letters to jump to a
  session, project, action or theme (themes only once something is typed). A run of the typed text
  ranks above letters that merely appear in order; ↑ ↓ move, ↵ runs, Esc closes. Sessions that need
  you say so on their row.
- **Inbox (⌘⇧J).** The tab bar's **Inbox** button (a count, amber when something needs you) opens a
  panel listing every session that needs you, failed or finished, across all projects, in that
  order. A needs-you card has **Yes, once**, **This session** and **No**, which type `y`, `a` or `n`
  into that session; the window never decides for it. Done and error cards have **Clear** (hides it
  until the session changes state) and **Open**.
- **Focus mode (⌘.).** Hides the sidebar and tab bar; a pill at the bottom says where you are and
  how many sessions need you elsewhere. Esc or the pill's Exit leaves it.
- **Broadcast (⌘⇧B).** Per tab. While on, every pane of the tab has a violet outline and a
  "Broadcast" pill, and what is typed in one pane is also typed into the tab's **idle** panes.
  A pane running something is skipped, so it never types into a busy agent.
- **Pane resize.** The gap between two columns drags (180px minimum, snaps to an even split,
  double-click resets). Widths are saved with the tab and reset when its columns change.
- **Drag a session into a pane.** Any sidebar row or tray item drags onto a pane: in the same tab it
  swaps, in another tab it moves here, otherwise it replaces the pane's session. The session that
  leaves keeps running.
- **Files dropped on a pane of another machine.** One sonner toast on the theme tokens (`bg-muted` track,
  `bg-primary` fill, `text-muted-foreground` percentage): the title `Sending <file> to <machine>`, with
  `(2 of 3)` for a drop of several, a thin `role="progressbar"` named `Sending <file>`, and sonner's own
  action button **Cancel**. It has no close mark while it runs. It is then replaced by a success, error or
  plain toast. See UI › *features that need a pane's files*.
- **Project menu.** Right-clicking a project, in the
  sidebar or the rail, offers New session here, Collapse or Expand, and Copy path.
- **Inspect (⌘I).** A card docked at the right of the window (over the panes under 900px) that
  reads the focused pane's repository and never changes it: **Files** (folders and a text
  preview), **Changes** (the working tree's diff, one file at a time), **Branches** (the current
  one, its distance from the upstream), **History** and **References**. The tab bar's **Inspect**
  button carries the focused pane's `+N −N`. History lists commits and the daemon's snapshots
  (taken before each agent turn) newest first. **Restore** on a snapshot asks first, says that
  what is there now is saved first, and is disabled while the agent works. **Compare** picks any
  two entries and shows what differs between them.
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
- **Close ends the session.** A pane's ×, a tab's × (and its menu's **Close tab** and **Close other
  tabs**), a session's **Close session**, **Close project tabs** and Ctrl+W all end the sessions they
  close, and a notice says so. When a program such as an agent is running, the first click only arms the
  control ("Close and stop claude? Click again", also for **Close other tabs**); a plain shell closes at
  once. Ctrl+W, which a shell also uses to delete a word, always asks to be pressed twice, for the same
  pane or tab, unless that pane already ended. Esc that leaves focus mode or closes a panel is not also
  sent to the terminal. There is no undo. Closing the window never ends a session.
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
  from you: a pill (**Update to x**, the download's percent, **Restart to update**, **Update failed**) that
  opens a card upward with the matching actions. The updater downloads first and installs only on
  **Restart now**, or by itself when the app is idle, so sessions keep running either way. Settings › General › Updates shows the state with
  one fitting button and two switches: download automatically (on) and **Install updates when idle** (on, helper
  "Never while an agent is working"). Idle means no agent working or waiting on an answer, no keyboard input
  for 5 minutes, and the window unfocused or minimized. After the relaunch a pill toast at bottom centre reads
  "Updated to X · What's new" for 9 s (dismissable), and What's new opens a 460 px dialog with the release's
  notes and "No session was interrupted."
- **Sidebar position.** Settings › Appearance › Sidebar › Sidebar position, or the palette, puts the sidebar
  on the left or right. On the right the window mirrors: the resize edge is on its left, the narrow-window
  drawer slides from the right, Inspect and the Inbox open from the left, and popovers open toward the middle.
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
- **Usage pill.** A pane's header shows what the provider reports the session has used (tokens,
  context filled, cost). It is not drawn when the provider said nothing, so no number is invented.
- **Turn timeline.** Inspect › Timeline lists the focused pane's latest events, newest first: the
  agent started working, stopped for you, finished (with how long the turn ran), and, in a shell
  that marks its prompts, each prompt and command with its exit code.
- **Inbox detail.** A needs-you card shows what the agent is asking, the exact command or file and its
  choices as the agent printed them, instead of its last prompt.
- **Status is shape and word.** Working is the TUI spinner, Needs you an amber tile with two pause
  bars, Done a green check, Exited a red `!`, Unknown a dashed square, Idle a small dot. Every pane
  but an idle one shows its status as a chip in its bottom-right corner, and hovering or focusing
  the chip opens a popover that says what the state means. An idle pane has no chip, and an idle
  session's sidebar row has no state line: idle is the default and says nothing. Amber still means needs you and only
  that, in every theme.
- **Colours follow the theme.** Agent tiles, badges and the folder text are
  expressions over the active theme's tokens (`--accent`, `--ansi-*`, `--warn`, `--danger`,
  `--on-accent`), never fixed hex values, so switching theme recolours all of them.
- **Status effects.** Settings › Appearance › Motion has, for Working, Needs you and Done, an on/off switch, a
  **Waves / Sweep** choice and a live preview chip clipped to the chip. **Waves** are two layers of soft sine
  bands (SVG paths in the status colour at low alpha) that drift at different speeds and in opposite directions
  (a 200%-wide layer translating -50% over 18 s and 28 s, linear). It is the default for Working and the one
  deliberate loop in the window; it stops while the window is hidden or in the background (`data-window` on
  `<html>`). **Sweep** is one soft band that crosses once in 1.4 s when the status appears and then rests; the
  default for Needs you and Done. Both are an absolutely positioned `.mx-fx` child of the row or tab
  (`StatusEffect`), `overflow: hidden; contain: paint`, with `will-change: transform` on the moving layers; only
  `transform` (`translate3d`) animates, never `box-shadow`, `filter` or `background-position`. A **Working
  effect** setting (All / Selected / Off) limits which working rows and tabs animate; Selected means only the
  one you are in. A style is capped by the animation level (`lib/animation-level.ts`), set by **Animations**
  (Settings › Appearance › Motion, `animations`): **Auto** (default) starts at Waves with WebGL 2 and at Sweep
  without it (`data-gpu="software"`), then samples animation-frame times for 3 s starting 2 s after launch and
  again each time Waves begin to run (`lib/frame-monitor.ts`, `useAdaptiveAnimations`); if a quarter of the
  frames take over 40 ms it steps down one level (Waves, Sweep, none) and never steps up within a run, and the
  monitor stops itself after each verdict. **Full** always allows Waves, **Light** only Sweep, **Off** none.
  Settings › Performance says what Auto chose and why (`describeAnimations`). Reduce motion (the setting or the
  system's) and a disconnected daemon remove every layer, and a fixed choice is never measured. The needs-you badge and the Inbox count pop once with a ring (.6s) and
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
  reconnecting #n in `--dim`; `▲` needs login and needs update and `✕` unreachable and refused in
  `--danger`), so colour never carries it alone, and nothing animates. Amber stays for agents that need
  you. A machine that is down shows `disconnected since 2m ago` and its fix (wrapped, in `--dim`) under the
  header, and its folders are drawn at 60% opacity. With one machine there is no header. **Settings ›
  Machines** is a card per machine (chip, how long, the fix in full, **Retry**, a confirmed **Remove**) and an
  **Add a machine** card (a field, then the `~/.ssh/config` hosts as rows); its states are loading, empty,
  error and checking, and it works with the keyboard alone.
