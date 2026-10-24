<h1 align="center">seshi</h1>

<p align="center"><strong>Every coding-agent session, one calm place.</strong><br>
Run agents side by side, see which ones need you, and return to them later.</p>

<p align="center"><a href="#install">Install</a> · <a href="#start-in-five-minutes">Quick start</a> · <a href="#why-seshi">Features</a> · <a href="#configure">Configuration</a> · <a href="https://github.com/CydoEntis/seshi/releases">Releases</a></p>


<p align="center"><sub>Demo screens use synthetic project names, prompts, paths and agent output; no real session data is shown.</sub></p>

Run Claude Code, Codex, Gemini and friends side by side, see which ones need you, jump to them,
and keep them running when you close the window. Native on Windows, macOS and Linux (Omarchy
included). Inspired by [herdr](https://github.com/ogulcancelik/herdr),
[nebula](https://github.com/agentSystemLabs/nebula) and [fut](https://github.com/mikker/fut).

- **Agents keep running after you close the UI.** A background daemon owns every pseudoterminal
  (ConPTY on Windows). `<leader> q` detaches, and running `seshi` again reattaches.
- **Survives restarts.** Sessions are saved as they change; after a reboot `seshi` rebuilds them
  in the right folders and resumes agents (`claude --resume <id>`, `codex resume --last`, ...).
  Each pane's recent output is kept on disk too, so its history is still there to scroll
  back through, above a line marking the restart.
- **One screen for everything.** Choose **Floating**, **Tiled** or **Compact** in Settings → Appearance → Layout.
  Cards (the default) shows a full-height sidebar and pane cards, with a pane's tabs as chips in
  its title. Flat draws those
  same sessions and panes edge to edge, with square edges (Settings → Rounded shapes switches every tab, button and selected row to round ends together, in either layout); Tab, Shift+Tab
  and Esc go to the program in the pane in both layouts. The leader is
  `Ctrl+Space`: a sky-blue pill says it's armed, a pause shows the key map,
  and `?` opens **All keys**.
- **Find anything.** `<leader> j` is the inbox: what needs you first, then type to go to any
  project or session. `<leader> Space` searches every command in plain words, with or without a key, and **Menu** at
  the bottom of the sidebar lists the common ones. `<leader> ?` lists every configured
  binding (including aliases), the keys that need no leader and the commands only search finds.
- **Status at a glance.** Working (a quiet orange, its name shimmering), needs you (the theme's alert colour), done (green until
  you look), idle. A rising sound marks a question; a soft note marks a background completion.
  Click a sidebar status icon or pane status tag to see why, the evidence's age and any uncertainty.
- **Subagents counted on the row.** When an agent reports subagents (Claude from its hooks,
  Codex from its session files), its working row says `· 4 agents` (Settings › Agents ›
  Count on the session row). The agent lists them in its own pane; Seshi shows no tree.
- **Limits.** An agent stopped by a limit is told "continue" once the limit resets (Settings →
  Continue after a limit), and its card says when. Model, context and cost are the agent's own
  status line to show; seshi doesn't repeat them. Claude's limit numbers come from its status line:
  `seshi integrate claude` makes seshi's run first and then yours, which looks the same as before.
- **Detection without setup.** The process tree finds `claude`, `codex`, `gemini`, `opencode`,
  `cursor-agent`, `copilot`, `amp`, `qwen`, `aider` and others, and screen patterns tell
  working from blocked. Hooks (`seshi integrate claude`) make it exact.
- **Git without leaving.** New agents get their own worktree, including `claude` or `codex`
  typed into a shell in a repo's main folder (turn it off with *Own worktree per agent* in
  Settings; `--continue`/`--resume` stay put). Changes (`d`) shows the diff with
  review marks, then commit, merge, or open a pull request. Files (`f`), find (`F`), search the
  code (`/`), switch branch (`B`).
- **Everything is configurable.** The leader, every binding, commands bound to keys, themes and
  per-colour overrides, icons, the sidebar side, detection patterns and your own agents. Settings
  (`<leader> ,`) save to your config with its comments kept.
- **Scriptable.** `seshi split -- claude`, `seshi send`, `seshi read` and `seshi ls --json`
  work from any shell, including from an agent running inside seshi.

## Install

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/CydoEntis/seshi/main/install.ps1 | iex
```

The installers pick the build for your machine, check its checksum, put it in
`~/.local/bin` (Windows: `%LOCALAPPDATA%\Programs\seshi`, added to your PATH) and tell you if
another `seshi` comes first. No admin rights needed. `SESHI_VERSION=v0.3.0` picks a version;
`SESHI_INSTALL_DIR` a folder. If scripts are blocked (a locked-down work PC), download the archive
for your machine from [Releases](https://github.com/CydoEntis/seshi/releases) and put `seshi` on your PATH.
To update later: Seshi checks each time you open it (and every few hours while it stays
open). When a newer version is out, an **Update** button appears in the tab bar. It is hidden
when Seshi is current. You can also search for "Update seshi" (`<leader> Space`).
Review the version and release notes, then confirm to install. Your sessions keep running
until you choose **Restart now**: a button on the notice that follows the install, the
`↻ Restart` pill that replaces the Update button at the top right, the *Restart seshi* search
entry, and `R` in Settings (which also shows **Installed here**, **This window**, **Running
daemon** and **Restart pending**). One click restarts the server and reopens Seshi on the new
version in the same terminal; agents resume with their conversation (one that was mid-turn is
stopped and picks up where it left off). It asks only if an agent is working or waiting for you,
saying how many, and Enter confirms. The same happens if you install from outside Seshi
(`cargo install`, the installer, `seshi update`): the open window shows `↻ Restart`.
If you run a new `seshi` while an older server is still running, it replaces the server for you:
a one-line `Updating seshi…` when nothing is working, or one screen (Enter to continue, Esc to
quit) listing how many sessions resume when something is. Scripts, pipes and `--remote` still get
the error; `seshi kill-server` stops the server by hand. A remote daemon is updated on its host.
Stopping the daemon stops its processes; saved sessions are restored on reopening. From a
shell it's `seshi update` (which offers to restart the server; or `seshi update --check` to just
look). To look right now, Settings has **Check for updates** by the version when no newer one is
known (also found by search). Turn the automatic check off in Settings (Check for updates).

**Coming from hydra?** Seshi is its new name. Install seshi as above; the first time it starts,
it brings hydra's config, saved sessions and history over (hydra's own folders are left as they
were), and the next server start points your Claude hooks at seshi. Run `seshi integrate mcp` again
if agents used hydra's MCP tools, then uninstall hydra when you're happy.

Then run `seshi doctor` to check your setup. For installed providers with native status adapters
(Claude, Codex, Pi, OpenCode, OMP, Kilo, Mastracode and Kimi), it checks the machine-local setup.
For a missing adapter, it gives the
matching `seshi integrate <agent>` command; configured adapters may need that agent restarted or
reloaded. Doctor distinguishes installed configuration from live reporting: process and screen
detection remain available as a fallback until Seshi receives a native event. Setup lives in each
machine's agent config and is not shared by installing Seshi.
For the best look: a terminal with true colour and a
[Nerd Font](https://www.nerdfonts.com) (Windows Terminal, Ghostty, Alacritty, WezTerm, iTerm2).
Optional: `git` (worktrees, changes), `gh` (opening a pull request from Changes).

From source (any platform with Rust): `cargo install --git https://github.com/CydoEntis/seshi`.

## Start in five minutes

1. Install Seshi, then open a terminal in the repository you want to work on and run `seshi`.
2. Choose **+ new** (or press `Ctrl+Space`, then `n`) for a shell, and type `claude` yourself. `Ctrl+Space`, then `s`, starts your default agent at once instead; `Ctrl+Space`, then `c`, picks the agent, place and project first.
   In a Git checkout, a new Claude or Codex (typed in a Seshi shell, or started with `s` or `c`) uses a separate worktree by default; resumes stay
   in their existing checkout. A folder that is not a Git repository gets no worktree.
3. Read the sidebar state: orange means working, the alert colour means the agent needs you,
   green means its turn finished, and gray means idle. Select a row to return to that session.
4. Press **Ctrl+Space**, release it, then **?** to find shortcuts. Press **Ctrl+Space**, then
   **q** to detach; the daemon keeps sessions running, and `seshi` reopens them.
5. If status seems wrong, run `seshi doctor` and follow the provider-specific setup or reload
   guidance it prints.

## Use

Click **menu** at the bottom of the sidebar (or press the leader, then `m`) for the common
commands: new session, shell, pane, tab, Inbox, Search commands, All keys, the sidebar and Settings.
**Search commands** (leader, then `Space`) finds every command, including **Connect agents**
and your configured custom ones; rows click and the wheel scrolls.

```sh
curl -fsSL https://raw.githubusercontent.com/CydoEntis/seshi/main/install.sh | sh
```

The installer downloads the matching release, verifies its checksum and installs
Seshi without administrator access. See [all releases](https://github.com/CydoEntis/seshi/releases)
for manual installation, or run `seshi update` to update an existing install.

```
┌─ SESSIONS 5 ──────────── ●1 ✕1 ┐
│                                │
│  ▾ affiliate-site ⎇ main · ch… │
│ ▎ ✕ fix-footer              2m │
│ ▎   failed: test · exit 1      │
│ ▎   footer shows current year  │
│   ⣶ add-payments           now │
│     claude · working           │
│                                │
│  ▾ pixls                no git │
│   ● warm-finch              3m │
│     asks: use pnpm or npm?     │
│                                │
│  ▾ seshi                 ⎇ dev │
│   ✓ quick-fox               9h │
│     finished                   │
└────────────────────────────────┘
```

- The title counts the sessions, then `●` what needs you and `✕` the checks that failed.
- A project is its name, with its branch and check on the right (`⎇ main · check: test`), or
  `no git` outside a repo.
- Under it, its sessions: state mark, name and age, then a line saying what it is doing or
  what its check found (`failed: test · exit 1`, `asks: …`, `finished`). A failed check adds
  its first failing line on wide screens. A session that needs you has its question under it. Every new claude / codex gets its own worktree (named for you), so agents
  never edit the same files.
- One row is selected: a neutral tint with a **▎** bar at its left edge (lime while the sidebar has
  the keys, dim when a pane does). The open session keeps a dim bar while you browse with the
  keyboard. Hover is a separate, quieter tint.
- Hiding the sidebar leaves a slim rail with status indicators and two-letter titles.
  Click to switch sessions, hover for full names, scroll to reach more, and use the
  chevron to expand. Its red count includes blocked agents outside the visible rows.
- Things that need you sort to the top; everything else keeps its place. Idle agents stay running; no sleep screen is shown for sessions saved by older releases.
- A session taking a real share of the machine says so in its pane's title, beside its state
  (`▲ 38% CPU · 2.1 GB`, from a fifth of the CPU or two gigabytes).
- The sidebar footer keeps **+ new** and **Menu** available in both layouts. **+ new**, `Ctrl+Space n` and a project's **+** open a plain shell where you are; type your agent there. `Ctrl+Space s` starts a session straight away (Settings › Sessions sets the agent and whether it gets a worktree).
- `⇆` under a row names a file that this checkout and another have each changed since their
  branches parted (a merge conflict waiting to happen); more than two are one line with a count.
- Drag a group's name, or a session, up or down to reorder (what needs you still comes first);
  drop a session on another group (its name or one of its sessions) to move it there, within its
  section. Click to fold or open. With the sidebar focused
  (`Ctrl+Space e`), a row's menu letters work directly (a row's menu lists them): `x` (or
  Delete) asks to close, `r` renames. Settings distinguishes installed, window and daemon versions.

The leader key is `Ctrl+Space`. Press it and a sky-blue pill appears at the top right
(the pane you're in turns sky too); press a key, or wait a moment for the compact key map.
In that key map a key runs its command, Tab (or typing a letter that is no key) searches every command,
`?` shows all keys, and keys marked `›` open a second step (`w` worktrees: new, switch, merge,
delete; Backspace goes back). Every action has a leader key or is found by search; no letter
acts by itself in the sidebar.
See [contextual shortcuts](docs/shortcuts.md) for every screen's controls.
The table below lists the default leader bindings; custom bindings appear in All keys.

| keys (after the leader) | action |
|---|---|
| `j` | **inbox**: what needs you (answer a question with its number, Enter goes there), then every session; type to find one |
| `M` / `i` / `m` | message the current agent / explain its status / the Menu |
| `n` / `s` | a plain shell right where you are (cd and run what you like, or type `claude`; it groups itself) / a new session at once (your default agent; you type the task there) |
| `c` / `S` | new session with options (agent, where, project) / stop the agent (Ctrl+C) |
| `p` | a shell beside this one |
| `z` / `Z` / `x` | collapse the sidebar into the top strip / hide the sessions (the same key brings them back) / close |
| arrows | focus the pane that way (left past the edge: the sidebar). `Alt`+arrows do it without the leader (Settings → General turns that off) |
| `-` / `H J K L` | split down / resize left, down, up, right (`p` splits right) |
| `t` / `r` / `]` `[` / `X` | **tabs**: new tab (a shell where you are) / rename it in its pill (or double-click it) / next, previous / close it. `Alt+1`–`9` (no leader) goes to a tab |
| `w` / `f` / `d` | worktrees › / files / changes |
| `W` | open the worktree picker directly; choose a worktree or type a new branch |
| `e` / `Space` | focus the sidebar / search commands |
| `F` / `/` / `B` | find a file / search the code / switch branch |
| `R` / `V` | rename a session / paste the clipboard's image |
| `y` / `{` `}` / `PageUp` `PageDown` | select text with keys / previous, next command in history / scroll half a pane |
| `,` / `?` / `N` / `q` | settings / All keys / what happened / quit (agents keep running) |
| `Ctrl+R` | reload configuration |
| `Ctrl+Space` | send the configured leader key to the program |

Without the leader: **Alt+1–9** selects tabs; **Alt+arrows** moves pane focus
when enabled in Settings. All keys shows changes to these bindings too.

In **Files**: Enter puts the path in the agent's prompt, `e` opens it in your editor (`editor`
in config; nvim, helix … open inside seshi), `y` copies the path. In **Changes**: `c` commit,
`p` push and open a PR, `e` editor, `x` mark the file reviewed (it sinks; the mark clears
if the file changes again), `r` goes to its agent.

Everything also works with the mouse. Hold Shift to select text with your terminal.

### Helpers

Any session may start helpers: other agents in their own worktrees, through `seshi mcp`
(`seshi_start`). The first time one tries, its row turns **needs you** and says `wants to
start 4 helpers`. Press Enter on the row (or click the line) to see the tasks, then
**1** Allow, **2** Always in this project, **3** No, do it alone, or Esc to decide later.
Their permission prompts come to your Inbox, and nothing merges.

Settings › Sessions sets the rest: **Sessions may start helpers** (`ask`, `always` or
`never`; `never` refuses with a clear error to the agent) and **Helpers at once** (2, 3 or 4
per session; an extra one is told to wait). "Always in this project" is kept in
`helper-projects.json` in Seshi's data folder, not in the repo, so an agent cannot give itself
helpers. An older `[mcp] helpers_at_once` in `config.toml` still counts until `[sessions]
helpers_at_once` is set. The commander and `[team]` jobs are gone; an old `config.toml` that
still has them loads, and they are ignored.

## Panes are real terminals

- **Scroll:** the wheel, or `PageUp` / `PageDown` at a prompt (full-screen programs keep those
  keys). Outside full-screen mode, `Shift+PageUp` / `Shift+PageDown` scroll half-pages and
  `Ctrl+Shift+Up` / `Ctrl+Shift+Down` scroll individual lines. A scrollbar shows when there's history;
  click or drag it. Typing (or clicking for the program) goes back to the bottom.
- **Mouse:** programs that use it (vim, lazygit, htop, full-screen agents) get clicks, wheel and
  drags. Hold Shift to select text yourself.
- **Copy:** drag to select; double-click copies a word, path or link. Programs that copy (OSC 52,
  e.g. Claude's `/copy`, nvim) put it on your clipboard. Ctrl+click opens a link.
- **Images:** `Ctrl+Space V` (or `Ctrl+Space Ctrl+V`, or a paste while an image is on the
  clipboard) saves the image and pastes its path; Claude Code and Codex attach it.
- **Keys:** on Windows, combos plain terminals can't send (Ctrl+Shift+letter, Ctrl+Enter, Ctrl+Tab)
  reach programs exactly, as native key records. Shift+Enter is still a newline for agents.
- **Resize:** text re-wraps to the new width (history included). Programs' cursor shape (bar,
  block, underline) shows. Synchronized redraws are drawn whole, so agents don't flicker.
- **Closing** a pane ends everything running in it after you confirm.

## Alerts

Needs-you alerts ring even in the focused pane and use two rising notes. A background
session's completion uses one soft note; the session you're viewing stays quiet. Other panes get an in-app completion notice. A needs-you alert is delayed briefly
to ignore transient prompts and is cancelled if the agent resumes work. Sessions started
by another agent do not alert when they finish. With no seshi window open, the server
plays the needs-you sound and shows a desktop notification. `[notify] sound = false`
(Settings → Alert sounds) turns sounds off; `seshi test-alert` plays both sample tones.

**On your phone** (off until you set it up): install the free [ntfy](https://ntfy.sh) app (no
account), subscribe to a topic name nobody would guess, and put it in Settings → Phone alerts
(`[notify] phone_topic`). An alert goes once an agent has needed you for a minute with nobody
answering (`phone_after`), so it stays quiet while you're at your desk; `phone_done` adds
finished ones. Alerts say which agent and where; `phone_text` adds what it's doing. On the
public server the topic name is all that keeps them private (`phone_server` for your own).
`seshi test-alert` sends a test.

Click a notification to go to its session: seshi comes to the front on it (its split too).
That works for the note inside seshi and the desktop notification on Windows and Linux (on
Omarchy / Hyprland the window is brought forward too); on macOS, install
`terminal-notifier` for clickable ones.

## Sessions from older versions

Seshi no longer stops idle agents automatically; they keep running until you close them or
their own agent process exits. Sessions saved while sleeping by an older version recover when you open them. Seshi does not
show a separate sleep screen or wake animation.

## Session visibility

Sessions stay visible in the sidebar until you close them. Idle, completed, and merged-worktree sessions are never hidden automatically.

## Follow-ups and snapshots

`m` on an agent's row writes it a follow-up without leaving where you are. While the agent is
working, Enter leaves the message with seshi and it's typed in when the turn is over (never
into a question the agent is asking); Ctrl+Enter sends it at once. What's waiting shows under
the row, and the row's menu drops it.

