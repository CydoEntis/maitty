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
title, a detail line and an optional preview in italics. A question waiting on you also has its
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

- **Floating cards on an ambient ground.** The window is `--bg` under two soft radial glows (an
  accent one top left, a violet one bottom right). The sidebar, the tab tray, each pane and
  Settings are cards: a faint top highlight over `--pane`, a `--pane-line` hairline, radius 14
  (Settings 16), a soft drop shadow and an 18px backdrop blur. 8px outer margin, 12px between panes.
- **Window style: Flat, Tiled or Glass** (Settings › Appearance › Window style, `style`, stored with the other
  preferences; the root carries `data-style` and `data-finish`). The finish and the layout only; features and
  status colours do not change. The text above is **Glass**, the default for a fresh install: floating frosted cards, colour icons, glow and the status sweeps.
  **Flat** makes the window `--bg` with no glow, cards flat `--main` with
  radius 8 (Settings 10), no blur or resting shadow, flat top-bar buttons and status pills (no gradient, lift or
  entrance), no outline on the selected row or open tab, and the session tray flush under its main row with no
  box. **Tiled** is Flat's finish pulled edge to edge: square corners, 4px gaps between panes and at the
  window's edge, the sidebar flush with a hairline, and a 40px tab bar that is a flush strip (tabs, **+**,
  Repository and Inbox divided by hairlines, one line under the bar). Flat and Tiled share `data-finish="flat"`;
  The styles differ only in CSS variables set on the app root (`--rk` radius multiplier, `--r14`/`--r16`, `--pg` pane gap, `--ptop`, `--tbh`/`--tbp`/`--tbg`/`--tbb` for the tab bar, `--tabh`/`--tabbr` for tabs, `--sh` overlay shadow, and a few more beside them in `workspace.css`); the layout rules read them, so a style is a block of values, not a second set of rules. The focused pane's ring and the needs-you amber
  stay in all three. Glass starts with the three status sweeps on and Flat and Tiled with them off; a choice made
  in Settings wins. Settings saved as `minimal` read as Flat and `expressive` as Glass. **Icons** (Settings › Appearance › Icons, `iconColor`, set on the root as
  `data-icons`) is separate from the style: `color` keeps app and folder icons in their own colours in every
  style; `grayscale` makes them neutral `--raise` tiles with the glyph in `--fg2`.
- **Settings dialog** (⌘,). A section list at the left and, at the right, a fixed header (the page title,
  a one-line description and a **Close** button with an `esc` keycap) above a body that scrolls; the page
  itself never scrolls. The title and description slide with the page.
- **Sidebar.** A `Projects` title with a **+** button (a bordered square that lights up in the accent on hover) and the collapse button at the right; groups by git repo (or by folder, in Settings), sorted by
  name. A shell outside any repo is grouped by its own folder, so there is no catch-all group.
  A group's tile carries a ring when it holds the focused session. Each group is one **main row**
  with a **tray** of its other sessions hanging off its bottom edge. The main row is the process
  icon with its status badge, the folder within the repo, then what the session is using and a
  status phrase. The lead is the session you are in, else the last one picked in that project, else
  the most urgent; Settings › Appearance › Sidebar can make it always the one that most needs you.
  Urgency runs needs you, error, done, working, idle, then most recently changed. The tray comes in
  three styles: **With names** (icon and process; two fit, more go behind **+N**), **Icons** (six
  fit) and **Stacked** (one line each with folder and state; three fit; the default). **+N** turns amber when a
  hidden session needs you and opens a side menu listing every session of the project. Clicking a
  session focuses its pane where it is open, else opens it in a **new tab**; it never replaces
  another pane. When the lead changes the new main row fades in. ⌘B folds
  it to a 60px rail. Every folder has its own colour, chosen from its path, in the sidebar and the rail.
  In the rail each project is a 48px group (radius 14, square in Tiled) tinted in its colour (6% collapsed,
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
  knows when it first saw the change. Anything with a `title` shows a themed tooltip after a short
  hover (`HoverTips`), not the browser's.
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
- **Command palette (⌘K).** A 600px card over a dimmed scrim: type a few letters to jump to a
  session, project, action or theme (themes only once something is typed). A run of the typed text
  ranks above letters that merely appear in order; ↑ ↓ move, ↵ runs, Esc closes. Sessions that need
  you say so on their row.
- **Inbox (⌘J).** The tab bar's **Inbox** button (a count, amber when something needs you) opens a
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
- **Project colours and menu.** Settings › Appearance › Sidebar lets each project take a colour from
  teal, violet, blue, green, pink or orange (never amber or red). Right-clicking a project, in the
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
  worktrees, and a **New worktree…** row. Worktrees group under their parent repo. **Close project** ends
  every session in it. Anything that ends a session (the row's ×, a pane's or row's **Close session**,
  **Close project**) opens one confirmation dialog first; closing a tab or pane only hides a view, the
  sessions keep running, and does not ask.
- **Notifications.** Settings › Notifications has one switch each for needs you, done and error. With
  one on, a session that changes to that state while you are not looking at it (the window is in the
  background, or it is not the focused pane) sends a system notification, once per change. It names
  the session and where it is and never what the agent asked. Clicking it brings the window forward
  and shows that session. Opening the app, a session you are in, and a reconnect after the daemon
  was unreachable send nothing.
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
  shows its status as a chip in its bottom-right corner, including Idle, and hovering or focusing
  the chip opens a popover that says what the state means. Amber still means needs you and only
  that, in every theme.
- **Colours follow the theme.** Folder colours, agent tiles, badges and the folder text are
  expressions over the active theme's tokens (`--accent`, `--ansi-*`, `--warn`, `--danger`,
  `--on-accent`), never fixed hex values, so switching theme recolours all of them.
- **Status effects.** Working rows and tabs carry a slow teal sweep, Needs you a breathing amber
  glow behind the icon, Done one green sweep. Each has its own switch; Reduce motion (the
  setting or the system's) and a disconnected daemon stop all of them, with every other
  animation.
- **Process icons** are generic tiles, never a product's mark: `>_` shell, `✻` codex, `rs` cargo.
- **Themes** are Midnight (default), Abyss and Paper, themes generated from six inputs
  (`bg fg accent mag blue green` and a light flag) by Oklab mixing, and **High Contrast** (pure
  black and white, lines `#5A5A5A`/`#8A8A8A`, amber and red kept). Muted and dim text keep 4.5:1
  on the pane; the theme tests check every theme. A pane's xterm palette comes from the same
  tokens. **Settings › Appearance › Theme** has an Appearance switch (System, Dark or Light:
  `themeMode`) and a searchable dropdown each for the dark and the light theme (`darkTheme`,
  `lightTheme`); the dropdown lists its theme in use first. System follows the OS's own dark or
  light setting as it changes. Hovering a theme in the slot that is in use previews it on the
  whole window; leaving the list or closing the menu restores it, and picking commits it. A
  single theme saved by an older version fills its own slot and pins the mode to it.
- **Type.** Geist for the UI, Geist Mono for terminals, paths, counts and keys, both bundled.

