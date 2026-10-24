# UI

This doc covers how the user interface is structured and built.
`AGENTS.md` is the operating contract; this doc holds the UI practices in depth.

**Delete this doc if the project has no user interface,** and remove its row
from `AGENTS.md`. Game menus and HUDs count as a user interface.

## Component structure

```text
components/ui/           primitives: button, input, dialog (design-system level, no domain knowledge)
components/              shared composites used by 2+ features
features/<f>/components/ feature components that know the domain
app/ or routes/          pages/screens: compose features, own layout and routing
```

- Primitives know nothing about the domain. Feature components know nothing
  about routing.
- Keep components small and focused. When one component fetches, transforms,
  and renders, move the fetching and transforming into a hook or service and
  keep the component for rendering.
- Pass data down and events up. Avoid prop drilling deeper than about three
  levels. Composition (`children`, slots) usually solves it before context does.
- Keep business rules out of components. They call feature hooks and services.

## State

| Kind | Examples | Where it lives |
| --- | --- | --- |
| Server state | records from the API or database | a query and cache layer, not a global store |
| URL state | filters, tabs, selected id, pagination | the URL, so it survives reload and can be shared |
| Form state | field values, touched, errors | the form library |
| Local UI state | open/closed, hover, draft text | component state |
| Shared UI state | theme, sidebar, current modal | a small store, only when several distant components need it |

- Derive values during render. Don't copy them into state and sync them with
  effects.
- Put state in the lowest component that needs it.

## Async states

Every view that loads data deliberately renders each state that applies:

- **Loading:** a skeleton or spinner that holds the layout, with no content
  jumping when data arrives.
- **Empty:** says what would appear here and, where there is one, the action
  that creates it.
- **Error:** says what failed in plain language and offers a retry. It never
  shows a raw error message.
- **Success.**
- **Partial or stale**, where it applies: refetching and optimistic states.

A mutation shows that it is pending, blocks double submission, and reports the
outcome.

## Forms

- Validate with the same schema the API uses, where possible.
- Show errors next to the field they belong to, after the user interacts with
  it. Show a form-level error for server failures.
- Disable submit while the form is submitting. Keep the user's input when a
  submission fails.

## Accessibility checklist

- The feature works with the keyboard alone, and focus order follows visual
  order.
- Focus is always visible. Dialogs trap focus and return it to the trigger when
  they close.
- Use semantic elements: a `button` for actions and an `a` for navigation. A
  clickable `div` is a defect.
- Every control has an accessible name. Every input has a label.
- Color is never the only signal, and text contrast meets WCAG AA.
- Status and errors are announced without stealing focus.
- Honour `prefers-reduced-motion`.
- The layout works at 200% zoom and at the smallest supported width.

## Styling

- Use design tokens (color, spacing, radius, type scale) rather than hard-coded
  values. A hex color or pixel value in a component is a magic value.
- Build mobile-first, with the project's breakpoints.
- Support light and dark themes through tokens, if the project supports both.

## Performance

- Split code by route. Lazy-load heavy, rarely used views.
- Virtualize long lists.
- Memoize only when there is a measured problem or a clear structural reason.
- Give images explicit dimensions and use modern formats.

## Project-specific: design system

A terminal UI (ratatui). Colours come only from the theme (`theme.rs`): semantic
slots `bg`, `card`, `card2`, `hov`, `accent`/`acc_ink`, `text`/`strong`/`muted`,
and status colours (`working`, `blocked` = needs you, `done`, `idle`, `err`).
A working row is one colour, `accent`, across its glyph and its name (shimmering with motion on). Needs you (`blocked`) is amber and failed (`err`) red in every theme, so they never read alike. `dark` sets its amber by hand; `mono` uses named yellow and red; every other theme keeps an amber of its own that stands apart from its other statuses, or gets one derived from its yellow (`with_distinct_needs` in `theme.rs`, darker on light themes). The audit in `theme.rs` asserts needs you looks unlike every other status, on the row and lit. A theme's amber `working` slot stays out of the sidebar. Chips on the check sheet, the Inbox's group heads and the approval tint use the same slots (`client/v7_frames_tests.rs` asserts the roles).
Every built-in theme passes a WCAG contrast audit test. Shared primitives in
`client/design.rs` and `client/screen/`: `panel()` (popup), `fill`, `put`/`seg`
(text runs), `keycap`/`keycaps`, `hints`, `button`, `dim_all`. Icons come from
config (`[icons]`), with Nerd Font glyphs where present.

## Project-specific: layout and navigation

<!-- local: user requested sidebar-bottom controls, tab-row update discovery and no global footer. -->

Maitty has one terminal interface: a sidebar of projects and sessions, with tabs and split panes in the remaining area. Settings > Appearance > Layout selects one look (ADR 0015): **Floating** (boxes with gaps, the default), **Tiled** (boxes edge to edge), **Compact** (full-bleed zones with square edges; its pill ends still follow Round pill ends) or **Grid** (boxes with gaps, each pane in its own colour by its place in the tab: `Theme::grid_colors`, top-left first, repeating from the fifth; the focused pane's border full and bold, the others' faded 42% toward `bg`). A Grid pane's top border reads its number in its colour and, for an agent, `▲ needs you` (`blocked`, bold) or the spinner and `working` in its colour on the left (a click opens why; the top-left pane's tabs follow), and its name as a tag on the right before the ✕: on its colour with `ink_on` when focused, on the colour faded 28% with `strong` when not. A narrow pane drops the tag, then the state's words; the status mark Floating shows on the right is not repeated. All use the same sessions, tabs, splits and sheets, and `ui.rounded` (`full`, `corners` or `off`, ADR 0021) applies to every look: `full` and `corners` round the corners of every box, and `full` also gives every chip, button, row and hover half-circle ends. `ui.padding` (`0`, `1` by default, or `2`) leaves that many cells empty round the whole screen in every look (`draw.rs` `framed`). Older config values still load: `cards` (with `panes`) as Floating or Tiled, `flat` as Compact. In this doc **Cards** means Floating and Tiled together and **Flat** means Compact. Tab, Shift+Tab and Esc go to the program in the pane in both layouts; the leader's Focus the sidebar moves focus to the sidebar. There is no application-wide footer bar and no separate tab row: the tabs are chips in the top-left pane's title, followed by a **+** chip that opens a tab (see *v7 frame*). Keep the **new** and **menu** chips at the bottom of the sidebar in both layouts (see *sidebar check result*), and collapse or hide the sidebar as *collapsed sidebar* says. The top right of row 0 shows **Update** only when a newer release is available.

The leader is `Ctrl+Space` by default. Inbox (`i`), Changes (`d`), Files (`f`) and Branch (`b`) use the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The key overlay (`Ctrl+Space`; `Space` searches, `?` lists all), the branch switcher and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll. Branch (`b`, `client/screen/branch_sheet.rs`) uses the right-hand sheet: the session's branch, ahead/behind its upstream and its base branch (the main checkout's, else `main`, else `master`), the last 10 commits, the session's last check and its snapshots, all read by git in a background task. Its first row, **Switch branch…**, opens the switcher popup; Enter on a snapshot starts the row menu's restore (asked first). A folder outside git says so instead of the lists; an upstream whose remote branch is gone reads `gone from the remote`, and a git error shows in the place of what it was reading. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