Each time an agent's turn begins, seshi keeps the checkout's files as they are (tracked or
not; never what git ignores). **Snapshots…** in the agent's menu lists the last nine;
picking one puts the files back as they were then. What's there now is kept as a snapshot
first, so a restore can be taken back the same way. It is refused while the agent is working,
since it would keep writing into the files being put back. Snapshots are commits under the
worktree's own refs (`refs/worktree/seshi/snapshots`): no branch, nothing in your history,
your staging untouched, the newest 30 kept. `[snapshots] enabled = false` turns them off.

## Checks

A check is a command Seshi runs after each agent turn, such as your tests: a session whose
check fails reads **failed**, not done, until the check passes or you override it. Name one
or more per project as `[[check]]` tables in config.toml (`build`, `test`, …; each is a
checkpoint), or let a repo ship its own in `.seshi/checks.toml`.

- A repo's command never runs until you approve it: Enter on its row opens the approval.
  Enter, `y` and Space do nothing there; `1` runs it once, `2` twice always runs this exact
  command, `3` never runs it (the session reads **not checked**). A changed command asks again.
  Your answers live in Seshi's data folder, never in the repo.
- Enter on a failed row opens its evidence: the command, where and when it ran, and its output.
  `r` re-runs, `O` overrides with a reason (the failure stays visible), `u` undoes that, Tab
  moves between checkpoints.
