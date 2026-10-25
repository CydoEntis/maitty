# Maitty guide

How each part of Maitty works, in more depth than the [README](../README.md). Shortcuts are
in [shortcuts](shortcuts.md); every setting is in [`config.example.toml`](../config.example.toml).

## Using it

Click **menu** at the bottom of the sidebar (or press the leader, then `m`) for the common
commands: new session, shell, pane, tab, Inbox, Search commands, All keys, the sidebar and Settings.
**Search commands** (leader, then `Space`) finds every command, including **Connect agents**
and your configured custom ones; rows click and the wheel scrolls.

```
┌─ SESSIONS 5 ──────────── ●1 ✕1 ┐
│                                │
│  ▾ affiliate-site ⎇ main · ch… │
│ ▎ ✕ fix-footer    ◆  +8 −2  2m │
│ ▎   failed: test · exit 1      │
│ ▎   footer shows current year  │
│   ⣶ add-payments  ✻ +40 −3 now │
│                                │
│  ▾ pixls                no git │
│   ● warm-finch           ◆  3m │
│     asks: use pnpm or npm?     │
│                                │
│  ▾ orbit                 ⎇ dev │
│   ✓ quick-fox            ✻  9h │
└────────────────────────────────┘
```

- The title counts the sessions, then `●` what needs you and `✕` the checks that failed.
- A project is its name, with its branch and check on the right (`⎇ main · check: test`), or
  `no git` outside a repo.
- Under it, its sessions, one line each: the state mark and the name (in its state's colour
  while it is worth a look), then at the right the agent's mark (`✻` Claude, `◆` Codex, `✦`
  Gemini, `◇` OpenCode, else its first letter), how full its context is once past half
  (`✻ 72%`, red from 85%; Claude and Codex report it), the lines changed but not committed
  (`+8 −2`) and the age. Click the `+N −N` to open its Changes.
- A second line shows only when it says more: a question (`asks: …`), a check result
  (`failed: test · exit 1`, with its first failing line on wide screens), helpers asked for,
  a quiet spell or a message waiting. Every new claude / codex gets its own worktree (named
  for you), so agents never edit the same files.
- One row is selected: a neutral tint with a **▎** bar at its left edge (lime while the sidebar has
  the keys, dim when a pane does). The open session keeps a dim bar while you browse with the
  keyboard. Hover is a separate, quieter tint.
- Collapsing the sidebar (leader, then `z`) moves the sessions into a strip of chips along
  the top row: click a chip to go to it, `»` (`«` with the sidebar on the right) or `+n more` to expand the sidebar again.
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
  Delete) asks to close, `r` renames. Settings › General › UPDATES shows the version; Settings warns at the top when the installed, window and daemon versions differ.

The bar along the bottom always says which mode you are in: `TYPING` while keys go to the
pane. The leader key is `Ctrl+Space`. Press it (or click the bar) for **shortcut mode**: the
bar turns sky blue and suggests keys (`n shell`, `f files`, `? all keys`, `Esc type`, …; a click
on one runs it), the pane you're in turns sky too, each key below runs its action, and the mode
stays for the next key. A key
that does nothing says "Shortcut mode — Esc to type". `Esc`, `Ctrl+Space` again or a click into a pane
go back to typing, and so does a new shell, session, tab or split: the next key goes to the
new pane. Closing a popup returns to shortcut mode. While the sidebar, a popup or a panel has
the keys, the bar names it and says how to use and leave it (`FILES · / filter · i edit · Enter
insert path · Esc close`). With **Leader waits for next key** on (Settings › General; off by default), a pause after the leader shows the compact key map: a key runs its
command, Tab (or typing a letter that is no key) searches every command, `?` shows all keys,
and keys marked `›` open a second step (`g` worktrees: new, switch, merge, delete; Backspace
goes back). No default but `?` needs Shift, and each key does one thing; no letter acts by itself in
the sidebar. Every default shortcut, and every screen's own keys, are in
[shortcuts](shortcuts.md).