The sidebar's **+ new** chip, a project row's **+**, the empty project row's new action and leader + `n` are one action (`Action::ShellHere`): a plain shell where you are, whose tab opens with focus. You type your agent in it (see *Agent launchers* below). Leader + `s` starts an agent session straight away with the defaults in Settings › Sessions (`Action::StartSession`, Menu › Session). **Menu** is in the sidebar footer. Every row of the key overlay and the Menu is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** at the top right of row 0 and open the existing confirmation flow when clicked. Do not show the control when Maitty is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. General groups controls under INPUT, LAYOUT, SHELL and UPDATES. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

Settings has six tabs: General, Sessions, Appearance, Agents, Check and Keys. There is no Team tab; `[team]` in `config.toml` still loads and is no longer shown. The label column is 33 columns and no setting label is longer; the value column starts after it, and a Choice whose chips would not fit beside the label shows the current value between `‹ ›` instead. The selected row's help (`Setting::help`) sits under a rule at the bottom of the page, word-wrapped over up to two lines, so a help text stays within two lines at 80 by 24.

**Settings › Sessions** opens on NEW SESSIONS (`[sessions]`: `Agent` last used, claude, codex or gemini, `Where` new worktree or this folder) and HELPERS (`Sessions may start helpers` ask, always or never; `Helpers at once` 2, 3 or 4), then the older alert, restart and history settings. Values outside those lists are refused when the config loads. `Agent` and `Where` are read by leader `s` and the Session with options dialog (an old `plus_starts` key is read and ignored); the helpers settings are read by the daemon when an agent starts a session (see *helpers*). `[mcp] helpers_at_once` is not a setting any more: it is read only as a fallback for `[sessions] helpers_at_once`. **Settings › Agents** opens on AGENT STATUS: Claude reads `✓ hooks installed` (read once in the background when the page first shows, `checking…` until then, `hooks not installed` otherwise; the same test as `maitty doctor`), Codex reads `✓ read from the process`, and `Set up another agent…` opens the Connect agents list (`Action::AgentStatusSetup`). SUBAGENTS has `Count on the session row` (`ui.subagent_count`, on by default): off hides the `· N agents` text on a working session row. The `mcp.*` and worktree settings follow. General's `Leader waits for next key` (`ui.which_key`, off by default) shows every key if you pause after the leader.

**Subagents have no tree.** The sidebar never lists an agent's subagents and has no inspector for them; the agent shows them in its own pane. A working row says `· N agents` only when the agent reports subagents (`ui.subagent_count`), and nothing else about them is drawn. The daemon still tracks them (`TermInfo::subagents`, `subagent_activity`) so the count is right and a parent stays working while its children run.

**Helpers are not a mode** (`client/screen/helpers_ask.rs`). Any session may start helpers (other agents it starts through `maitty mcp`); `[sessions] helpers` (ask, always, never) decides. When a session asks and the setting is `ask`, its row is **needs you** (amber, as for any question) and its second line reads `wants to start N helpers` (`wants N helpers` on a narrow sidebar). Enter on the row with the sidebar focused, Enter (or a digit) on its row in the Inbox, or a click on that line, opens a popover beside the row (from the Inbox, Esc or an answer goes back to the Inbox on that row): the screen dimmed except that row, a full accent border, a `◂` on the row, the title `START HELPERS? · <session>` and `Esc later` at the right. It says which agent wants to split the work into N helpers, lists the tasks (five, then `+ n more`), then "Their permission prompts come to your Inbox. Nothing merges." Buttons and keys: `1` Allow, `2` Always in <project> (`Always here` when narrow), `3` No, do it alone, each clickable; Esc decides later and the row keeps asking. Enter, `y` and Space do nothing there, and `1`/`2`/`3` act only inside the popover: no bare key answers from the sidebar or the collapsed strip (ADR 0014). The answer is a command the daemon accepts only from you and only while it holds the request (`docs/API_PATTERNS.md`).

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running until **Restart now**. For a local daemon, when no panes are open, restart it automatically (see below). For a remote daemon, explain that it must be updated on its host after its sessions are closed. The one `↑ Update` pill at the top right of row 0 (ADR 0014) becomes `↻ Restart` once a newer maitty is installed (here, or from outside maitty) and the server or this window still runs the old one; it clicks through to the same restart. The splash carries the matching line; there is no second notice in the working area.

**Restart now** is one step: it restarts the server onto the installed maitty **and** replaces this window with the installed one in the same terminal. It shows in Settings' restart-pending line (`R`), on the toast after an install (it stays 12 s), on the `↻ Restart` pill, and as the palette action *Restart maitty* (`Action::Restart`, `restart`); none show for a remote daemon or while a restart is under way. It asks only when a session is working or waiting for you (or when `restore.enabled` is off, since then sessions close for good): a confirm (`menu::Confirm`, danger) reading "4 sessions stop and resume, 2 are working, 1 waiting for you" with Restart (Enter) and Cancel. Otherwise it restarts at once. The server saves its session and every pane's output first (`KillServer`), then the window closes and the installed maitty opens in its place (`restart::relaunch`: exec on Unix, spawn-and-exit on Windows, same arguments and `MAITTY_SOCKET`), which starts the new server and restores the sessions. A window that is already current (only the server is old) reconnects in place; a current server with an old window swaps only the window and leaves every session running. An agent mid-turn is stopped when the server stops and resumed with its conversation when it comes back; the turn in flight is not replayed.

A new `maitty` that finds a server of another version replaces it on the way in. A server of an older release whose protocol this one speaks, and which is idle, is stopped at once (`restart::replace_older_idle_server`, from the activity file the server keeps beside its pid file: `restart::Activity`). A server whose protocol this one cannot speak (`ipc::OtherVersion`) is judged by that file, or for a server too old to write one by the agents in the saved session (`restart::plan_for_mismatch`): when nothing is working or waiting, the terminal says `Updating maitty…`, the server is stopped by its process as `maitty kill-server` does across versions, and the window opens with the sessions restored, with no prompt. When something is working or waiting, or the server is newer than this maitty (never replaced unasked), the full-screen prompt (`client/restart_prompt.rs`) shows the protocols, how many sessions resume and how many are mid-turn, and **Restart and open** (Enter) and **Quit** (Esc, Ctrl+C), each clickable; Quit prints the usual error and exits 1. Without a terminal on stdin and stdout, or with `--remote`, there is no automatic step or prompt: the error text and exit code are unchanged, and a remote server is never restarted this way. A same-protocol older server that is busy is left running; the `↻ Restart` pill offers the restart.

**One key overlay** replaces the command palette, the long shortcuts reference and the old leader overview (`client/screen/keyoverlay.rs`). It has three faces in one card with one typing row:

- **Key map** (`Ctrl+Space`, or the pause after it). A banner, "Every key goes after the leader: `Ctrl+Space` then `v` = split", then Sessions, Panes, Tabs, Project and Maitty, each key a keycap followed by its name, from `key_groups()` and the real bindings. A pressed key runs its command; `Tab`, or an unbound letter, switches to the search. Three columns when the card is 100 or more wide, else two; each group's key column is as wide as its widest key. A key marked `›` opens a second step (Worktrees). Action bar: `key` run it, `Tab` search, `?` show all, `Esc` close.
- **Search** (`Space` after the leader, or the Menu's Search commands). Every command, keyed or not: the name with the match in the accent, its group in dim, its key at the right. With nothing typed it lists RECENT (the last commands run from the overlay; a few defaults before there are any). Enter runs, `Tab` returns to the map. A set of keys (the arrows) is searched one key at a time. Every command the old palette offered stays reachable here.
- **All keys** (`F1`, Menu → All keys). The map in full, then "WITHOUT THE LEADER · press directly" (Alt+1–9, Alt+arrows when on) and "SEARCH ONLY" (commands with no key), both headings dim. It scrolls when it is taller than the screen; a letter typed there starts a search.

Rules for keys and their labels: one key per action (no alias keys; a user's own `[keys.prefix]` entries still load), no default needing Shift, keys other apps use (ADR 0017), sentence case, no bracketed explanations (the group gives the context). Every action has a leader key or is search only; nothing in the sidebar acts on a bare letter. `docs/shortcuts.md` documents the other screens' keys and is not embedded in the app.

**Cursor.** The focused pane shows the cursor its program asked for (DECSCUSR, read from its output); where it asks for none, or for the default, Settings › Appearance › Cursor (`ui.cursor`: `terminal`, the terminal's own, by default; `block`, `underline` or `bar`) and Cursor blink (`ui.cursor_blink`) decide (`client/mod.rs` `cursor_code`). An unknown `ui.cursor` is refused when the config loads.

**Keycaps.** Every key the UI shows is a keycap from `design::keycap`: ` key ` on `btn`, `strong` and bold, with the ends round when `ui.rounded` is `full`. A combo splits on `+` into one cap per part joined by a dim `+`. The map, the search, hint rows, Settings key rows, dialogs and the copy hint all use it.

**Action bars.** A panel's key hints sit in their own one-row bar (`action_bar`, `panel_bar` in `popups.rs`) on a ground one step above the panel's: `card` for a sheet on the app background (Inbox, check evidence) and `card2` inside a modal (Settings, the key overlay, confirms, pickers). It spans the panel's width with one column of padding, shows keycaps and labels, and is always the panel's last row. When the hints do not fit they close their gaps, then give up the ones before `Esc`.

**Menu.** Four groups a blank row apart, no headers, labels capitalised, no letter lit: Shell here, Session, Session with options…, Pane beside this, Tab; Inbox, Search commands, All keys; Collapse sidebar, Hide sidebar; Settings. Each row shows at the right the leader key that does the same, and that key runs the row inside the Menu too. Footer: "`Ctrl+Space` then a key works anywhere". "Shell here" is leader `n` (a plain shell where you are) and "Session" an instant agent session (leader `s`).

**Agent launchers.** Typing an agent (`claude`, `codex`, `gemini`, `opencode`, a quick agent) in a Maitty shell in a git repo's main checkout starts it in a new worktree (`worktree.per_agent`); anywhere else, or in a folder that is not a git repo, it starts where you are, silently. It works from any shell because `<data>/shell/bin/` is first on the pane's `PATH`; see ARCHITECTURE › Agent launchers. No toast or warning is shown for main folders.

**New sessions** (`client/screen/newsession.rs`). Leader `s` starts a session with no form: agent `sessions.agent` (`last used` is the last agent started from Maitty, kept as `last_agent` in `maitty-ui.json`, else claude), project the one under the sidebar cursor or the one you are in, `sessions.where` (`new worktree`, which falls back to the folder in a project that is not a git repo, or `this folder`). The task is typed in the session, whose row reads `<agent> · new · type to begin` with the accent `○`. `c` (Menu › Session with options…) opens the dialog instead: a small NEW SESSION dialog with Agent, Where and Project rows (↑↓ or Tab choose a row, ←→ pick an option, the project as `‹ maitty ›   1 of 3`), one summary line ("Opens claude in a new worktree of maitty. You type the task there.") and Start (Enter) / Cancel (Esc). It has no goal or task field.

## Project-specific: pane focus

Only the area with the keyboard is lit: the focused pane's whole border (Cards), its title chip and left `│` rule (Flat), or the open sheet. Unfocused panes fade by the Dim setting (30% by default; "40%" in an older config counts as 30%). Do not reserve a full-width bottom row for global hints. The pane has no status line: its branch is in the sidebar's folder header.

## Project-specific: UI conventions

<!-- local: user requested distinct active, keyboard and hover sidebar states, mouse-first sidebar controls and parent-owned child-agent visibility. -->

- Popups are centred `panel()`s (`panel_at()` for a given rectangle) with an
  accent title row and `Esc close`, `│` sides and a `─` bottom in `line`, and no
  shadow; the screen behind is dimmed. Small explanations (why, a row's menu) are
  `popover_at()`s: a raised title row, no border, no shadow, beside the sidebar
  (`popover_rect()`). Settings is a
  centred modal that preserves the workspace layout and keeps its General,
  Sessions, Appearance, Agents and Keys sections, update controls, and keyboard
  and mouse editing available.
- The pointer's tint is one pass after drawing (`render.rs`): the topmost
  clickable region under it is tinted once, `mix(bg, text, 0.10)`, its ends round like every other shape when `ui.rounded` is `full` (`card.rs` `round_tinted`: an end cell holding only ground, or already a rounded end, becomes a cap or soft corner; an end with text stays square). Widgets do
  not tint themselves on hover.
- Selection is a neutral tint (`sel_on` of the surface it sits on) with a lime `▎` bar;
  never the accent colour as a row fill.
- Every popup row is clickable and keyboard-reachable; every key is shown as a
  keycap somewhere (hint bars, the key overlay, Settings).
- Short-lived notes are toasts above the pane content; destructive actions ask
  first (`Confirm`) and go last in menus.
- Anything filled that you click or read as a control (a button, a toggle, a tab, a tag,
  a key) is drawn with `button` / `btn`, `chip` or `pill`, so its ends follow the pill
  setting everywhere. Never a bare `seg` on a button colour.
- Copy is plain words ("Focus the sidebar", not "browse-tree").

## Project-specific: theme browser

Appearance has one Theme control. Enter or clicking its value opens a searchable
browser with palette swatches, an applied marker, a result count, and a preview.
Typing filters by name or flavor; arrows, Home/End, page keys, and the mouse wheel
move the selection. Click a row to preview; click it again or press Enter to
save. Esc or the close button returns to Appearance without changing the theme.
Saving runs in a background task; failure keeps the selection and shows a toast.
The preview shows built-in defaults; configured overrides still apply after saving.
See [theme sources](theme-credits.md) for upstream palettes and local UI adaptations.

## Project-specific: working spinner picker

Appearance's Working spinner opens a mouse/keyboard picker with animated preset
previews and an applied marker: Braille, Dots, Line (ASCII), Snake, Pulse (each frame held
twice, so it breathes at half pace), Fill, Orbit, Bounce, Pair, Arc, Moon, Star (the last
seven from ratatui-spinner's flux frames), Blocks and Bars. There is no custom or static
row; frames set by hand in `ui.spinner` still load and work. Preview leaves the active config
unchanged. Enter or a repeated row click applies; Esc/close returns to the same settings row.
Save in a background task; disable duplicate saves and keep the applied choice on failure.
Choices affect only the working glyph; blocked/done/idle status colors and icons keep their
meaning.

The **Bars** preset is the v7 handoff's 3-bar Braille spinner. A frame is one column, or
two when every character is Braille; a two-column mark moves the sidebar row's name one
column right. The ASCII glyph set draws Bars as `. o O o`; other choices are never
rewritten. Working also shimmers its sidebar name in the working colour. With
motion off (`app.motion_on()`) the spinner rests on its third frame and the shimmer stands
still, so nothing depends on motion.

## Project-specific: sidebar check result
<!-- local: the user chose on 2026-10-23 to drop the `idle` line on a plain idle row at every width (v7 drew it from 140 columns) -->
<!-- local: the user chose on 2026-10-24 one-line session rows with the agent's mark and uncommitted lines, names in their state's colour (ADR 0021) -->

The sidebar is the v7 handoff's `drawSide` (`docs/design/v7/checks7.js`); its rows are
tested cell for cell against the reference frames (`client/screen/side_frames.txt`), read
through `v7_fixture::as_one_line_rows`, which applies ADR 0021's one-line rows.

- **Title:** `SESSIONS` bold and the count, then right-aligned `●n` (`blocked`, bold:
  asking or waiting for check approval) and `✕n` (`err`, bold: failed checks), each only
  above zero.
- **Folder:** a one-cell `−` (open) or `+` (closed) button in the state bars' column, bold, in a colour picked at random each time the folder opens (no ground of its own), name (`strong`, bold, never cut before the meta),
  meta right-aligned in `muted`: `⎇ main` plus ` · check: <name>` or ` · N checkpoints`,
  or `no git`. The meta shows only with 6 or more columns free, cut with `…`. A blank row
  follows each folder.
- **Row line 1:** the session's mark (`client/screen/glyphs.rs` `Kind`; bold for failed,
  needs you and approve; a new session's `○` is `accent`, bold), the name, then
  right-aligned the agent's mark (`muted`: `✻` Claude, `◆` Codex, `✦` Gemini, `◇` OpenCode,
  else its first letter; `C`, `X`, `G`, `O` with the ASCII glyph set), the checkout's
  uncommitted lines (`+N` in `done`, `−N` in `err`; shown only when there are any) and the
  age (`muted`), two spaces apart (`side.rs` `row_meta`). The lines give way first, then the
  agent's mark, so the name keeps 8 columns. The `+N −N` is its own click target
  (`ScreenHit::RowChanges`): it opens that checkout's Changes in the side panel, as `d` on
  the session does, and the hover tint covers just it; the rest of the row opens the
  session. A right-click or a drag from it acts on the row, as anywhere else on it. Hover
  never moves it: the row's `⋯` takes the age's place, and a row with no age keeps two
  columns free for it. A name in a state worth a look (working,
  checking, done and not seen, needs you) is drawn in its state's colour, the glyph's, and
  bold; with motion on it shimmers up toward `strong` (working, checking, not seen) or pulses
  (needs you) (`side.rs` `NameMotion`). Another row that wants a look (failed, new, a check
  result other than a plain pass) has a `strong` bold name; a done, seen or asleep row a
  `text` name, not bold, `strong` only while selected. Only the selected row carries a `▎`
  edge bar, in `accent`, on each of its lines; with `ui.rounded = "full"` a one-line
  selected row has half-circle ends instead.
- **Uncommitted lines** are counted off the UI thread every 5 s for each checkout with a
  session (`tasks::uncommitted_lines`: `git diff --numstat HEAD` plus untracked files' lines),
  kept in `Screen::lines_changed`. A checkout git cannot read shows none.
- **Row lines 2 and 3** are `muted` and never bold (`side.rs` `quiet`), whatever they say.
- **Row line 2** shows only when it says more than line 1 (`side.rs` `has_detail`): the
  headline check's result (`TermInfo::check()`) by checkpoint name, the question, helpers it
  asks for, a plan-limit wait, a new agent's first step, `asleep zzZ`, a message just sent,
  or for a working agent its helpers (`2 agents`, `ui.subagent_count`) and a quiet spell
  (`quiet 12m`, `TermInfo::quiet_since`). It never names the state alone (`working`,
  `needs you`, `done · not seen`, `idle`): the mark and the name's colour say it. While the
  agent works or asks, its last result is about an older turn and is not shown. An asleep
  row keeps `asleep zzZ` from 140 columns.
- **Below 140 columns** line 3 goes, and an asleep row is one line too.
- **Row line 3** (140 columns and up): a failed check's first failing line (`muted`), or
  every checkpoint by name: `build ✓ · test ✕` (a failed name in `err`).

- **Foot:** one row off the bottom (Cards: the last row inside the border), a `line` divider above it, no band behind it. Two `btn` chips: `+ New` a column in (the `+` in `accent` bold) and `Menu` at the right, the words in `text`, no letter lit (`side.rs` `word_chip`). `Menu` drops first when the sidebar is too narrow for both. Chips are square on the plain `bg`, since a cap there has no ground of its own; `ui.rounded` rounds them where the ground is raised.
- **Amber is for needs you.** `blocked` colours only what needs you (its mark, question, counts and approvals), never a heading: Inbox "HEADS UP", the both-diffs sheet and the tidy list use `sky()`.

Settings › Appearance › Status glyphs picks how a session is marked. **Dots** (the default, and what `auto` gives unless your status icons are plain ASCII) draws a coloured `●` for every state that wants you or says something (needs you, done, failed, stale, overridden, not checked) and `○` at rest (idle, asleep); the colour and the word beside the mark carry the meaning. Working keeps your spinner and checking its ring. A check's own `✓` and `✕` beside its name (`build ✓ · test ✕`) keep their shapes. **Unicode**, **ASCII** and **Nerd** are the handoff's sets, where every state has a glyph of its own, as the table shows.

The colour slot is the glyph's; line 2 is always `muted` (see above).

| State | Glyph (Unicode / ASCII) | Colour slot | Line 2 |
| --- | --- | --- | --- |
| working | the spinner you chose (Bars is 3-bar), bold | `accent` | none; `4 agents` only when the agent reports helpers, `quiet 12m` when it has gone quiet |
| needs you | `●` / `!` | `blocked` | `asks: <question>`, else none |
| done, not seen / seen | `✓` / `+`; `○` / `o` | `done`; `muted` | none: the row is one line |
| new (an agent told nothing yet) | `○` / `o`, bold | `accent` | `claude · new · type to begin` |
| asleep | `z` | `muted` | `asleep zzZ` |
| checking | `◜◝◞◟` / `- \ \| /` | `sky()` | `checking test · 6s` |
| passed | `✓` / `+` | `done` | `passed: test · 14.2s · 14:22` (local clock) |
| failed | `✕` / `x` | `err` | `failed: test · exit 1` |
| overridden | `⊘` / `/` | `text`, never green | `failed, overridden: test` |
| could not run | `?` / `?` | `violet()`, not a failure | `test could not run · not found` |
| stale | `◷` / `~` | `text`, never green | `stale pass: test · 3 files changed` |
| never for this repo | `?` / `?` | `violet()` | `not checked: never for this repo` |
| waits for approval | `●` / `!` | `blocked` | `approve check: <command>` |

- **Several checkpoints:** line 2 is the headline (worst) checkpoint's word, then one bold
  glyph per checkpoint (pass `✓` `done`, fail `✕` `err`, ring `sky()`, could not run or
  never `?` `violet()`, stale `◷` `muted`, overridden `⊘` `text`, approval `●` `blocked`,
  pending `…` `muted`). Words: `failed: <name>` (`err`), `could not run: <name>`,
  `needs you: <name>` (`blocked`), `checking: <name>` (`sky()`), `next: <name>` (`muted`),
  `stale pass: <name>`, `not checked: <name>`, `overridden: <name>`, `all passed` (`done`).
  The glyphs are clipped, never the word.
- **What gives way:** the folder meta first (by its room), then the age (below 100
  columns), then line 3 (below 140), then line 2 of rows with no check result, question
  or plan-limit wait. Queued follow-ups keep their lines after these.
- **Selection:** the keyboard's row (while the sidebar has the keys) or the open session
  is tinted (`Theme::sel_bg()`) from X to one past RX on all its lines, with `▎` at X:
  `accent` while the sidebar has the keys, else `muted`; while the keyboard browses, the
  open row keeps a `muted` bar. A failed row carries `▎` in `err` on every line, selected
  or not. X is 2 columns in (1 in Flat), RX 3 from the right (2 in Flat).
- **Foot:** `+ new` at the left and `menu` (its `m` lit) right-aligned, on the last row inside.
- Shape and word carry the meaning; colour only backs them, so every state reads in the
  ASCII set and without colour. Glyphs come from `[ui] glyphs` (`unicode`, `ascii`,
  `nerd`, or `auto`: ASCII when the `blocked`, `done` and `idle` icons are all ASCII);
  a Nerd icon is clipped to one cell. Take marks from `check::marks`, never a literal.
- Colours are existing theme slots only; there is no check role. `Theme::violet()` and
  `sky()` are derived, not roles. Code: `client/screen/side.rs` (the card),
  `client/screen/check.rs` (a result's words and marks).

## Project-specific: check evidence sheet

The check evidence sheet follows v7 (`drawEvidence` in [`docs/design/v7/checks7.js`](design/v7/checks7.js)). Enter on a sidebar row whose check failed, could not run, was overridden, went stale or is never run for its repo opens it (a row waiting for approval opens the approval; every other row still focuses its session), as do Output in the Inbox and `⏎` in the why popover. It docks right like the Inbox and takes the pane column below `NARROW`. Its title is `CHECK · <session> › <checkpoint>`; content starts one column into the card's inside, and in Flat the sheet keeps the Cards box (the Inbox too): only the frame changes.

- A result chip (` <glyph> <word> ` bold on the state's colour: `failed` on `err`, `failed · overridden` on `text`, `could not run` and `not checked` on `violet()`, `passed` on `done`, `stale pass` on `btn`), then what it came to in `strong` bold (`exit 1`, `exit 0 on older code`, `never for this repo`; `command not found` in `violet()` for exit 127).
- Command, Folder (the run's folder, home as `~`), Ran (`14:22:05 · took 14.2s · after turn 3 ended 14:21:51`, local clock; `did not run` when it never started) and Seen by (`Maitty ran this itself. The agent said "…".`), or Source (`.maitty/checks.toml · <commit> · you chose never`) for a repo's own command. Rows are clipped at the sheet's edge, not wrapped.
- A banner on a tint for a result that needs explaining: overridden (by whom, when, `u undo`, the reason), stale (how many files changed and when, then one `M path` row per file that fits), never, could not run.
- `OUTPUT`, then `all N lines`, `last N of 1,284 lines` (numbered as the whole output, from `CheckInfo.line_count`; `f full output` when the daemon kept only the tail) or `empty`, on a darker block with 5-column line numbers; `✓` lines `done`, failure lines `err`, `▲` when there is more above. Empty output says why: `(no output)`, `(the command printed nothing)` / `exit 0 is what passed it.`, or `(nothing ran)`.
- Buttons on the third row from the bottom, each a click of its key: `Check settings ,` for never, else `Re-run r` (primary when stale, absent while it runs), `Override… O` (failed only), `Undo override u` (overridden only), and `Open session o` when it fits. The last row is always `Esc back   r re-run   f full output   i why`.
- Keys: Esc closes; `r`, `O`, `u` act as the buttons; `f` and the arrows open all of the output and scroll it; `o` goes to the session; `i` opens the why popover; `,` opens `PROJECT · <name>` for the session's project. With several checkpoints it opens on the headline one, and Tab / Shift+Tab move to the next / previous; the title names the one shown, its output is fetched by name, and `r`, `O` and `u` act on it.

The row menu of a session with a check adds Check output (any result), Re-run check (not while it runs), Override check… (failed only) and Undo override (overridden only); letters come from `menu_keys` like every item. Output comes from `Query::CheckOutput` and is fetched again when a new run replaces the result. Command output is not trusted: colour codes and control characters are stripped before drawing.

## Project-specific: why popover

The evidence popover (`i`, or a click on a status) explains a session's status from its hook, screen and process reports. For a session with a check it follows v7 (`drawWhy`): the title ` WHY · <session> reads <word> ` (the session's word: `failed`, `done · stale`, `not checked`, …), then one line each with its local clock time: `agent's turn ended · it said "<said>"`, `Maitty ran <command> in its worktree`, and what came of it in the state's colour, bold: `✕ exit 1 after 14.2s` for a pass or failure, the row's words for the rest (`⊘ failed, overridden: test`, `◷ stale pass: test · 3 files changed`, `? not checked: never for this repo`, `● approve check: npm test`). A line whose time is unknown leaves the time blank. Its hints list only the keys that act, in v7's style: `⏎ evidence` always, `r re-run` once there is a result, `O override` for a failed one, `Esc close`. A key the popover does not show closes it and is not passed to the pane. A session without a check keeps the source rows; a quiet working one opens with `No output or report for 12m — it may be stuck.` The frame and placement are the v7 frame's.

## Project-specific: check modals and settings

Deciding is a modal; nothing here pops up on its own. Each matches the v7 handoff (`checks7.js` `drawModal`) inside the modal frame, cell for cell, in every layout at 160×45, 100×30 and 80×24 (`client/screen/check_modals_tests.rs`). Colours are existing slots: `blocked` for "needs you" amber, `done`, `err`, `violet()`, `sky()`, `card2` for fields.

**Override** (`client/screen/check_actions.rs`): `OVERRIDE A FAILED CHECK`, 70 wide and 15 tall. It says what the session will read (`done (overridden)`) though which checkpoint failed and when, what it hides and where the failure stays visible (⊘ on the row, in the Inbox history and in the evidence, for this result only), and takes an optional reason (at most `OVERRIDE_REASON_MAX_CHARS`; typing, paste, Backspace and Ctrl+U edit it). Enter overrides; `O` overrides only while the reason is empty, so a typed reason can hold an O; Esc or Cancel does not. Opened from the Inbox, the Inbox stays under it and Esc or Cancel goes back to it, on the same item. The sheet, Inbox, popover and row menu all open this one modal. After it the evidence sheet shows that checkpoint and a toast says `Overridden. <session> reads done (overridden); the failure stays in its evidence.` A re-run says `Re-running <cmd> in <session>'s worktree…` in a toast, or `re-run queued` while the agent works; a daemon error (`ServerMsg::Error`) is a toast. While a sheet takes the pane column, toasts show over the sheet.

**Approving a repo's command** (`client/screen/check_approve.rs`): opens only from its waiting item, by Enter on the Inbox item or the sidebar row of a session whose checkpoint needs approval. Several waiting: one modal per checkpoint, the next opening after a choice. It shows the command (`$ npm test`, or `- old` / `+ new` when the repo changed one you had said Always to) in full: wrapped over as many `card2` rows as it needs, the modal growing to fit; `\n`, `\r`, `\t` and other control or invisible characters spelled out (`\u{001b}`), and a run of spaces shown as `[N spaces]`. Where it runs, the file and commit it came from (`added in a1b2c3d, 3 days ago`, `changed in <sha>, 2h ago`, or `changed, not committed`), when it runs, and "Read it. It can do anything you can in that folder." `1` and `2` answer only once the last frame showed the whole command, and only for that command: one too long for the window ends its last row in `…`, says `The whole command doesn't fit. Make the window bigger and read it first.`, and 1 and 2 only say so; a command that changes disarms `2` (`The command changed. Read it again before you choose.`). There is no default: `1` runs once, `2` arms Always (a strip spells out what it means and the button reads `2 again`) and a second `2` sends it, `3` is Never and opens the evidence sheet, Esc leaves the item waiting (back in the Inbox, on the same item, when opened there). Enter, `y` and Space send nothing and say so; other keys and pastes do nothing. Clicking Always once arms it like `2`. Below 80 columns the buttons use short labels; below 30 rows the modal loses the blank line after the command.

**Settings › Check** (`client/screen/check_settings.rs`) is one renderer with two hosts: the `PROJECT · <name>` modal (`,` from the evidence sheet or an Inbox row, for that session's project) and the Check page of Settings (the workspace you are in). Empty, it explains what a check is and takes a first command. Set, it shows the selected checkpoint's command, where it runs, its timeout (`‹ ›`), its latest result by clock time (`✓ passed on fix-footer 14:22 · 14.2s`, `… never run`, `? could not run … pnpm: not found (exit 127)` with what to do) and its source, then `CHECKPOINTS`: `1 of 1`, or `N of M · ‹ › switch` for several (←/→ on that row, or a click). Your checkpoints are `[[check]]` tables in config.toml; Ctrl+S writes the shown one by name in place, keeping comments, off the UI thread (a failed save keeps the draft and toasts; `Saved. Runs after the next turn ends.` for a first command, else `Saved.`). Del removes its table. A repo's checkpoint is read-only here: its source reads `.maitty/checks.toml · <commit> · always | never | asks each time` and Del (`Forget choice`) sends `ForgetCheckApproval`. Further checkpoints are added in config.toml or the repo's file; there is no add screen. Run now (`r`, or its button) re-runs that checkpoint in the session with the latest result, else the focused or first agent of the project; a working agent queues it. Esc drops the draft.

## Project-specific: Inbox check groups

The Inbox (`j`) follows v7 (`inboxItems`, `drawInbox`). Its title is `INBOX <n>`, n counting
everything not just finished. It lists, in this order: NEEDS YOU, FAILED CHECKS, COULD NOT RUN,
FINISHED, then HEADS UP, each a chip head on its colour (`blocked`, `err`, `violet()`, `done`)
with its count. A question is never below a check.

- A group with no rows is not drawn; with nothing at all it says "Nothing is waiting on you.".
  A project with no check has the groups it always had, FINISHED renamed from JUST FINISHED.
- NEEDS YOU holds questions, then repo check commands waiting for approval
  (`approve check command: npm test`), then quiet working agents (`no output for 12m · may be
  stuck`, no sound; Enter opens the session, Delete hides it until it next goes quiet).
- FINISHED lists sessions with a check result first, then the newest first.
- A Done session is finished (`check::finished`) when it has no check, or its check passed, was
  overridden, passed on older code (stale) or you chose never to run it. It is not finished while
  checking, failed, could not run or waiting for approval. The home screen counts finished by the
  same rule. A session whose agent is working or asking is not listed for its old result (see
  `check::shown`).
- A failed check stays listed until a later run replaces it, even after you have looked at the
  session. A could-not-run check is listed only until you look (`Del` marks it seen).
- An item is the session glyph (`glyphs::Kind`), name (`strong` bold), ` · <folder>` and an age;
  at +2 the question or the result (`failed: test · exit 1 · 14:22`, `test could not run: pnpm
  not found`, `passed: test · 14.2s`, `failed, overridden: test`, `stale pass: test · 3 files
  changed since`, `not checked: never for this repo`, `finished · not seen`); a failure adds its
  first failing output line. Several checkpoints read as one word first, then
  `build ✓ · test ✕`, or the glyph strip when that doesn't fit.
- The selected item is tinted across the sheet with `▎` (`err` for a failure, else `accent`); a
  failure keeps its red `▎` unselected. It grows its buttons, each a click of its key: question
  `Reply m` `Open o`; approval `Read the command ⏎`; failed `Output ⏎` `Re-run r` `Override… O`;
  could not run `Output ⏎` `Settings ,`; stale `Re-run r` `Output ⏎`. Enter runs the first
  button (Read the command opens the approval); an item with no buttons opens its session. `o` opens the session of any item; `r`, `O`, `u`, `,` act on an
  item's check where they apply and are typed otherwise. 1–9 answer a question; d, M, x and m
  act on a finished item; d, m and k on a heads-up; Delete marks seen.
- Typing finds any session; the `› query` row shows only once you type. The last row is
  `↑↓ select   ⏎ open   r re-run   O override   Esc close`.
- Existing theme slots only. Code lives in `client/screen/inbox.rs` (groups, drawing) and
  `client/screen/check_inbox.rs` (what each item says and its buttons).

## Project-specific: v7 frame

The v7 layout and checkpoints follow [`docs/design/v7/`](design/v7/README.md) (`checks7.js` is the spec). The frame is [ADR 0013](adr/0013-v7-frame.md); `client/screen/v7_frame_tests.rs` holds it to the reference frames, and `client/v7_frames_tests.rs` holds every preset's whole screen to them (see [TESTING](TESTING.md)).

- **Grid** (`draw.rs` `grid`): inside `ui.padding` (`framed`), no margin of its own. The sidebar is 40 columns from 200, 34 from 140, else 26 (a width you dragged wins), the full height. Floating leaves `ui.gap` between cards, tiled none (borders side by side, never merged), Flat starts the panes on the sidebar's last column.
- **Cards** (`card.rs` `card`): `┌┐└┘`, or `╭╮╰╯` when `ui.rounded` is `full` or `corners`, `line`, the focused card's whole border in the accent. The title chip sits on the top border two in; the right-hand segments end a padding space before the corner. **Flat**: no box; row 0 is a header strip of `card`, the chip one in, the right-hand segments a column from the edge; a zone not at the screen's edge starts with a `│` rule. Flat has no card edges and no box, so rounded corners reach only the bottom corners of popups there (`popups.rs`); it never draws a box around a Flat zone. `ui.rounded = "full"` rounds Flat's pill ends (buttons, selected rows in modals and popovers, the answer bar, the toast) as it does in Cards; each cap sits on the ground of the zone around it (`card` in a modal or popover, the pane's `bg` in its body), not on the theme's `bg` throughout. The same `full` caps the tab chips (number and name as one shape, and the **+** chip) and the sidebar's selected folder row, in both layouts. The chips' caps sit on the header strip's `card` in Flat and on the top border's own ground (no background) in Cards; each chip is two cells wider and its hit rect includes them. A session that keeps a second line is a block with soft quarter-cell corners, since a half-circle on every line would scallop; a one-line row gets half-circles. With `round_tabs` off the chips are blocks, as before.
- **Pane title** (`draw.rs` `draw_session`): the top-left pane of the tab carries the session's tabs (a number chip and a name chip each, the current one in the accent; those that don't fit drop from the right, the **+** chip first of all when there is no room for it; with `ui.rounded = "full"` each chip also has a cap cell at each end); other panes show their session's name. On the right, the session's mark (glyph and word; it opens why) and a ✕ that closes the pane. In shortcut mode (ADR 0016, ADR 0021) the ` SHORTCUT · Esc ` pill on `sky()` sits at the top right of row 0, a column in, and the focused pane's border is `sky()` in every look (Grid too); otherwise **Update** sits there when a release is waiting. Input mode shows no mode label.
- **Sheet** (`sheet.rs` `split_for_sheet`): docked right from 140 columns, the layout's gap from the panes; below 140, or when the panes would keep fewer than 52 columns (`PANES_KEEP`), it takes the pane column and Esc brings the panes back. Its width is per kind of view and saved (`Saved::sheet_w`): Inbox, checks and both-diffs start at 72 columns, Changes at half the screen, Files at 55%, Branch at 30% (all less when the panes would keep fewer than 52, so about 70 at 160×45); never under 32 (`SHEET_MIN`). A click into a pane gives that pane the keys and leaves the view open beside it, unlit (`view_set_aside`); a click in the sheet, or asking for a view again (`f`, `d`, `b`), gives the view the keys back. Drag its left edge (it lights up while pointed at), or press `[` `]` (4 columns) and `=` (default) while it has the keys; in the Inbox only while its search is empty, in Files not while its filter or editor takes text. **Files** (`f`, Ctrl+click on `path:line`) shows the tree beside the selected file; a sheet under 60 columns (`FILES_BOTH_FROM`) shows the tree, or the opened file with a `‹` back to the tree. A `.md` or `.markdown` file previews drawn (`markdown.rs`: headings bold `strong`, a `line` rule under the first level, fenced code on `card`, inline code on `card2`, lists, quotes behind a muted `│`, tables cut to the width, links with their URL muted after them), without line numbers since drawn lines are not lines of the file; `m` (shown in the action row as Raw or Pretty) switches that file to its text and back, `i` and `e` always edit the text, and Ctrl+click on `file.md:42` opens it as text at that line. `f` again or Esc in the tree closes it.
- **Differences kept on purpose.** The app draws its own data where the handoff has samples, and these on purpose: Settings › Check marks the row the keys are on by its label in bold `accent` (the handoff draws every label dim and has no row cursor); evidence output colours only `✓` and failure lines (the handoff also dims `>` command echoes, `❯ file:line` and `Duration`, and colours an assertion's continuation as a failure); the why popover's last line leaves out the handoff's `· 1 of 59 tests failed` (output is not parsed for counts) and the evidence Source row leaves out when you chose never (`you chose never 14:31`; the time of a choice is not shown); and the several-checkpoints words add `could not run: <name>` and `checking: <name>` to the handoff's four (`failed`, `needs you`, `next`, `all passed`), because a checkpoint that could not run is not a failure and checking must read apart from next. A re-run keeps the sheet or Inbox it was started from open (the handoff closes it), and its result is told by the daemon's notices (`<place>: check failed`) and the row, not by the handoff's `test passed (exit 0, 13.9s). fix-footer reads done.` / `test failed (exit 1). fix-footer is in the Inbox.` toasts.
- **Message popover** (`popover.rs` `draw_compose_pop`): the follow-up to an agent, opened from the sidebar row or the leader (`M`). The screen is dimmed except the popover and its row. A full `accent` border in every layout (corners by `ui.rounded`), `card` ground, the input on `bg`, no shadow. Width `min(68, room right of the sidebar - 1)`. Title chip `MESSAGE <session> · <agent>`, with `● needs you` (the only amber), `working` or `done` at the right. Quote: the tail of the agent's last message, at most 4 / 3 / 2 lines at 160x45 / 100x30 / 80x24, labelled `<agent> asked` (blocked) or `<agent> said` plus `· last n of m lines` when cut, `muted` `▏` gutter, `text` words, not italic. Input: one row, growing a row per line to 6 / 4 / 3, then the top row reads `↑ n more lines`. Placement: top border a line above the row with `◂` on the row's first line; no room below, it opens upward with its bottom border just under the row; neither fits, pinned to the bottom. Hints are keycaps. Sending closes it and the row's second line carries the receipt (`receipts` on `Screen`): `◷ queued "…"` in `muted` until the server hands the follow-up to the agent, then `› sent "…"` in `accent`; it clears after `RECEIPT_MAX_SHOWN`, or on a status change once `RECEIPT_MIN_SHOWN` has passed. No toast.
- **Toast** (`draw_toast`): one component for every Maitty notice, bottom right, its bottom edge `TOAST_ABOVE_INPUT` (5) rows above the screen's so it sits over the agent's input area. A `card` with no frame and a full-height `▌` bar in the notice's colour (error `err`, about a session that needs you `blocked`, about a finished one `done`, otherwise `accent`; amber only for needs-you). Row 1 is the mark and the bold `strong` title with a `✕` at the right; row 2 the detail in `text`; an optional row 3 a preview in `muted` italics. `App::notify(msg, error)` makes a one-row toast whose title is the message; `App::notify_card(title, detail, preview, error)` adds the detail and preview rows. A plain toast clears after `TOAST_CLEAR` (2.4 s); errors (4.5 s), a note about a session (6 s) and the update toast with its **Restart now** button (12 s) stay longer. The `✕` or Esc clears it at once (Esc still reaches the pane). With motion on it slides in and fades out.
- **Selecting and copying**: a mouse drag or `v`/`V` in copy mode tints the selection sky (`blend(bg, sky, 0.38)`) with `strong` text, and, in keyboard copy mode, draws `y copy · Esc cancel` as keycaps (`design::keycap`) just after the selection's end; a mouse drag shows none, since releasing the button copies. `y` copies and closes copy mode; the copied cells then glow accent (`blend(bg, accent, 0.34)`) for `copy::FLASH_FOR` while the toast reads `Copied to clipboard`, `<n> characters · <m> lines from <session>` and a quoted preview of the first line.

## Project-specific: home screen check counters

The home screen's "while you were away" line counts a session as finished by the Inbox's rule
(`check::finished`: see Inbox check groups; a stale pass or a check never run here counts as
finished), and a check waiting for approval as needing you.
A `✕ N check failed` (`N checks failed`) count follows, in `err`, only when N is above zero.
When the line does not fit the width, the lead-in "while you were away" is dropped so no count
is cut; below 100 columns the lead-in goes too. A project with no check draws the line it always did.
Code lives in `client/screen/splash.rs`.

## Project-specific: splash

The design is `docs/design/splash/` (`splash.js` is the spec). Centred, no bottom bar, in the
theme's colours (`Theme::splash()`: Night has its own wordmark, A-and-I, swell and deep colours;
other themes take them from their strongest text and accent). Top to bottom: the wordmark in 5-row
block letters as filled cell backgrounds (never `█`, which leaves seams), two cells per pixel from
110 columns, the first "AI" in the name in the second colour; a blank row; the swell, one rolling
`▁▂▃▄▅▄▃▂` line 4 cells wider than the wordmark on each side, fading into the ground over 6 cells
at each end; a blank row; the tagline from `Cargo.toml` in italic; the "while you were away" counts
when anything happened; the choice cards, 56 columns or the width less 8; the keycap hints; the
version faint at the bottom right, or in its place an available update or a pending restart.

Cards: **Resume where you left off** `r` (only while sessions run; its second row is project ›
session), **New session** `n` and **Open a project…** `o`. The chosen card is `card2` with an
accent `▌` down its edge, a bold strong label and an accent keycap. Arrows or `j` `k`, and a card's
key, choose; Enter opens the chosen one; a click chooses and a click on the chosen card opens it.
Below 30 rows Resume loses its second row and the gaps between and above the cards close.

The intro, at 50 ms a frame: letter k rises from the deep colour to its own from frame 3k over 8
frames (eased out), nothing moving; the swell fades in over 14 frames and rolls a step every 3; the
tagline fades in over frames 10–20. Cards and hints are drawn from the first frame and keys act at
once. `ui.splash_animation` (Settings › Appearance): `cold start` (default: the first start of a
version, or nothing left running), `always`, `never`. Motion off, or a remote session, never plays
it and holds the swell still.

## Project-specific: collapsed sidebar

The sidebar has three states (`App::sidebar`, `App::side_hidden`): full, **collapsed** and **hidden**.
Leader `z` (`Action::ToggleSidebar`) swaps full and collapsed; Hide sidebar (`Action::HideSidebar`, search only) hides
the sessions or brings the sidebar back. The `»` chip (`«` with the sidebar on the right) and the `+n more` chip
restore it. There are no bare-letter keys for this in the sidebar.

- **Collapsed** (`screen/rail.rs`): the sessions are chips along row 0, not a column. A `»` chip first, then one chip per
  session in the sidebar's order: its mark (same glyph and colour as the row) and its full name, at most 16
  characters, on `card` (`card2` in Flat). The selected chip is `accent` with `acc_ink` text while the strip has the keys,
  else `btn`. Chips that don't fit give way to a `+n more` chip (with an amber `●n` when hidden ones need you) that opens the sidebar; the selected chip is kept in view. A `menu` chip keeps mouse access to the Menu at the right, before the Update pill, which keeps the top right.
  The panes take the full width and start on row 1, so their tabs stay visible. `ui.rounded` (Settings › Appearance › Rounded: `full`, `corners` or `off`, off by default) is the one shape setting. `corners` rounds every box's corners only. `full` rounds them too, and caps every chip (tabs, the SESSIONS title, card titles, + New, Menu, buttons, menu rows) on whatever ground it sits, the pointer's hover, and draws a selected or failed session row as one block, in place of the edge bar: half-circle caps for one line, quarter-cell corners and half-cell sides for more (`card::block_ends`), never a pill per line; Flat's floating cards are softened the same way; what is right-aligned stops a cell short of the cap. The Settings tab strip carries no update dot: the top-right pill and the splash announce updates, and the version lives in one row, General › UPDATES › Version (`maitty 0.42.0 · up to date · u check now`, or `9.9.9 is out · Enter to update`; Enter or a click acts, `u` checks from any tab). Above the pages, only what is off shows: an installed or daemon version that differs, or a pending restart with Restart now.
- **Strip focus** is the sidebar focus with the sidebar collapsed (leader Focus the sidebar). `←` `→` and `1`–`9` switch
  sessions, opening each in the pane while the keys stay on the strip; `1`–`9` mean nothing elsewhere. Enter focuses the pane, Esc leaves.
- **Hidden**: no sessions and no strip; the panes take everything. Reach them again with leader `z` or `Z`.

A session an agent started (a helper) steps in two columns under the session that started it, to two levels. A row asking to start helpers keeps its second line (`wants to start N helpers`), which gives way last.

## Project-specific: frame presentation

Wrap each outer terminal frame in synchronized-update markers; end the update
even when drawing fails. An unfinished child update from the previous frame
must not delay switching to another session.

Keep pane padding and the card interior on the same surface as a terminal app
when at least three screen corners use one explicit background color. Preserve
the pane insets; isolated colored panels keep their existing theme treatment.
In Flat layout, a zone's body (the sidebar's, a pane's padding and its default
terminal cells) is the theme's `bg`, as in Cards; only row 0 is the `card` header
strip. Preserve colors applications paint explicitly.