- A pass goes **stale** when the files change afterwards; `r` runs it again.
- `<leader> j` (Inbox) lists failed checks and approvals with what needs you.

## Merging

In Changes, `m` merges a worktree's branch into the main one, then closes its
sessions and removes the worktree and the branch. Closing the last session in a worktree
seshi made removes the folder; its branch goes too when it's already merged.

## Recipes

```toml
[[recipes]]
name = "feature"
worktree = true                           # its own worktree
run = ["claude", "npm run dev", "lazygit"] # the first is the main one
```

They show up in + New as `⚙ feature`.

## Agents that steer agents (MCP)

```sh
seshi                 # open the current directory
seshi ~/code/my-app   # open a project
```

Use **+ new** or **Menu** at the bottom of the sidebar to start a session,
open a shell, split a pane, or reach Settings. For example, the CLI can start an
agent directly:

```sh
seshi new ~/code/my-app -- claude
seshi worktree feature/login -- codex
```

The worktree command creates or opens a separate checkout for the task. When an
agent asks for input, its status turns red and it appears in the Inbox. Select it,
answer, and return to your other work.

## Essential keys

The default leader is **Ctrl+Space**. Press it, release, then press the next key.
Open the full, searchable list with **Ctrl+Space**, then **?**. The list reflects
your configured bindings and includes controls for each screen.

