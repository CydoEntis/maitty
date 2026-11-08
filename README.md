<h1 align="center">Maitty</h1>

<p align="center"><strong>All agents on deck.</strong><br>
Run your coding agents side by side, and always know which one needs you.</p>

<p align="center"><a href="#install">Install</a> · <a href="#quick-start">Quick start</a> · <a href="#shortcuts">Shortcuts</a> · <a href="docs/guide.md">Guide</a> · <a href="https://github.com/CydoEntis/maitty/releases">Releases</a></p>

Maitty is a terminal workspace for Claude Code, Codex, Gemini, OpenCode and other
command-line agents. Start as many as you like, see at a glance which are working, which
are waiting for you and which have finished, and jump to any of them in a keystroke. Close
the window and they keep running; open it again and everything is where you left it.

It is built for developers who run several agents across several projects, and it runs
natively on Windows, macOS and Linux.

## Highlights

- **Status you can trust.** Every session shows whether its agent is working, needs you,
  has finished or is idle, from the agent's own hooks where available and from its process
  and screen otherwise. An agent that has gone silent for ten minutes is flagged as quiet, and one calling the same tool over and over as repeating.
- **One line per session.** Each row shows the agent's state, which agent it is and the
  lines it has changed but not committed (`+40 −3`). Click the numbers to see the diff.
- **Nothing is lost.** A background service owns every session, so closing the window
  doesn't stop your agents. After a crash or a reboot, Maitty reopens them in the right
  folders and resumes their conversations.
- **One inbox.** Questions, failed checks and finished work collect in one place, most
  urgent first. A sound and an optional phone notification tell you when an agent needs you.
- **Each agent in its own branch.** New agents get their own Git worktree, so they never
  edit the same files. Review the diff, commit, merge or open a pull request without leaving.
- **A side panel for the code.** Files, Changes, Branch and References open beside your panes
  as tabs: browse the tree, read markdown drawn or raw, review each file's diff, see where the
  branch stands, and keep the screenshots and designs your agents build from. Drag its edge to resize it, or press `[` `]` while it has the keys.
- **Checks you can rely on.** Name a command, such as your tests, and Maitty runs it after
  each agent turn. A session whose check fails reads *failed*, not *done*.
- **Agents that work together.** Agents can list, read, message and start other sessions
  through MCP tools, within limits you set, and find the project's reference images there.
- **Your look.** Floating boxes, each pane in its own colour if you like, rounded or square corners,
  padding round the screen, your cursor's shape, 170 themes with every colour configurable,
  and rebindable shortcuts.

## Install

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/CydoEntis/maitty/main/install.ps1 | iex
```

**macOS and Linux:**

```sh
curl -fsSL https://raw.githubusercontent.com/CydoEntis/maitty/main/install.sh | sh
```

The installer picks the build for your machine, verifies its checksum and needs no
administrator rights. It then runs `maitty integrate`, which connects every agent installed on
the machine (see below); set `MAITTY_NO_INTEGRATE=1` to skip that. Re-running the installer or
updating connects them again. You can also download an archive from
[Releases](https://github.com/CydoEntis/maitty/releases), or build from source with
`cargo install --git https://github.com/CydoEntis/maitty maitty`.

### Desktop GUI

The same release also publishes desktop installers for Windows, macOS and Linux.
The GUI package includes its matching daemon. The `install.sh` and `install.ps1`
commands above install the terminal app only. To use both interfaces with the
same sessions, install the GUI and TUI from the same release; both connect to
the same local daemon protocol.

Maitty tells you when an update is out; one click installs it and restarts without
losing your sessions. **Settings › General › UPDATES** shows the version you have.

**Coming from Vibecasso or Seshi?** Maitty is the new name. Install Maitty as above. The
first time it starts, it brings over your config, saved sessions and history, and points your
Claude hooks at Maitty. Run `maitty doctor`: it names anything still set up under the old
name, such as the MCP server (`maitty integrate mcp` renames it) or another agent's integration
(`maitty integrate <agent>`). Projects keep working with their `.vibecasso.toml` or
`.seshi.toml` and checks files until you rename them. Uninstall the old app when you're happy.

