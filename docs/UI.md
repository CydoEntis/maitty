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

The optional desktop GUI is a separate React application in `apps/gui/`. Its
components, state and styling follow the TypeScript, React and shadcn/Tailwind
stack docs. It shares session and conversation contracts with the TUI, while
its visual design is owned by the GUI roadmap phases.

A chat (a conversation the desktop app runs) is a row in the TUI sidebar like any
session. Its name and whether it is closed are the daemon's, shared with the desktop
app: **Rename** (the rename key, or Rename in its menu) names it for both, and
**Close** puts it away in both while it keeps running (closing a chat's terminal does
the same). A chat has no terminal, so Enter or → on its row says it opens in the desktop
app instead of showing an error; a terminal session opens in both apps. A session is a
chat or a terminal session from the moment it starts ([ADR 0044](adr/0044-chat-and-terminal-are-separate-sessions.md)),
and nothing switches a live session between them. The desktop app shows terminal sessions
only: a new session there is a plain shell, closing a pane or tab detaches, and ending a
session is its own action ([ADR 0046](adr/0046-desktop-app-is-a-terminal-multiplexer.md);
its look is in [DESIGN](DESIGN.md#desktop-app)).

The desktop app also shows other machines' terminal sessions (ADR 0048).
Its host holds one supervised connection per machine and the web view only renders: it knows a machine by
name, never by where it points. A session of another machine has the id `build/t12` (the machine's name, so a
saved layout survives a restart; this computer's stay `t12`). **Settings › Machines** retries, removes and
adds machines (the behaviour is the terminal app's [Machines dialog](#project-specific-machines-dialog),
with the host list read off the UI thread, the look at the machine ended by a 12 s limit or Cancel, and a
login that needs a person run in a session named `ssh <name>`); **New session** on a pane of another machine
asks for a folder on that machine. A screen reader is told when a machine goes down (`build disconnected: unreachable`) and when it is back, once each and not for every retry, through a polite live region; every state is a word as well as a shape and a colour, nothing about machines animates, and the Machines page works with the keyboard alone (Tab, Enter, Esc ends a look in progress without closing Settings). Features that read a pane's files (Inspect, worktreesFeatures that read a pane's files (Inspect, worktrees, opening a file from the output, dropping files, the screen text) are for this computer's panes only: each says `<Feature> is local-only in v1: the files are on <machine>, not on this computer.` and the Repository button shows disabled with that text.

A terminal UI (ratatui). Colours come only from the theme (`theme.rs`): semantic
slots `bg`, `card`, `card2`, `hov`, `accent`/`acc_ink`, `text`/`strong`/`muted`,
and status colours (`working`, `blocked` = needs you, `done`, `idle`, `err`).
A working row is one colour, `accent`, across its glyph and its name (shimmering with motion on). Needs you (`blocked`) is amber and failed (`err`) red in every theme, so they never read alike. `dark` sets its amber by hand; `mono` uses named yellow and red; every other theme keeps an amber of its own that stands apart from its other statuses, or gets one derived from its yellow (`with_distinct_needs` in `theme.rs`, darker on light themes). The audit in `theme.rs` asserts needs you looks unlike every other status, on the row and lit. A theme's amber `working` slot stays out of the sidebar. Chips on the check sheet, the Inbox's group heads and the approval tint use the same slots (`client/reference_frames_tests.rs` asserts the roles).
Every built-in theme passes a WCAG contrast audit test. Shared primitives in
`client/design.rs` and `client/screen/`: `panel()` (popup), `fill`, `put`/`seg`
(text runs), `keycap`/`keycaps`, `hints`, `button`, `dim_all`. Icons come from
config (`[icons]`), with Nerd Font glyphs where present.

## Project-specific: GUI design system rules

Rules for `apps/gui/`. Owner-approved 2026-11-04.

1. Every colour, radius and shadow comes from a theme token (a CSS variable),
   mapped into Tailwind by the `@theme inline` block in `apps/gui/src/index.css`.
   No raw hex, `rgb()` or `hsl()` in components.
2. Every popup (menu, dropdown, dialog, tooltip, popover, listbox) is a
   shadcn/Radix primitive from `src/components/ui/`. No hand-rolled focus, Esc
   or arrow-key handling.
3. New components use Tailwind utilities on those tokens. Heavy animation and
   per-style rules (Flat, Tiled, Glass, `data-gpu`) stay in `workspace.css`.
4. Old CSS moves to these rules only when a ticket touches that area. There is
   no big-bang rewrite.

`pnpm lint` enforces rule 1 in TS and TSX: `eslint.config.ts` rejects a hex,
`rgb()` or `hsl()` literal in `apps/gui/src`. Exempt: test files, the theme
files in `features/workspace/theme/`, and `TerminalView.tsx` (the xterm.js
palette takes literal colours). Don't widen the exemptions to pass a change.

## Project-specific: usage and cost page (desktop)

The Usage and cost page (`apps/gui/src/features/usage/`, look in [DESIGN](DESIGN.md#desktop-app)) shows only
what providers reported through the daemon's usage store ([ADR 0051](adr/0051-the-daemon-keeps-provider-reported-usage.md)).
A figure nobody reported is a dash in the UI, never zero and never computed from tokens. The monthly
budget is a client preference (`monthlyBudget`, validated in `preferences.ts`); the ring needs a budget, a
known cost and the Month window. The report is fetched with a query (no polling faster than 60 s), and
the page never blocks the UI thread.

## Project-specific: layout and navigation

<!-- local: user requested sidebar-bottom controls and no global footer; on 2026-10-24 they chose one exception, the mode bar along the bottom (ADR 0024). -->

Maitty has one terminal interface: a sidebar of projects and session cards (this computer's under Local, other machines' under Remote, [ADR 0050](adr/0050-local-and-remote-sidebar-with-session-cards.md)), with tabs and split panes in the remaining area. There is one look, **Floating** (ADR 0039): a box per pane, `ui.gap` (`small`, `medium` or `large`) between them. **Pane colours** (`ui.pane_colors`) draws each pane's border in its own colour by its place in the tab: `Theme::grid_colors`, top-left first, repeating from the fifth; the focused pane's border full and bold, the others' faded 42% toward `bg`. `ui.rounded` (`full`, `corners` or `off`) rounds the corners of every box (`╭╮╰╯`) with `full` or `corners`; with `corners` chips, buttons and rows keep square ends, and with `full` they end in half-circle pills (a Nerd Font glyph). `ui.padding` (`small` by default, `medium` or `large`) leaves that many cells empty round the whole screen (`draw.rs` `padded`; `framed` is that room less the mode bar's row). Older config values still load (`Ui::fold_retired_layout`, the file not rewritten): `tiled` (and `cards` with `panes = "tiled"`) as Floating with `gap = 0`, `grid` as Floating with pane colours, and `compact`, `flat` and `hairline` as Floating. The first start with such a file says so once in a toast (`RetiredLayout::note`) and writes what stands for it into the file (`client/mod.rs` `settle_retired_layout`). In this doc **Cards** means Floating. Tab, Shift+Tab and Esc go to the program in the pane; the leader's Focus the sidebar moves focus to the sidebar. There is no application-wide footer bar but the mode bar (see *Mode bar*), and the tabs are chips on the first pane's top border ([ADR 0038](adr/0038-tabs-sit-on-the-pane-border.md)), `ui.tab_gap` leaving a cell clear between them (or none), followed by a **+** chip (`strong` bold on `btn`) that opens a tab (see *the frame*). Keep the **new** and **menu** chips at the bottom of the sidebar (see *sidebar check result*), and collapse or hide the sidebar as *collapsed sidebar* says. The mode bar along the bottom always shows the mode, and **Update** at its right end only when a newer release is available.

The leader is `Ctrl+Space` by default. Inbox (`i`), Changes (`c`), Files (`f`), Branch (`b`) and References (`a`) use the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The key overlay (`Ctrl+Space`; `Space` searches, `?` lists all), the branch switcher and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. A click on a toast (a finished session's note, the Restart now button) is the toast's even where it covers a pane whose program asked for the mouse; the program never sees it (`Screen::toast_rect`). A right-click menu answers each item's key while it is open but does not draw it, as the Menu doesn't (a bare letter beside an item reads as a key that works anywhere): the leader key bound to the same action in the live keymap (`menu.rs` `menu_keys`: split `v` / `h`, close `x`, move `Alt`+arrows, Changes `c`), else a letter of its own, clear of those keys. A bound key the menu answers itself (`j` / `k` and the arrows, Enter, Esc, Delete, a check's `r` `o` `O` `u`, or the `x` that closes) goes to the menu, and that item takes a letter instead. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll. Branch (`b`, `client/screen/branch_sheet.rs`) uses the right-hand sheet: the session's branch, ahead/behind its upstream and its base branch (the main checkout's, else `main`, else `master`), the last 10 commits, the session's last check and its snapshots, all read by git in a background task. Its first row, **Switch branch…**, opens the switcher popup; Enter on a snapshot starts the row menu's restore (asked first). A folder outside git says so instead of the lists; an upstream whose remote branch is gone reads `gone from the remote`, and a git error shows in the place of what it was reading. References (`a`, `client/screen/references_sheet.rs`, files through `crates/maitty-terminal-client/src/references.rs`) lists the project's screenshots and design files from `.maitty/references/` at the repo's main checkout (a folder outside git keeps its own), so agents in every worktree see one set: name, kind, size and age per row, newest first, the selected row a neutral tint with an accent `▎`. Enter (or `o`) opens one in the system viewer as a Ctrl+click path does, `p` (or Ctrl+V) adds the clipboard image as `screenshot-<UTC time>.png`, `n` asks for a file's path (typed or pasted; a drop's quotes come off) and copies it in, `d` (or Delete) removes the chosen one after a danger confirm, `y` copies its absolute path; `a` again or Esc closes it. A paste while it has the keys stays in it: one line naming a file (a file dropped on the terminal) is added, an empty paste adds the clipboard image, and anything else gets the toast "References has the keys: …", which never repeats what was pasted. A name starting with a dot is refused. Entries the listing left out (unreadable, or links out of the folder) are counted on the list's last row in `err`, with the first reason. Empty, it reads "No references yet · paste an image or add a file", where they are kept and that agents find them through MCP, and its action bar holds only the ways to add one. Every file step runs in `spawn_bg`; the first time Maitty makes the folder in a repo it adds `.maitty/references/` to the repo's `info/exclude`. Over `--remote` it is refused with a toast, like Files. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

**Send a file** (`client/send_file.rs`, ADR 0055). In a pane on another machine the leader key `d` opens the prompt `Send a file to this machine: its path` (mode bar `SEND FILE`; a paste or a drop puts its first line in the box, a drop's quotes come off). Enter starts the upload in `spawn_bg`; a toast reads `sending <file> to <machine>… <n>%` and is refreshed once a second from the byte count, without adding to the notification history. `d` again, or a click on the toast's close mark, stops it (`stopping the send to <machine>…`, then `stopped sending the file to <machine>`); Esc only dismisses the toast and does not stop it, because Esc also belongs to the program in the pane. One file is sent at a time. On success the far-side path is typed into the pane like a local paste, and the toast says `sent <file> to <machine>`; every failure is an error toast (not a file, over 100 MB, no ssh to the machine, a login that needs a prompt, the machine could not keep it). On a local pane the key says there is nothing to send.

The sidebar's **+ new** chip, a project row's **+**, the empty project row's new action and leader + `n` are one action (`Action::ShellHere`): a plain shell where you are, whose tab opens with focus. You type your agent in it (see *Agent launchers* below). Leader + `s` starts an agent session straight away with the defaults in Settings › Sessions (`Action::StartSession`, Menu › Session). **Menu** is in the sidebar footer. Every row of the key overlay and the Menu is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** at the right end of the mode bar and open the existing confirmation flow when clicked. Do not show the control when Maitty is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. When a newer release is out, the first line under the tabs, on every tab, says so in the accent with an **Update now** button (`dialogs.rs` `draw_settings`). The tabs are General, Appearance, Sessions, Alerts, Agents, Check and Keys (`modal::Cat::ALL`); each groups its rows under headings in `design.rs` `GROUP_ORDER`, rows keeping their `SETTINGS` order within a group: General: UPDATES (the Version row first, then Check for updates), STARTUP, INPUT, SHELL; Appearance: THEME, LAYOUT, PANES, STATUS; Sessions: NEW SESSIONS, SIDEBAR, WORKTREES, RESTARTS, HISTORY; Alerts: SOUND (Alert sounds, then a switch for the needs-you sound and one for the finished sound, and Sound volume: `quiet`, `normal`, `loud`), PHONE; Agents: AGENT STATUS, HELPERS, LIMITS, AGENTS TALKING TO AGENTS; Keys: the leader's shortcuts by group, then WITHOUT THE LEADER (the Alt+letters, Alt+1–9, Alt+arrows; `design.rs` `direct_key_rows`). Enter on a one-key row takes the next key pressed and writes it to `[keys.prefix]`, or `[keys.global]` for a key without the leader. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

There is no Team tab; `[team]` in `config.toml` still loads and is no longer shown. The label column is 33 columns and no setting label is longer; the value column starts after it, and a Choice whose chips would not fit beside the label shows the current value between `‹ ›` instead. The selected row's help (`Setting::help`) sits under a rule at the bottom of the page, word-wrapped over up to two lines, so a help text stays within two lines at 80 by 24.

**Settings › Sessions** opens on NEW SESSIONS (`[sessions]`: `Agent` last used, claude, codex or gemini, `Where` new worktree or this folder) and HELPERS (`Sessions may start helpers` ask, always or never; `Helpers at once` 2, 3 or 4), then the older alert, restart and history settings. Values outside those lists are refused when the config loads. `Agent` and `Where` are read by leader `s` and the Session with options dialog (an old `plus_starts` key is read and ignored); the helpers settings are read by the daemon when an agent starts a session (see *helpers*). `[mcp] helpers_at_once` is not a setting any more: it is read only as a fallback for `[sessions] helpers_at_once`. **Settings › Agents** opens on AGENT STATUS: Claude Code reads `✓ hooks installed` (read once in the background when the page first shows, `checking…` until then, `hooks not installed` otherwise; the same test as `maitty doctor`), Codex reads `✓ read from the process and screen` until its lifecycle hooks are installed, then `hooks installed · trust them in Codex /hooks` until Codex has recorded trusting each, then `✓ hooks installed and trusted` (read once in the background; its help says Connect agents installs them and you review them once in Codex's `/hooks`), and `Connect agents…` opens the Connect agents list (`Action::AgentStatusSetup`), which offers exactly the agents `maitty integrate` sets up (`STATUS_AGENTS` in `crates/maitty-terminal-client/src/cli/integrations.rs`). SUBAGENTS has `Count on the session row` (`ui.subagent_count`, on by default): off hides the `· N agents` text on a working session row. The `mcp.*` and worktree settings follow. General's `Leader waits for next key` (`ui.which_key`, off by default) shows every key if you pause after the leader.

**Subagents have no tree.** The sidebar never lists an agent's subagents and has no inspector for them; the agent shows them in its own pane. A working row says `· N agents` only when the agent reports subagents (`ui.subagent_count`), and nothing else about them is drawn. The daemon still tracks them (`TermInfo::subagents`, `subagent_activity`) so the count is right and a parent stays working while its children run.

**Secret requests open by themselves** (`client/screen/secret_ask.rs` `auto_open_secret`, called after each state update). A request not yet offered opens its prompt beside the row when the mode is Normal or Side and no side view is open; otherwise it waits and the row's `asks for a secret` line stands. Each request id is offered once, so Esc is final. A prompt opened this way ignores keys and pastes for 1.5 s (Esc still declines), so a line being typed in a pane is not given away as the secret; one opened by Enter or a click on the row has no wait. Closing returns to the mode it opened from.

**Helpers are not a mode** (`client/screen/helpers_ask.rs`). Any session may start helpers (other agents it starts through `maitty mcp`); `[sessions] helpers` (ask, always, never) decides. When a session asks and the setting is `ask`, its row is **needs you** (amber, as for any question) and its second line reads `wants to start N helpers` (`wants N helpers` on a narrow sidebar). Enter on the row with the sidebar focused, Enter (or a digit) on its row in the Inbox, or a click on that line, opens a popover beside the row (from the Inbox, Esc or an answer goes back to the Inbox on that row): the screen dimmed except that row, a full accent border, a `◂` on the row, the title `START HELPERS? · <session>` and `Esc later` at the right. It says which agent wants to split the work into N helpers, lists the tasks (five, then `+ n more`), then "Their permission prompts come to your Inbox. Nothing merges." Buttons and keys: `1` Allow, `2` Always in <project> (`Always here` when narrow), `3` No, do it alone, each clickable; Esc decides later and the row keeps asking. Enter, `y` and Space do nothing there, and `1`/`2`/`3` act only inside the popover: no bare key answers from the sidebar or the collapsed strip (ADR 0014). The answer is a command the daemon accepts only from you and only while it holds the request (`docs/API_PATTERNS.md`).

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running until **Restart now**. For a local daemon, when no panes are open, restart it automatically (see below). For a remote daemon, explain that it must be updated on its host after its sessions are closed. The one `↑ Update` pill sits at the sidebar's foot above its divider (ADR 0038), or on the mode bar (ADR 0024) when the sidebar is collapsed, hidden or short, and becomes `↻ Restart` once a newer maitty is installed (here, or from outside maitty) and the server or this window still runs the old one; it clicks through to the same restart. The splash carries the matching line; there is no second notice in the working area.

**Restart now** is one step: it restarts the server onto the installed maitty **and** replaces this window with the installed one in the same terminal. It shows in Settings' restart-pending line (`R`), on the toast after an install (it stays 12 s), on the `↻ Restart` pill, and as the palette action *Restart maitty* (`Action::Restart`, `restart`); none show for a remote daemon or while a restart is under way. It asks only when a session is working or waiting for you (or when `restore.enabled` is off, since then sessions close for good): a confirm (`menu::Confirm`, danger) reading "4 sessions stop and resume, 2 are working, 1 waiting for you" with Restart (Enter) and Cancel. Otherwise it restarts at once. The server saves its session and every pane's output first (`KillServer`), then the window closes and the installed maitty opens in its place (`restart::relaunch`: exec on Unix, spawn-and-exit on Windows, same arguments and `MAITTY_SOCKET`), which starts the new server and restores the sessions. A window that is already current (only the server is old) reconnects in place; a current server with an old window swaps only the window and leaves every session running. An agent mid-turn is stopped when the server stops and resumed with its conversation when it comes back; the turn in flight is not replayed.

A new `maitty` that finds a server of another version replaces it on the way in. A server of an older release whose protocol this one speaks, and which is idle, is stopped at once (`restart::replace_older_idle_server`, from the activity file the server keeps beside its pid file: `restart::Activity`). A server whose protocol this one cannot speak (`ipc::OtherVersion`) is judged by that file, or for a server too old to write one by the agents in the saved session (`restart::plan_for_mismatch`): when nothing is working or waiting, the terminal says `Updating maitty…`, the server is stopped by its process as `maitty kill-server` does across versions, and the window opens with the sessions restored, with no prompt. When something is working or waiting, or the server is newer than this maitty (never replaced unasked), the full-screen prompt (`client/restart_prompt.rs`) shows the protocols, how many sessions resume and how many are mid-turn, and **Restart and open** (Enter) and **Quit** (Esc, Ctrl+C), each clickable; Quit prints the usual error and exits 1. Without a terminal on stdin and stdout, or with `--remote`, there is no automatic step or prompt: the error text and exit code are unchanged, and a remote server is never restarted this way. A same-protocol older server that is busy is left running; the `↻ Restart` pill offers the restart.

**One key overlay** replaces the command palette, the long shortcuts reference and the old leader overview (`client/screen/keyoverlay.rs`). It has three faces in one card with one typing row: Its card has no fill (its lifted rows are flattened to the ground), a red `×` close on the top border, and its key hints are chips on the bottom border.

- **Key map** (`Ctrl+Space`, or the pause after it). A banner, "Every key goes after the leader: `Ctrl+Space` then `v` = split", then Sessions, Panes, Tabs, Project and Maitty, each key a keycap followed by its name, from `key_groups()` and the real bindings. A pressed key runs its command; `Tab`, or an unbound letter, switches to the search. Three columns when the card is 100 or more wide, else two; each group's key column is as wide as its widest key. A key marked `›` opens a second step (Worktrees). Action bar: `key` run it, `Tab` search, `?` show all, `Esc` close.
- **Search** (`Space` after the leader, or the Menu's Search commands). Every command, keyed or not: the name with the match in the accent, its group in dim, its key at the right. With nothing typed it lists RECENT (the last commands run from the overlay; a few defaults before there are any). Enter runs, `Tab` returns to the map. A set of keys (the arrows) is searched one key at a time. Every command the old palette offered stays reachable here.
- **All keys** (`?`, Menu → All keys). The map in full, then "WITHOUT THE LEADER · press directly" (Alt+1–9, Alt+arrows and the Alt+letters when on; ADR 0029) and "SEARCH ONLY" (commands with no key), both headings dim. It scrolls when it is taller than the screen; a letter typed there starts a search.

Rules for keys and their labels: one key per action (no alias keys; a user's own `[keys.prefix]` entries still load), no default needing Shift, keys other apps use (ADR 0017), sentence case, no bracketed explanations (the group gives the context). Every action has a leader key or is search only; nothing in the sidebar acts on a bare letter. `docs/shortcuts.md` documents the other screens' keys and is not embedded in the app.

**Tool rail and drawer** (`client/screen/tools.rs`, `sheet.rs`). A 5-column rail on the right edge,
always drawn from 100 columns (`ui.tool_rail`, off by default since the Menu opens the same four; a left sidebar only), has a chip per tool
(`f` Files, `c` Changes, `b` Branch, `a` References, two rows apart, the keys of the keymap). A click opens that
tool in the drawer, switches to another, or closes the one open. The drawer is a column beside the terminal by default: the terminal shrinks for it, so nothing is hidden under it; each tool keeps its own width (drag its left edge, or `[` `]` or `<` `>`, 4 at a time, while it has the keys). With `drawer_floats` in the saved UI state it instead floats over the right of the terminal, which keeps its whole column (never resized by opening, closing or sizing it) and shares one width of 84 columns between the four tools; there is no button for it yet. Its top border is its title (` Files `, ` Changes `, ` Branch ` or ` References `) and the `×`; the rail is where the others are, and Tab and Shift+Tab still step along them while it has the keys. It has no tab strip, no legend row and no width chip. Files shows the tree and the open file side by side from 60 columns, and by turns under that. The Inbox and the check sheets stay docked. A draggable edge (the sidebar's, the drawer's) wears a quiet
three-row `┃` grip, in the accent while pointed at or dragged.

**Cursor.** The focused pane shows the cursor its program asked for (DECSCUSR, read from its output); where it asks for none, or for the default, Settings › Appearance › Cursor (`ui.cursor`: `block` by default, `underline` or `line`; an older `terminal` reads as `block` and `bar` as `line`) and Cursor blink (`ui.cursor_blink`) decide (`client/mod.rs` `cursor_code`). An unknown `ui.cursor` is refused when the config loads.

**Background.** Settings › Appearance › Background (`ui.background`): `solid`, the default, paints every cell the theme's `bg` and sets the terminal's own background to it (OSC 11, given back with OSC 111 on exit) so a plain terminal shows the whole theme; `terminal` leaves the chrome on the terminal's background (SGR 49) for transparent or image backgrounds (`client/render.rs` `draw`).

**Keycaps.** Every key the UI shows is a keycap from `design::keycap`: ` key ` on `btn`, `strong` and bold, with the ends round when `ui.rounded` is `full`. A combo splits on `+` into one cap per part joined by a dim `+`. The map, the search, hint rows, Settings key rows, dialogs and the copy hint all use it.

**Action bars.** A panel's key hints sit in their own one-row bar (`action_bar`, `panel_bar` in `popups.rs`) on a ground one step above the panel's: `card` for a sheet on the app background (Inbox, check evidence) and `card2` inside a modal (Settings, the key overlay, confirms, pickers). It spans the panel's width with one column of padding, shows keycaps and labels, and is always the panel's last row. When the hints do not fit they close their gaps, then give up the ones before `Esc`.

**Menu.** Sections a blank row apart, each under a `muted` title that says what its rows are for, labels capitalised, no letter lit: `NEW` (Shell here, Session, Pane beside this, Tab); `TOOLS` (Files, Changes, Branch, References, which open in the drawer); `FIND` (Inbox, Search commands, All keys); `SIDEBAR` (Collapse sidebar, Hide sidebar); then Settings, untitled. In a short window the blank rows give way first, then the titles, so every row that does something stays (`leader.rs` `MenuRoom`). No row shows a key (a bare letter reads as one that works anywhere), but the leader key that does the same runs the row inside the Menu too; there is no footer. "Shell here" is leader `n` (a plain shell where you are) and "Session" an instant agent session (leader `s`).

**Agent launchers.** Typing an agent (`claude`, `codex`, `gemini`, `opencode`, a quick agent) in a Maitty shell in a git repo's main checkout starts it in a new worktree (`worktree.per_agent`); anywhere else, or in a folder that is not a git repo, it starts where you are, silently. It works from any shell because `<data>/shell/bin/` is first on the pane's `PATH`; see ARCHITECTURE › Agent launchers. No toast or warning is shown for main folders.

**New sessions** (`client/screen/newsession.rs`). Leader `s` starts a session with no form: agent `sessions.agent` (`last used` is the last agent started from Maitty, kept as `last_agent` in `maitty-ui.json`, else claude), project the one under the sidebar cursor or the one you are in, `sessions.where` (`new worktree`, which falls back to the folder in a project that is not a git repo, or `this folder`). The task is typed in the session, whose row reads `<agent> · new · type to begin` with the accent `○`. A new tab (leader `t`) takes only the shell that answers it: it waits ten seconds at most, takes only a pane that did not exist when the tab was asked for and that no agent started (another pane taking the focus leaves it waiting), and starting any other session before it arrives cancels the tab, so that session gets its own row and tab bar rather than becoming a tab of the one you were in.

## Project-specific: pane focus

Only the area with the keyboard is lit: the focused pane's whole border, or the open sheet. Unfocused panes fade by the Dim setting (off, light, normal or heavy; normal by default, and an older config's "subtle", "30%" and "60%" read as light, normal and heavy). Do not reserve a full-width bottom row for global hints; the mode bar (ADR 0024) is the one exception. The pane has no status line: its branch is in the sidebar's folder header.

## Project-specific: UI conventions

<!-- local: user requested distinct active, keyboard and hover sidebar states, mouse-first sidebar controls and parent-owned child-agent visibility. -->

- Popups are centred `panel()`s (`panel_at()` for a given rectangle) framed like a
  pane, in every layout: a box in the `accent` (a modal holds the keys) with corners by
  `ui.rounded`, the title as an accent chip on the top border two in, and a red × chip
  before the top right corner (the whole chip closes it); with `ui.rounded = full`
  both chips have caps. There is no accent bar across the top: the user chose the
  pane-like frame over an accent title bar (2026-10-24), and the frame
  tests apply that one difference (`as_bordered_header`). No shadow; the screen
  behind is left as it is, not dimmed: the modal clears its own area to the ground (`dim_all` only
  marks what is behind as taking no hover). Small explanations (why, a row's menu) are
  `popover_at()`s: a raised title row, no border, no shadow, beside the sidebar
  (`popover_rect()`). Settings is a
  centred modal that preserves the workspace layout and keeps its General,
  Sessions, Appearance, Agents and Keys sections, update controls, and keyboard
  and mouse editing available.
- The pointer's tint is one pass after drawing (`render.rs`): the topmost
  clickable region under it is tinted once, `mix(bg, text, 0.10)`, its ends round like every other shape when `ui.rounded` is `full` (`card.rs` `round_tinted`: an end cell holding only ground, or already a rounded end, becomes a cap or soft corner; an end with text stays square). Widgets do
  not tint themselves on hover. Only the top layer takes the pointer: `dim_all` marks every
  region drawn before it as under the overlay (`hits_under_overlay`), so behind a modal,
  popover or floating pane nothing tints and the pointer keeps its arrow, and an overlay's
  backdrop (its click-outside region) is never tinted. Inside the overlay hover works as
  anywhere.
- Shortcut mode ends on Esc, the leader again, a click, or an action that takes you into a
  pane to type (ADR 0025 lists them: `n`, `s`, `t`, the splits, `spawn-*`, `o` once its
  dialog starts one, the worktree picker, opening a project). Each calls
  `leave_shortcut_mode`, which also sets an open side panel aside so the next key reaches the
  pane; `screen_new_session` and `screen_new_worktree` call it for you, and a new way to start
  a pane must too. `screen_start_pane` starts one without it, only for a pane nobody asked
  for (the home shell after the last session closes).
- Leader `x` closes what has the keys (`actions.rs` `ClosePane`): a side panel that has them
  (`view_has_keys`) closes with no question, so x never stops the program in the pane behind
  it; Files with unsaved edits stays open and says so. A click into a pane, or moving into the
  sidebar (`screen_side_set`), sets the panel aside: then x closes the focused pane, or in the
  sidebar the cursor's row, with its `Close pane` confirm. `f` or a click in the panel gives
  it the keys back.
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

Dark and light are two remembered slots and a switch. Appearance has three rows:
**Appearance** (`appearance`: `dark` or `light`, which slot shows), **Dark theme**
(`theme_dark`, `night` by default) and **Light theme** (`theme_light`, `day` by default).
`theme` stays "the theme showing now": `Config::settle_theme` sets it from the slot
`appearance` picks, so once a file has slots a `theme` written by hand is overwritten.
A file with only an older `theme` is not rewritten: its theme fills the slot its ground
belongs to and decides the appearance, one slot at a time (a light pick written beside an
old dark `theme` leaves that dark theme showing).
Enter or clicking a theme row's value opens a searchable browser with two tabs, **Dark** and
**Light** (`Tab` / `Shift+Tab`, or a click on the chip at the foot), each listing only its
themes, opened on the tab of the row you came from and on that slot's theme. Which tab a
theme is on is read from its background (`theme::is_light`), not from a list. The browser
shows palette swatches, an applied marker for that slot, a result count, and a preview.
Themes are listed alphabetically by display name, ignoring case (`BUILTIN` in
`theme.rs` is kept in that order, and every list of themes follows it); there are
no user-defined themes, only overrides of a built-in's colours.
Typing filters by name or flavor; arrows, Home/End, page keys, and the mouse wheel
move the selection. Click a row to preview; click it again or press Enter to
save that tab's slot, which changes the screen only if that slot is the one showing. Esc or
the close button returns to Appearance without changing anything.
Saving runs in a background task; failure keeps the selection and shows a toast.
The key overlay's search has **Switch dark or light theme** (`toggle-appearance`, no default
key, ADR 0017), which flips `appearance` and changes the whole screen at once.
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

The **Bars** preset is the 3-bar Braille spinner the reference frames draw. A frame is one column, or
two when every character is Braille; a two-column mark moves the sidebar row's name one
column right. Other choices are never rewritten. Working also shimmers its sidebar name in the working colour. With
motion off (`app.motion_on()`) the spinner rests on its third frame and the shimmer stands
still, so nothing depends on motion.

## Project-specific: sidebar check result
<!-- local: the user chose on 2026-10-23 to drop the `idle` line on a plain idle row at every width (the reference drew it from 140 columns) -->
<!-- local: the user chose on 2026-10-24 one-line session rows with the agent's mark and uncommitted lines, names in their state's colour (ADR 0021) -->

The sidebar is two halves of session cards ([ADR 0050](adr/0050-local-and-remote-sidebar-with-session-cards.md)); the reference frames do not compare its column, and `screen::side` and `machine_group_tests` hold its tests (`side.rs` `card_lines`, `draw_side`, `draw_rows`).

- **Halves:** `LOCAL` is a title chip on the card's top border (the accent while the sidebar has the keys, else `btn`, with ` ⇄ host` after it when the whole window is on a remote server). The Remote half starts on a rule across the card, `├ REMOTE n ── + add ┤`, n the machines known (omitted at none), the `+ Add` chip opening the Machines dialog (`ScreenHit::AddMachine`). The list below the title is shared evenly (`halve`: one row for the rule, Local keeping an odd one); under four list rows only Local shows. Each half has its own scroll (`Screen::side_scroll`, `remote_scroll`; the wheel scrolls the half under the pointer) and the selected or open card is kept in view in whichever half holds it.
- **Project row** (`draw_proj`): `▾` or `▸` in the state bars' column, the name (`strong`, bold, never cut before the meta) and right-aligned `proj_meta`: `⎇ main` and `✓` or `✕` in `done` or `err` once a session has a check result, or `no git`. Folded, `n hidden` with an amber `●` when one needs you. A click or the fold button folds it; a project drags within its machine.
- **Card, row 1:** the session's mark (`client/screen/glyphs.rs` `Kind`; bold for failed, needs you and approve), the name, then right-aligned `● you` in `blocked` when it needs you, else the agent's mark (`muted`: `✻` Claude, `◆` Codex, `✦` Gemini, `◇` OpenCode, else its first letter), a space and the age (`muted`, from 100 columns). The mark gives way first, so the name keeps 8 columns. A name in a state worth a look (working, checking, done and not seen, needs you) is drawn in its state's colour, the glyph's, and bold; with motion on it shimmers up toward `strong` (working, checking, not seen) or pulses (needs you) (`side.rs` `NameMotion`). Another row that wants a look (failed, a check result other than a plain pass) has a `strong` bold name; a done, seen or asleep row a `text` name, not bold, `strong` only while selected. The hovered row's `⋯` takes the age's place.
- **Card, row 2** (`activity`): the headline check's result by checkpoint name, the question (`asks: …`, in `blocked`), helpers it asks for, a plan-limit wait, a new agent's first step, `asleep zzZ`, a message just sent, or for a working agent its helpers (`2 agents`, `ui.subagent_count`) and a quiet spell (`quiet 12m`); else the agent's name or `shell`. `muted` and never bold, except a question or an approval, which stay amber. While the agent works or asks, its last result is about an older turn and is not shown.
- **Machine header and items:** each Remote machine starts with ` ⇄ build ` (a badge on `btn`, the icon from `App::machine_icon`; dim while it is down) and its connection at the right as one dot (`App::machine_dot`: `done` up, `blocked` amber while it connects or reconnects, `err` unreachable). The amber here is the one place it does not mean needs you, at the user's request. Local has none. A machine that is not connected, or has no sessions, is that header and one line of how it stands (`machine_group.rs` `state_forms`: `no sessions yet`, `connecting…`, `reconnecting…`, or `needs login`, `needs update`, `unreachable` or `refused` followed by ` · click for the fix`, which gives way in a narrow sidebar) in place of its projects. Both rows are the machine's (`ScreenHit::MachineRow`): a click opens the Machines dialog on that machine, a right-click its menu. This computer with nothing running says `no sessions  new shell` and a click starts a shell. Sessions of a machine whose line is down are not drawn and not counted; its needs-you and failed marks leave the Inbox until the line is back.
- **Blank rows:** one follows each project. A helper session is listed like any other, not stepped in under the session that started it.

- **Tabs** (`side_tabs.rs`, ADR 0027, 0031): a session with two tabs or more has one row per tab
  under its card, in line with it (mark under mark, name under name). A thin `muted` line a column left of the marks runs from the session's own row to its last tab and groups them: the tab's state glyph (the agent's mark, the spinner while it works; `○` for a plain shell), its name (its own, else
  its focused session's), and its number in `muted` at the right. A project has a line of its own in its project colour, down the fold arrows' column along everything inside it, and a faint rule across the blank row divides it from the next project. The tab on screen is the selected row: it is lit exactly as a selected session's row is (`side.rs` `light_row`, the same cells and ends), its name `strong` and bold. A click on a session's row opens it as a click on a tab's row does, the keys going to its pane (only a session with no pane keeps them in the sidebar); one that needs you has an amber name. Short of room the name shortens. The arrows skip these rows (they are not `SideItem`s). A session's own card shows only the state of the panes in its own tab (`fold_splits`); another tab's working, done or question is its own row's to say. A plain shell is one row: it has no second line. A click shows that tab (`ScreenHit::TabPick`), so a drag reorders them and a
  middle click closes one, as on the tab chips. A session with one tab has none.
  `ui.sidebar_tabs` (Settings › Sessions › Tabs in the sidebar) shows `all` of them, only the
  `busy` ones (working or needing you) and the one on screen, or none (`off`). Never the tab the
  session's own pane is in: the session's row is that one (ADR 0030).
- **Foot:** one row off the bottom (Cards: the last row inside the border), a `line` divider above it, no band behind it. Two `btn` chips: `+ New` a column in (the `+` in `accent` bold) and `Menu` at the right, the words in `text`, no letter lit (`side.rs` `word_chip`). `Menu` drops first when the sidebar is too narrow for both. Chips are square on the plain `bg`, since a cap there has no ground of its own; `ui.rounded` rounds them where the ground is raised.
- **Amber is for needs you.** `blocked` colours only what needs you (its mark, question, counts and approvals), never a heading: Inbox "HEADS UP", the both-diffs sheet and the tidy list use `sky()`.

Settings › Appearance › Status glyphs picks how a session is marked. **Dots** (the default; an older `auto`, `ascii` or `nerd` reads as dots) draws a coloured `●` for every state that wants you or says something (needs you, done, failed, stale, overridden, not checked) and `○` at rest (idle, asleep); the colour and the word beside the mark carry the meaning. Working keeps your spinner and checking its ring. A check's own `✓` and `✕` beside its name (`build ✓ · test ✕`) keep their shapes. **Unicode** is the full set, where every state has a glyph of its own, as the table shows.

The colour slot is the glyph's; line 2 is always `muted` (see above).

| State | Glyph | Colour slot | Line 2 |
| --- | --- | --- | --- |
| working | the spinner you chose (Bars is 3-bar), bold | `accent` | none; `4 agents` only when the agent reports helpers, `quiet 12m` when it has gone quiet |
| needs you | `●` / `!` | `blocked` | `asks: <question>`, else none |
| done, not seen / seen | `✓` / `+`; `○` / `o` | `done`; `muted` | none: the row is one line |
| idle, and new (an agent told nothing yet) | `○` / `o` | `muted` | none: the card is one line |
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
- **What gives way:** the agent's mark (before the age), then the age (below 100 columns). Every card keeps its three rows; row 3 is cut with `…` at the sidebar's edge.
- **Selection:** the keyboard's card (while the sidebar has the keys) or the open session is tinted (`Theme::sel_bg()`) across both rows with `▎` at X: `accent` while the sidebar has the keys, else `muted`; while the keyboard browses, the open card keeps a `muted` bar. A failed card carries `▎` in `err` on every row, selected or not. X is 2 columns in, RX 3 from the right. With `ui.rounded = full` the block keeps square ends: both lines lit across the same columns.
- **Foot:** `+ New` at the left and `Menu` right-aligned, on the last row inside.
- Shape and word carry the meaning; colour only backs them, so every state reads in the
  Unicode set and without colour. Glyphs come from `[ui] glyphs` (`dots` or `unicode`; an older
  `auto`, `ascii` or `nerd` reads as `dots`, `Ui::fold_legacy_sizes`). Take marks from
  `check::marks`, never a literal.
- Colours are existing theme slots only; there is no check role. `Theme::violet()` and
  `sky()` are derived, not roles. Code: `client/screen/side.rs` (the card),
  `client/screen/check.rs` (a result's words and marks).

## Project-specific: check evidence sheet

The check evidence sheet is part of the frame (see *the frame*). Enter on a sidebar row whose check failed, could not run, was overridden, went stale or is never run for its repo opens it (a row waiting for approval opens the approval; every other row still focuses its session), as does Output in the Inbox. It docks right like the Inbox and takes the pane column below `NARROW`. Its title is `CHECK · <session> › <checkpoint>`; content starts one column into the card's inside.

- A result chip (` <glyph> <word> ` bold on the state's colour: `failed` on `err`, `failed · overridden` on `text`, `could not run` and `not checked` on `violet()`, `passed` on `done`, `stale pass` on `btn`), then what it came to in `strong` bold (`exit 1`, `exit 0 on older code`, `never for this repo`; `command not found` in `violet()` for exit 127).
- Command, Folder (the run's folder, home as `~`), Ran (`14:22:05 · took 14.2s · after turn 3 ended 14:21:51`, local clock; `did not run` when it never started) and Seen by (`Maitty ran this itself. The agent said "…".`), or Source (`.maitty/checks.toml · <commit> · you chose never`) for a repo's own command. Rows are clipped at the sheet's edge, not wrapped.
- A banner on a tint for a result that needs explaining: overridden (by whom, when, `u undo`, the reason), stale (how many files changed and when, then one `M path` row per file that fits), never, could not run.
- `OUTPUT`, then `all N lines`, `last N of 1,284 lines` (numbered as the whole output, from `CheckInfo.line_count`; `f full output` when the daemon kept only the tail) or `empty`, on a darker block with 5-column line numbers; `✓` lines `done`, failure lines `err`, `▲` when there is more above. Empty output says why: `(no output)`, `(the command printed nothing)` / `exit 0 is what passed it.`, or `(nothing ran)`.
- Buttons on the third row from the bottom, each a click of its key: `Check settings ,` for never, else `Re-run r` (primary when stale, absent while it runs), `Override… O` (failed only), `Send to agent m` (failed only), `Undo override u` (overridden only), and `Open session o` when it fits. The last row is always `Esc back   r re-run   f full output   i why`.
- Keys: Esc closes; `r`, `O`, `u` act as the buttons; `m` sends a failure (below); `f` and the arrows open all of the output and scroll it; `o` goes to the session; `,` opens `PROJECT · <name>` for the session's project. With several checkpoints it opens on the headline one, and Tab / Shift+Tab move to the next / previous; the title names the one shown, its output is fetched by name, and `r`, `O` and `u` act on it.

**Sending a failure** (`client/screen/failure_note.rs`). `m` on a failed check, from its Inbox item or its evidence sheet, opens the agent's message box (the Inbox row's, or the popover beside the session's row from the sheet) with the failure written in: ``The test check failed: `npm test` exited with 1.``, then the last 40 lines of its output in a fenced block (`Its output:` when there are fewer; `It printed nothing.` when there are none). Colour codes and control characters are stripped, from the name and command too, and a line past 400 characters is cut. A draft the box held stays, above the failure. Nothing is sent until Enter, which sends it as any message is (several lines go in as one bracketed paste, queued or not). From the Inbox the output is asked for first (`Query::CheckOutput`) and the box opens when it arrives, unless an open sheet on that checkpoint holds it; a daemon error stops the wait. The sheet already holds it. The mode bar reads `m send to agent` while a failure is selected in the Inbox or shown in the sheet.

The row menu of a session with a check adds Check output (any result), Re-run check (not while it runs), Override check… (failed only) and Undo override (overridden only); letters come from `menu_keys` like every item. Output comes from `Query::CheckOutput` and is fetched again when a new run replaces the result. Command output is not trusted: colour codes and control characters are stripped before drawing.

## Project-specific: check modals and settings

Deciding is a modal; nothing here pops up on its own. Each matches its reference frames inside the modal frame, cell for cell, in every layout at 160×45, 100×30 and 80×24 (`client/screen/check_modals_tests.rs`). Colours are existing slots: `blocked` for "needs you" amber, `done`, `err`, `violet()`, `sky()`, `card2` for fields.

**Override** (`client/screen/check_actions.rs`): `OVERRIDE A FAILED CHECK`, 70 wide and 15 tall. It says what the session will read (`done (overridden)`) though which checkpoint failed and when, what it hides and where the failure stays visible (⊘ on the row, in the Inbox history and in the evidence, for this result only), and takes an optional reason (at most `OVERRIDE_REASON_MAX_CHARS`; typing, paste, Backspace and Ctrl+U edit it). Enter overrides; `O` overrides only while the reason is empty, so a typed reason can hold an O; Esc or Cancel does not. Opened from the Inbox, the Inbox stays under it and Esc or Cancel goes back to it, on the same item. The sheet, Inbox, popover and row menu all open this one modal. After it the evidence sheet shows that checkpoint and a toast says `Overridden. <session> reads done (overridden); the failure stays in its evidence.` A re-run says `Re-running <cmd> in <session>'s worktree…` in a toast, or `re-run queued` while the agent works; a daemon error (`ServerMsg::Error`) is a toast. While a sheet takes the pane column, toasts show over the sheet.

**Approving a repo's command** (`client/screen/check_approve.rs`): opens only from its waiting item, by Enter on the Inbox item or the sidebar row of a session whose checkpoint needs approval. Several waiting: one modal per checkpoint, the next opening after a choice. It shows the command (`$ npm test`, or `- old` / `+ new` when the repo changed one you had said Always to) in full: wrapped over as many `card2` rows as it needs, the modal growing to fit; `\n`, `\r`, `\t` and other control or invisible characters spelled out (`\u{001b}`), and a run of spaces shown as `[N spaces]`. Where it runs, the file and commit it came from (`added in a1b2c3d, 3 days ago`, `changed in <sha>, 2h ago`, or `changed, not committed`), when it runs, and "Read it. It can do anything you can in that folder." `1` and `2` answer only once the last frame showed the whole command, and only for that command: one too long for the window ends its last row in `…`, says `The whole command doesn't fit. Make the window bigger and read it first.`, and 1 and 2 only say so; a command that changes disarms `2` (`The command changed. Read it again before you choose.`). There is no default: `1` runs once, `2` arms Always (a strip spells out what it means and the button reads `2 again`) and a second `2` sends it, `3` is Never and opens the evidence sheet, Esc leaves the item waiting (back in the Inbox, on the same item, when opened there). Enter, `y` and Space send nothing and say so; other keys and pastes do nothing. Clicking Always once arms it like `2`. Below 80 columns the buttons use short labels; below 30 rows the modal loses the blank line after the command.

**Settings › Check** (`client/screen/check_settings.rs`) is one renderer with two hosts: the `PROJECT · <name>` modal (`,` from the evidence sheet or an Inbox row, for that session's project) and the Check page of Settings (the workspace you are in). Empty, it explains what a check is and takes a first command. Set, it shows the selected checkpoint's command, where it runs, its timeout (`‹ ›`), its latest result by clock time (`✓ passed on fix-footer 14:22 · 14.2s`, `… never run`, `? could not run … pnpm: not found (exit 127)` with what to do) and its source, then `CHECKPOINTS`: `1 of 1`, or `N of M · ‹ › switch` for several (←/→ on that row, or a click). Your checkpoints are `[[check]]` tables in config.toml; Ctrl+S writes the shown one by name in place, keeping comments, off the UI thread (a failed save keeps the draft and toasts; `Saved. Runs after the next turn ends.` for a first command, else `Saved.`). Del removes its table. A repo's checkpoint is read-only here: its source reads `.maitty/checks.toml · <commit> · always | never | asks each time` and Del (`Forget choice`) sends `ForgetCheckApproval`. Further checkpoints are added in config.toml or the repo's file; there is no add screen. Run now (`r`, or its button) re-runs that checkpoint in the session with the latest result, else the focused or first agent of the project; a working agent queues it. Esc drops the draft.

## Project-specific: Inbox check groups

The Inbox (`j`) is part of the frame. Its title is `INBOX <n>`, n counting
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
  changed since`, `not checked: never for this repo`); a failure adds its
  first failing output line. A plain finish, and an item that waits on you without having asked
  through the app, say what the agent last said (`check_inbox.rs` `last_said`: `TermInfo.said`,
  else `TermInfo.last_lines`, the last lines of its screen for an agent whose hooks report no
  message): one line cut with `…`, and on the selected item up to four wrapped lines over its
  **Reply** `m` and **Open** `o` buttons. With nothing said, a finish is its head line alone (its
  heading already says finished) and a waiting item reads `waiting on you`. Several checkpoints read as one word first, then
  `build ✓ · test ✕`, or the glyph strip when that doesn't fit.
- The selected item is tinted across the sheet with `▎` (`err` for a failure, else `accent`); a
  failure keeps its red `▎` unselected. It grows its buttons, each a click of its key: question
  `Reply m` `Open o`; approval `Read the command ⏎`; failed `Output ⏎` `Re-run r` `Override… O`
  `Send to agent m` (the last, dropped where it doesn't fit);
  could not run `Output ⏎` `Settings ,`; stale `Re-run r` `Output ⏎`. Enter runs the first
  button (Read the command opens the approval); an item with no buttons opens its session. `o` opens the session of any item; `r`, `O`, `u`, `,` act on an
  item's check where they apply and are typed otherwise. 1–9 answer a question; d, M, x and m
  act on a finished item; d, m and k on a heads-up; m on a failure sends it (see *Sending a
  failure*); Delete marks seen.
- Typing finds any session; the `› query` row shows only once you type. The last row is
  `↑↓ select   ⏎ open   r re-run   O override   Esc close`.
- A session on another machine ends its head line with ` · on <machine>`, and a machine whose line
  is down has no items (*machines in the sidebar*).
- Existing theme slots only. Code lives in `client/screen/inbox.rs` (groups, drawing) and
  `client/screen/check_inbox.rs` (what each item says and its buttons).

## Project-specific: the frame

The layout and checkpoints are summed up in [DESIGN](DESIGN.md). The frame is [ADR 0013](adr/0013-v7-frame.md); `client/screen/frame_tests.rs` holds it to the reference frames, and `client/reference_frames_tests.rs` holds every preset's whole screen to them (see [TESTING](TESTING.md)).

- **Grid** (`draw.rs` `grid`): inside `ui.padding` (`framed`), no margin of its own. The sidebar is 40 columns from 200, 34 from 140, else 26 (a width you dragged wins), the full height. Floating leaves `ui.gap` between cards (with `gap = 0` borders sit side by side, never merged).
- **Cards** (`card.rs` `card`): `┌┐└┘`, or `╭╮╰╯` when `ui.rounded` is `full` or `corners`, `line`, the focused card's whole border in the accent. The title chip sits on the top border two in; the right-hand segments end a padding space before the corner. `ui.rounded = "full"` rounds pill ends (buttons, selected rows in modals and popovers, the answer bar, the toast); each cap sits on the ground around it (`card` in a modal or popover, the pane's `bg` in its body), not on the theme's `bg` throughout. The same `full` caps the tab chips (number and name as one shape, and the **+** chip) The chips' caps sit on the top border's own ground (no background); each chip is two cells wider and its hit rect includes them. A card's close (a plain `×`, red, over the border's corner cell) with `full` is a chip of its own (`card::close_width`): a space each side of the ✕ inside its caps, so the pointer's tint lights a whole chip, a blank cell clear of what is before it and of the border or the next zone's rule after it; square, it keeps the reference frames' place. A session card is one even block with square ends, selected or hovered: every line lit across the same columns, since a half-circle on every line would scallop and quarter-cell corners notch the block where its lines meet. With `round_tabs` off the chips are blocks, as before.
- **Tab row** (`draw.rs` `draw_tab_chips`, `grid`'s `tabs`; ADR 0028, ADR 0035): Floating's tabs are chips on one plain row level with the sidebar's top edge, over the pane boxes and outside them (ADR 0035): the boxes start on the row under it, for a lone pane and for splits alike; the side sheet keeps its full height beside them. Each tab leads with its most urgent agent's glyph (`tab_agent`: needs you first, then by state, read from the daemon's snapshot whether the tab is on screen or not; in the state's colour, in `acc_ink` on the current tab's accent), then a number chip and a name chip (the current one in the accent; a project with one tab has no number chip) and, on the open tab only, its `×` (`tab_close`: the tab while the project has several, else its only pane; a lone tab that is split has none, each pane keeping its own). The row lists every tab of the project in the sidebar's order (`session_tabs`; ADR 0031), named by `tab_label`: the tab's own name, else its focused session's, as the sidebar's tab lines say it. Those that don't fit drop, the **+** chip first of all (it never shows without every tab before it), then the other tabs, before and after the current one, which keeps its whole name while any other shows, and the current tab shortens its name rather than drop. A click picks a tab, a drag reorders, a middle click closes. Splitting and zooming are keys and the Menu, not buttons.
- **Pane edge** (`draw.rs` `card_frame`, ADR 0031): a pane has no title: a tab's only pane has a plain top border, and a split's panes are told apart by their edge alone, the tab naming where you are and showing its state (a click on the tab's glyph opens why). A pane's border is the accent on the focused pane (`sky()` in shortcut mode), `blocked` amber on one that needs you (focus wins), else `line`. The load a pane takes is in the bottom bar. A name you gave the tab stays on the tab (`1 build`, or `build` alone for a lone tab). The open tab has a ✕ (`tab_close`): it closes the tab, or a session's only tab's only pane; a pane closes with Alt+x, leader `x` or its menu. Closing asks first: the confirm reads `Close pane 2 · claude` (`Close pane · claude` for a lone pane, `Close panes` for a tab of several) over the pane's folder, and while it is up those panes' borders are `err`, kept bright over the dimmed screen (`draw.rs` `closing`, `menu.rs` `closing_lines`). In shortcut mode the focused pane's border is `sky()` in every look (with pane colours too); while a popup, modal or view an action opened holds the keys it is not. There is no frame round the window (ADR 0024).
- **Swapping panes** (`input.rs` `Drag::Pane`, `behaviour.rs` `swap_panes`): a press on a pane's top border focuses it and, in a tab of two or more panes, starts a drag; let go on another pane of the tab and the two trade places, each taking the other's size (the split's ratios stay with their slots). While dragging, the pane under the pointer lights its border in `accent` with a centred `⇄ swap here` pill, and a `btn` pill with the dragged pane's name follows the pointer (`draw.rs` `draw_pane_drag`). Let go on the same pane or outside the panes, move with no button held, or press Esc, and nothing moves. The same swap from the keys is leader `Alt`+arrows (`swap-left/right/up/down`, the neighbour that way, as `focus-*` finds it) and from the pane's right-click menu, "Move pane left/right/up/down" for each side that has a pane. The client swaps its own view at once and sends `Command::SwapPanes` only when the server's tab holds both panes (a split the server made, or restored), so the saved session keeps the new order.
- **Mode bar** (`mode_bar.rs`, ADR 0024): the bottom row of the room inside `ui.padding` (`draw.rs` `mode_bar_row`; `framed` is the room above it, so every look is a row shorter). Popups and their dimming sit above it (`above_bar`), so it is never covered or dimmed. Typing: ` TYPING ` (`muted` on `btn`) then `· Ctrl+Space for shortcuts` in `muted` on `bg`, the leader key from the keymap; a click on either starts shortcut mode as the leader does. Lit: the whole row in `sky()`, a bold ` SHORTCUT ` chip (`sky` on `bg`; a click opens the key map), then suggestions read from the keymap, key bold and label plain in `ink_on(sky)`: `n shell`, `s session`, `c changes`, `f files`, `i inbox`, `, settings`, then `? all keys` and `Esc type`. A rebound key shows its new key; an action with no key is left out. A narrow bar drops suggestions from the right, then Update shortens (` ↑ 0.44.0 `, ` ↑ `) and goes, then `? all keys`, then `Esc type`; the tag stays. A click on a suggestion presses its key in shortcut mode, so it does what the key does: the mode stays, unless the key starts a pane (ADR 0025). Who has the keys (anything but the panes, whether or not shortcut mode is held: the sidebar, a popup or modal, a side-panel view with the keys, the key map's search or All keys; `keys_holder`): the bar is on `bg`, never sky, and names it in a calm chip (`strong` bold on `btn`: ` SIDEBAR `, ` SETTINGS `, ` FILES `, ` INBOX `, ` CHANGES `, ` BRANCH `, ` KEY MAP `, ` CONFIRM `, ` NAME `, ` MESSAGE `, …), then its main keys and the way out, each after a `·`, key bold `strong`, words `muted`: `SIDEBAR · ↑↓ move · Enter open · Esc back`, `CONFIRM · y yes · n no`, `NAME · type · Enter save · Esc cancel`, `MESSAGE · Enter send · Shift+Enter new line · Esc keep draft`. The keys are the ones that thing's own handler reads in the state it is in (Files' tree, filter, preview or editor; Changes asking y/n; each step of Switch branch; Settings waiting for a key or editing a value; Settings' Check page by its row, while its command is typed, before the project has a checkpoint (a letter starts the command) and with no project open (only Tab and Esc), and the project check likewise; the Inbox asking to merge or throw away; copy mode's search and selection; the key map's second step; a folder picked for a setting, where Esc goes back); add a row there when a mode or view is added (the match is exhaustive). While shortcut mode is held it adds `· shortcut mode waits` before the way out. A narrow bar drops these hints from the right (`shortcut mode waits` first), then Update, then the way out; the label stays. The key map before a search stays lit, since its keys still run shortcuts. Under anything that has the keys (a popup or modal from either mode, a view shortcut mode opened, or the key map and its search) the bar is only words: its chips take no click, and a click on its row does nothing (`only_says`), so it never closes the popup, runs a suggestion or drops typed text. **Update** or **Restart**, when waiting, sits at the bar's right end, clickable except under anything that has the keys. With `ui.rounded = "full"` each chip on the bar is capped on the bar's ground and its click covers the caps. In shortcut mode a key with no action shows the toast `Shortcut mode — Esc to type` (`input.rs` `hint_dead_key`, ADR 0023), marked with a sky `i` and bar since it is information, not a success: at most once every 6 s, never over a toast still showing, and not kept in the notification history; keys that act are unchanged.
- **Sheet** (`sheet.rs` `split_for_sheet`): docked right from 140 columns, the layout's gap from the panes; below 140, or when the panes would keep fewer than 52 columns (`PANES_KEEP`), it takes the pane column and Esc brings the panes back. Its width is per kind of view and saved (`Saved::sheet_w`): Inbox, checks and both-diffs start at 72 columns, Changes at half the screen, Files at 55%, Branch at 30%, References at 40% (all less when the panes would keep fewer than 52, so about 70 at 160×45); never under 32 (`SHEET_MIN`). Files, Changes, Branch and References open with a tab strip on the card's first inside row (`sheet.rs` `draw_drawer_tabs`): `Files f · Changes c · Branch b · References a`, each with its first leader key from the keymap, the open one a pill (`pill`, so `ui.rounded` caps apply) in `accent` while the panel has the keys, else `btn`; each is a click target (`ScreenHit::DrawerTab`). Where the strip does not fit, the keys go, then the long names (`Files · Chg · Br · Refs`), so every tab stays whole. In Changes, `c` commits only after a y/n question, since `c` is also its tab's key; Tab answers no to an open question rather than switching. Tab / Shift+Tab move along it while one of these views has the keys, except while Files' filter or editor takes text; switching runs that tab's action, so it opens for the same checkout (`screen_target_dir`) at its own remembered width. The Inbox and the check sheets have no strip. A click into a pane gives that pane the keys and leaves the view open beside it, unlit (`view_set_aside`); a click in the sheet, or asking for a view again (`f`, `c`, `b`, `a`), gives the view the keys back. Drag its left edge (it lights up while pointed at), or press `[` `]` (4 columns) and `=` (default) while it has the keys; in the Inbox only while its search is empty, in Files not while its filter or editor takes text. **Files** (`f`, Ctrl+click on `path:line`) shows the tree beside the selected file; a sheet under 60 columns (`FILES_BOTH_FROM`) shows the tree, or the opened file with a `‹` back to the tree. A `.md` or `.markdown` file previews drawn (`markdown.rs`: headings bold `strong`, a `line` rule under the first level, fenced code on `card`, inline code on `card2`, lists, quotes behind a muted `│`, tables cut to the width, links with their URL muted after them), without line numbers since drawn lines are not lines of the file; `m` (shown in the action row as Raw or Pretty) switches that file to its text and back, `i` and `e` always edit the text, and Ctrl+click on `file.md:42` opens it as text at that line. `f` again or Esc in the tree closes it. A drag (or a double-click, for a word) over the file shown selects its text, drawn or raw (`FilesTree::pick`, cleared when the file, its look or its width changes), and Ctrl+C copies it; `y` stays Copy path. `i` edits the file in the preview (`views::Edit`): Shift with the arrows, Home/End or PgUp/PgDn, Ctrl+A, or a mouse drag in the preview select, drawn in copy mode's colour (`render::select_style`); a click moves the cursor (Shift+click extends), a double-click takes a word, and typing or deleting replaces the selection. Ctrl+C and Ctrl+X copy and cut the selection, or the cursor's whole line when nothing is selected; Ctrl+V reads the clipboard off the UI thread (`copy::clipboard_reader`; a failed read toasts) and pastes at the cursor, and the terminal's own paste (`Event::Paste`, also a key burst) lands there as text, never in the pane and never as an image paste. Ctrl+Z undoes and Ctrl+Y redoes, a run of typing or of deleting being one step; the history keeps whole snapshots, capped at 32 MB and 500 steps (`UNDO_BYTES`, `UNDO_STEPS`), and an undo marks the file unsaved even when it gets back to what is on disk. A typed run over a selection is one step with the replacing, and undoing a line cut puts the cursor back where it was; Ctrl+X in an empty file does nothing. The header says `Ctrl+S save · Esc done`. While editing, the action row holds the editor's own buttons instead of the file browser's (whose letters would be typed into the file): `^S Save · ^C Copy · ^V Paste · ^Z Undo · Esc Done`, each a `Btn::ViewChord` that runs that key. Clicks and highlights count wide characters (CJK, emoji) as two columns and tabs as four; a pointer below the last line means the end of the file. A paste while Files has the keys stays in Files: the editor takes it, the filter takes its first line, and otherwise it is dropped with a toast rather than sent to the pane behind; once a click in a pane sets Files aside, or Files closes, pastes (and the empty paste that means an image) go to the pane as before. Windows hands a paste over as keys read in one burst (`input.rs` `paste_from_burst`); while Files has the keys (nothing over it), any two different characters in one burst are a paste however short, so its letters never run commands; one character repeated (a held `j`) and typing ahead that ends in Enter stay keys. Other views, References included, keep a burst as keys, as the panes do. References takes a real paste itself; while Changes, Branch or a check sheet has the keys, a paste is dropped with a toast and never reaches the pane behind. Copying from the file shown gives tabs as the four spaces the preview reads them as.
- **Differences kept on purpose.** The app draws its own data where the reference frames have samples, and these on purpose: a finished Inbox item says what its agent last said where the reference says `finished · not seen`; Settings › Check marks the row the keys are on by its label in bold `accent` (the reference draws every label dim and has no row cursor); evidence output colours only `✓` and failure lines (the reference also dims `>` command echoes, `❯ file:line` and `Duration`, and colours an assertion's continuation as a failure); the evidence Source row leaves out when you chose never (`you chose never 14:31`; the time of a choice is not shown); and the several-checkpoints words add `could not run: <name>` and `checking: <name>` to the reference's four (`failed`, `needs you`, `next`, `all passed`), because a checkpoint that could not run is not a failure and checking must read apart from next. A re-run keeps the sheet or Inbox it was started from open (the reference closes it), and its result is told by the daemon's notices (`<place>: check failed`) and the row, not by the reference's `test passed (exit 0, 13.9s). fix-footer reads done.` / `test failed (exit 1). fix-footer is in the Inbox.` toasts.
- **Message popover** (`popover.rs` `draw_compose_pop`): the follow-up to an agent, opened from the sidebar row or the leader (`M`). The screen is dimmed except the popover and its row. A full `accent` border in every layout (corners by `ui.rounded`), `card` ground, the input on `bg`, no shadow. Width `min(68, room right of the sidebar - 1)`. Title chip `MESSAGE <session> · <agent>`, with `● needs you` (the only amber), `working` or `done` at the right. Quote: the tail of the agent's last message, at most 4 / 3 / 2 lines at 160x45 / 100x30 / 80x24, labelled `<agent> asked` (blocked) or `<agent> said` plus `· last n of m lines` when cut, `muted` `▏` gutter, `text` words, not italic. Input: one row, growing a row per line to 6 / 4 / 3, then the top row reads `↑ n more lines`. Placement: top border a line above the row with `◂` on the row's first line; no room below, it opens upward with its bottom border just under the row; neither fits, pinned to the bottom. Hints are keycaps. Sending closes it and the row's second line carries the receipt (`receipts` on `Screen`): `◷ queued "…"` in `muted` until the server hands the follow-up to the agent, then `› sent "…"` in `accent`; it clears after `RECEIPT_MAX_SHOWN`, or on a status change once `RECEIPT_MIN_SHOWN` has passed. No toast.
- **Toast** (`draw_toast`): one component for every Maitty notice, bottom right, its bottom edge `TOAST_ABOVE_INPUT` (5) rows above the screen's so it sits over the agent's input area. It draws a `line` outline on `bg` with round corners whatever `ui.rounded` says (`popups.rs` `panel_box`), its left side between the corners a heavy `┃` (ADR 0030). The `┃` is in the notice's colour (error `err`, about a session that needs you `blocked`, about a finished one `done`, otherwise `accent`; amber only for needs-you). The bold `strong` title starts with the mark and has a `✕` at the right; the detail follows in `text`, then an optional preview in `muted` italics. Each wraps at the toast's settled width (`popover::wrap_capped`, so the text does not reflow during the slide-in): the title and detail on up to 3 lines (`TOAST_TITLE_LINES`, `TOAST_DETAIL_LINES`), the preview on up to 2 (`TOAST_PREVIEW_LINES`), the last ending in `…` only past that, the box's height following its lines and the limit shrinking to what the window can hold. The update toast's button is a single row. `App::notify(msg, error)` makes a one-row toast whose title is the message; `App::notify_card(title, detail, preview, error)` adds the detail and preview rows. A plain toast clears after `TOAST_CLEAR` (2.4 s); errors (4.5 s), a note about a session (6 s) and the update toast with its **Restart now** button (12 s) stay longer. The `✕` or Esc clears it at once (Esc still reaches the pane). With motion on it slides in and fades out.
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

Centred, no bottom bar, in the
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

**The splash** (`client/screen/splash_wide.rs`) is drawn on the plain ground, with no box round
the screen, no chips on its edge, no row of key hints and no version line. The wordmark, swell
and tagline as above; under them two boxes side by side, **START** (64 columns: the three choices
as 3-row highlights with the sub-text and a key chip on the middle row, then a `RECENT PROJECTS`
rule and up to five projects with a machine badge and `⎇ branch`) and **WHILE YOU WERE AWAY**
(72 columns: `●n ✻n ✓n` chips on its top edge, up to five agents, the ones that need you first,
each two rows: glyph, name, machine badge, project, `● you` or its age, then what it asks or does;
a `MACHINES` rule with each machine's badge and dot). The box that has the keys has the accent
border and title chip. Tab switches box, the arrows walk the one that has them, Enter opens: a
choice, a project (the splash goes, the project is unfolded and selected in the sidebar) or a
session. A waiting release is an accent chip at the right of START's top edge at every size (`u`
or a click acts).

It shows less as the terminal gets smaller, and is the splash at every size that holds the START
box (66 columns, 22 rows). Too narrow for both boxes (under 140 columns): START alone, and what
the away box would say is the counts line over it (`● 1 need you   ✻ 2 still working   ✓ 1
finished`), so who needs you is never dropped. Too short for the wordmark over the boxes: the
boxes alone; the wordmark gives way before the counts do. Only a terminal that cannot hold the
START box keeps the cards above.

The intro, at 50 ms a frame: letter k rises from the deep colour to its own from frame 3k over 8
frames (eased out), nothing moving; the swell fades in over 14 frames and rolls a step every 3; the
tagline fades in over frames 10–20. Cards and hints are drawn from the first frame and keys act at
once. `ui.splash_animation` (Settings › Appearance): `cold start` (default: the first start of a
version, or nothing left running), `always`, `never`. Motion off, or a remote session, never plays
it and holds the swell still.

## Project-specific: collapsed sidebar

The sidebar has three states (`App::sidebar`, `App::side_hidden`): full, **collapsed** and **hidden**.
Leader `z` (`Action::ToggleSidebar`) swaps full and collapsed; Hide sidebar (`Action::HideSidebar`, search only) hides
the sessions or brings the sidebar back. The `»` chip (`«` with the sidebar on the right) and the `…` row
restore it. There are no bare-letter keys for this in the sidebar.

- **Collapsed** (`screen/rail.rs`, ADR 0033): a box `RAIL_W` (5) wide down the sidebar's side; the panes take the rest.
  With `ui.rounded = full` its chips and its selected row are pills (the glyph between two caps), as the open sidebar's are.
  A `»` chip (`«` with the sidebar on the right) is set into its top edge; under it, in amber, how many sessions need
  you (left out at none); a blank row; this computer's `⌂` mark; then one state glyph per session (the glyph and
  colour of its card), a `─` between projects. A rule `├───┤` joins the box where the other machines begin, and each
  has a `⇄` mark, dim while it is down. The chosen session has the `card2` ground and, while the column has the keys, an
  accent `▌` at its left. At the foot a `+` chip starts a shell and a `≡` chip opens the Menu. Rows that don't fit scroll,
  the selected one kept in view, and a `…` in the last place opens the sidebar. Each glyph is a click target that opens
  its session. The full sidebar has a `«` chip on its top border at the right that collapses it; `[` while the sidebar
  has the keys cycles full, collapsed, hidden. Hidden, a `»` chip leads the tabs on the terminal's top border.
  `ui.rounded` (Settings › Appearance › Rounded: `full`, `corners` or `off`, off by default) is the one shape setting. `corners` rounds every box's corners only. `full` rounds them too, and caps every chip (tabs, the LOCAL and REMOTE titles, card titles, + New, Menu, buttons, menu rows) on whatever ground it sits, the pointer's hover, and rounds the right end of a selected or failed sidebar row (a session's, a tab's or a project's): the row runs from the left edge, where the accent bar stands in the column of the project's line and is never rounded, to a half-circle cap at the right for one line, a square end for more so its lines read as one even block, never a cap per line; what is right-aligned stops a cell short of the cap. The Settings tab strip carries no update dot: the mode bar's pill and the splash announce updates, and the version lives in one row, General › UPDATES › Version (`maitty 0.42.0 · up to date · u check now`, or `9.9.9 is out · Enter to update`; Enter or a click acts, `u` checks from any tab). Above the pages, only what is off shows: an installed or daemon version that differs, or a pending restart with Restart now.
- **Column focus** is the sidebar focus with the sidebar collapsed (leader Focus the sidebar). `↑` `↓`, `←` `→` and `1`–`9` switch
  sessions, opening each in the pane while the keys stay on the column; `1`–`9` mean nothing elsewhere. Enter focuses the pane, Esc leaves.
- **Hidden**: no sessions and no strip; the panes take everything. Reach them again with leader `z` or `Z`.

A session an agent started (a helper) steps in two columns under the session that started it, to two levels. A row asking to start helpers keeps its second line (`wants to start N helpers`), which gives way last.

## Project-specific: machines in the sidebar

Code: `client/screen/machine_group.rs` (header, notes, grey), `side.rs` (`Line::Machine`,
`Line::MachineNote`), `rail.rs`, `check_inbox.rs`. Look: [DESIGN](DESIGN.md#sidebar).

- **Local and Remote.** This computer's sessions are always the sidebar's Local half and other
  machines' the Remote half (ADR 0050), whether or not any machine is known; a window with none
  says so in Remote and offers to add one. There are no machine headers: a card's badge names its
  machine. A machine nobody has heard from yet is a two-row item that says `connecting…`; it never
  leaves a gap.
- **Order.** This computer's cards (in the order you dragged them) in Local; in Remote each other
  machine in id order, its cards in the saved order of its own machine. A project and a card drag only within
  their machine.
- **Status is the daemon's word.** A remote card is drawn by the same code from the same `Status`
  its daemon reported; nothing is recomputed or translated here (non-negotiable 5).
- **Down.** `ConnectionState::is_down` (reconnecting, needs login, needs update, unreachable)
  turns the machine into one two-row item and its cards are not drawn (`Session::down` still drops
  them from the Inbox groups); connecting shows the same item. How long it has been down is
  `Remote::down_since`, kept across the failure states.
- **Per-machine values.** Limits, today's spend and the daemon version come from that machine's
  snapshot and are shown in the Machines dialog, never summed.
- **Narrow widths.** Every line is cut to the sidebar with display-width-aware `…`; the machine's
  badge shortens before its connection word, and the name keeps eight cells.
- **Inbox.** A remote item ends its head line with ` · on <machine>`; the sort across machines is the
  existing one (longest waiting first) with this computer's before another's at the same moment.
  Its keys reach that machine's daemon (`ClientTerm.machine`), and the local-only features say so as
  they do for any remote pane.
- **Alerts.** A remote agent that starts needing you or finishes is added to the notification
  history named by its machine. Its toast and sound still wait for that machine's `notices` and
  `sounds` opt-ins.

## Project-specific: Machines dialog

Code: `client/screen/machines.rs` (the view, its keys and its drawing), `client/machine_flow.rs` (what a
choice does), `client/machine_config.rs` (where `config.local.toml` is; the `[[machines]]` entry edits are in `maitty-machines`),
`maitty-machines/src/machine_probe.rs` (the look at a machine). Opened by leader `w` or the command palette
(`Machines`); `Mode::Machines`.

- **One input, typed like scp.** `ADD MACHINE`: `Where's the machine? Type it like scp or ssh.`, the
  input (`› user@host`, on `card2`) and under it a parse line, `user` and `host` each a faint label and a
  bold value, a faint `—` while unknown. A host that is a `Host` of `~/.ssh/config` fills its user and host
  name. Under it, `FROM ~/.ssh/config` lists the hosts (read off the UI thread; `loading hosts from
  ~/.ssh/config…`, then `alias  user@hostname`), filtered by the host part of the input; a host that is
  already a machine is not offered. The keys start in the input (the cursor `█`); `↓` (or `Tab`) walks into the list, where the chosen
  host is marked with `⏎ connect`, and `↑` from its first row (or typing) is back in the input.
  Keys: typing, `↑↓`, `Enter` connects (to the typed text, or the chosen host), `Esc` cancels. A
  `:path` after the host is refused for now (opening a folder after connecting is not built).
- **Your machines.** With a machine configured, `YOUR MACHINES` lists them above the input, by name: ` ⇄ build  ci@build` and at the right how its connection stands (`✕ needs update`, in the state's colour). `↑` from the input walks onto the list and `↓` from its last row is back in the input; a click on a machine's line in the sidebar opens the dialog with that machine chosen. Under the chosen one is what to do about it, in full (the fix text, `<name>` filled in). On it: `r` (or Enter) reconnects, `u` updates, `e` edits where it is, `x` (or Delete) removes; the title is `MACHINES` while the keys are there. The same three are in the machine's right-click menu in the sidebar (see below).
- **Windows.** The dialog says, in a dim line at its top and on Windows only, `Windows: use ssh keys or an
  agent. Password prompts work in the popup but are not remembered.`
- **Adding is one step for a key-based host.** Enter on a host row (or on the typed text) first refuses a
  target ssh would read as an option or that holds whitespace (the loader's own words, shown under the lists;
  nothing runs and nothing is written). The name is the alias, made valid and unique (`build`, then
  `build-2`); when nothing usable is left of the target, a name step asks for one and shows the loader's
  reason inline. The entry is `name` and `target` only, and `target` is the alias, so ssh's own config for
  it (user, port, key, jump host) still applies. Then the machine is looked at, off the UI thread, with `ssh
  <target> maitty --version` that never asks anything (`BatchMode`, a short `ConnectTimeout`, stdin closed;
  `Checking…`, `Esc` ends ssh and adds nothing, and a limit of 12 s ends it whatever ssh does).
  - *It answers:* the entry is appended to `config.local.toml` (comments and the rest of the file kept,
    written atomically, never `config.toml`), the machine's supervisor starts, and its group is in the
    sidebar as `connecting` at once.
  - *ssh needs a person* (a password, a code, an unknown host key): the entry is written, and ssh is run
    **once**, interactively, in a popup pane on this computer's daemon (`Command::Popup`), so the prompts can
    be answered. The popup line is the probe's without `BatchMode`, with the same hardening (`ForwardAgent=no`,
    `-a`, `ConnectTimeout`, `--` before the target). When the popup pane closes the machine is dialled with
    `BatchMode` as every connection is; whether the login worked is the connection's to say, and if it did
    not the machine shows `needs login` with the fix. A popup has the keys only in plain typing mode, so
    the dialog closes when it asks for the popup and a toast says `ssh needs you to log in: use the popup`;
    the keys then go to ssh. A popup that is never seen (it ended between two reports of the daemon) is
    taken as closed after 20 s. A `remote_cmd` that needs quoting, or an `ssh_args` word that holds a quote, cannot
    be put on a pane's command line safely: the machine is added and the dialog says to run `ssh` once in a
    terminal.
  - *The app is missing, old or not running there:* the machine is added; its state shows `needs update`
    or `unreachable` with the exact fix text.
  - *ssh cannot reach it* (a bad host): ssh's last line, cleaned of control characters and cut, is shown
    under the lists, and nothing is added. Nothing hangs: every look ends at its limit, on `Esc`, or when
    ssh does.
  A Windows remote is not supported in v1; the dialog does not guess one from the probe.
- **Checklist.** After Enter the dialog shows the steps connecting takes: `reach host`, `verify host
  key`, `sign in as user`, `maitty-daemon`, with `✓` done, a spinner while running, `●` (amber) waiting on you,
  `○` pending and `✕` failed. While ssh is being asked, the first step runs. When the machine is written the
  title becomes `MACHINE ADDED` and the steps show how it stands: all `✓` with the daemon's version; `●` at
  the sign-in with `log in in the popup`; or `✕` at `maitty-daemon` with `the connection shows the fix`.
  `Enter` or `Esc` closes. The host key, password and daemon install are still asked by ssh in its popup,
  not inline.
- **Reconnect**, **Update…**, **Edit…** and **Remove** are in the menu of a right-click on either row of a remote machine in the sidebar
  (the machine's name as its title, `Remove` last), and on the dialog's keys. Reconnect dials it now and clears a refusal; a machine
  with no supervisor gets one. Update runs `maitty machine update <name>` (with `--install` when the app is not there at all) in a popup pane on this computer, where it says what it found and asks before it installs; the machine is dialled again when the popup closes. Edit asks `Where is build? Type it like scp or ssh.` with the address it has now in the field: Enter writes the new `target` into its entry in `config.local.toml` (its name, its other keys and the rest of the file kept), lets the old connection go, forgets the daemon id recorded for the name and dials the new address at once; a target the loader refuses is said under the field and nothing is written. The name is not edited: its saved layout is kept under it. Remove says `Removes build from this list. Its sessions keep running on
  build.` in a toast. It deletes the entry from `config.local.toml`, stops the supervisor and the connection, takes the machine out
  of the sidebar and the window's records, and forgets the daemon id recorded for the name. The machine, its
  daemon, its sessions and its saved layout (kept under its name) are not touched.
- **A refused machine** (a different daemon answers at its name) shows `refused` with the reason and the
  fix: remove it and add it again, or fix the host name or DNS and reconnect.
- Adding and removing take effect in the running window. Editing the file by hand still takes effect at the
  next start.

## Project-specific: Hairline

Not applicable: the Hairline look was removed ([ADR 0039](adr/0039-floating-is-the-only-look.md)); Floating is the only look.

## Project-specific: features that need a pane's files

Files, Changes, Find, Branches, References, connecting an agent, and worktree
actions read or change the files of the pane they apply to. (Pasting an image is not on this list in
the desktop app: it works for panes on other machines, see below. The terminal app's own list is
unchanged.) For a pane on another machine they do
nothing but show one error toast, always in this form: "<Feature> is local-only in v1: the files
are on <machine>, not on this computer." The check is per pane: moving the focus or the sidebar
selection between a local and a remote pane turns the feature off and on. Menu entries that would
run git in a remote pane's folder (Changes, Snapshots, Make it a git repo) are left out of that
pane's menu instead.

**Dropping files on a pane** (`features/terminal/dropRoute.ts`, `useFileSends.tsx`, host `file_send.rs`,
ADR 0055). On a pane of this computer the dropped paths are typed into it, quoted for its shell. On a
pane of another machine a path here names nothing there, so the files are sent to that machine over its ssh
line (regular files only, 100 MB each, at most 10 at a time; more are refused whole with one error toast,
never cut) and the path each one is kept at is typed into the pane as it lands. A machine that is not
`connected` is refused with one error toast. The send runs on the host's background worker, one at a time. A
progress toast (`Sending a.log to build (2 of 3)`, a named progress bar with the percentage, and a
**Cancel** button) stays until the host reports the end; it has no close button, because hiding it would not
stop the send. It ends as `Sent 3 files to build`, an error toast with what ssh said (the files after the
failed one are not tried), or `Stopped sending to build`. Cancel cuts the file now going and tries no other.
What the machine reports is untrusted: the window types a path only if it is absolute, at most 4096
characters and free of control and invisible characters, and a path that is not typed is named in the final
toast instead.

**Pasting an image into a pane** (`features/terminal/clipboardImage.ts`, `useImagePaste.ts`, `api/imagePaste.ts`,
host `image_paste.rs`, ADR 0049 and the amendment to 0055). Ctrl+Shift+V on Windows and Linux, or Cmd+V on macOS, pastes through the
webview's paste event, whose clipboard holds text, an image or both. **Text comes first**: any text and the
terminal pastes it as it always has (bracketed paste included); only an image with no text becomes an image
paste. Plain Ctrl+V on Windows and Linux is never taken: like every plain Ctrl+letter there, it goes to the program
in the pane as a raw `^V` (so Claude Code's own clipboard handling and vim's `^V` work). The chat composer takes any
paste chord, since its paste event is not behind a terminal. PNG, JPEG, GIF and WebP are taken (the host checks the
first bytes, not the name the webview gives); another image type is refused with one error toast. At most 25 MB,
refused before the image is read. On a pane of this computer the host keeps the image in its private cache
(`pastes` in the app's cache folder, mode 600, cleared after a week) and the path is typed into the pane, quoted
for the shell. On a pane of another machine the image is sent as a dropped file is, by the same runner and events:
the same progress toast (`Sending the image to build`, progress bar, **Cancel**), the same error toasts, and the
far-side path typed only if it is absolute, at most 4096 characters and free of control and invisible characters.
A machine that is not `connected`, one reached by a custom transport (`via`), or an unknown one is refused with one
error toast, in the words used for dropped files. The image never goes through an event and is never logged: it is
the raw body of the command.

## Project-specific: frame presentation

Wrap each outer terminal frame in synchronized-update markers; end the update
even when drawing fails. An unfinished child update from the previous frame
must not delay switching to another session.

Keep pane padding and the card interior on the same surface as a terminal app
when at least three screen corners use one explicit background color. Preserve
the pane insets; isolated colored panels keep their existing theme treatment.
Preserve colors applications paint explicitly.

## Project-specific: desktop chat view

The desktop app's chat view is a second view of a Claude Code or Codex session, never a second session
([chat view plan](features/chat-gui/chat-view-plan.md), ADR 0036). Rules:

- **Source.** It renders only the daemon's structured events, through `features/chat/` (the projection).
  It never reads terminal output. A session has a chat only when `conversationsFromSnapshot` found a daemon
  conversation whose native terminal is that pane (`Conversation.chat`); `model/chatView.ts` decides
  availability: `ready`, `no-events`, `remote` (local-only in v1), `unsupported` (another agent) or `shell`.
  The daemon gives a Claude or Codex agent started in a pane a **read-only** conversation that follows the agent's
  own transcript ([ADR 0052](adr/0052-an-agent-in-a-pane-gets-a-read-only-conversation.md)); its link has
  `readOnly: true`, and the conversation is not listed as a session of its own (the pane is the session).
- **A chat session is the other kind of session** ([ADR 0054](adr/0054-a-chat-session-is-a-daemon-owned-conversation-started-from-the-desktop.md)):
  a structured conversation the daemon started (`Conversation.execution` is `Structured`, `chat` is set without
  `readOnly`, `native_term` is none; `model/chatSession.ts` `isChatSession`). It is a sidebar row, tab and pane of its
  own, grouped by its repository (`ConversationSummary.root`, `branch`). It has **no terminal**: the pane is the chat
  always, there is no Terminal | Chat switch, `chat-toggle` does nothing, the terminal slot holds a one-line note
  (`NoTerminalNote`), and nothing that needs a pane number (find, restart, paste, broadcast, typed answers, the
  worktree menu, Inspect) is offered for it. Its composer is on (send, queue with Remove, Stop, attachments); an
  approval is answered by request id (y / a / n) and a user-input request by a form (`UserInputForm`), so neither
  points to a terminal. It starts from **New agent on a branch** with **Session: Chat** (Terminal is the
  default) or from a shell's empty chat card (**Start a Claude chat** / **Start a Codex chat**). Closing it stops
  the agent (`closeChatSession`: end, then put away); if the stop fails it stays open. The read-only chat that
  follows an agent typed in a shell keeps its pane, its switch and its disabled composer.
- **Switching changes the window, not the session.** The choice is per session (`usePaneViews`), the terminal stays
  mounted and its pane keeps its size, and no daemon command is sent. The shortcut is `chat-toggle`
  (Ctrl+Shift+E, Cmd+Shift+E) on the focused pane.
- **Keyboard.** While the chat covers a pane its terminal is not focused (`TerminalView focused` is false) and is
  hidden from Tab and screen readers. The chat adds no layer to the Esc ladder.
- **History.** `useChatFeed` follows one conversation while its chat is open (`chatFeed.ts` is the pure core). The
  first read pages the whole history; later polls (1 s while the agent works or waits on a person, 3 s otherwise,
  none while the window is hidden) read from the daemon's `resume` cursor, which returns only the entries added or
  changed since, so a 1,000-entry conversation costs one small read per poll. A page that says `reset` (the history was
  rebuilt, or the conversation started over) replaces what the feed held. A failed read, or a daemon that predates
  `resume`, falls back to the old way: ask again for the last page read, or read from the start when that cursor is
  stale. Entries already applied unchanged are skipped, so nothing is shown twice and unchanged events stay the same
  objects. A new generation shows a fresh projection. Pending approvals come with every page and go to `applyApprovals`. The last
  feed per conversation is kept (8 at most), so reopening a chat shows it at once.
- **List.** `VirtualList` draws only the rows near the view (`transform` placement, measured heights, stable event
  ids as keys), follows the newest row until the reader scrolls away, then offers "Jump to the latest message". Rows
  are `buildRows(events)`: one agent message per turn (adjacent agent messages of a turn are joined; a step between
  two messages keeps their order), user bubbles, step cards, approvals, turn dividers, and the working line last.
- **Cards** (`features/chat/components/`). Every fact the history carries is shown, and a missing one is said to be
  unavailable (a diff the history did not keep, a checklist without items, a command time or exit code the provider
  does not report, a turn without the daemon's times shows "Turn finished", not an invented duration). Only a step
  that is running animates (its spinner).
  - *Tool chip:* a 28px mono chip with the tool's name and what it acted on (read from the tool's input; Codex: its
    label); click opens the input and the result, bounded.
  - *Edit block:* header (pencil, verb, file relative to the session's folder, `+N` `-N`), the diff with tinted
    lines (theme tokens at 10%), folded past 12 lines with "N more lines" / "Show less". The header opens the file's
    diff in the session panel when the window has one.
  - *Command card:* the command, then `Running · m:ss` (live) or the elapsed time and an `exit N` pill, and the last
    3 lines of output with "Show all N lines". The elapsed time is the provider's duration, else the daemon's start
    and completion times; the exit code exists for Codex only, so for Claude it reads "exit code unavailable".
  - *Approval card:* see Approvals. *Helper agents:* consecutive helper calls are one card, "3 agents, 2 working",
    a row each (title, type, elapsed or "Done · 5m 30s"); a row opens what it was asked and its report. *Checklist:*
    the agent's own to-do items with a progress bar. *Working line:* "claude is working · m:ss"; no token count,
    because the daemon's history carries none.
  - *Markdown* (`markdown.ts`, `components/Markdown.tsx`): an agent message is read into a data tree and drawn as
    elements. There is no HTML anywhere (no `dangerouslySetInnerHTML`), raw tags are text, images are never loaded
    (their alt text shows), and a link is kept only for an `http(s)` address. The app has no route that opens an
    address outside the window, so a link is a button that copies its address; it never navigates the webview.
    Supported: paragraphs, headings, fenced code, lists, quotes, rules, inline code, bold, italic.
  - *Rows* are memoised on their events (`ChatItem`), and the virtual list measures heights, so a card that expands
    resizes its own row only.
- **Composer.** `useChatComposer` sends through the conversation's own send (`send_conversation`) with an operation
  number (the receipt's plus one; a failed send is retried with the same number so the daemon can tell a retry from a
  new message). Enter sends, Shift+Enter adds a line (not while an input method is composing), Esc or Stop interrupts
  the running turn (`cancel_conversation`). It is off, with the reason as its placeholder, while an approval is
  pending ("Answer the request above first") or the daemon is offline. While the daemon reports the turn working,
  Enter still sends: the daemon holds the message (`ReadResult.queued`) and delivers it when the turn ends, in order
  and once each, so it is shown as a dashed pending bubble with Remove (`dequeue_conversation`) for as long as the
  daemon holds it, and closing the window loses nothing. The queue is the daemon's memory: a daemon restart drops it,
  and a message queued for a conversation that has since started over is dropped, not sent into the new one. The
  boundary is the end of the turn, the one the daemon can observe for both providers; a message is never delivered
  into a running turn. A send that the daemon refuses as `stale or reused` (another window sent in between) is read
  and sent again under a new number; a retry of any other failure keeps its number.
- **Attachments** (ticket 20, ADR 0053). A paperclip left of the text box (tooltip and `aria-label` "Attach files",
  or "Attach files: <reason>" while the composer is off; `aria-disabled`, not `disabled`, so the reason can be read),
  a drop on the composer, or a bare image pasted into the box adds files to the draft; a paste that holds any text
  stays a text paste. The picker is the dialog plugin's `open` (`api/attachments.chooseFilesToAttach`); a drop
  reaches the composer through `chat/composerDrop.ts`, which `DesktopApp`'s one file-drop handler asks before it
  routes the drop to a pane. `useAttachmentIntake` is the only code that adds or removes a draft file: at most 10 per
  message (a drop over the limit is refused whole), a pasted image over 25 MB is refused before it is read, the
  host (`attach_local_file`, `attachment.rs`) checks everything else and returns the cleaned name that is shown.
  Draft files live in `composerStore` beside the text, so switching views loses nothing. Each is a chip (kind icon,
  name, size, a Remove button) or, for a pasted image, a thumbnail made from the bytes the window already holds
  (an object URL, revoked on remove or send). Sending builds the text with `buildAttachmentMessage` (the text, then a
  fixed block naming each kept file by path); a message may be only files. A failed send gives text and files back.
  A sent message is shown by `parseAttachmentMessage` as its text and chips (icon and name; "no longer available"
  when `attachments_exist` says the kept file was swept). The composer takes a `machine` (the chat link's):
  for another machine the same picker, drop and paste send the file there (`sendAttachments` over `send_files_to_machine` /
  `send_pasted_image`, with the terminal's progress toast and Cancel) and the chip holds the far-side path, checked
  before it is used; see ADR 0053 point 8. No chat sets a machine yet. A read-only chat shows no paperclip.
- **Read-only chat.** For a link with `readOnly` the composer is off with "Read-only: talk to the agent in the
  terminal" as its placeholder, and offers no Stop or queue wording; the daemon refuses send, approve and cancel
  for such a conversation. The history and status still follow the agent.
- **Approvals.** The card answers through `approve_conversation` by request id and then reads "You answered: ...",
  with the buttons gone. While it is pending and focused, `y` (once), `a` (this session) and `n` (no) answer it, the
  same letters as typing into the terminal, which stays a working fallback. The daemon's `answered_approvals` are
  read with every page, so a request answered in the terminal or in another window shows as answered with the real
  choice, even when this chat was opened afterwards. The card names the tool step it is about (`item_id`) and its
  "For: ..." link scrolls the list to that step. A request that asks the person a question (`UserInput`) is answered
  in the terminal.
- **Screen readers.** The history is a labelled region, not a live region. A separate polite status line says only
  that the agent is working, is waiting for an answer, or finished.
- **Styling.** Tailwind utilities on theme tokens only; the keyframes for the view change live in `workspace.css`.

## Project-specific: desktop session panel

The panel beside the panes for the focused agent session ([chat view plan](features/chat-gui/chat-view-plan.md),
v18 C.2). It reads the same daemon conversation as the chat, through `useChatFeed`, so it works in the terminal
view too. Local sessions only.

- **Toggle.** `Ctrl/Cmd+\` (`session-panel`), or the panel button in a pane head beside the Terminal | Chat
  switch. Both flip the one window-level state in `features/panel/useSessionPanelState.ts`.
- **Placement.** A window of 1100px or more (`COMPACT_WIDTH_PX`) docks it: a 320px floating card (`mx-card`)
  beside the panes, which shrink; the choice is remembered (`maitty.gui.sessionPanel.v1`). Under 1100px it is an
  overlay from the window edge over a scrim, closed by default and not remembered, closed by the scrim, Hide, or
  Esc (`session-panel` in `model/escLadder.ts`, below Settings and above the sidebar drawer). It sits on the side
  opposite the sidebar. A docked panel offsets the Inspect drawer by `--session-panel-w`, so both can be open.
  The panel's width does not animate (a layout property); Reduce motion changes nothing for it.
- **Content.** `panelModel.ts` derives everything from the chat projection and adds nothing: the **Checklist** is
  the agent's last to-do list that carried its items ("Checklist · 2 of 5", a 3px progress bar, the current item
  with a spinner and its running time only while the agent works; the time is that of the first list in an
  unbroken run that named the item current), and **Agents** is every helper call ("Agents · 2 working" or "all
  done"). A session with no agent, no event stream, or on another machine shows "Nothing to track yet" and one
  sentence saying why (`emptyReason`); an agent with neither list nor helpers says so too.
- **Changes** (`ChangesSegment.tsx`, `DiffView.tsx`). The agent's worktree through `conversation_diff` (read every
  4 s while the panel is open, and at once when the agent edits a file): the branch name from the session list,
  then one list of changed files (directory dim, file name, a comment badge, `+a` `-d`). The daemon's diff does
  not say which files are staged, how far the branch is ahead or behind, or whether it was pushed, so none of that
  is shown. A file opens its diff in the panel, which widens to 520px: back arrow, the path truncated from the
  start, `+a -d`, then numbered lines (the new side's numbers, from the hunk headers; a removed line has none and
  takes no comment). Hovering a line shows a **+**; it opens a composer under the line ("Tell claude what to change
  here...", Enter or Ctrl/Cmd+Enter sends, Shift+Enter adds a line, Cancel or Esc closes it; Esc is the top layer
  of the ladder, `comment-composer`). Diffs over 1,500 lines draw in steps. An edit block's header opens its file
  here (`panelCommands.ts`), opening the panel first.
- **Line comments.** The daemon's send takes text only, so a comment goes as one message,
  `Comment on <file>:<line>`, the line's code quoted, a blank line, the text (`chat/commentMessage.ts`); the chat
  draws that message as a comment bubble. It is sent to the daemon as an ordinary message (`useCommentDelivery`): while the agent works the daemon holds it and delivers it when the turn ends, whether or not the chat is open. The panel holds it itself, and sends it in order afterwards, while an approval waits or the daemon is offline; the note under the composer says which. It takes its operation number from the same per-conversation counter as the chat's composer (`chat/sendOperation.ts`), reuses it on a retry, and takes a new one after a "stale or reused" refusal. A read-only chat cannot take comments: the + buttons are disabled with "Read-only: talk to the agent in the terminal". A line with comments shows a count pill whose label lists them, and
  the file row a badge. The written comments are the window's memory only (`commentStore.ts`).
- **Files and Artifacts** (`FilesSegment.tsx`, `ArtifactsSegment.tsx`, `artifacts.ts`). The segment bar is
  Changes, Files, Artifacts, each with a count. **Files** lists what the agent's tool calls named, in the order it
  first touched them: Edited (amber), Created (green), Read (dim), with `+a -d` for edits. An edited or created file
  that the worktree still has a diff for opens that diff; any other (a file only read, or one committed since) opens
  in the Inspect drawer's Files view. Claude's `Write` does not say whether it created the file, but Claude Code
  will not overwrite a file it has not read, so a `Write` to an untouched path counts as a new file (Codex says
  `add`). **Artifacts** are the created files that are documents, reports, data or pictures (by extension: md, mdx,
  txt, rst, html, json, csv, xml, log, png, jpg, gif, webp, svg); the daemon has no artifact concept. A card has a
  tinted tile, the file name and a subtitle. Opening one reads it with `conversation_file` (UTF-8 text, size
  limited by the daemon, path relative to the session's folder) and shows it as text, markdown drawn through the
  safe renderer; a picture cannot be shown because files arrive as text, so it offers the Files drawer. The
  daemon's `conversation_files` listing is not used: the panel shows what the session touched, not the project tree.
- **Sidebar progress.** A session row shows the agent's checklist under the name, "2 of 5 · Write the fix" and a 2px
  bar (`progressStore.ts`). The daemon reports no per-session progress and the sidebar must not poll every
  conversation, so the row appears only for a session whose conversation this window has read (its chat or its
  panel was open) and shows the last list read; a session never opened has no such row.
- **Cost.** The panel reads the conversation with its own `useChatFeed`, so an open chat and an open panel each
  poll (a read of the last page, 1 s while the agent works). The panel polls the worktree diff every 4 s and
  stops while the window is hidden.