`Ctrl+Space ,` opens **Settings** in a right-hand drawer that slides beside the panes
(on a narrow window it uses the pane column). Sections (Tab cycles): General ? Sessions ? Appearance ? Agents ?
Keys. Values are chips you click (or ←→ / Enter). General is in sections (INPUT, LAYOUT, SHELL, UPDATES): **Layout** (Floating, Tiled or Compact) and **Background**
(your terminal's own, so transparency shows through evenly, or solid) are in LAYOUT, and one line at the
top says which seshi this is and whether it is current (`u` checks now). Appearance has the
Cards pane controls and a **Theme**
browser. **Working spinner** previews Braille, Dots, Line (ASCII), Circle, Arrows,
Pulse, Blocks and Static. Choose **Custom** to type or paste space-separated frames.
Each frame must fit one terminal column; up to 64 frames are supported. Enter
applies the choice, Esc cancels, and the change takes effect immediately.

The Theme control opens a browser of 108 themes. Type a name or flavor to filter, use arrows or
PgUp/PgDn to move, and preview the palette before pressing Enter to apply it. Click a row
to preview and click it again to apply; Esc returns without changing your theme.

The catalog includes all four Catppuccin flavors, Gruvbox Dark/Light, Nord, Dracula,
Tokyo Night Night/Storm/Moon/Day, Solarized Dark/Light, One Dark, One Half Dark/Light,
Kanagawa Wave/Dragon/Lotus, Ayu Dark/Mirage/Light, Night Owl, Nightfly, Moonfly,
Everforest Dark/Light, Rosé Pine/Moon/Dawn, Cobalt2, Jellybeans, Iceberg, Sonokai,
Shades of Purple, Panda, Lucario, Gotham, Apprentice, Zenbones Dark/Light, Nightfox/Duskfox/Carbonfox,
GitHub Dark/Dark Dimmed/Light, Gruvbox Material, Gruvbox Baby, Oceanic Next, Tomorrow/Night/Eighties/Bright,
Base16 Default Dark, Palenight, One Light, Monokai Pro, Oxocarbon, Cyberdream, Nordic, OneNord, Edge,
VSCode Dark, Code Dark, Melange, Falcon, Embark, Lackluster, Miasma, Bamboo, Mellifluous, Mellow,
Kanagawa Paper Ink, Zephyr, Koda, Aquarium, Fluoromachine, SynthWave '84, Horizon, Material Darker,
Modus Vivendi/Operandi, and Bluloco Dark, alongside
Seshi Dark (the default) and Seshi Light, Seshi Walnut/Night/Day, Harbor/Day, Moss/Day, Ember/Ash,
Ink/Paper, Arcade, Hydra, Tinkery/Light, Aurora, Moonlit, and the existing themes. [Palette sources and variant notes](docs/theme-credits.md).
New palettes theme ANSI agent output; explicit RGB output keeps the program's colors.
UI text and status colors are adapted for contrast. Existing theme overrides remain active.
On Keys, Enter then press a new key to rebind a shortcut. Changes save to `config.toml`
with comments preserved and apply at once.

Seshi groups agents and terminals by project. The sidebar shows each session's
status, project and Git branch. Select a session to focus it; hover only previews
it. Hide the sidebar to use the compact status rail, which still shows sessions
and needs-you alerts.

The background daemon owns every terminal pane. It saves the layout and recent
output, restores sessions after Seshi restarts, and resumes supported agents where
their CLI provides a resume command. Closing Seshi does not close its sessions.

In a Git repository, new agent sessions use separate worktrees by default. Use
**Changes** to inspect a diff, mark files reviewed, commit, merge or open a pull
request. You can change the worktree behavior in Settings.

## Configure

```sh
seshi config init   # writes the annotated example to the config path
seshi config path   # %APPDATA%\seshi\config.toml, or ~/.config/seshi/config.toml
```

See [`config.example.toml`](config.example.toml) for every option. Some highlights:

```toml
prefix = "ctrl+a"
theme = "tokyo-night"          # browse all choices in Settings > Appearance > Theme

[theme_overrides]
accent = "#ff9e64"

[keys.prefix]
C = "spawn-right:claude"       # any command, in a split or a new tab
g = "spawn-tab:lazygit"
x = "none"                     # unbind a default

[keys.global]                  # no prefix needed
"alt+h" = "focus-left"

[[agents]]                     # teach it a new agent, or tune a built-in by name
name = "my-agent"
process = ["my-agent"]
working_patterns = ["esc to interrupt"]
blocked_patterns = ["\\(y/n\\)"]
```

## Never leave seshi

| Leader + | Panel | What it does |
|---|---|---|
| `f` | **Files** | *Recent*: files that just appeared in Downloads, Desktop, Documents or the project (docs, zips, screenshots). *Project*: fuzzy search over the project's files. Enter types the path into your prompt; `^O` opens it, `^F` shows it in the file manager, `^Y` copies the path. |
| `d` | **Changes** | A worktree's changed files and the diff: `c` commit, `m` merge into the base branch, `p` push and open a pull request, `r` go to its agent, `x` throw it away. |

Typing in a panel filters it; the action keys that would collide with typing use Ctrl.

## Agents

The new-agent dialog starts any agent in `[quick] agents`; `{prompt}` is the task, quoted for
your shell. Agents are just commands, so anything works:

```toml
[quick]
agents = [
  { name = "claude", command = "claude {prompt}" },
  { name = "codex", command = "codex {prompt}" },
  { name = "opus", command = "claude --model opus {prompt}" },
]
```

### Accounts

An agent entry with an `env` of its own is an account: the same agent, signed in as someone
else. It shows up wherever you pick an agent, and a session keeps its account through sleep,
a move to another worktree, and a restart.

```toml
[[quick.agents]]
name = "claude work"
command = "claude {prompt}"
env = { CLAUDE_CONFIG_DIR = "~/.claude-work" }
```

Set one up once, from any terminal: `seshi account "claude work" -- claude` to sign in, and
`seshi account "claude work" -- seshi integrate claude` so that account reports its status
to seshi too. When one account hits its limit, start the next session on the other.

## Restarts

The session (workspaces, tabs, splits, each pane's directory and command, agent sessions) is
saved to the data directory a couple of seconds after it changes. On the next start:

- Panes reopen in their last known directory. For PowerShell, that requires your prompt to report it
  via OSC 7 or OSC 9;9, as oh-my-posh and starship can. Other shells are tracked automatically.
- Agents resume. With hooks installed, seshi knows the exact session (`claude --resume <id>`).
  Otherwise it uses the agent's "most recent" form (`claude --continue`, `codex resume --last`).
- Panes started with a command (`spawn-right:lazygit`, `seshi split -- x`) run it again, except
  an agent's: `codex "<task>"` would start the task over, so an agent that can't be resumed
  opens as a shell.

`seshi kill-server` keeps the session for next time, and `seshi kill-server --forget` discards it.
Closing every pane yourself also starts fresh. A burst of panes dying at once, as at logoff or a
crash, is not recorded, so the last good layout survives. Configure all of this under `[restore]`,
and per agent with `resume` / `resume_last`. Each pane's recent output is kept on disk so it is
there after a restart; it can hold secrets that were on screen, so `output = false` (Settings ›
Sessions › Keep each pane's recent output) stops saving it and deletes what was saved.

## Worktrees

```sh
seshi worktree feat/login -- claude      # new branch from HEAD, opened with claude running
seshi worktree fix-123 --base origin/main
seshi worktree-remove [--force]          # closes the workspace and removes the checkout
```

New worktrees go to `{repo_parent}/{repo}-worktrees/{branch}` (set `[worktree] dir`). An existing
local or `origin/` branch is checked out instead of created. Set `[worktree] command = "claude"`
to start an agent in every new worktree.

With **Own worktree per agent** enabled (the default), typing `claude`, `codex`, `gemini`,
`opencode` (or a quick agent's name) in a Seshi shell in a Git repository's main checkout
creates a worktree and starts the agent in it, including with `-c` configuration
or `-p` profile options. `codex resume` and `codex fork` stay in the current folder;
explicit `--cd` and Codex's own `--worktree` are honored. Starting inside an existing
linked worktree reuses it, and a folder that is not a Git repository gets no worktree and no
error. This applies to shells opened by Seshi, whatever shell they run (bash, zsh, fish,
PowerShell, nu, cmd, or a `shell` setting with arguments): each has Seshi's small launcher for
each agent first on its `PATH`, so an alias or function of your own, a shim manager, `command
claude`, or a shell started inside the pane (tmux, `bash -l`) still reaches it. The launcher
asks Seshi, which checks the pane's own secret, so only a plain shell in that pane can ask. Two
cases it cannot cover: an alias that runs another command or an absolute path (`alias
claude=~/bin/claude`), and a shell that rebuilds `PATH` without the launcher folder and never
puts it back.

**Cleaning up.** Closing the last session in a worktree seshi made removes its folder (never
with uncommitted changes; the branch is kept unless it's merged). One left behind by a session
that crashed is removed the same way a few minutes later, once nothing in it exists only there.
For everything else there is **Clean up worktrees** (a project's menu, or search):
every worktree of the repo with why it's still around (in use, uncommitted changes, not merged,
when it last committed). The safe ones come ticked; tick any others, and one confirm removes
them. It says so first when ticked ones hold uncommitted changes, which would be lost.

## Copy mode

| key | |
|---|---|
| `h j k l`, arrows, `w b`, `0 ^ $`, `g G`, `H M L` | move |
| `Ctrl-u/d`, `Ctrl-b/f`, `PgUp/PgDn`, wheel | scroll |
| `v` / `V` | select characters / lines |
| `y`, `Enter` with selection | copy selection and leave; without selection, `y` copies the line and Enter exits without copying |
| `/` `?` then `n` `N` | search forward / backward (smart case) |
| `Tab` / `Shift+Tab` | keep this selection and go to the next / previous pane of the split, the same search there; `y` then copies them all, each under its pane's name |
| `q`, `Esc` | leave |

`Esc` clears an active selection before exiting; **Ctrl+C** exits immediately.
Without a selection, `y` or `Y` copies the current line; Enter exits without copying.
**Ctrl+Y / Ctrl+E** scroll one line up / down. Full details: [Copy mode controls](docs/shortcuts.md#copy-mode).

Copies go to the system clipboard and, through OSC 52, to your outer terminal (works over SSH).

## Agent status

Seshi detects common command-line agents automatically and combines process,
terminal-screen and native lifecycle evidence where available. Status means:

1. **Native hooks** report lifecycle events. `seshi integrate claude` adds hooks to `~/.claude/settings.json`.
   They stay inert outside seshi panes and are tagged so a re-run or `--uninstall` replaces them cleanly.
   Any tool can report its own state with `seshi hook <name> --status working|blocked|done|idle`.
   `seshi integrate gemini` and `seshi integrate qwen` do the same for Gemini CLI and Qwen Code;
   `seshi integrate opencode` writes a small OpenCode plugin that tells seshi what it's doing.
   `seshi integrate pi` installs Pi's native extension. These, and Codex's hooks, can also
   be installed with the mouse through **Settings › Agents → Set up another agent…** (or search for Connect agents). Restart Codex or
   OpenCode after setup, or reload Pi. Status follows the app with any provider, including
   z.ai and DeepSeek; see [status setup and requirements](docs/agent-status.md).
   For Codex, `seshi integrate codex` adds native `SubagentStart` / `SubagentStop`
   hooks in `~/.codex/config.toml`, so helper events carry their parent session ID;
   it also configures `notify` when free, preserving another program's notify.
2. **Screen evidence**: 22 pinned Herdr manifests match live screen regions, OSC
   titles/progress and paired dialog controls. A fresh visible blocker can override
   non-authoritative hook state; a transcript viewer preserves the previous state.
3. **Activity**: recent output that isn't the echo of your own typing.

A turn that finishes while you're looking elsewhere is **done** until you focus that pane.
An unanswered Codex queued or expanded question takes priority over its working spinner and shows
**needs you**. Finished prose that mentions the interrupt shortcut does not count as active
work. Codex reports turn completion through its notify hook; Seshi detects the next
turn's activity footer, including soft wraps and background-terminal details. New
input after a finished turn allows work detection immediately or after a delayed start.
An advancing footer also detects automatic follow-up work without keyboard input.
Native Codex subagent hooks attach every helper to its parent pane as soon as it
starts and remove it when it stops.
Hook authority ends when a recognized agent's process exits. Seshi recognizes
Herdr's 24 agent identities plus its five additional existing agents; OMP,
Mastracode, Kilo and Kimi native status setup is also available through the Menu.
See [status setup and its limits](docs/agent-status.md).
Terminal bells do not replace the status icon or add musical-note history entries.
Click a status icon/tag (or `<leader> i`) for a plain-language explanation and the ages of
hook, screen and process evidence; a status without confirmation is shown as uncertain.

## Configure

Open Settings from the **Menu** or press **Ctrl+Space**, then `,`. Choose a theme,
layout, spinner, keys, sidebar behavior and agent settings. Settings are saved in
your user config; `config.example.toml` lists the available options.

```sh
seshi config init   # create a commented config file
seshi config path   # print its location
seshi doctor        # check the local setup
```

Seshi also provides a CLI and MCP tools for agents and scripts:

```sh
seshi ls --json
seshi split -- claude
seshi send -p 3 "run the tests"
seshi read 3
seshi handoff 3 --to codex
seshi integrate mcp
```

`seshi handoff` starts another agent beside pane 3, in the same checkout, with a short
text brief (task, branch, changed files, recent output); it does not carry the first
agent's conversation or memory.

For more detail, see [status setup](docs/agent-status.md),
[screen-specific controls](docs/shortcuts.md),
[all configuration options](config.example.toml), and the
[architecture overview](docs/ARCHITECTURE.md).

## Requirements

Seshi supports Windows, macOS and Linux. Use a terminal with true color for the
intended appearance; a Nerd Font adds optional icons. Git enables worktrees,
branch management and Changes. GitHub CLI (`gh`) enables pull-request actions.

## Build from source

Install Rust, clone this repository and run:

```sh
cargo build --release
cargo test
```

See [architecture](docs/ARCHITECTURE.md), [testing](docs/TESTING.md) and
[contributing standards](docs/CODE-STANDARDS.md) before changing the project.

## License

MIT.
