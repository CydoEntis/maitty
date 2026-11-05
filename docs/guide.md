# Maitty guide

How each part of Maitty works, in more depth than the [README](../README.md). Shortcuts are
in [shortcuts](shortcuts.md); every setting is in [`config.example.toml`](../config.example.toml).

## Using it

Structured Codex conversations use the daemon without creating a terminal pane.
They appear in the existing session list; selecting an unbound conversation keeps
the sidebar focused. Native terminal handoff and interactive request answers are
later work. Existing `maitty send` still types into a PTY.

Start with a verified Codex 0.164.0 executable and existing absolute paths:

```powershell
maitty conversation start --executable C:\tools\codex.exe --config-root C:\codex-home --cwd C:\project
maitty conversation send --conversation 1 --generation 1 --operation 1 "Review the failing test"
maitty conversation read --conversation 1 --limit 128
```

Commands print JSON. Start returns `conversation` and `generation`; use those
values for send. Increment `--operation` for each new prompt. Repeating an
acknowledged operation with identical text returns its receipt without sending
again. Uncertain delivery refuses automatic resend; inspect native/provider
evidence before deciding how to continue.

`start --session <exact-Thread.id>` resumes that exact native thread with the same
explicit executable/config-root/cwd inputs. A missing thread fails. Reconnect
recorded Maitty identity with `start --conversation <id> --generation <current>`;
it uses recorded settings and returns a newly fenced generation. No picker,
latest-session selection or implicit fork is used. Interrupted creation intent
has no confirmed identity and cannot automatically create another thread.

New bindings also accept `--model`, `--model-provider`, `--approval`
(`untrusted`, `on-request`, `never`; default `on-request`) and `--sandbox`
(`read-only`, `workspace-write`; default `read-only`). Workspace write disables
network and additional temporary-directory writable roots. Native profiles are
unsupported. A provider posture mismatch fails before turn submission.

`read --cursor '<JSON cursor>'` continues a page; limit is 1–128 and each page is
at most 256 KiB. A cursor becomes stale when history or generation changes.
Completeness and item state are separate from page exhaustion. Unsupported or
incomplete native history disables sending. Provider approval/question requests
remain visibly blocked and unanswered. Checkpoint failures and pending approvals
remain visible even when the provider has finished.

Structured mutations require `[restore] enabled=true`. Setting `output=false`
removes/prevents Maitty's transcript cache while preserving content-free identity
and delivery receipts; explicit reads may hydrate memory for that connection.

Structured tools authenticate as that runtime and follow configured MCP grants.
Granted list/read/send/reference operations remain available. Starting further
helpers from a structured runtime is refused until conversation-owned helper
consent is supported; native helper starts keep their existing behavior.
Structured admission supports the default/all Codex shell environment policy.
Custom filtering, environment-set/profile policies or explicit MCP environment
values that conflict with its reserved identity fail with a native-terminal
alternative. Runtime MCP environment allowlists are augmented without editing
your configuration or storing authentication values in thread settings.

Click **menu** at the bottom of the sidebar (or press the leader, then `m`) for the common
commands: new session, shell, pane, tab, Inbox, Search commands, All keys, the sidebar and Settings.
**Search commands** (leader, then `Space`) finds every command, including **Connect agents**
and your configured custom ones; rows click and the wheel scrolls.

```
╭─ LOCAL ────────────────────────╮
│                                │
│ ▾  maitty                ⎇ dev ✓│
│   ● bold-raven            ● you │
│     asks: what should the…     │
│   ○ shell                   2   │
│                                │
│ ▾  AeVoxGrowth          ⎇ main ✓│
│   ⣻ Growth Tests           12m  │
│   ✓ Confirm Modal           1h  │
│                                │
├─ REMOTE 1 ────────────── + add ┤
│                                │
│  ⇄ kingdom        ● connected  │
│ ▾  maitty         ⎇ fixes/fir ✓│
│   ⣻ Maitty GUI Work       48m  │
╰────────────────────────────────╯
```

- The top half, **Local**, is this computer. The bottom half, **Remote**, is every other
  machine, each under its own header; `+ add` (or a click on "+ add one" when there is none) opens
  the Machines dialog. Each half scrolls on its own.
- A project is its name with its branch and a `✓` (or `✕` if a check failed) on the right, or
  `no git` outside a repo. Click it to fold it; a folded project says how many sessions it hides
  and shows an amber `●` when one needs you.