Without the leader: **Alt+1–9** selects tabs; **Alt+arrows** moves pane focus
when enabled in Settings. All keys shows changes to these bindings too.

**Files** opens in the side panel beside your panes: the tree, and the file you pick beside
it (on a narrow panel the file takes the panel, `‹` goes back). `f` again or Esc closes it;
Ctrl+click on a `path:line` in a pane opens it at that line. In **Files**: Enter puts the
path in the agent's prompt, `e` opens it in your editor (`editor` in config; nvim, helix …
open inside maitty), `y` copies the path, `r` lists recent files.

Files, Changes and Branch share the side panel: a tab strip at its top names each with its
key, and Tab / Shift+Tab (or a click on a tab) moves between them for the same checkout.

In **Changes**: `c` commits every changed file and `p` pushes and opens a PR (each asks
first), `e` opens the file in your editor, `x` marks it reviewed (it sinks; the mark clears
if the file changes again), `r` goes to its agent. In a worktree, `m` merges and `d` throws
it away, also after asking.

The side panel's width is kept for each view. While the panel has the keys, `[` and `]`
narrow or widen it and `=` puts it back (after the leader, `[` and `]` switch tabs instead);
or drag its left edge. A markdown file in Files is drawn by default; `m` switches
between drawn and raw text.

Everything also works with the mouse. Hold Shift to select text with your terminal.

### Helpers

Any session may start helpers: other agents in their own worktrees, through `maitty mcp`
(`maitty_start`). The first time one tries, its row turns **needs you** and says `wants to
start 4 helpers`. Press Enter on the row (or click the line) to see the tasks, then
**1** Allow, **2** Always in this project, **3** No, do it alone, or Esc to decide later.
Their permission prompts come to your Inbox, and nothing merges.

