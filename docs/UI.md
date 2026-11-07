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
chat or a terminal session from the moment it starts,
and nothing switches a live session between them. The desktop app shows terminal sessions
only: a new session there is a plain shell, closing a pane or tab detaches, and ending a
session is its own action (its look is in [DESIGN](DESIGN.md#desktop-app)).

The desktop app also shows other machines' terminal sessions (ADR 0007).
Its host holds one supervised connection per machine and the web view only renders: it knows a machine by
name, never by where it points. A session of another machine has the id `build/t12` (the machine's name, so a
saved layout survives a restart; this computer's stay `t12`). **Settings › Machines** retries, removes and
adds machines (the behaviour is the terminal app's [Machines dialog](#project-specific-machines-dialog),
with the host list read off the UI thread, the look at the machine ended by a 12 s limit or Cancel, and a
login that needs a person run in a session named `ssh <name>`); **New session** (⌘T, Ctrl+Shift+T, ⌘K) always opens the **New session modal**: one 560px dialog of fixed height
whose header reads "Start in `<project>` on `<machine>` as `<agent>` in `[>_ | chat]`". Each part is a pill that jumps to its
step; the toggle locks to the terminal for a plain shell ("Shells always open in Terminal") and for another machine ("Chat
runs on this computer"). **1 Project:** Browse… (⌘O, Ctrl+O; this computer's folder picker) comes first, then the recent
projects (the focused session's folder is highlighted, so Enter takes it); the field completes in grey after what is typed
(Tab or → accepts) from known projects for a name and from the folders that exist on the machine for a path starting with `~`
or `/` (`Query::ListFolders`, [API_PATTERNS](API_PATTERNS.md)), and a folder that is not there shows a red "No such folder on
`<machine>`" and blocks Enter. **2 Machine:** this computer, then each machine with how its line is; one that is down is greyed
out with "Reconnecting…" and cannot be picked. **3 Start as:** Terminal, Claude, Codex, and a disabled "Crew (soon)", with the
default agent highlighted. Enter goes on, Esc goes back and then cancels; the dialog traps focus and gives it back on close.
Starting on another machine opens its shell there and, for an agent, types the agent's name into it. `NewSessionModal` is
presentation only; the window starts the session through the same paths as before (`startShellSession`, an agent typed into the
new shell, `startSessionOnMachine`). `New shell on <name>` in ⌘K opens the same modal on that machine. The sidebar's header starts without it: **New agent chat** (⌘⇧A, Ctrl+Shift+A) runs the default agent in the selected fleet and shows it as a chat, **New shell** (⌘⇧T, Ctrl+Alt+T) a shell there (a fleet on another machine asks for a folder, so it opens the modal), and **Add a remote machine** opens Settings › Machines. A new shell joins the selected fleet through `startShellInFleet` (`api/fleetCommands.ts`). A screen reader is told when a machine goes down (`build disconnected: unreachable`) and when it is back, once each and not for every retry, through a polite live region; every state is a word as well as a shape and a colour, nothing about machines animates, and the Machines page works with the keyboard alone (Tab, Enter, Esc ends a look in progress without closing Settings). Features that read a pane's files (Inspect, worktreesFeatures that read a pane's files (Inspect, worktrees, opening a file from the output, dropping files, the screen text) are for this computer's panes only: each says `<Feature> is local-only in v1: the files are on <machine>, not on this computer.` and the Repository button shows disabled with that text.

**Desktop service states** (`ServiceStatus`, `model/serviceState.ts`). While maittyd does not answer, every status reads
Unknown, spinners stop and input waits. The first four failed tries show a thin bar under the pane header, "Reconnecting to
maittyd · your sessions are still running", with "Try N of 5 · next in Ns" (it ticks each second) and **Retry now**. After the
fifth the bar becomes a centred red card, "maittyd isn't responding": three things to try, the error under Details, and
**Start maittyd** and **Retry**. The desktop's connection starts maittyd itself on every try (`open_or_spawn_for_user`), so
both buttons try again at once; there is no separate start-service command. A running service of another protocol gets an
amber card, "Restart the background service to finish updating", with what a restart keeps and stops and a **Restart
service** button that asks for a second click (`restart_mismatched_daemon`, then "Service restarted · N sessions back");
**Not now** leaves the read-only bar. Agents that were mid-turn offer **Resume** in the restore banner; nothing resumes by itself.

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

## Project-specific: desktop window sizes and sidebar order

Rules for `apps/gui/` (the look is in [DESIGN](DESIGN.md#desktop-app)).

1. The window is at least 720px wide. Under 1100px the sidebar is the 60px rail and
   expanding it opens the full sidebar as an overlay; under 800px panes stack in one
   column. The widths are constants in `features/workspace/model/windowSize.ts`; change
   them there and in DESIGN, nowhere else.
2. The sidebar has a FLEETS section and a SHIPS section (spec v31). A **fleet** is a daemon
   workspace (a name, a colour, a machine); a **ship** is one session in it, an agent or a shell. FLEETS
   lists this computer and each other machine as a collapsible heading (laptop or server mark, name, a
   status dot, and a + that adds a fleet there with an inline name field), and a row per fleet below it
   (initials tile in the fleet's colour with a needs-you count, name, up to three stacked app icons then +N).
   SHIPS shows only the selected fleet, in **Agents** and **Shells** (an agent's line is its branch, a
   shell's its folder and never a branch), or, with the Fleets | Priority switch, every agent across
   fleets grouped Needs you, Exited, Working, Done, Idle, each row naming its fleet. Ships are in
   creation order (`model/session.ts`, by the daemon's pane number) and nothing (status, pin, focus, a
   click, the filter) reorders them. Rows show no diff counts and no folder badge. Which fleet a session
   is in is the daemon's (`api/fleets.ts` reads `workspaces`); the window never decides it. Fleet colours
   are teal, violet, blue, green, pink and orange, never amber or red. The pure parts are in
   `model/fleets.ts`; `useFleets` holds the selection, which follows the focused ship.
   The workspace is a second card: a **tab tray** on top holds one tab per ship of the selected fleet
   (`model/tabs.ts`; a ship split beside another's pane is part of that tab, and the tab shows the pane
   count), then the panes. Split layouts are this window's own state (`model/layout.ts`), up to four
   panes per tab; a session closes with a 6 second Undo. The Crewmates mode is not built.
3. A new surface that sits on the terminal's ground uses `--term`, not `--main`,
   so a terminal pane reads as one surface.

## Project-specific: usage and cost page (desktop)

The Usage and cost page, a section of Settings (`apps/gui/src/features/usage/`, look in [DESIGN](DESIGN.md#desktop-app)) shows only
what providers reported through the daemon's usage store ([ADR 0009](adr/0009-usage-is-provider-reported-never-estimated.md)).
A figure nobody reported is a dash in the UI, never zero and never computed from tokens. The monthly
budget is a client preference (`monthlyBudget`, validated in `preferences.ts`); the ring needs a budget, a
known cost and the Month window. The report is fetched with a query (no polling faster than 60 s), and
the page never blocks the UI thread.

## Project-specific: layout and navigation

<!-- local: user requested sidebar-bottom controls and no global footer; on 2026-10-24 they chose one exception, the mode bar along the bottom. -->

Maitty has one terminal interface, drawn to the v27 TUI design (fleets and ships): two separate light frames with a one-column gap, the **sidebar** and the **workspace**, and the mode bar along the bottom. The sidebar lists fleets above and agent ships below (see *the sidebar*); the top border of the workspace is the tab row, one tab per ship of the fleet you are in (see *the frame*). The workspace is one pane, or up to four (see *the frame*). `ui.gap` is the space between the frames. `ui.rounded` (`full`, `corners` or `off`) rounds the corners of every box (`╭╮╰╯`) with `full` or `corners`; with `corners` chips, buttons and rows keep square ends, and with `full` they end in half-circle pills (a Nerd Font glyph). `ui.padding` is the blank space inside the terminal round both frames (`draw.rs` `padded`): `small` none, `medium` (the default) 1 row and 2 columns, `large` 2 rows and 4 columns; `framed` is that room less the mode bar's row. Older config values still load (`Ui::fold_retired_layout`, the file not rewritten): `tiled` (and `cards` with `panes = "tiled"`) as Floating with `gap = 0`, `grid` as Floating, and `compact`, `flat` and `hairline` as Floating. The first start with such a file says so once in a toast (`RetiredLayout::note`) and writes what stands for it into the file (`client/mod.rs` `settle_retired_layout`). Tab, Shift+Tab and Esc go to the program in the pane; the leader's Focus the sidebar moves focus to the sidebar. There is no application-wide footer bar but the mode bar (see *Mode bar*). The sidebar's foot holds the **+ New** and **Menu** chips, and the sidebar collapses as *collapsed sidebar* says. The mode bar along the bottom always shows the mode, then where the keys go (`keys go to claude`), and **Update** at its right end only when a newer release is available.

The prefix is `Ctrl+A` by default (`Ctrl+Space` still works for one release unless it is bound to something or the prefix was set on purpose). Inbox (`Ctrl+J`, prefix `i`), Changes (`c`), Files (`f`), Branches (`g`) and References (`e`) use the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The key overlay (the prefix; Search commands is `Ctrl+K`, `?` lists all), the branch switcher and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. A click on a toast (a finished session's note, the Restart now button) is the toast's even where it covers a pane whose program asked for the mouse; the program never sees it (`Screen::toast_rect`). A right-click menu answers each item's key while it is open but does not draw it, as the Menu doesn't (a bare letter beside an item reads as a key that works anywhere): the leader key bound to the same action in the live keymap (`menu.rs` `menu_keys`: split `-` / `|`, close `x`, Changes `c`), else a letter of its own, clear of those keys. A bound key the menu answers itself (`j` / `k` and the arrows, Enter, Esc, Delete, a check's `r` `o` `O` `u`, or the `x` that closes) goes to the menu, and that item takes a letter instead. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll. Branch (`b`, `client/screen/branch_sheet.rs`) uses the right-hand sheet: the session's branch, ahead/behind its upstream and its base branch (the main checkout's, else `main`, else `master`), the last 10 commits, the session's last check and its snapshots, all read by git in a background task. Its first row, **Switch branch…**, opens the switcher popup; Enter on a snapshot starts the row menu's restore (asked first). A folder outside git says so instead of the lists; an upstream whose remote branch is gone reads `gone from the remote`, and a git error shows in the place of what it was reading. References (`a`, `client/screen/references_sheet.rs`, files through `crates/maitty-terminal-client/src/references.rs`) lists the project's screenshots and design files from `.maitty/references/` at the repo's main checkout (a folder outside git keeps its own), so agents in every worktree see one set: name, kind, size and age per row, newest first, the selected row a neutral tint with an accent `▎`. Enter (or `o`) opens one in the system viewer as a Ctrl+click path does, `p` (or Ctrl+V) adds the clipboard image as `screenshot-<UTC time>.png`, `n` asks for a file's path (typed or pasted; a drop's quotes come off) and copies it in, `d` (or Delete) removes the chosen one after a danger confirm, `y` copies its absolute path; `a` again or Esc closes it. A paste while it has the keys stays in it: one line naming a file (a file dropped on the terminal) is added, an empty paste adds the clipboard image, and anything else gets the toast "References has the keys: …", which never repeats what was pasted. A name starting with a dot is refused. Entries the listing left out (unreadable, or links out of the folder) are counted on the list's last row in `err`, with the first reason. Empty, it reads "No references yet · paste an image or add a file", where they are kept and that agents find them through MCP, and its action bar holds only the ways to add one. Every file step runs in `spawn_bg`; the first time Maitty makes the folder in a repo it adds `.maitty/references/` to the repo's `info/exclude`. Over `--remote` it is refused with a toast, like Files. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

**Send a file** (`client/send_file.rs`, ADR 0007). In a pane on another machine the leader key `d` opens the prompt `Send a file to this machine: its path` (mode bar `SEND FILE`; a paste or a drop puts its first line in the box, a drop's quotes come off). Enter starts the upload in `spawn_bg`; a toast reads `sending <file> to <machine>… <n>%` and is refreshed once a second from the byte count, without adding to the notification history. `d` again, or a click on the toast's close mark, stops it (`stopping the send to <machine>…`, then `stopped sending the file to <machine>`); Esc only dismisses the toast and does not stop it, because Esc also belongs to the program in the pane. One file is sent at a time. On success the far-side path is typed into the pane like a local paste, and the toast says `sent <file> to <machine>`; every failure is an error toast (not a file, over 100 MB, no ssh to the machine, a login that needs a prompt, the machine could not keep it). On a local pane the key says there is nothing to send.

The sidebar's **+ New** chip and `n` open the New session dialog (see *New session dialog*). A project row's **+**, the empty project row's new action and leader + `n` are one action (`Action::ShellHere`): a plain shell where you are, whose tab opens with focus. You type your agent in it (see *Agent launchers* below). The Menu's `Session...` opens the same dialog; there is no instant agent session any more. **Menu** is in the sidebar footer. Every row of the key overlay and the Menu is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** at the right end of the mode bar and open the existing confirmation flow when clicked. Do not show the control when Maitty is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. When a newer release is out, the first line under the tabs, on every tab, says so in the accent with an **Update now** button (`dialogs.rs` `draw_settings`). The tabs are General, Appearance, Sessions, Alerts, Agents, Check and Keys (`modal::Cat::ALL`); each groups its rows under headings in `design.rs` `GROUP_ORDER`, rows keeping their `SETTINGS` order within a group: General: UPDATES (the Version row first, then Check for updates), STARTUP, INPUT, SHELL; Appearance: THEME, LAYOUT, PANES, STATUS; Sessions: NEW SESSIONS, SIDEBAR, WORKTREES, RESTARTS, HISTORY; Alerts: SOUND (Alert sounds, then a switch for the needs-you sound and one for the finished sound, and Sound volume: `quiet`, `normal`, `loud`), PHONE; Agents: AGENT STATUS, HELPERS, LIMITS, AGENTS TALKING TO AGENTS; Keys: the leader's shortcuts by group, then WITHOUT THE LEADER (the Alt+letters, Alt+1–9, Alt+arrows; `design.rs` `direct_key_rows`). Enter on a one-key row takes the next key pressed and writes it to `[keys.prefix]`, or `[keys.global]` for a key without the leader. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

There is no Team tab; `[team]` in `config.toml` still loads and is no longer shown. The label column is 33 columns and no setting label is longer; the value column starts after it, and a Choice whose chips would not fit beside the label shows the current value between `‹ ›` instead. The selected row's help (`Setting::help`) sits under a rule at the bottom of the page, word-wrapped over up to two lines, so a help text stays within two lines at 80 by 24.

**Settings › Sessions** opens on NEW SESSIONS (`[sessions]`: `Agent` last used, claude, codex or gemini, `Where` new worktree or this folder) and HELPERS (`Sessions may start helpers` ask, always or never; `Helpers at once` 2, 3 or 4), then the older alert, restart and history settings. Values outside those lists are refused when the config loads. `Agent` and `Where` are read only by the worktree and agent flows that remain (an old `plus_starts` key is read and ignored); the helpers settings are read by the daemon when an agent starts a session (see *helpers*). `[mcp] helpers_at_once` is not a setting any more: it is read only as a fallback for `[sessions] helpers_at_once`. **Settings › Agents** opens on AGENT STATUS: Claude Code reads `✓ hooks installed` (read once in the background when the page first shows, `checking…` until then, `hooks not installed` otherwise; the same test as `maitty doctor`), Codex reads `✓ read from the process and screen` until its lifecycle hooks are installed, then `hooks installed · trust them in Codex /hooks` until Codex has recorded trusting each, then `✓ hooks installed and trusted` (read once in the background; its help says Connect agents installs them and you review them once in Codex's `/hooks`), and `Connect agents…` opens the Connect agents list (`Action::AgentStatusSetup`), which offers exactly the agents `maitty integrate` sets up (`STATUS_AGENTS` in `crates/maitty-terminal-client/src/cli/integrations.rs`). SUBAGENTS has `Count on the session row` (`ui.subagent_count`, on by default): off hides the `· N agents` text on a working session row. The `mcp.*` and worktree settings follow. General's `Leader waits for next key` (`ui.which_key`, off by default) shows every key if you pause after the leader.

**Subagents have no tree.** The sidebar never lists an agent's subagents and has no inspector for them; the agent shows them in its own pane. A working row says `· N agents` only when the agent reports subagents (`ui.subagent_count`), and nothing else about them is drawn. The daemon still tracks them (`TermInfo::subagents`, `subagent_activity`) so the count is right and a parent stays working while its children run.

**Secret requests open by themselves** (`client/screen/secret_ask.rs` `auto_open_secret`, called after each state update). A request not yet offered opens its prompt beside the row when the mode is Normal or Side and no side view is open; otherwise it waits and the row's `asks for a secret` line stands. Each request id is offered once, so Esc is final. A prompt opened this way ignores keys and pastes for 1.5 s (Esc still declines), so a line being typed in a pane is not given away as the secret; one opened by Enter or a click on the row has no wait. Closing returns to the mode it opened from.

**Helpers are not a mode** (`client/screen/helpers_ask.rs`). Any session may start helpers (other agents it starts through `maitty mcp`); `[sessions] helpers` (ask, always, never) decides. When a session asks and the setting is `ask`, its row is **needs you** (amber, as for any question) and its second line reads `wants to start N helpers` (`wants N helpers` on a narrow sidebar). Enter on the row with the sidebar focused, Enter (or a digit) on its row in the Inbox, or a click on that line, opens a popover beside the row (from the Inbox, Esc or an answer goes back to the Inbox on that row): the screen dimmed except that row, a full accent border, a `◂` on the row, the title `START HELPERS? · <session>` and `Esc later` at the right. It says which agent wants to split the work into N helpers, lists the tasks (five, then `+ n more`), then "Their permission prompts come to your Inbox. Nothing merges." Buttons and keys: `1` Allow, `2` Always in <project> (`Always here` when narrow), `3` No, do it alone, each clickable; Esc decides later and the row keeps asking. Enter, `y` and Space do nothing there, and `1`/`2`/`3` act only inside the popover: no bare key answers from the sidebar or the collapsed strip. The answer is a command the daemon accepts only from you and only while it holds the request (`docs/API_PATTERNS.md`).

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running until **Restart now**. For a local daemon, when no panes are open, restart it automatically (see below). For a remote daemon, explain that it must be updated on its host after its sessions are closed. The one `↑ Update` pill sits at the sidebar's foot above its divider, or on the mode bar when the sidebar is collapsed, hidden or short, and becomes `↻ Restart` once a newer maitty is installed (here, or from outside maitty) and the server or this window still runs the old one; it clicks through to the same restart. The splash carries the matching line; there is no second notice in the working area.

**Restart now** is one step: it restarts the server onto the installed maitty **and** replaces this window with the installed one in the same terminal. It shows in Settings' restart-pending line (`R`), on the toast after an install (it stays 12 s), on the `↻ Restart` pill, and as the palette action *Restart maitty* (`Action::Restart`, `restart`); none show for a remote daemon or while a restart is under way. It asks only when a session is working or waiting for you (or when `restore.enabled` is off, since then sessions close for good): a confirm (`menu::Confirm`, danger) reading "4 sessions stop and resume, 2 are working, 1 waiting for you" with Restart (Enter) and Cancel. Otherwise it restarts at once. The server saves its session and every pane's output first (`KillServer`), then the window closes and the installed maitty opens in its place (`restart::relaunch`: exec on Unix, spawn-and-exit on Windows, same arguments and `MAITTY_SOCKET`), which starts the new server and restores the sessions. A window that is already current (only the server is old) reconnects in place; a current server with an old window swaps only the window and leaves every session running. An agent mid-turn is stopped when the server stops and resumed with its conversation when it comes back; the turn in flight is not replayed.

A new `maitty` that finds a server of another version replaces it on the way in. A server of an older release whose protocol this one speaks, and which is idle, is stopped at once (`restart::replace_older_idle_server`, from the activity file the server keeps beside its pid file: `restart::Activity`). A server whose protocol this one cannot speak (`ipc::OtherVersion`) is judged by that file, or for a server too old to write one by the agents in the saved session (`restart::plan_for_mismatch`): when nothing is working or waiting, the terminal says `Updating maitty…`, the server is stopped by its process as `maitty kill-server` does across versions, and the window opens with the sessions restored, with no prompt. When something is working or waiting, or the server is newer than this maitty (never replaced unasked), the full-screen prompt (`client/restart_prompt.rs`) shows the protocols, how many sessions resume and how many are mid-turn, and **Restart and open** (Enter) and **Quit** (Esc, Ctrl+C), each clickable; Quit prints the usual error and exits 1. Without a terminal on stdin and stdout, or with `--remote`, there is no automatic step or prompt: the error text and exit code are unchanged, and a remote server is never restarted this way. A same-protocol older server that is busy is left running; the `↻ Restart` pill offers the restart.

**One key overlay** replaces the command palette, the long shortcuts reference and the old leader overview (`client/screen/keyoverlay.rs`). It has three faces in one card with one typing row: Its card has no fill (its lifted rows are flattened to the ground), a red `×` close on the top border, and its key hints are chips on the bottom border.

- **Key map** (the prefix, or the pause after it). A banner, "Every key goes after the leader: `Ctrl+A` then `-` = split", then Sessions, Panes, Sidebar, Tools and Maitty, each key a keycap followed by its name, from `key_groups()` and the real bindings. A pressed key runs its command; `Tab`, or an unbound letter, switches to the search. Three columns when the card is 100 or more wide, else two; each group's key column is as wide as its widest key. A key marked `›` opens a second step (Worktrees). Action bar: `key` run it, `Tab` search, `?` show all, `Esc` close.
- **Search commands** (`Ctrl+K`, `Space` after the prefix, or the Menu). A box with an amber border up to 110 by 26 (`draw_search`), an amber title chip and a pink `×`. `› type what you want to do`, then COMMANDS (RESULTS while typing) and SESSIONS. A command row is its label with the matched letters in the accent, a category (General, Panes, Tools, Find, Sidebar, Appearance) and its key as a keycap; a session row is its status dot, name and `project · machine`, and Enter goes to it. Matching is a case-blind in-order subsequence (`spld` finds Split down), names that begin with the text first. With nothing typed COMMANDS lists the last commands run and the first sessions. `↑ ↓ move`, `Enter run` and `Esc close` are chips on the bottom border. A set of keys (the arrows) is searched one key at a time. Every command the old palette offered stays reachable here.
- **All keys** (`?`, Menu → All keys). The map in full, then "WITHOUT THE LEADER · press directly" (Ctrl+T, Ctrl+K, Ctrl+J, F2, Alt+Up and Down, Alt+1–9, Alt+Left and Right and the Alt+letters when on) and "SEARCH ONLY" (commands with no key), both headings dim. It scrolls when it is taller than the screen; a letter typed there starts a search.

Rules for keys and their labels: one key per action (no alias keys; a user's own `[keys.prefix]` entries still load), no default needing Shift, keys other apps use, sentence case, no bracketed explanations (the group gives the context). Every action has a leader key or is search only; nothing in the sidebar acts on a bare letter. `docs/shortcuts.md` documents the other screens' keys and is not embedded in the app.

**Project tools** (`client/screen/sheet.rs`, `menu.rs` `project_items`). The pane's ` ▾ Project ` chip (or the key of each tool: leader `f` Files, `c` Changes, `g` Branches, `h` History, `e` References) opens Files, Changes, Branches, History and References. A picked tool opens as a panel 60 columns wide over the right side of the workspace (one saved width, `Saved::sheet_w["drawer"]`, drag its left edge or `[` `]`), so the panes are never resized by it; its title chip names the tool, and Tab / Shift+Tab step along the tools while it has the keys. Files shows the tree and the open file side by side from 60 columns, and by turns under that. Under the five tools the menu also keeps what the folder offers that the flat design has no place for: New / Open / Tidy worktrees, Rename and Close project, Snapshots and the check's output, re-run and override. The check sheets stay docked beside the panes (`PANES_KEEP` columns kept for them); the Inbox floats (see *the frame*). There is no tool rail. A draggable edge (the sidebar's, the panel's) lights in the accent while pointed at or dragged.

**Cursor.** The focused pane shows the cursor its program asked for (DECSCUSR, read from its output); where it asks for none, or for the default, Settings › Appearance › Cursor (`ui.cursor`: `block` by default, `underline` or `line`; an older `terminal` reads as `block` and `bar` as `line`) and Cursor blink (`ui.cursor_blink`) decide (`client/mod.rs` `cursor_code`). An unknown `ui.cursor` is refused when the config loads.

**Background.** Settings › Appearance › Background (`ui.background`): `solid`, the default, paints every cell the theme's `bg` and sets the terminal's own background to it (OSC 11, given back with OSC 111 on exit) so a plain terminal shows the whole theme; `terminal` leaves the chrome on the terminal's background (SGR 49) for transparent or image backgrounds (`client/render.rs` `draw`).

**Keycaps.** Every key the UI shows is a keycap from `design::keycap`: ` key ` on `btn`, `strong` and bold, with the ends round when `ui.rounded` is `full`. A combo splits on `+` into one cap per part joined by a dim `+`. The map, the search, hint rows, Settings key rows, dialogs and the copy hint all use it.

**Action bars.** A panel's key hints sit in their own one-row bar (`action_bar`, `panel_bar` in `popups.rs`) on a ground one step above the panel's: `card` for a sheet on the app background (Inbox, check evidence) and `card2` inside a modal (Settings, the key overlay, confirms, pickers). It spans the panel's width with one column of padding, shows keycaps and labels, and is always the panel's last row. When the hints do not fit they close their gaps, then give up the ones before `Esc`.

**Menu.** Sections a blank row apart, each under a `muted` title that says what its rows are for, labels capitalised, no letter lit: `NEW` (Shell here, Session..., Pane beside this); `TOOLS` (Files, Changes, Branches, History, References); `FIND` (Inbox, Search commands, All keys); `SIDEBAR` (Collapse sidebar); then Settings, untitled. The arrows move over rows and skip the headers; the selected row has an accent `▌`. In a short window the blank rows give way first, then the titles, so every row that does something stays (`leader.rs` `MenuRoom`). No row shows a key (a bare letter reads as one that works anywhere), but the leader key that does the same runs the row inside the Menu too; there is no footer. "Shell here" is leader `n` (a plain shell where you are) and "Session..." the New session dialog (Ctrl+T).

**Agent launchers.** Typing an agent (`claude`, `codex`, `gemini`, `opencode`, a quick agent) in a Maitty shell in a git repo's main checkout starts it in a new worktree (`worktree.per_agent`); anywhere else, or in a folder that is not a git repo, it starts where you are, silently. It works from any shell because `<data>/shell/bin/` is first on the pane's `PATH`; see ARCHITECTURE › Agent launchers. No toast or warning is shown for main folders.

**New sessions** (`client/screen/newsession.rs`). Leader `s` starts a session with no form: agent `sessions.agent` (`last used` is the last agent started from Maitty, kept as `last_agent` in `maitty-ui.json`, else claude), project the one under the sidebar cursor or the one you are in, `sessions.where` (`new worktree`, which falls back to the folder in a project that is not a git repo, or `this folder`). The task is typed in the session, whose row reads `<agent> · new · type to begin` with the accent `○`. A new tab (leader `t`) takes only the shell that answers it: it waits ten seconds at most, takes only a pane that did not exist when the tab was asked for and that no agent started (another pane taking the focus leaves it waiting), and starting any other session before it arrives cancels the tab, so that session gets its own row and tab bar rather than becoming a tab of the one you were in.

## Project-specific: pane focus

Only the area with the keyboard is lit: the focused pane's whole border, or the open sheet. Unfocused panes fade by the Dim setting (off, light, normal or heavy; normal by default, and an older config's "subtle", "30%" and "60%" read as light, normal and heavy). Do not reserve a full-width bottom row for global hints; the mode bar is the one exception. The pane has no status line: its branch is in the sidebar's folder header.

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
  pane to type (`n`, `s`, `t`, the splits, `spawn-*`, `o` once its
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
key), which flips `appearance` and changes the whole screen at once.
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

A session's row carries no check line. A failed check, or a command that ended in failure (`TermInfo.exit`), is the **problem** state: the red status glyph on its row, on its pane's border and in the collapsed column. The check's words, its checkpoints and its output stay where they were: the Inbox, the evidence sheet (`o` on a check in the Project menu, or Enter on a row that wants a look) and the Project menu's re-run and override items.

## Project-specific: check evidence sheet

The check evidence sheet is part of the frame (see *the frame*). Enter on a sidebar row whose check failed, could not run, was overridden, went stale or is never run for its repo opens it (a row waiting for approval opens the approval; every other row still focuses its session), as does Output in the Inbox. It docks right (`split_for_sheet`) and takes the pane column below `NARROW`. Its title is `CHECK · <session> › <checkpoint>`; content starts one column into the card's inside.

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
everything not just finished or exited. It lists, in this order: NEEDS YOU, FAILED CHECKS, COULD NOT RUN,
FINISHED, EXITED, then HEADS UP, each a chip head on its colour (`blocked`, `err`, `violet()`, `done`, `err`)
with its count. A question is never below a check. EXITED lists a session whose last command ended in
failure (`TermInfo.exit`, set by the daemon) that asks for nothing and has no check of its own to show:
its line reads `exited with code N`, Enter or `o` opens it, and `c` or Delete clears it until another
command fails (`Saved`-free: `screen.exit_seen`, by when the command ended). A session that is both
finished and exited is listed as exited only.

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

The layout and checkpoints are summed up in [DESIGN](DESIGN.md). `client/screen/flat_tests.rs` holds the sidebar, the collapsed column, the status glyphs, the tab row and the workspace to the v27 design; `client/screen/frame_tests.rs` holds the modal and popover frames.

- **Grid** (`draw.rs` `grid`): inside `ui.padding` (`framed`). The sidebar is `SIDE_DEFAULT` (46) columns, resizable between `SIDE_MIN` (24) and `SIDE_MAX` (60) (a width you dragged wins), the full height, then `ui.gap` columns, then the workspace. Below `COLLAPSE_BELOW` (110) columns the sidebar is the collapsed column and the full one opens over the workspace.
- **Chips** (`chips.rs`): title (accent when its frame has the focus, else `btn`), action (accent on `btn`), quiet (`muted` on `btn`), alert (amber, for ` inbox N ` when anything needs you) and the red close `×`. A chip is its text and a cell each side; the ends follow `ui.rounded`.
- **Tab row** (`tabs.rs` `draw_tab_row`): the top border of the workspace, for a lone pane and for a split alike. One tab per ship of the current fleet (`App::fleet_tabs`), in creation order; a pane that sits in another ship's split is part of that ship's tab. A tab is ` ● title N × `: the status glyph, the title (`pane_title`, cut at 24 columns), the pane count when it is 2 or more, and the `×`. The active tab is the accent with `acc_ink` text while the workspace has the keys, else `btn` with `strong`; the others `btn` with `text`. Click opens the ship (`screen_focus`), the `×` closes it (`menu_act(Act::End)`: with an undo, asking again for a running agent), right-click is the ship's row menu. When the tabs do not fit the active one stays in view and ` …N ` counts the rest. ` + ` makes a shell in the fleet. At the right ` Files ` and ` Changes ` (`ScreenHit::FilesChip`, `ChangesChip`) open or close the project panels and are lit while their panel is open; an open panel docks or floats over the right side and the chips move left of it. Ctrl+Tab and the prefix then `n` / `p` step through the tabs; Alt+1 to 9 pick one.
- **Lone pane** (`workspace.rs` `draw_session` with `Chrome::Tabs`): one frame, no header; the tab row is its top border and the program starts on the row under it. A floating pane (`<app> popup`, `Chrome::Title`) keeps its own status glyph, title and red `×` on its border.
- **Splits** (`workspace.rs` `draw_main`): up to four panes (`MAX_PANES`) in one frame cut by single lines. A `LineGrid` collects which directions the lines leave each cell in, so corners, tees and crosses (`┌ ┬ ├ ┼ …`) follow from what meets there. Each pane has a two-row header: the status glyph and title at the left with `⋯ ×` at the right (`⊞ ⊟ ⤢ ⋯ ×` from 60 columns), then its folder, `⎇ branch` and, for a pane on another machine, `▤ name`. A split is a daemon `Split`, so the panes stay in one fleet and a restart brings the layout back. Split commands (leader `_` up, `-` down, `\` left, `|` right) put the new pane before the focused one for up and left and after it for down and right. Drag a line to resize. Exactly one frame or pane has the accent: the focused pane's lines (the joints they touch included); a pane being closed lights its lines red.
- **Focus**: a click on a pane's header or body focuses it; a header button acts on that pane. While the sidebar or a side panel has the keys, no pane is lit.
- **Swapping panes** (`input.rs` `Drag::Pane`, `behaviour.rs` `swap_panes`): a press on a split pane's title row starts a drag; let go on another pane of the view and the two trade places. While dragging, the pane under the pointer lights its lines in `accent` with a centred `⇄ swap here` pill, and a `btn` pill with the dragged pane's name follows the pointer.
- **Mode bar** (`mode_bar.rs`): the bottom row of the room inside `ui.padding` (`draw.rs` `mode_bar_row`; `framed` is the room above it, so every look is a row shorter). Popups and their dimming sit above it (`above_bar`), so it is never covered or dimmed. Typing: ` TYPING ` (`muted` on `btn`), `keys go to <agent or process>`, then hints spelled out with the prefix from the keymap, key bold and words `muted`: `Ctrl+A then ← sidebar`, `Ctrl+A then . menu`, `Ctrl+T new`, `Ctrl+K palette`, `Ctrl+J inbox`; a click on the chip or a hint starts shortcut mode as the prefix does. Needs-you, the branch and the load keep the right end and the hints give way first, whole, from the right. Lit: the whole row in `sky()`, a bold ` ⌃A ` chip (the prefix; `sky` on `bg`; a click opens the key map), then suggestions read from the keymap, key bold and label plain in `ink_on(sky)`: `t shell`, `n next tab`, `. menu`, `i inbox`, `c changes`, `f files`, `, settings`, then `? all keys` and `Esc type`. A rebound key shows its new key; an action with no key is left out. A narrow bar drops suggestions from the right, then Update shortens (` ↑ 0.44.0 `, ` ↑ `) and goes, then `? all keys`, then `Esc type`; the tag stays. A click on a suggestion presses its key in shortcut mode, so it does what the key does: the mode stays, unless the key starts a pane. Who has the keys (anything but the panes, whether or not shortcut mode is held: the sidebar, a popup or modal, a side-panel view with the keys, the key map's search or All keys; `keys_holder`): the bar is on `bg`, never sky, and names it in a calm chip (`strong` bold on `btn`: ` SIDEBAR `, ` SETTINGS `, ` FILES `, ` INBOX `, ` CHANGES `, ` BRANCH `, ` PALETTE `, ` MENU `, ` UPDATES `, ` CONFIRM `, ` NAME `, ` MESSAGE `, …), then its main keys and the way out, each after a `·`, key bold `strong`, words `muted`: `SIDEBAR · ↑↓ move · Enter open · Esc back`, `CONFIRM · y yes · n no`, `NAME · type · Enter save · Esc cancel`, `MESSAGE · Enter send · Shift+Enter new line · Esc keep draft`. The keys are the ones that thing's own handler reads in the state it is in (Files' tree, filter, preview or editor; Changes asking y/n; each step of Switch branch; Settings waiting for a key or editing a value; Settings' Check page by its row, while its command is typed, before the project has a checkpoint (a letter starts the command) and with no project open (only Tab and Esc), and the project check likewise; the Inbox asking to merge or throw away; copy mode's search and selection; the key map's second step; a folder picked for a setting, where Esc goes back); add a row there when a mode or view is added (the match is exhaustive). While shortcut mode is held it adds `· shortcut mode waits` before the way out. A narrow bar drops these hints from the right (`shortcut mode waits` first), then Update, then the way out; the label stays. The key map before a search stays lit, since its keys still run shortcuts. Under anything that has the keys (a popup or modal from either mode, a view shortcut mode opened, or the key map and its search) the bar is only words: its chips take no click, and a click on its row does nothing (`only_says`), so it never closes the popup, runs a suggestion or drops typed text. **Update** or **Restart**, when waiting, sits at the bar's right end, clickable except under anything that has the keys. With `ui.rounded = "full"` each chip on the bar is capped on the bar's ground and its click covers the caps. In shortcut mode a key with no action shows the toast `Shortcut mode — Esc to type` (`input.rs` `hint_dead_key`), marked with a sky `i` and bar since it is information, not a success: at most once every 6 s, never over a toast still showing, and not kept in the notification history; keys that act are unchanged.
- **Sheet** (`sheet.rs` `split_for_sheet`): the check sheets and both-diffs dock right from 140 columns, the layout's gap from the panes; below 140, or when the panes would keep fewer than 40 columns (`PANES_KEEP`), they take the pane column and Esc brings the panes back. They start at 72 columns.
- **Inbox** (`draw.rs` `grid`, `sheet.rs` `INBOX_W`): a panel 62 columns wide that floats over the workspace from the sidebar's side (the left edge of the pane column, or its right edge when the sidebar is on the right). The panes keep their column; the panel has a fixed width, so `[` `]` and its edge do nothing. Its cards are NEEDS YOU, FAILED CHECKS, COULD NOT RUN, FINISHED and EXITED.
- **Message popover** (`popover.rs` `draw_compose_pop`): the follow-up to an agent, opened from the sidebar row or the leader (`M`). The screen is dimmed except the popover and its row. A full `accent` border (corners by `ui.rounded`), `card` ground, the input on `bg`, no shadow.
- **Toast** (`draw_toast`): one component for every Maitty notice, bottom right, its bottom edge `TOAST_ABOVE_INPUT` (5) rows above the screen's so it sits over the agent's input area. It draws a `line` outline on `bg` with round corners whatever `ui.rounded` says, its left side a heavy `┃` in the notice's colour (error `err`, about a session that needs you `blocked`, about a finished one `done`, otherwise `accent`).
- **Selecting and copying**: a mouse drag or `v`/`V` in copy mode tints the selection sky (`blend(bg, sky, 0.38)`) with `strong` text; `y` copies and closes copy mode; the copied cells then glow accent for `copy::FLASH_FOR` while the toast reads `Copied to clipboard`.

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

The sidebar has three states (`App::sidebar`, `App::side_hidden`): full, **collapsed** and **hidden**. Leader `z` (`Action::ToggleSidebar`) swaps full and collapsed; Hide sidebar (`Action::HideSidebar`, search only) hides the sessions or brings the sidebar back. The `»` chip (`«` with the sidebar on the right) and the `…` row restore it.

Until a sidebar key is pressed (`App::side_auto`), a screen narrower than `COLLAPSE_BELOW` (110) columns shows the collapsed column and a wider one the full sidebar (`App::side_folded` remembers the sidebar was folded by this rule, so it comes back when the window widens). On a narrow screen the toggle opens the **full sidebar as an overlay** above the workspace (`draw.rs` `draw_overlay_side`, `Screen::overlay_open`); picking a session, or the toggle again, puts it away.

- **Collapsed** (`screen/rail.rs`): a box `RAIL_W` (10) wide down the sidebar's side; the workspace takes the rest. A ` » ` chip (` « ` with the sidebar on the right) is set into its top border; under it the inbox count (amber background when anything needs you) and a ` FLT ` divider; then one line per fleet: the accent `▌` and the `raise` ground for the current fleet, its amber needs count or a `◆` in its colour (a dim `┆` when its machine is down) and the **initials** of its name (the first letters of its first two words, or the first two letters of a single word). A ` SHP ` divider at the middle (`(FLT row + last row) / 2`) starts the agents, one line each: the `▌` and `raise` for the open one, the status glyph, the initials and an accent `•` when it has something unread. Each half scrolls on its own, the current row kept in view, with `▲` `▼` marks. At the foot, under a rule, a `+` chip starts a shell and a `≡` chip opens the Menu. Each line is a click target (a fleet opens its last active session).
- **Column focus** is the sidebar focus with the sidebar collapsed (leader Focus the sidebar). `↑` `↓`, `←` `→` and `1`–`9` switch sessions, opening each in the pane while the keys stay on the column; `1`–`9` mean nothing elsewhere. Enter focuses the pane, Esc leaves.
- **Hidden**: no sessions and no column; the panes take everything. Reach them again with leader `z` or `Z`.

## Project-specific: the sidebar

(`client/screen/side.rs`, the model in `screen/fleets.rs`.) **Fleets above, ships below.** A fleet is what the daemon calls a workspace (name, colour, tabs); there is one daemon per machine (ADR 0007), so each machine's workspaces are its fleets (`App::fleet_groups`, `MachineGroup`, `Fleet`, `FleetId { machine, ws }`). A ship is a pane in a fleet. A fleet with no name goes by its folder; its colour is the workspace's palette index modulo teal, violet, blue, orange (`fleets::colour_of`), and a fleet named `Scratch` has none (muted).

- **Top border**: ` « ` at the left; ` + ` (a shell in the fleet you are in) and ` inbox N ` (amber while anything needs you) at the right. **Row 2**: the search field, a filled bar on `btn` (`sel` and an accent `▌` while typing) with a ` / ` chip, which becomes ` x ` (clears it) once there is text. `/` while the sidebar has the keys, or a click, starts typing; it matches fleet names and sessions by name, folder, process and machine, ↑ ↓ move through what is left (folds are ignored while a filter is typed), Enter opens, Esc clears.
- **Two halves.** ` FLEETS ` is a title chip on a full-width divider at row 4 and ` SHIPS N ` one at `MID = floor((4 + H - 5) / 2)` (H is the sidebar's height; a short sidebar drops the blank rows). Each half clips at its own divider and scrolls on its own (`Screen::side_scroll`, `ships_scroll`; `side::window` keeps the wanted row in view). `SideModel::lists` is the one ordered list of selectable rows (`SideItem::Proj` for a heading, `Fleet`, `Sess`): drawing, ↑ ↓ and Enter all read it, as `Screen::side_items`.
- **Machine heading**: the fold arrow, `⌂` (this computer) or `▤`, the name in `muted`, then a green `●` (up) or dim `┆` (the line is down) and an accent `+` that starts a shell in the machine's first fleet (a warning, and nothing started, while it is offline). Folded, the number of fleets follows the name. Fold key `m:<machine>` (`local` for this computer).
- **Fleet row**: an amber bold count of the sessions that need you, else a `◆` in the fleet's colour (a dim `┆` on a machine that is down); the name in bold (`strong` for the current fleet, `text` otherwise); at the right `✻ N` (agents) and `>_ N` (shells) when they fit. The current fleet has the `raise` ground and an accent `▌`; the selected row `sel`. Enter or a click opens the fleet's last active session (`Screen::fleet_active`, set whenever the focus moves), else its first.
- **Ship group heading**: the fold arrow, a `◆` in the fleet's colour, the fleet name in `muted`, ` · machine` for another machine; folded, the number of ships and `N need you` in amber. Fold key `c:<machine>/<workspace id>` (`Saved::closed`). Only fleets with an agent have a group. A chat with no fleet is listed on its own at the end.
- **Ship row** (agents only, two lines, creation order so a row never moves when its status changes): the status glyph, the name in bold (your name for it, else the process; a chat goes by its title; idle ones in `muted`), a `•` when it has something unread (`TermInfo.bell`), a `⌖` when pinned, and at the right `>_` (terminal) or `◌` (a chat with no terminal), or `⏎ resume` in the accent for an agent a restart stopped (`TermInfo.interrupted`); under it `⎇ branch` in violet and `worktree` dim at the right (the folder when there is no repo). The ship open in the workspace has `raise` and the accent `▌` on both lines; the selected one `sel`. A new row's bar flashes for 500 ms (not with motion off). A session whose machine is down is drawn all in `idle`.
- **Foot**: the version in `idle`, or the update marker (`mode_bar::release_segs`), a rule, then ` + New ` and ` Menu `.
- **Row menu** (right-click on a ship or a tab, or `.` with the cursor on a ship): Rename, Pin / Unpin, Open in split right, Open in split down (not for the session on screen), Copy path, and Close session last. Anywhere else `.` is the Menu. The project tools (`Project…`) are in the menu of a right-click on a pane's content.
- **Keys.** ↑ ↓ walk the list; → and ← unfold and fold a heading (← from a ship goes to its group heading); Enter folds a heading, opens a fleet, opens a ship; `n` starts a shell in the fleet under the cursor.

**Status glyphs** (`screen/status_mark.rs`): working is the chosen spinner in the accent (Dots, the design's Braille, held 80 ms a frame, Snake 90 ms, Bars 85 ms, other spinners 90 ms; Reduce motion shows the first frame); needs you is a `●` in `blocked` amber, done in `done` green, problem (a failed command, a failed check) in `err` red, idle in `idle` with the name dimmed; `┆` for a machine that cannot be reached. By default the states differ by colour only, as the product owner chose. Settings › Appearance › **Status glyphs** `Unicode` (`ui.glyphs = "unicode"`) gives each its own shape for colour-blind users: `●` needs you, `✓` done, `✕` problem, `○` idle.

## Project-specific: New session dialog

Choose another folder or machine for a new shell (`client/screen/new_session.rs` the model,
`new_session_ui.rs` the keys and drawing). It opens from the Menu's `Session...` (`Action::PickSession`,
palette `New session`, Alt+A), the splash's New session and a machine's ` + ` chip in Machines. Every
session it starts is a **shell**: there is no agent or view to choose, agents start inside the shell.

**A new shell does not use the dialog.** **Ctrl+T** (prefix `t`), the sidebar's ` + ` and ` + New `,
the tab row's ` + `, and `n` in the sidebar start a shell at once in the fleet of the focused session
(`App::new_ship`; a machine heading's `+` asks for a name and starts a named fleet there with `NewFleet`, in the first of the four fleet colours that is free, and an empty name starts a shell in that machine's first fleet instead; with no fleet, a Scratch
fleet): a daemon `NewShip` in that workspace, so it is on the fleet's machine, becomes the fleet's
active session and shows as a new tab. The status bar says `New shell in <fleet> · <folder>`. The
shell starts in the folder you are working in when you are in that fleet, else where the fleet was last
used, else the fleet's own (`NewShip` carries it); a session started
from the dialog in a folder some fleet of that machine already works in joins that fleet, else the
folder starts a fleet of its own (`screen_start_pane_on`).

- **Box.** A fixed 78 by 19 cells, centred, titled ` NEW SESSION `, whatever the content. The header reads
  `Start in [project] on [machine] · opens a shell`; the chip of the active step is in reverse video.
  The keys are chips on the bottom border: `↑ ↓ move`, `Enter pick` (`start` in step 2), `Tab next`,
  `Esc close` (`back` in step 2 and in the picker).
- **Step 1, project.** A search row `⌕` with the typed text. A query starting with `~`, `/` or `.` is a
  path on the chosen machine; anything else filters the list. The list is `Browse…` (Ctrl+O), then the
  recent projects: the last `RECENT_KEEP` folders a shell was started in (`recent_paths`, now absolute
  paths, `host:path` on another machine) and the folders sessions are running in, each with its name,
  its path and its machine (`local` or `⇄ name`). A typed path lists the folder it names first, then
  the folders inside it that begin with what follows the last `/`.
- **Completion.** The first folder that begins with the last segment is drawn dim after the cursor;
  `Tab` or `→` accepts it (`Tab complete` shows at the right). Folders are read with `Query::ListFolders`
  on the chosen machine's own connection ([API_PATTERNS](API_PATTERNS.md)), one folder at a time and
  only when it is needed, never on the UI thread. An answer nobody asked for is dropped.
- **A folder that is not there** shows `✗ No such folder on <machine>` in red and Enter does nothing; while
  what is typed begins a real folder's name it is only incomplete, with no red.
- **Browse.** `Ctrl+O` or Enter on `Browse…` lists `Use <folder>`, `..` and the folders of the folder
  being browsed; Enter on a folder goes in, Esc leaves the picker.
- **Step 2, machine.** Enter on a project goes here when more than one machine is up; with only this
  computer it starts the shell at once. Lists this computer and the remotes. A machine that is
  reconnecting is dim, says so, and is skipped by the arrows. The project's folder is read on the machine
  picked; if it is not there the red line names the folder (`✗ <path> is not a folder on <machine>`) and
  Enter does nothing, while the answer is awaited it says `checking the folder on <machine>…`, and a
  project remembered from a machine that is down shows `✗ <machine> is offline: pick another machine`
  (with only this computer up, such a project is refused with a notice instead of starting there). Enter starts the shell
  through that machine's connection (ADR 0007). No latency is shown: the app does not measure it yet.
- **After starting** the dialog closes, the folder is remembered, the daemon focuses the new session (the
  last row of the sidebar) and a green line names it in the mode bar for `BAR_FLASH_FOR`.

## Project-specific: closing, service and update states

- **Closing a session** (`client/screen/closing.rs`): the pane menu, the row menu, leader `x` and a middle
  click on a tab send `Command::CloseSession`; the daemon hides the session and keeps its process for
  six seconds (`UNDO_WINDOW`), `Command::UndoClose` brings it back. A banner on the first row of the focused pane
  reads `Closed <name> · Ctrl+A then z undo 6s`, counting down; leader `z` undoes. A session with an agent that is working
  or waiting asks first: the banner reads `Close and stop <agent>? press again` and the same key a second time
  closes it. Closing several panes at once (a tab's `✕`) still asks in a dialog and has no undo.
- **Row menu** (`.` or right-click on a session row): Rename, Pin or Unpin, Open in split right / down, Copy path
  and, when they apply, Changes, Snapshots, the check items and queued follow-ups; **Close session** last, in red, after a rule.
  **Pane menu**: Split up, down, left, right, Find, Rename, Copy path, the right-click toggle; **Close pane**
  (several panes shown) or **Close session** last.
- **Reconnecting** (`client/screen/service.rs`): when the window loses its own daemon without being told to
  quit, it stays. An amber bar on the first row of the pane says
  `┆ Reconnecting to maittyd · sessions still running · try N of 5 · next in Ns · r retry`; tries wait 1, 2, 4, 8 and 15 s
  (`BACKOFF`). Typing goes nowhere; `r` tries now and `Esc` leaves. The status glyphs do not change to `┆` yet.
- **Service down.** After five tries a red box, ` MAITTYD ISN'T RESPONDING `, says sessions are still running and
  gives three things to try: `r` connect again, `s` start the service (`ipc::open_or_spawn_installed`), `maitty doctor`.
- **Service update needed** is `restart_prompt.rs`, shown before the window opens when the running daemon is another
  protocol version: an amber title, SURVIVES (shells and their scrollback), STOPS (agents), `r` restart service or `Esc`
  not now (which leaves the window; a client and daemon that cannot read each other do not talk, so there is no
  read-only mode).
- **After a restart** a line reads `maittyd restarted · N agents stopped · Enter on its row resumes` for 30 seconds
  while any session has `TermInfo.interrupted`; Enter on that row in the sidebar types its resume command.
  Resuming is never automatic.
- **No sessions**: `No sessions running`, `Sessions keep running when you close the app.`, `Ctrl+T new session`.
- **Updates box** (leader `u`; `client/screen/updates.rs`): above the sidebar footer. Available: `<current> → <version>`, the
  notes under NEW, `d Download · s Skip · l Later`; Downloading: a spinner, the step (`Downloading`, `Checking the download`, `Unpacking`, `Installing`) and a bar of the steps done, `step 2 of 4`
  (the installer reports no byte count, so the bar moves in whole steps and never shows a percentage; `cli::update::step`); Ready: `r Restart now` (this window reopens; the daemon is never stopped by it); Failed: the error and
  `r Retry`. It reads the existing state (`update_available`, `updating`, `restart_needed`) and uses `start_update` and `ask_restart`.

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
  with no supervisor gets one. Update runs `maitty machine update <name>` (with `--install` when the app is not there at all) in a popup pane on this computer, where it says what it found and asks before it installs; the machine is dialled again when the popup closes. Edit asks `Where is build? Type it like scp or ssh.` with the address it has now in the field: Enter writes the new `target` into its entry in `config.local.toml` (its name, its other keys and the rest of the file kept), lets the old connection go, forgets the daemon id recorded for the name and dials the new address at once; a target the loader refuses is said under the field and nothing is written. The name is not edited: its saved layout is kept under it. Remove asks first (`Remove machine`, with the address and "Its sessions keep running on it, and you can add it again"; Enter removes, Esc keeps it; in the Machines dialog, `x` asks the same on its own step), then says `Removes build from this list. Its sessions keep running on
  build.` in a toast. It deletes the entry from `config.local.toml`, stops the supervisor and the connection, takes the machine out
  of the sidebar and the window's records, and forgets the daemon id recorded for the name. The machine, its
  daemon, its sessions and its saved layout (kept under its name) are not touched.
- **A refused machine** (a different daemon answers at its name) shows `refused` with the reason and the
  fix: remove it and add it again, or fix the host name or DNS and reconnect.
- Adding and removing take effect in the running window. Editing the file by hand still takes effect at the
  next start.

## Project-specific: Hairline

Not applicable: the Hairline look was removed; Floating is the only look.

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
ADR 0007). On a pane of this computer the dropped paths are typed into it, quoted for its shell. On a
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
host `image_paste.rs`, ADR 0007 and the amendment to 0055). Ctrl+Shift+V on Windows and Linux, or Cmd+V on macOS, pastes through the
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
([chat view plan](features/chat-gui/chat-view-plan.md), ADR 0006). Rules:

- **Source.** It renders only the daemon's structured events, through `features/chat/` (the projection).
  It never reads terminal output. A session has a chat only when `conversationsFromSnapshot` found a daemon
  conversation whose native terminal is that pane (`Conversation.chat`); `model/chatView.ts` decides
  availability: `ready`, `no-events`, `remote` (local-only in v1), `unsupported` (another agent) or `shell`.
  The daemon gives a Claude or Codex agent started in a pane a **read-only** conversation that follows the agent's
  own transcript ([ADR 0003](adr/0003-status-and-conversation-trust.md)); its link has
  `readOnly: true`, and the conversation is not listed as a session of its own (the pane is the session).
- **A chat session is the other kind of session** ([ADR 0003](adr/0003-status-and-conversation-trust.md)):
  a structured conversation the daemon started (`Conversation.execution` is `Structured`, `chat` is set without
  `readOnly`, `native_term` is none; `model/chatSession.ts` `isChatSession`). It is a sidebar row, tab and pane of its
  own, grouped by its repository (`ConversationSummary.root`, `branch`). It has **no terminal**: the pane is the chat
  always, there is no Terminal | Chat switch, `chat-toggle` does nothing, the terminal slot holds a one-line note
  (`NoTerminalNote`), and nothing that needs a pane number (find, restart, paste, broadcast, typed answers, the
  worktree menu, Inspect) is offered for it. Its composer is on (send, queue with Remove, Stop, attachments); an
  approval is answered by request id (y / a / n) and a user-input request by a form (`UserInputForm`), so neither
  points to a terminal.
  **A new session** (the sidebar +, Ctrl/Cmd+T) starts the default
  agent (Settings › General › Chat view › **Default agent**, Claude or Codex) in a new shell the daemon owns and types
  the agent's command into it, and opens the pane in the view **New sessions open as** names (Chat by default). It is
  one session either way: **the switch between Terminal and Chat never restarts the agent**
  ([ADR 0003](adr/0003-status-and-conversation-trust.md), v18 B.3). The chat reads the
  agent's own transcript (ADR 0003), and its composer **types into the agent's terminal** as one bracketed paste and
  Enter, only while the pane's trusted status says the agent is idle or done; while it works the composer is off and
  **Stop** sends Esc, and while it asks something in the terminal the chat shows a card, built from the agent's own
  `PermissionRequest` hook and never the screen: "<agent> wants to use <tool>" and one line of what it acts on. For
  Claude the card has **Approve once** (sends `1`) and **Deny** (sends Esc); for an agent whose answer keys are not
  verified (Codex) it says "Answer in the terminal." and sends nothing. The composer stays off while the card shows,
  because typed text would answer that prompt. Attachments are off for it. **New terminal** (the sidebar button, Ctrl/Cmd+Shift+T,
  a project's **New terminal here**, the palette) is a plain shell with no chat view. A chat session of ADR 0003, with
  no terminal, still starts from a shell's empty chat card (**Start a Claude chat** / **Start a Codex chat**) and
  stops the agent when closed (`closeChatSession`: end, then put away; if the stop fails it stays open).
- **Choosing the agent for one session.** A small chevron beside the sidebar's + opens **New Claude chat** / **New Codex chat**
  (the default is marked); the + still starts the default from Settings. It starts the chosen agent in the start folder and
  opens in the view Settings names. Beside a session on another machine it opens the New session modal on that machine, as + does.
- **Agent, model and reasoning.** Under the composer a button reads `Claude · Sonnet · Medium` and opens one popover with
  three radio columns (`AgentPicker`): Agent (the other agent is greyed out with "Fixed for this chat · start a new chat
  to use Codex"), Model (the agent's list, a one-line description each) and Reasoning (Low, Medium, High, Max with a
  four-bar meter). Picking ticks the choice and leaves the popover open; Esc or a click outside closes it; the footer says
  "Changes apply from your next message". The choice is kept per conversation in this window's storage
  (`agentChoice.ts`). The model lists are placeholders (`PLACEHOLDER_MODELS`) until the adapters report their own, and
  nothing yet sends the choice to the agent.
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
  ids as keys), follows the newest row until the reader moves up (any move up, however small, and a wheel move up at once),
  then offers "Jump to the latest message"; it follows again when the reader scrolls back down to within 48 px of the
  bottom (`followAfterScroll`). Rows
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
    diff in the Inspect panel's Changes view.
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
    A table (a header line, a `| --- |` delimiter row of the same width, then rows, with `:` for alignment) is a
    real `<table>` drawn as a hairline grid in existing tokens, inside its own named, focusable, sideways-scrolling
    region, so a wide table never widens the message; cells hold inline markdown, and a right-aligned column stays on
    one line.
    A reply still arriving (`running`) is read in its finished form (`streamingText`): an opened bold, italic or code
    span, or a link whose address is still arriving, reads as if it closed; a mark with nothing after it yet and a
    last line that is only a list, quote or heading marker are left out; a table waits for its delimiter row; an open
    code fence is untouched. A finished reply is read as written.
    Supported: paragraphs, headings, fenced code, lists, quotes, rules, inline code, bold, italic.
  - *Rows* are memoised on their events (`ChatItem`), and the virtual list measures heights, so a card that expands
    resizes its own row only.
- **While it waits.** The chat never looks stuck. First read of a history: `ChatSkeleton`, the shape of a conversation,
  static (an endless animation costs a repaint every frame, PERFORMANCE). A new session's chat is `StartingChat`, there
  at once with its message box; what is typed then is held per session (`firstMessage.ts`) and sent when the real chat can
  send, and a half-typed draft carries over. A message sent into the terminal is drawn at once as an `echo` row, with the
  working line under it, until the history has those words (`countUserMessages` against the count at send time); it is
  dropped after 20 s rather than left standing as sent. The **working line** says what the agent is doing from the newest
  step still running (`workPhase.ts`): "reading src/app.ts", "writing hello.txt", "running npm test", "thinking". It says
  only what the history says.
- **Mode and model.** A chip in the chat header says how Claude is set up, from its own transcript and never a guess:
  the permission mode (`Auto`, `Plan`, `Accept edits`, `Default`) then the model (`sonnet 5.5`). `Bypass permissions` is drawn
  in the danger colour, like Full access. It is read-only: the mode cycles with Shift+Tab in the terminal and the model with
  `/model`, and the chip follows at the agent's next line in its file. Nothing shows until the file has said it, and a
  Codex chat shows none yet (`chatMeta.ts`, `SessionMeta` in the read result).
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
- **Attachments** (ticket 20). A paperclip left of the text box (tooltip and `aria-label` "Attach files",
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
  before it is used. No chat sets a machine yet. A read-only chat shows no paperclip.
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