- A session is two rows. First: the state mark and the name (in its state's colour while it is
  worth a look), then at the right `● you` when it needs you, else the agent's mark (`✻` Claude,
  `◆` Codex, `✦` Gemini, `◇` OpenCode, else its first letter) and the age. Second: what it is doing,
  or what it asks (`asks: …`, in amber), a check result (`failed: test · exit 1`), helpers asked
  for, a quiet spell or a message waiting. Every new claude / codex gets its own worktree (named
  for you), so agents never edit the same files.
- A session with other tabs lists them after it on the same level (`○ shell  2`); click one to show it.
- One card is selected: a neutral tint with a **▎** bar at its left edge (lime while the sidebar
  has the keys, dim when a pane does). The open session keeps a dim bar while you browse with the
  keyboard. Hover is a separate, quieter tint.
- Collapsing the sidebar (leader, then `z`) leaves a narrow column down the side, each session as its mark and first letter: click a row to go to it, `»` (`«` with the sidebar on the right) or `…` to expand the sidebar again.
- A machine whose line is down, or that has no sessions, is its header and one line instead of its
  projects; click it for the Machines dialog. See *Machines* below.
- Nothing moves in the sidebar by itself: a session that needs you or finishes keeps its row (its mark and name say so, and the Inbox lists what needs you). Idle agents stay running; no sleep screen is shown for sessions saved by older releases.
- A session taking a real share of the machine says so in its pane's title, beside its state
  (`▲ 38% CPU · 2.1 GB`, from a fifth of the CPU or two gigabytes).
- The sidebar footer keeps **+ new** and **Menu** available. **+ new**, `Ctrl+Space n` and a project's **+** open a plain shell where you are; type your agent there. `Ctrl+Space s` starts a session straight away (Settings › Sessions sets the agent and whether it gets a worktree).
- `⇆` under a row names a file that this checkout and another have each changed since their
  branches parted (a merge conflict waiting to happen); more than two are one line with a count.
- Drag a project, or a session within its project, up or down to reorder it. Click a project to fold or open it. With the sidebar focused
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
when enabled in Settings (after the leader, Alt+arrows swaps panes instead). All keys shows changes to these bindings too.

**Files**, **Changes**, **Branch** and **References** share the side panel docked beside
your panes. A tab strip at its top names each with its key (`Files f · Changes c · Branch b ·
References a`), the open one lit
while the panel has the keys; Tab / Shift+Tab (or a click on a tab) moves between them for the
same checkout. A click into a pane gives that pane the keys and leaves the panel open beside
it; a click in the panel, or its key again, gives the keys back.

**Files** shows the project's tree and the file you pick beside it (on a narrow panel the
file takes the panel, `‹` goes back). `f` again or Esc closes it; Ctrl+click on a `path:line`
in a pane opens it at that line. Enter puts the path in the agent's prompt, `e` opens it in
your editor (`editor` in config; nvim, helix … open inside maitty), `o` opens it outside,
`y` copies the path, `/` filters, `r` lists recent files. Drag over the file shown (or
double-click a word) and Ctrl+C copies the text.

`i` edits the file right in the panel: Shift with the arrows, Ctrl+A or the mouse selects;
Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste (with nothing selected, Ctrl+C and Ctrl+X take
the whole line); Ctrl+Z and Ctrl+Y undo and redo; Ctrl+S saves and Esc is done (with
unsaved changes it warns first, and Esc again throws them away). While Files has the keys a paste stays in Files: in the editor
at the cursor, while filtering in the filter, and never in the pane behind.

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
- **Move:** drag a pane by its title onto another pane of the tab and the two swap places
  (each takes the other's size); Esc cancels. `Ctrl+Space` then `Alt`+an arrow swaps the pane
  you're in with the one that way, and the pane's right-click menu has "Move pane left" and so
  on.
- **Copy:** drag to select; double-click copies a word, path or link. Programs that copy (OSC 52,
  e.g. Claude's `/copy`, nvim) put it on your clipboard. Ctrl+click opens a link.
- **Images:** `Ctrl+Space`, then `Ctrl+V` (or a paste while an image is on the
  clipboard) saves the image and pastes its path; Claude Code and Codex attach it. In a pane on
  another machine the image is sent to that machine over ssh (up to 25 MB), kept there in
  `~/.cache/maitty/pastes` (private, cleared after a week), and its path there is pasted. It needs
  ssh to that machine (not a custom transport) and a login that works without a prompt.
- **Keys:** on Windows, combos plain terminals can't send (Ctrl+Shift+letter, Ctrl+Enter, Ctrl+Tab)
  reach programs exactly, as native key records. Shift+Enter is still a newline for agents.
- **Resize:** text re-wraps to the new width (history included). Programs' cursor shape (bar,
  block, underline) shows. Synchronized redraws are drawn whole, so agents don't flicker.
- **Closing** a pane ends everything running in it after you confirm.

## Alerts

Needs-you alerts use two rising notes and stay quiet for the session you're looking at; the
same session asking again within 2 minutes stays quiet too. A background
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

## References

Keep the screenshots and design files an agent should build from in the References view
(leader + `a`). They are plain files in `.maitty/references/` at the repo's main checkout (a
folder outside git keeps its own), so every agent, in every worktree, sees the same set, and
any agent can read them by path. The first time Maitty makes the folder in a repo it adds
`.maitty/references/` to the repo's `.git/info/exclude`, so references stay out of git without
touching your `.gitignore`; `git add -f` commits one if you want to share it. Agents connected
through `maitty integrate mcp` are told to call `maitty_references` before UI or design work; it
lists each file's absolute path, kind, size and time, and the agent opens the images itself.
Subagents usually inherit their parent's MCP tools. Over `--remote` the view isn't available yet.

