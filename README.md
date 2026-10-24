<h1 align="center">Vibecasso</h1>

<p align="center"><strong>Run your coding agents side by side, and always know which one needs you.</strong></p>

<p align="center"><a href="#install">Install</a> · <a href="#quick-start">Quick start</a> · <a href="#shortcuts">Shortcuts</a> · <a href="docs/guide.md">Guide</a> · <a href="https://github.com/CydoEntis/vibecasso/releases">Releases</a></p>

Vibecasso is a terminal workspace for Claude Code, Codex, Gemini, OpenCode and other
command-line agents. Start as many as you like, see at a glance which are working, which
are waiting for you and which have finished, and jump to any of them in a keystroke. Close
the window and they keep running; open it again and everything is where you left it.

It runs natively on Windows, macOS and Linux.

## Highlights

- **Status you can trust.** Every session shows whether its agent is working, needs you,
  has finished or is idle, from the agent's own hooks where available and from its process
  and screen otherwise. An agent that has gone silent for ten minutes is flagged as quiet.
- **Nothing is lost.** A background service owns every session. Closing the window, a crash
  or a reboot doesn't stop your agents: Vibecasso reopens them in the right folders and
  resumes their conversations.
- **One inbox.** Questions, failed checks and finished work collect in one place, most
  urgent first. A sound and an optional phone notification tell you when an agent needs you.
- **Each agent in its own branch.** New agents get their own Git worktree, so they never
  edit the same files. Review the diff, commit, merge or open a pull request without leaving.
- **Checks you can rely on.** Name a command, such as your tests, and Vibecasso runs it after
  each agent turn. A session whose check fails reads *failed*, not *done*.
- **Agents that work together.** Agents can list, read, message and start other sessions
  through MCP tools, within limits you set.
- **Your look.** Four layouts (Floating, Tiled, Compact and Grid) with square or rounded
  corners, 65 themes, and every key and colour configurable.

## Install

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/CydoEntis/vibecasso/main/install.ps1 | iex
```

**macOS and Linux:**

```sh
curl -fsSL https://raw.githubusercontent.com/CydoEntis/vibecasso/main/install.sh | sh
```

The installer picks the build for your machine, verifies its checksum and needs no
administrator rights. You can also download an archive from
[Releases](https://github.com/CydoEntis/vibecasso/releases), or build from source with
`cargo install --git https://github.com/CydoEntis/vibecasso`.

Vibecasso tells you when an update is out; one click installs it and restarts without
losing your sessions.

**Coming from Seshi?** Vibecasso is its new name. Install Vibecasso as above. The first time
it starts, it brings over your config, saved sessions and history, and points your Claude
hooks at Vibecasso. Run `vibecasso doctor`: it names anything still set up for Seshi, such as
the MCP server (`vibecasso integrate mcp` renames it) or another agent's integration
(`vibecasso integrate <agent>`). Projects keep working with their `.seshi.toml` and
`.seshi/checks.toml` until you rename them. Uninstall Seshi when you're happy.

## Quick start

1. Open a terminal in your project and run `vibecasso`.
2. Click **+ new** for a shell and type `claude` (or any agent), or press `Ctrl+Space`, then
   `s` to start your default agent straight away.
3. Watch the sidebar: orange is working, the alert colour means it needs you, green means
   it finished, gray is idle. Click a row to go to that session.
4. Press `Ctrl+Space`, then `q` to step away. Your agents keep running; run `vibecasso`
   to come back.
5. Run `vibecasso doctor` to check your setup and connect agents for exact status.

## Shortcuts

Press `Ctrl+Space` to enter **shortcut mode**: a `SHORTCUT` label appears and each key runs
an action, until you press `Esc` or click into a pane. Every key is a single key, with no
Shift needed. Pause after `Ctrl+Space` to see them all.

| | |
|---|---|
| **Sessions** | `n` new shell · `s` new session · `o` session with options · `Enter` message the agent · `k` stop it · `i` inbox · `y` why this status |
| **Panes** | arrows move · `v` split side by side · `h` split stacked · `x` close · `Ctrl`+arrows resize · `c` copy mode · `z` collapse the sidebar |
| **Tabs** | `t` new · `Ctrl+W` close · `r` rename · `]` / `[` next / previous · `1`–`9` go to tab |
| **Project** | `f` files · `p` find a file · `/` search code · `d` changes · `g` worktrees · `b` branches · `e` the sidebar |
| **Vibecasso** | `Space` search commands · `m` menu · `,` settings · `F1` all keys · `l` history · `Ctrl+V` paste an image · `q` step away |

Scrolled up in a pane? Click **↓ Back to bottom**. Every screen's own keys are in
[shortcuts](docs/shortcuts.md).

## Connect your agents

Vibecasso recognises common agents with no setup, from their process and screen. For exact
status, connect them once on each machine:

```sh
vibecasso integrate claude     # also: codex, gemini, qwen, opencode, pi, omp, kilo, kimi, mastracode
vibecasso integrate mcp        # lets agents see and steer other sessions
vibecasso doctor               # checks what's connected
```

Or use **Settings › Agents › Set up another agent…**. See [agent status](docs/agent-status.md)
for what each integration reports.

## From the command line

Everything works from any shell, including from an agent running inside Vibecasso:

```sh
vibecasso ~/code/my-app                   # open a project
vibecasso new ~/code/my-app -- claude     # start an agent there
vibecasso worktree feat/login -- codex    # a new branch in its own worktree, with codex
vibecasso ls --json                       # every session and its status
vibecasso send -p 3 "run the tests"       # message a session
vibecasso read 3                          # read its screen
vibecasso handoff 3 --to codex            # hand a task to another agent
```

## Configure

Open **Settings** with `Ctrl+Space`, then `,`. Changes save to your config file with its
comments kept.

```sh
vibecasso config init    # write a commented config file
vibecasso config path    # where it lives
```

A few examples:

```toml
theme = "tokyo-night"

[ui]
layout = "grid"            # floating | tiled | compact | grid
corners = "rounded"        # square | rounded

[keys.prefix]
"ctrl+l" = "spawn-tab:lazygit"   # any command, in a split or a new tab

[[check]]                  # run after every agent turn
name = "test"
command = "cargo test"
```

Every option is described in [`config.example.toml`](config.example.toml).

## Requirements

- Windows, macOS or Linux.
- A terminal with true colour. A [Nerd Font](https://www.nerdfonts.com) adds icons.
- Optional: `git` for worktrees, branches and changes; the GitHub CLI (`gh`) for pull requests.

## Learn more

- [Guide](docs/guide.md): sessions, panes, alerts, checks, worktrees, MCP and more.
- [Shortcuts](docs/shortcuts.md): every key, by screen.
- [Agent status](docs/agent-status.md): how status is detected and what each integration adds.
- [Architecture](docs/ARCHITECTURE.md) and [testing](docs/TESTING.md), for contributors.

Inspired by [herdr](https://github.com/ogulcancelik/herdr),
[nebula](https://github.com/agentSystemLabs/nebula) and [fut](https://github.com/mikker/fut).

## License

MIT.