Settings › Sessions sets the rest: **Sessions may start helpers** (`ask`, `always` or
`never`; `never` refuses with a clear error to the agent) and **Helpers at once** (2, 3 or 4
per session; an extra one is told to wait). "Always in this project" is kept in
`helper-projects.json` in Maitty's data folder, not in the repo, so an agent cannot give itself
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
- **Images:** `Ctrl+Space`, then `Ctrl+V` (or a paste while an image is on the
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
by another agent do not alert when they finish. With no maitty window open, the server
plays the needs-you sound and shows a desktop notification. `[notify] sound = false`
(Settings → Alert sounds) turns sounds off; `maitty test-alert` plays both sample tones.

**On your phone** (off until you set it up): install the free [ntfy](https://ntfy.sh) app (no
account), subscribe to a topic name nobody would guess, and put it in Settings → Phone alerts
(`[notify] phone_topic`). An alert goes once an agent has needed you for a minute with nobody
answering (`phone_after`), so it stays quiet while you're at your desk; `phone_done` adds
finished ones. Alerts say which agent and where; `phone_text` adds what it's doing. On the
public server the topic name is all that keeps them private (`phone_server` for your own).
`maitty test-alert` sends a test.

Click a notification to go to its session: maitty comes to the front on it (its split too).
That works for the note inside maitty and the desktop notification on Windows and Linux (on
Omarchy / Hyprland the window is brought forward too); on macOS, install
`terminal-notifier` for clickable ones.

## Sessions from older versions

Maitty no longer stops idle agents automatically; they keep running until you close them or
their own agent process exits. Sessions saved while sleeping by an older version recover when you open them. Maitty does not
show a separate sleep screen or wake animation.

## Session visibility

Sessions stay visible in the sidebar until you close them. Idle, completed, and merged-worktree sessions are never hidden automatically.

## Follow-ups and snapshots

`Enter` in shortcut mode (or **Message** in the row's menu) writes an agent a follow-up without leaving where you are. While the agent is
working, Enter leaves the message with maitty and it's typed in when the turn is over (never
into a question the agent is asking); Ctrl+Enter sends it at once. What's waiting shows under
the row, and the row's menu drops it.

Each time an agent's turn begins, maitty keeps the checkout's files as they are (tracked or
not; never what git ignores). **Snapshots…** in the agent's menu lists the last nine, and
the Branch view (leader + `b`) lists them all; picking one puts the files back as they were then. What's there now is kept as a snapshot
first, so a restore can be taken back the same way. It is refused while the agent is working,
since it would keep writing into the files being put back. Snapshots are commits under the
worktree's own refs (`refs/worktree/maitty/snapshots`): no branch, nothing in your history,
your staging untouched, the newest 30 kept. `[snapshots] enabled = false` turns them off.

## Checks

A check is a command Maitty runs after each agent turn, such as your tests: a session whose
check fails reads **failed**, not done, until the check passes or you override it. Name one
or more per project as `[[check]]` tables in config.toml (`build`, `test`, …; each is a
checkpoint), or let a repo ship its own in `.maitty/checks.toml`.

- A repo's command never runs until you approve it: Enter on its row opens the approval.
  Enter, `y` and Space do nothing there; `1` runs it once, `2` twice always runs this exact
  command, `3` never runs it (the session reads **not checked**). A changed command asks again.
  Your answers live in Maitty's data folder, never in the repo.
- Enter on a failed row opens its evidence: the command, where and when it ran, and its output.
  `r` re-runs, `O` overrides with a reason (the failure stays visible), `u` undoes that, Tab
  moves between checkpoints.
- `m` on a failed check, in its evidence or its Inbox item, opens the agent's message box with
  the command, its exit code and the last 40 lines of its output written in. Add what you want
  done and press Enter; nothing is sent before.
- A pass goes **stale** when the files change afterwards; `r` runs it again.
- `<leader> i` (Inbox) lists failed checks and approvals with what needs you.

## Merging

In Changes, `m` merges a worktree's branch into the main one, then closes its
sessions and removes the worktree and the branch. Closing the last session in a worktree
maitty made removes the folder; its branch goes too when it's already merged.

## Recipes

```toml
[[recipes]]
name = "feature"
worktree = true                           # its own worktree
run = ["claude", "npm run dev", "lazygit"] # the first is the main one
```

They show up in + New as `⚙ feature`.

## Agents that steer agents (MCP)

`maitty integrate mcp` registers `maitty mcp` with every agent you have that takes one, and
prints a line per agent (registered, already registered, or skipped when it isn't installed):

- Claude Code, through `claude mcp add --scope user` (every project);
- Codex, as `[mcp_servers.maitty]` in `$CODEX_HOME/config.toml` (or `~/.codex/config.toml`);
- Qwen Code and Gemini CLI, as `mcpServers.maitty` in `~/.qwen/settings.json` and
  `~/.gemini/settings.json` (`$QWEN_HOME` is Qwen's config folder itself; `$GEMINI_CLI_HOME`
  stands in for the home folder, so Gemini's is `$GEMINI_CLI_HOME/.gemini`).

Each entry runs this `maitty` with `args = ["mcp"]`; the rest of each file, comments included,
is kept, and a backup sits beside it. An agent whose config folder doesn't exist is skipped, not
set up. Running it again changes nothing; `--uninstall` removes only Maitty's entry. Restart a
running agent to load the server. `maitty doctor` shows which agents have it. An agent then has tools to
list sessions (`maitty_list`), read a screen (`maitty_read`), send a message
(`maitty_send`), wait for a turn to end (`maitty_wait`), answer a prompt (`maitty_answer`),
interrupt (`maitty_interrupt`) and start helpers (`maitty_start`, see *Helpers* above).
`[mcp] approve` sets whether agents may answer "yes" to prompts and `[mcp] scope` whether
they reach only their project or every session; `maitty grant <pane> read,write,…` changes
one pane's rights.

## From any shell

```sh
maitty                 # open the current directory
maitty ~/code/my-app   # open a project
```

Use **+ new** or **Menu** at the bottom of the sidebar to start a session,
open a shell, split a pane, or reach Settings. For example, the CLI can start an
agent directly:

```sh
maitty new ~/code/my-app -- claude
maitty worktree feature/login -- codex
```

The worktree command creates or opens a separate checkout for the task. When an
agent asks for input, its status turns red and it appears in the Inbox. Select it,
answer, and return to your other work.

## Never leave maitty

| Leader + | Panel | What it does |
|---|---|---|
| `f` | **Files** | The project's tree, with the file you pick beside it. `r` switches to recent files (just appeared in Downloads, Desktop, Documents or the project), `/` filters. Enter types the path into your prompt; `e` your editor, `i` edit it in place (Shift+arrows or the mouse select; Ctrl+C, Ctrl+X, Ctrl+V copy, cut and paste), `o` open it outside, `y` copy the path. `f` again or Esc closes it. |
| `c` | **Changes** | A checkout's changed files and the diff: `c` commit, `p` push and open a pull request, `x` mark reviewed, `r` go to its agent; in a worktree `m` merges into the base branch and `d` throws it away. Each of `c`, `p`, `m` and `d` asks first. |
| `b` | **Branch** | Where the session's checkout stands, in the side panel: its branch, how far it is ahead of and behind its upstream and its base branch, the last 10 commits, the last check result and its snapshots. Its first row, **Switch branch…**, opens the branch switcher; Enter on a snapshot restores it (asked first). `b` again or Esc closes it. |

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

Set one up once, from any terminal: `maitty account "claude work" -- claude` to sign in, and
`maitty account "claude work" -- maitty integrate claude` so that account reports its status
to maitty too. When one account hits its limit, start the next session on the other.

## Restarts

The session (workspaces, tabs, splits, each pane's directory and command, agent sessions) is
saved to the data directory a couple of seconds after it changes. On the next start:

- Panes reopen in their last known directory. For PowerShell, that requires your prompt to report it
  via OSC 7 or OSC 9;9, as oh-my-posh and starship can. Other shells are tracked automatically.
- Agents resume. With hooks installed, maitty knows the exact session (`claude --resume <id>`).
  Otherwise it uses the agent's "most recent" form (`claude --continue`, `codex resume --last`).
- Panes started with a command (`spawn-right:lazygit`, `maitty split -- x`) run it again, except
  an agent's: `codex "<task>"` would start the task over, so an agent that can't be resumed
  opens as a shell.

`maitty kill-server` keeps the session for next time, and `maitty kill-server --forget` discards it.
Closing every pane yourself also starts fresh. A burst of panes dying at once, as at logoff or a
crash, is not recorded, so the last good layout survives. Configure all of this under `[restore]`,
and per agent with `resume` / `resume_last`. Each pane's recent output is kept on disk so it is
there after a restart; it can hold secrets that were on screen, so `output = false` (Settings ›
Sessions › Keep each pane's recent output) stops saving it and deletes what was saved.

## Worktrees

```sh
maitty worktree feat/login -- claude      # new branch from HEAD, opened with claude running
maitty worktree fix-123 --base origin/main
maitty worktree-remove [--force]          # closes the workspace and removes the checkout
```

New worktrees go to `{repo_parent}/{repo}-worktrees/{branch}` (set `[worktree] dir`). An existing
local or `origin/` branch is checked out instead of created. Set `[worktree] command = "claude"`
to start an agent in every new worktree.

With **Own worktree per agent** enabled (the default), typing `claude`, `codex`, `gemini`,
`opencode` (or a quick agent's name) in a Maitty shell in a Git repository's main checkout
creates a worktree and starts the agent in it, including with `-c` configuration
or `-p` profile options. `codex resume` and `codex fork` stay in the current folder;
explicit `--cd` and Codex's own `--worktree` are honored. Starting inside an existing
linked worktree reuses it, and a folder that is not a Git repository gets no worktree and no
error. This applies to shells opened by Maitty, whatever shell they run (bash, zsh, fish,
PowerShell, nu, cmd, or a `shell` setting with arguments): each has Maitty's small launcher for
each agent first on its `PATH`, so an alias or function of your own, a shim manager, `command
claude`, or a shell started inside the pane (tmux, `bash -l`) still reaches it. The launcher
asks Maitty, which checks the pane's own secret, so only a plain shell in that pane can ask. Two
cases it cannot cover: an alias that runs another command or an absolute path (`alias
claude=~/bin/claude`), and a shell that rebuilds `PATH` without the launcher folder and never
puts it back.

**Cleaning up.** Closing the last session in a worktree maitty made removes its folder (never
with uncommitted changes; the branch is kept unless it's merged). One left behind by a session
that crashed is removed the same way a few minutes later, once nothing in it exists only there.
For everything else there is **Clean up worktrees** (a project's menu, or search):
every worktree of the repo with why it's still around (in use, uncommitted changes, not merged,
when it last committed). The safe ones come ticked; tick any others, and one confirm removes
them. It says so first when ticked ones hold uncommitted changes, which would be lost.

## Copy mode

Copy mode has no default key: open it from search commands (leader, then `Space`) as
**Select text**, or bind `copy-mode` under `[keys.prefix]`.

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
**Ctrl+Y / Ctrl+E** scroll one line up / down. Full details: [Copy mode controls](shortcuts.md#copy-mode).

Copies go to the system clipboard and, through OSC 52, to your outer terminal (works over SSH).

## Agent status

Maitty detects common command-line agents automatically and combines process,
terminal-screen and native lifecycle evidence where available. Status means:

1. **Native hooks** report lifecycle events. `maitty integrate claude` adds hooks to `~/.claude/settings.json`.
   They stay inert outside maitty panes and are tagged so a re-run or `--uninstall` replaces them cleanly.
   Any tool can report its own state with `maitty hook <name> --status working|blocked|done|idle`.
   `maitty integrate gemini` and `maitty integrate qwen` do the same for Gemini CLI and Qwen Code;
   `maitty integrate opencode` writes a small OpenCode plugin that tells maitty what it's doing.
   `maitty integrate pi` installs Pi's native extension. Every agent `maitty integrate` sets up
   (Claude Code, Codex, Gemini CLI, Qwen Code, OpenCode, Pi, OMP, Kilo, Kimi and Mastra Code) can also
   be connected with the mouse through **Settings › Agents → Connect agents…** (or search for Connect agents). Restart Codex or
   OpenCode after setup, or reload Pi. Status follows the app with any provider, including
   z.ai and DeepSeek; see [status setup and requirements](agent-status.md).
   For Codex, `maitty integrate codex` adds native `SubagentStart` / `SubagentStop`
   hooks in `~/.codex/config.toml`, so helper events carry their parent session ID;
   it also configures `notify` when free, preserving another program's notify.
2. **Screen evidence**: 22 pinned Herdr manifests match live screen regions, OSC
   titles/progress and paired dialog controls. A fresh visible blocker can override
   non-authoritative hook state; a transcript viewer preserves the previous state.
3. **Activity**: recent output that isn't the echo of your own typing.

A turn that finishes while you're looking elsewhere is **done** until you focus that pane.
An unanswered Codex queued or expanded question takes priority over its working spinner and shows
**needs you**. Finished prose that mentions the interrupt shortcut does not count as active
work. Codex reports turn completion through its notify hook; Maitty detects the next
turn's activity footer, including soft wraps and background-terminal details. New
input after a finished turn allows work detection immediately or after a delayed start.
An advancing footer also detects automatic follow-up work without keyboard input.
Native Codex subagent hooks attach every helper to its parent pane as soon as it
starts and remove it when it stops.
Hook authority ends when a recognized agent's process exits. Maitty recognizes
Herdr's 24 agent identities plus its five additional existing agents.
See [status setup and its limits](agent-status.md).
Terminal bells do not replace the status icon or add musical-note history entries.
Click a status icon/tag (or `<leader> y`) for a plain-language explanation and the ages of
hook, screen and process evidence; a status without confirmation is shown as uncertain.