## Secrets and pull requests

**Secrets.** An agent that needs an API key or token calls `maitty_request_secret` instead of
asking in chat. Its row turns "needs you" with "asks for a secret" (and shows in the Inbox);
The prompt opens by itself beside the row when nothing else has the screen (it waits its turn
behind a dialog or sheet, and a request you declined is not offered again); keys are ignored for
the first second and a half, so what you were typing is not taken as the secret. Enter on the row,
or a click on the line, opens it any time. What you type shows as dots.
Enter gives it and Esc declines (the agent is told to carry on without it); a paste works too. By
default the agent gets the path of a file only you can read, with the secret in it; if it asks for
delivery `run`, it gets a reference and runs `maitty secret run <reference> --env NAME -- <command>`
to put the secret in one command's environment. A reference works once, only for the pane that
asked, and runs out after a day. Maitty keeps secrets in memory only: a restart forgets them,
and delivered files are removed within a day or when the daemon starts. They never appear in the
chat, the state or a log.

**Pull requests.** With the GitHub CLI (`gh`) installed and logged in, Maitty finds the pull
request open on each pane's branch every couple of minutes and shows the focused pane's in the
bottom bar: `#7 2 failing`, `#7 changes requested`, `#7 conflict`, `#7 checking`, `#7 approved`,
`#7 passing`, `#7 merged`. `maitty ls --json` and `maitty_list` carry the details. When the
checks fail, a review remark arrives or a merge conflict appears, the agent is told once its turn
is over (never into a question), at most ten times per commit; Settings › Agents › "Tell agents
about their PR" turns that off. If the branch doesn't find it, `maitty pr link
<github pull request address>` links the pane by hand (`maitty pr unlink` goes back). Only
reviews and conversation comments are read, not inline code comments. A link is forgotten when the
server restarts.

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
interrupt (`maitty_interrupt`), start helpers (`maitty_start`, see *Helpers* above) and list the
project's references (`maitty_references`, see *References* above) and ask you for a secret
(`maitty_request_secret`, see *Secrets and pull requests* below).
`[mcp] approve` sets whether agents may answer "yes" to prompts and `[mcp] scope` whether
they reach only their project or every session; `maitty grant <pane> read,write,…` changes
one pane's rights.

An agent can also reach another machine you have opened to agents; see *Agents on a machine* under *Machines*.

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

## Machines

maitty can show the agents of other computers next to yours. Each computer keeps running its own maitty;
this window connects to it over `ssh` and shows its folders and sessions under a header of their own.
Closing the window, or losing the connection, never stops anything over there.

### Add a machine

Leader `w` (or `Machines` in the command palette) lists the computers maitty reaches over ssh and
adds more. Pick a host from your `~/.ssh/config` and press Enter: if ssh logs in on its own (a key, an
agent, a known host) the machine is added and connecting in that one step. If ssh needs you (a password, a
code, a host key you have not accepted) it opens in a popup, you answer, and the machine connects when the
popup closes. You can also type `user@host`. Windows has no ssh agent to remember a password, so use keys or
the agent there.

Each machine in the dialog shows how it stands; when it is down the reason and what to do about it are written
in full under it. `r` tries it again, `x` removes it from the list: the machine and its sessions keep running,
and its saved layout is kept in case you add it back.

The dialog writes the machine to `config.local.toml`, beside `config.toml`. That file belongs to this computer
and is never synced; a `[[machines]]` entry in the synced `config.toml` is refused. You can write the entry
yourself, and a change made by hand takes effect the next time maitty starts:

```toml
[[machines]]
name = "build"                      # how you address it: --on build, build/4
target = "me@10.0.0.2"              # a host, user@host or a Host alias from ~/.ssh/config
ssh_args = ["-p", "2222"]           # optional: extra ssh arguments, one per entry
remote_cmd = "~/.local/bin/maitty"  # optional: where maitty is on that machine (default: maitty)
```

Four more keys decide what a machine may do, all `false` until you turn them on. `clipboard` lets its panes
write your clipboard, `notices` lets it raise notices (which include bringing this window to the front) and
`sounds` lets it play sounds; what it raises starts with `[build]`. `mcp` opens it to agents (see *Agents on a
machine*). `config.example.toml` lists every key.

Adding a machine starts maitty there if it is not running, so you can create the first session from this
computer. If the connection drops later, maitty only reconnects to a maitty that is still running there; it
never starts one you stopped. Press retry on the machine to start it there again. If maitty is not
installed there, see *Install or update maitty on a machine*.

### What you see

Sessions on this computer are in the sidebar's **Local** half; every other machine's are in **Remote**,
each machine under a `⇄ build` header. A machine that is not connected, or has no sessions, is one item: its name with
how its connection is at the right. A session's status is the one its own maitty reports, and what its agents
need shows in the Inbox with `· on build`.

| Shown | What it means, and what to do |
| --- | --- |
| `connecting` | The first try this run. |
| `connected` | The line is up. |
| `reconnecting #2` | A line that was up dropped. maitty tries again at once, then after 250 ms, doubling up to 30 s; three tries in a row without an answer make it `unreachable`. |
| `needs login` | ssh cannot log in without asking you. Make plain `ssh` to that machine work in a terminal (load your key, accept its host key), then press `r`. |
| `needs update` | The machine runs another version of maitty, or has none on its PATH. Run `maitty machine update <name>` (add `--install` when there is none, or set `remote_cmd` to its path), then press `r`. |
| `unreachable` | No answer, or maitty is not running there. The reason is written under it (ssh's last line, or "maitty is not running on that machine; retry to start it there"); maitty keeps trying in the background. |
| `refused` | A different computer now answers at that name. If you replaced the machine, remove it and add it again; otherwise check the host name and DNS. |

When a machine drops, its cards give way to one item saying `disconnected · click for the fix` (click it for the
Machines dialog, which says what to do). They come back by themselves when the line does, each pane showing
its screen again. Its needs-you and failed marks leave the Inbox while it is down. Typing sent to a machine while it
was down is dropped, never queued (replaying keystrokes into a pane that moved on could run commands you no
longer meant); when the machine is back, maitty says how much was not sent.

### The command line on another machine

The commands you script with work on a machine you added: `--on <machine>` runs one there, and a pane
there is written `machine/id`.

```sh
maitty ls --on build                     # build's workspaces; its panes are listed as build/4
maitty send --pane build/4 "run the tests"
maitty read build/4
maitty wait --pane build/4 --regex "tests passed"
maitty split --pane build/4 -- htop
maitty new --on build ~/code/api -- claude
maitty ls --all                          # this computer and every machine; one that is down is named on stderr
```

`--on` may come before or after the command, `local` means this computer, and `machine/id` works
wherever a pane does (`read`, `focus`, `grant`, `handoff`, `--pane`). It works with `ls`, `read`,
`send`, `wait`, `split`, `handoff`, `new`, `focus`, `close`, `grant`, `teach`, `popup` and
`kill-server`. A name that is not `local` or a machine you added is refused, and the error lists the names
that work.

The pane a command runs in is on this computer, so on another machine you always say which pane:
`maitty send --on build "hi"` is refused until you add `--pane build/4`. The other machine is never
told which pane here you are in. What a machine prints is cleaned before it reaches your terminal;
`read --raw` prints a machine's screen exactly as it sent it. If a machine cannot be reached the error
is one line with the fix (log in with plain `ssh`, `maitty machine update <name>` to install or update maitty there, retry the machine to start maitty there).
`kill-server --on build` stops every session on that machine, so it needs `--yes`. `--remote host`
still works as before, and cannot be combined with `--on`.

### What a machine needs

A machine you add only needs the `maitty` program. There is no desktop app on it and nothing to build.

- **The program.** The same `maitty` you run here, for Linux or macOS (x86_64 or aarch64). The Linux downloads
  are static, so they run on any Linux. Installing it puts one file at `~/.local/bin/maitty` for your user.
  `maitty machine update <name> --install` does this over ssh for you, so you do not need to log in and
  install it by hand. A Windows machine cannot be the remote yet.
- **A way in.** ssh must get in without asking you anything: a key (or your ssh agent) for that user. Run plain
  `ssh <target>` once first, so its host key is known and you can see that it works.
- **maitty on the PATH ssh uses.** A program in `~/.local/bin` is on the PATH of most logins. If ssh cannot find
  it, set `remote_cmd` on the machine to its full path.
- **The same version.** This computer, the machine and any desktop app must speak the same protocol. A machine
  on another version shows `needs update`; run `maitty machine update <name>`.

Nothing else needs installing. Agents such as Claude Code or Codex run there as they do anywhere, so install
the ones you want to use on that machine yourself.

`maitty doctor` checks the way in, the program, its version and the protocol for every machine you added (see "Check a machine with doctor" below).

### Install or update maitty on a machine

`maitty machine update <name>` installs or updates maitty on a machine you added, over ssh. It looks first,
says what it found, and changes nothing without a yes.

```sh
maitty machine update build --install   # not installed there yet: install this version
maitty machine update build --yes       # an older version there: replace it
maitty machine update build             # asks, when you are at a keyboard
maitty machine update build --force     # a newer version there: put this one on instead (a downgrade)
```

```
probing build…
build: linux x86_64, maitty 0.58.0 (this is 0.60.0)
sending 14.2 MB (sha256 3f2a9c1b7d04)…
verified: it arrived whole and runs there
installed to ~/.local/bin/maitty
asking the daemon on build to restart if no session is open…
daemon: left running: 3 open session(s); it keeps them
build: maitty 0.60.0 installed to ~/.local/bin/maitty
```

- **Never silent.** A machine with no maitty needs `--install` (or a `y` at the question); an older one needs
  `--yes` (or a `y`). With neither and no keyboard the command stops and says which flag to pass. A machine
  that already has this version is left alone; a newer one is not downgraded without `--force`.
- **The binary.** If the machine has the same system and CPU as this computer, this computer's own maitty is
  sent. Otherwise the matching release is fetched here (this version's, else the latest, said aloud), checked
  against the release's `sha256sums.txt`, unpacked, and then sent: the machine never needs GitHub. A bad or
  missing checksum stops before anything is sent.
- **It goes in whole or not at all.** The file is written beside its destination, its size is checked, it is
  run with `--version`, and only then renamed over the old one. Any failed step leaves the old maitty as it
  was and says so ("the old version is unchanged", or "nothing was installed" on a first install).
- **Where.** At the machine's `remote_cmd` when that is an absolute path or starts with `~/`; otherwise at
  `~/.local/bin/maitty`. If `maitty` is not found on that machine's PATH afterwards, the command says to set
  `remote_cmd = "~/.local/bin/maitty"` for it in `config.local.toml` (it does not edit your config).
- **Running sessions survive.** Nothing is killed. The old daemon keeps running on the old file. The new
  maitty then restarts it only when it has no session open; otherwise it says it left it running, and the
  daemon takes the new version the next time it restarts (when its sessions are closed).
- **Not supported: a Windows machine as the remote.** In v1 Windows is never the remote; the command says so
  and exits. Linux and macOS (x86_64 and aarch64) are supported.

Exit status: 0 done or already up to date, 2 a mistake in how it was asked (an unknown machine, a flag
missing), 1 a step that did not work.

### Agents on a machine

An agent can reach a machine from your `[[machines]]` list through the same MCP tools it uses here
(see *Agents that steer agents (MCP)* above), but only a machine you opened to agents with `mcp = true` in
its entry. `maitty_list`, `maitty_read`, `maitty_send`,
`maitty_wait` and `maitty_start` take an optional `machine`; a session on another machine is
listed, and passed back to any tool, as `machine/id` (`build/4`), and `machine = "all"` lists this
computer and every open machine, with one that is down reported inline. The agent's own pane
needs the `read` grant to list, read or wait on another machine and `write` to send to it, as it
does here. Every reply from another machine starts with `[machine: build]`, so the agent can tell
whose words follow, and is cleaned of control characters and cut at 32,768 characters. Answering
a prompt, interrupting, and starting agents or worktrees are local-only: they return
`unsupported_for_remote`. A machine that is not opened returns `machine_not_enabled` and is never
contacted.

### Custom transports

A machine can be reached through a program of your own instead of ssh, for example `fly ssh console`.
In `config.local.toml` (never the synced file: it is a program this computer runs):

```toml
[[machines]]
name = "fly"
target = "my-app"
via = ["fly", "ssh", "console", "-C", "{command}"]
```

`via` is the program and its arguments, one entry each, not a shell line. `{command}` is replaced by what
must run on the far side (`'maitty' proxy --attach-only`); leave it out and the command is added as the last
argument. The program must pass stdin and stdout through unchanged and must not ask questions or use a
terminal. Text it prints before maitty starts is skipped, as for ssh. It does not work on Windows, and
`ssh_args` cannot be set beside it. `maitty machine update` will not install over it: install maitty on that
machine yourself. `maitty doctor` names the program (never its arguments) and checks the protocol.

### Check a machine with doctor

With machines configured, `maitty doctor` ends with a section for each, checked all at once, one line a
check:

```
 machine build
 ✓ reachable              ssh logs in without a prompt
 ✓ maitty version         0.60.0 (same as this computer)
 ✓ protocol               protocol 69, the same as this computer
 ✓ login PATH             maitty found at /home/me/.local/bin/maitty
 ✓ ssh options            BatchMode=yes ForwardAgent=no -a -T ConnectTimeout=15 ServerAlive=15x3 + your ssh_args: -p 2222
```

A machine ssh cannot log in to gets one ✕ line with the fix, then `skipped: not reachable` and the ssh options.
A version that differs says `maitty machine update <name>`; a daemon on another protocol says the same. The
login PATH line shows where the machine's login shell finds maitty (the program, never the whole PATH) and
notes when your `remote_cmd` is something else. Each machine has about 30 seconds in all, so one that never
answers does not hold up the rest. The section is not printed when there are no machines.

### Limits

- The machine you reach must run Linux or macOS (x86_64 or aarch64). A Windows machine as the remote is not
  supported. A Windows computer can reach those machines, but it has no ssh agent to remember a password, so use
  keys or an agent there.
- Files, Changes, Find, Branches, References, connecting an agent and worktrees read this
  computer's disk, so for a pane on another machine they do nothing but say so: "Files is local-only in v1: the
  files are on build, not on this computer."
- `maitty --remote host` still works as before: one ad-hoc machine named after the host, not in your list. Unlike
  a machine you added, it starts the daemon there when none is running, and it keeps its own worktrees.
- The desktop app has no machines; this is the terminal app's.

## Never leave maitty

| Leader + | Panel | What it does |
|---|---|---|
| `f` | **Files** | The project's tree, with the file you pick beside it. `r` switches to recent files (just appeared in Downloads, Desktop, Documents or the project), `/` filters. Enter types the path into your prompt; `e` your editor, `i` edit it in place (Shift+arrows or the mouse select; Ctrl+C, Ctrl+X, Ctrl+V copy, cut and paste), `o` open it outside, `y` copy the path. `f` again or Esc closes it. |
| `c` | **Changes** | A checkout's changed files and the diff: `c` commit, `p` push and open a pull request, `x` mark reviewed, `r` go to its agent; in a worktree `m` merges into the base branch and `d` throws it away. Each of `c`, `p`, `m` and `d` asks first. |
| `a` | **References** | The project's screenshots and design files, newest first: `p` adds the clipboard image, `n` a file by its path (type it or drop it in), Enter opens one in your image viewer, `y` copies its path, `d` removes it (asked first). `a` again or Esc closes it. See *References* below. |
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
Closing every pane yourself also starts fresh. A pane or agent that ends on its own is left out
of the saved session for 15 seconds: a shutdown, reboot or logoff kills agents a moment before
maitty, so the session from before it is the one that comes back. Configure all of this under `[restore]`,
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

1. **Native hooks** report lifecycle events. The installer and `maitty update` run `maitty integrate`, which
   connects every installed agent at once (`MAITTY_NO_INTEGRATE=1` skips it). `maitty integrate claude` adds hooks to `~/.claude/settings.json`.
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