## Quick start

1. Open a terminal in your project and run `maitty`.
2. Press `Ctrl+T` (or click a machine's **+** in the sidebar) and a shell opens at once where
   you are; type `claude` (or any agent) in it. `Ctrl+Space` then `n` opens **New session** for
   another folder or machine.
3. Watch the sidebar. Each machine lists its sessions with a status mark: teal spinner working,
   yellow needs you, green finished, red stopped, grey idle. Click a row to go to that session.
4. Press `Ctrl+Space`, then `q` to step away. Your agents keep running; run `maitty`
   to come back.
5. Run `maitty doctor` to check your setup and connect agents for exact status.

## Shortcuts

The bar along the bottom always shows the mode. It reads ` PANE ` with the session you are on
and `^space keys  ^space j/k sessions  ^space ? help` (each leader key is shown with the leader, since the bare key would be typed into the pane) while keys go to the terminal. Press `Ctrl+Space` (or
click ` PANE `) for **key mode**: the chip turns to a yellow ` KEYS ` and the bar lists keys to
press (`n new  s split  x close  tab pane  ? help  esc back`; click one to run it), and each key
runs an action until you press `Esc`, `Ctrl+Space` again or click into a pane, or start a new
shell, session or split to type into. While the sidebar, a popup or a panel has the keys, the bar
names it and its keys (` SETTINGS ` … `j/k move  tab section  enter change  esc close`), and a key
that isn't a shortcut says "Shortcut mode — Esc to type". Every key is a single key, and only `?`,
`S` and `_` need Shift. Press `?` to see them.

| | |
|---|---|
| **Sessions** | `n` new session · `t` new shell · `j` / `k` next / previous · `1`–`9` jump to session · `r` rename · `/` filter · `i` inbox |
| **Panes** | `s` split right · `S` split down · `tab` focus next pane · `z` zoom · `x` close · `m` move · `Ctrl`+arrows resize |
| **Window** | `f` files · `c` changes · `u` usage and cost · `,` settings · `b` toggle the sidebar · `?` keys help |
| **Maitty** | `Space` search commands · `.` menu · `h` history · `Ctrl+V` paste an image · `q` step away |

Scrolled up in a pane? Click **↓ Back to bottom**. Every screen's own keys are in
[shortcuts](docs/shortcuts.md).

## Connect your agents

Maitty recognises common agents with no setup, from their process and screen. For exact
status, the installer connects every agent it finds on the machine. To do it again (say, after
installing a new agent):

```sh
maitty integrate            # status hooks for every installed agent, plus MCP
maitty integrate claude     # just one: codex, gemini, qwen, opencode, pi, omp, kilo, kimi, mastracode
maitty integrate mcp        # lets Claude, Codex, Qwen and Gemini see and steer other sessions
maitty integrate --uninstall  # removes every one
maitty doctor               # checks what's connected
```

Restart agents that were already running so they load it.

Or use **Settings › Agents › Connect agents…**, which offers the same agents. See [agent status](docs/agent-status.md)
for what each integration reports.

## From the command line

Everything works from any shell, including from an agent running inside Maitty:

```sh
maitty ~/code/my-app                   # open a project
maitty new ~/code/my-app -- claude     # start an agent there
maitty worktree feat/login -- codex    # a new branch in its own worktree, with codex
maitty ls --json                       # every session and its status
maitty send -p 3 "run the tests"       # message a session
maitty read 3                          # read its screen
maitty handoff 3 --to codex            # hand a task to another agent
```

## Configure

Open **Settings** with `Ctrl+Space`, then `,`. Changes save to your config file with its
comments kept.

```sh
maitty config init    # write a commented config file
maitty config path    # where it lives
```

A few examples:

```toml
theme = "tokyo-night"

[ui]
rounded = true             # rounded card corners
padding = "small"          # small | medium | large: space round the screen

[keys.prefix]
"ctrl+l" = "spawn-tab:lazygit"   # any command, in a split or a new tab

[[check]]                  # run after every agent turn
project = "/home/me/code/my-app"   # the repo's main checkout
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
