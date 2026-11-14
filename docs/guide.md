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

Click **menu** at the bottom of the sidebar (or press the leader, then `.`) for the common
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
├─ REMOTE 1 ─────────────── m add ┤
│                                │
│  ⇄ kingdom                  ●  │
│ ▾  maitty         ⎇ fixes/fir ✓│
│   ⣻ Maitty GUI Work       48m  │
╰────────────────────────────────╯
```

- The top half, **Local**, is this computer. The bottom half, **Remote**, is every other
  machine, each under its own header; `m add` opens
  the Machines dialog. Each half scrolls on its own.
- A project is its name with its branch and a `✓` (or `✕` if a check failed) on the right, or
  `no git` outside a repo. Click it to fold it; a folded project says how many sessions it hides
  and shows an amber `●` when one needs you.
- A session is two rows. First: the state mark and the name (in its state's colour while it is
  worth a look), then at the right `● you` when it needs you, else the agent's mark (`✻` Claude,
  `◆` Codex, `✦` Gemini, `◇` OpenCode, else its first letter) and the age. Second: what it is doing,
  or what it asks (`asks: …`, in amber), a check result (`failed: test · exit 1`), helpers asked
  for, a quiet spell, a repeat (`repeating 6x`: the same call over and over) or a message waiting. Every new claude / codex gets its own worktree (named
  for you), so agents never edit the same files.
- A session with other tabs lists them with a thin line beside them (`○ shell  2`); click one to show it. Each tab shows its own state; the session's row shows only its own tab's.
- One card is selected: a neutral tint with a **▎** bar at its left edge (lime while the sidebar
  has the keys, dim when a pane does). The open session keeps a dim bar while you browse with the
  keyboard. Hover is a separate, quieter tint.
- Collapsing the sidebar (the `«` chip on its top border) leaves a narrow box down the side with one state glyph per session and, in amber, how many need you: click a glyph to go to it, `»` (`«` with the sidebar on the right) or `…` to expand the sidebar again. Leader then `b` hides the sidebar and brings it back; hidden, a `»` chip at the start of the tabs also brings it back. After the leader, `[` and `]` make the sidebar narrower and wider.
- A machine whose line is down, or that has no sessions, is its header and one line instead of its
  projects; click it for the Machines dialog. See *Machines* below.
- Nothing moves in the sidebar by itself: a session that needs you or finishes keeps its row (its mark and name say so, and the Inbox lists what needs you). Idle agents stay running; no sleep screen is shown for sessions saved by older releases.
- A session taking a real share of the machine says so in its pane's title, beside its state
  (`▲ 38% CPU · 2.1 GB`, from a fifth of the CPU or two gigabytes).
- The sidebar lists each machine with its sessions. `Ctrl+T`, `Ctrl+Space t` and a machine's **+** start a shell at once where you are; `Ctrl+Space n` and the **n new session** chip when no pane is open do the same. Type your agent in the shell.
- `⇆` under a row names a file that this checkout and another have each changed since their
  branches parted (a merge conflict waiting to happen); more than two are one line with a count.
- Drag a project, or a session within its project, up or down to reorder it. Click a project to fold or open it. With the sidebar focused
  (`Ctrl+Space` then `←`), a row's menu letters work directly (a row's menu lists them): `x` (or
  Delete) asks to close, `r` renames. Settings › General › UPDATES shows the version; Settings warns at the top when the installed, window and daemon versions differ.

The bar along the bottom always says which mode you are in: ` PANE ` while keys go to the
pane, with the session you are on. The leader key is `Ctrl+Space`. Press it (or click ` PANE `) for
**key mode**: the chip turns to a yellow ` KEYS ` and the bar lists keys (`n new  s split  x close
tab pane  ? help  esc back`; a click on one runs it), the pane you're in turns sky, each key below
runs its action, and the mode stays for the next key, so `j j k` walks the sessions. A key that does
nothing says "Shortcut mode — Esc to type". `Esc`, `Ctrl+Space` again or a click into a pane go back to
the pane, and so does a new shell, session or split: the next key goes to the new pane. Closing a
popup returns to key mode. While the sidebar, a popup or a panel has the keys, the bar names it and
lists its keys (` FILES ` … `/ filter  i edit  enter insert path  esc close`). With **Leader waits for
next key** on (Settings › General; off by default), a pause after the leader shows the compact key map:
a key runs its command, Tab (or typing a letter that is no key) searches every command, and keys marked
`›` open a second step (Worktrees: new, switch, merge, delete; Backspace goes back). `?` is the
keys help. Only `?`, `S` and `_` need Shift, and each key does one thing; no letter acts by itself in
the sidebar. Every default shortcut, and every screen's own keys, are in [shortcuts](shortcuts.md).

Without the leader: **Alt+1–9** opens that session of the sidebar; **Alt+arrows** moves pane focus
when enabled in Settings (after the leader, Alt+arrows swaps panes instead).

**Files**, **Changes**, **Branch** and **References** share the side panel that slides in from
the pane's right edge, over its box (it covers the pane's top line and its own `✕`): its title chip names the open one (lit while the panel has
the keys) and `✕` or Esc closes it. A pane's `files` and `changes` chips open those two; Tab / Shift+Tab
moves between the four for the same checkout. A click into the pane gives it the keys and leaves the
panel open; a click in the panel, or its key again, gives the keys back.

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
narrow or widen it and `=` puts it back (after the leader, `[` and `]` resize the sidebar instead);
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
- **Files to another machine:** in a pane on another machine, `Ctrl+Space`, then `d`, asks for a
  file's path (type it, or drop the file on the terminal: the path is pasted into the box). The
  file (up to 100 MB; a folder is sent too, up to 1 GB, see *Sending a file or a folder to a machine*) goes to that machine over ssh the same way as an image,
  into the same private folder (`~/.cache/maitty/pastes`), and its path there is typed into the pane.
  The name there is `paste-<time>-<id>-<your file's name>`, so two files never overwrite each other.
  A toast shows the percentage; `Ctrl+Space`, `d` again (or the toast's close mark) stops it.
  A sent file stays on that machine for a week, readable by you only, so do not send a secret you
  would not leave there. A paste of text never sends a file, even when the text is a path.
- **Keys:** on Windows, combos plain terminals can't send (Ctrl+Shift+letter, Ctrl+Enter, Ctrl+Tab)
  reach programs exactly, as native key records. Shift+Enter is still a newline for agents.
- **Resize:** text re-wraps to the new width (history included). Programs' cursor shape (bar,
  block, underline) shows. Synchronized redraws are drawn whole, so agents don't flicker.
- **Closing** a pane ends everything running in it after you confirm.

## Alerts

Needs-you alerts use two rising notes and stay quiet for the session you're looking at; the
same session asking again within 2 minutes stays quiet too. A background
session's completion uses one soft note, once for each thing you asked for: an agent that stops, picks itself back up (a helper or a background job reported in) and stops again does not ring a second time until you type into its pane, or two minutes have passed. The session you're viewing stays quiet. Other panes get an in-app completion notice. A needs-you alert is delayed briefly
to ignore transient prompts and is cancelled if the agent resumes work. Sessions started
by another agent do not alert when they finish. With no maitty window open, the server
plays the needs-you sound and shows a desktop notification. `[notify] sound = false`
(Settings → Alerts → Alert sounds) turns every sound off. Each sound has its own switch beside it: `sound_on_needs` (Sound when an agent needs you) and `sound_on_done` (Sound when an agent finishes), and `volume` (Sound volume) is `quiet`, `normal` or `loud`. `maitty test-alert` plays both sample tones at that volume.

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

## What a session's status means

Every agent session has one status. Maitty works it out from the agent's own reports, its screen and its process, in that
order of trust; how to connect an agent so it reports for itself is in [agent-status](agent-status.md).

| Status | What it means | Desktop app | Terminal app |
| --- | --- | --- | --- |
| Working | The agent is busy on a turn. | Spinner; listed under **Working**. | Spinner in the accent colour. |
| Needs you | It asks a question, wants permission, asks for a secret or wants to start helpers. | Amber mark; under **Needs you**. | Amber `●`, and it is in the Inbox. |
| Done | A turn finished and you have not looked yet. | Green check; under **Finished**. | Green mark. |
| Idle | Nothing to do: you looked at a finished turn, or the agent has not been given work. | Under **Idle**. | A muted `●` (`○` with Unicode status glyphs on). |
| Limited | It hit its plan limit and is waiting for it to reset (see below). | Under **Needs you**. | Amber `limited 16:30`. |
| Failed | The last command in the session failed (in the terminal app, a failed check too). | A shell reads `exited`, an agent `stopped`; listed under **Needs you**. | Red mark. |

A finished turn stays **done** until you look at it: open the session in the desktop app, or focus its pane in the
terminal app. Then it becomes **idle**. In the desktop app a finished session's menu also has **Mark as done**, which files it
under Idle without opening it. A session you cannot reach (its machine is down) is shown as not known, with its last status,
until the machine is back.

### Why a session has its status

Every status change records a short reason, in the daemon's own words. Typical ones:

- `hook: Stop`: the agent reported the event itself (here, the end of a turn).
- `screen: matched …` or `screen: it's printing`: Maitty read it off the agent's terminal.
- `process: an agent started`: Maitty saw an agent start in the pane.
- `you looked at it`: you opened a finished session, so it went from done to idle.
- `you answered it`: you answered a question the agent asked through Maitty.

**In the desktop app**, rest the pointer on a session's row for about half a second. A card opens beside it with the
agent, its changes and, under **Set by**, the reason and how long ago it happened (`hook: Stop, 3 s ago`). The line is left
out where it would be about some other status: when the status is not known, for a stopped or a limited session, and
for one waiting for you to allow helpers.

**In the terminal app** there is no such card. The reason goes to `daemon.log` in the data folder, one line per change
(`pane 4 (claude): Working -> Done: hook: Stop`), and `maitty ls --json` carries it as `status_why`.

### Limited

When an agent stops at a plan limit (the screen says things like "usage limit reached", "you've hit your weekly limit" or
"out of extra usage"), Maitty works out when it can go on. That time is the reset of a limit its usage tracking shows as
95% used or more; with none, it tries again in half an hour. Exactly one minute after that time Maitty types `continue`
into the agent and says so in a notice. This needs auto-continue, which is on by default; with it off, nothing is typed
and the session is not marked limited.

Until then the session is **limited**: its row says when, in your computer's time (`limited until 14:05` in the desktop app,
`limited 16:30` in the terminal app; with the weekday when that is not today). It goes back to its normal status when the
time comes. If you continue the agent yourself, the limit is forgotten.

Only a session that was done or idle becomes limited; one that is working, asking or failed keeps that status. In the
desktop app a limited session is listed under **Needs you**, but it waits on the limit and not on you, so it is left out of
the count of sessions elsewhere that need you, the jump to the next session that needs you, the tray and the notices. (A
failed session is counted in the jump and the tray.) A limited session cannot be put away; you can [snooze](#snooze) it.

## Session visibility

Sessions stay visible in the sidebar until you close them. By default, idle, completed, and merged-worktree sessions are
never hidden automatically. To change that, set `put_away_merged = true` under `[sessions]` in `config.toml`: a finished or
idle session whose pull request has merged is then put away on its own, within a couple of minutes. It keeps running. If you
bring it back, that merge never puts it away again.

In the desktop app you can tidy the list yourself: right-click a finished or idle session (an agent in a terminal, or a
shell) and choose **Put away**. A toast offers **Undo** for 6 seconds. The session keeps running; it leaves the status,
project and machine lists and waits in the folded **Put away** section at the end of the session list, above Terminals. Click its arrow, or
choose **Bring back** in its menu, to list it as before. If a put-away session needs you, it shows under **Needs you**
until you have answered it, then goes back under Put away. A chat started in the desktop app has no Put away; close it
instead.

The TUI does the same from a session's row menu (`.` or right-click): **Put away**, with Undo for 6 seconds (the leader,
then Ctrl+Z). The row leaves the sidebar while the session keeps running. The sidebar's **Menu** chip → **Put away…** (or
**Sessions put away** in Search commands) lists them; pick one to bring it back. Putting away in either app puts the
session away in both.

A session that needs you is not hidden. In the terminal app it stays in the sidebar while it needs you, and in the desktop
app it moves to **Needs you**; either way it keeps its put-away mark and goes back out of the way once it no longer needs you.
A session whose last command failed, or a limited one, cannot be put away; an agent that fails or becomes limited after being put away is listed again until that passes.

### Snooze

Snooze puts a session away until a time, then brings it back by itself. It is in the desktop app only; the terminal app
has no snooze. Right-click a done, idle or limited session (an agent in a terminal, or a shell; not a chat) and choose one of:

- **Snooze 1 hour**;
- **Snooze until tomorrow 9:00**, by your computer's clock;
- **Snooze until the limit resets**, offered only for a limited session whose reset is less than a week away.

A toast offers **Undo** for 6 seconds. A snoozed session waits in the **Put away** section with `until 14:05` (`until Thu 09:00`
on another day) beside it, and **Unsnooze** on hover and in its menu. **Bring back** ends the snooze too. A snooze ends by
itself when its time comes, and at once when the session needs you: you cannot snooze one that needs you now. A snooze
lasts at most a week and is kept by the daemon, so every window sees it.

### Undo close

A closed session is not gone at once. It leaves the list but keeps running for 6 seconds, and **Undo** brings it back. In the desktop app, click **Undo** in the toast, or press
`Cmd+Z` on macOS and `Ctrl+Shift+Z` elsewhere. In the terminal app, press the leader (`Ctrl+Space`), then `Ctrl+Z`, while the banner
shows. After 6 seconds the process ends. Putting away has the same 6-second Undo; it ends nothing, so Undo only lists it again.

## Follow-ups and snapshots

`Enter` in shortcut mode (or **Message** in the row's menu) writes an agent a follow-up without leaving where you are. While the agent is
working, Enter leaves the message with maitty and it's typed in when the turn is over (never
into a question the agent is asking); Ctrl+Enter sends it at once. What's waiting shows under
the row, and the row's menu drops it.

Each time an agent's turn begins, and again when it ends, maitty keeps the checkout's files
as they are (tracked or not; never what git ignores), so one turn's changes are the
difference between its two snapshots. A turn that changed nothing adds none. A turn ends
when it finishes, fails or is cancelled; stopping to ask you something is not an end.
**Snapshots…** in the agent's menu lists the last nine, and the Branch view (leader + `g`)
lists them all; picking one puts the files back as they were then. What's there now is kept
as a snapshot first, so a restore can be taken back the same way. It is refused while the
agent is working, since it would keep writing into the files being put back. Snapshots are
commits under the worktree's own refs (`refs/worktree/maitty/snapshots`): no branch, nothing
in your history, your staging untouched, the newest 60 kept (at least thirty turns).
`[snapshots] enabled = false` turns them off.

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
(leader + `e`). They are plain files in `.maitty/references/` at the repo's main checkout (a
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
request open on each pane's branch every couple of minutes and shows it on the session's row in
the sidebar (and in the desktop app's pane header): `#7 2 failing`, `#7 changes requested`, `#7 conflict`, `#7 checking`, `#7 approved`,
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

Four more keys decide what a machine may do. `clipboard` is on unless you write `clipboard = false`: a copy made in its panes reaches your clipboard. The other three are `false` until you turn them on. `clipboard` lets its panes
write your clipboard, `notices` lets it raise notices (which include bringing this window to the front) and
`sounds` lets it play sounds; what it raises starts with `[build]`. `mcp` opens it to agents (see *Agents on a
machine*). `config.example.toml` lists every key.

Adding a machine starts maitty there if it is not running, so you can create the first session from this
computer. If the connection drops later, maitty only reconnects to a maitty that is still running there; it
never starts one you stopped. Right-click the machine's heading in the sidebar and choose Reconnect to start it there again. If maitty is not
installed there, its heading has an **install** button: see *Install or update maitty on a machine*.

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
| `needs login` | ssh cannot log in without asking you. Make plain `ssh` to that machine work in a terminal (load your key, accept its host key), then right-click the machine and choose Reconnect. |
| `needs update` | The machine runs another version of maitty, or has none on its PATH. Click **install** (none there) or **update** (another version) on the machine's heading in the sidebar, or right-click the heading and choose Install Maitty / Update Maitty: it asks in a popup, then reconnects. From a terminal: `maitty machine update <name>` (add `--install` when there is none, or set `remote_cmd` to its path). |
| `unreachable` | No answer, or maitty is not running there. The reason is written under it (ssh's last line, or "maitty is not running on that machine; retry to start it there"); maitty keeps trying in the background. |
| `refused` | A different computer now answers at that name. If you replaced the machine, remove it and add it again; otherwise check the host name and DNS. |

When a machine drops, its cards give way to one item saying how it stands, such as `needs update · click for the fix` (click it for the
Machines dialog, which says what to do and can reconnect, install or update, edit or remove it; right-click the machine's heading for Install / Update Maitty, a new shell, Reconnect and the Machines dialog). They come back by themselves when the line does, each pane showing
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
- **maitty where ssh finds it.** Maitty looks in `~/.local/bin` (where it installs itself) first, then on the
  PATH ssh gives it. Only a copy kept somewhere else needs `remote_cmd` set to its full path.
- **A version close enough.** This computer, the machine and any desktop app talk when their versions are the
  same or close enough that the newer one still speaks the older one's protocol; releases say when that
  stops being true. A machine too old to talk to shows `needs update`; run `maitty machine update <name>`. A machine too new for
  this computer shows `newer than this app` and has no update button: update this computer instead
  (`maitty update`, or the desktop app's own update). A machine is never downgraded from here.

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
maitty machine update build --yes --restart  # also restart its daemon now, even with sessions open
maitty machine activity build           # how many sessions are open there and how many are busy (changes nothing)
```

```
probing build…
build: linux x86_64, maitty 0.58.0 (this is 0.60.0)
sending 14.2 MB (sha256 3f2a9c1b7d04)…
verified: it arrived whole and runs there
installed to ~/.local/bin/maitty
asking the daemon on build to restart if no session is open…
daemon: left running: 3 open session(s); it keeps them   (with --restart: restarted, sessions come back)
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
  `~/.local/bin/maitty`, which is the first place Maitty looks on that machine, so nothing else needs setting.
- **Running sessions survive an ordinary update.** The old daemon keeps running on the old file. The new
  maitty restarts it only when it has no session open; otherwise it says it left it running. Add `--restart`
  (the desktop app's Update button does, after asking) to restart it anyway: its sessions are saved, so the
  new daemon restores them and resumes their agents. A turn in progress is lost. A daemon whose sessions are
  not saved is never restarted.
- **Not supported: a Windows machine as the remote.** In v1 Windows is never the remote; the command says so
  and exits. Linux and macOS (x86_64 and aarch64) are supported.

**From the terminal app.** You do not need the command or its flags: a machine that `needs update` has an
**install** or **update** button on its heading in the sidebar (also in its right-click menu, the sidebar's
**menu**, and `u` on it in Machines, leader `w`). It runs the same command in a popup on this computer, which
says what it found and asks before it installs or replaces anything; your `y` is the consent. When it is done
the popup shows how it ended (installed, already up to date, not confirmed, refused because a newer version is
there, or why the machine could not be reached) until you press Enter, then the machine reconnects.

When the version there already matches, nothing is sent, but its daemon is still asked to restart if it is not running
that version (an earlier update may have left it running because sessions were open). A daemon that already runs it is left alone.

Exit status: 0 done or already up to date, 2 a mistake in how it was asked (an unknown machine, a flag
missing), 1 a step that did not work.

### Getting a file or a folder from a machine

```sh
maitty machine get build ~/out.log          # saved in the folder you are in
maitty machine get build /srv/app/app.zip ~/Downloads   # saved in that folder (made if missing)
```

`maitty machine get <machine> <path> [folder]` pulls one file or one folder from a machine you added, over ssh,
and prints the path it was saved to. A `~/` at the start of the path is the machine's home folder.

- **One file or one folder, up to 1 GB.** A folder arrives as a folder, with its sub-folders. A `.zip` you already
  have is just a file, so it travels like any other. Symlinks inside a folder arrive as the files they point to.
- **The name is made here.** The file keeps the last part of the remote path with anything but letters, digits,
  `.`, `_`, `-` and spaces changed to `_`. Windows reserved names (`CON`, `NUL`, ...) get a leading `_`.
- **Nothing is overwritten.** If the name is taken the file is called `out-1.log`, `out-2.log`, and so on.
- **All or nothing.** The file or folder appears under its name only when everything arrived. A failed or stopped
  pull leaves nothing; a hidden `.<name>...` file or folder can stay only if the program itself is killed mid-pull,
  and is safe to delete.
- **A folder is checked before it is unpacked.** The machine is not trusted, so a folder is refused whole, with the
  first bad entry named, if it holds a link, a device or any special file, a path that leaves the folder, a name
  this computer could not store safely (a backslash or colon, a Windows reserved name such as `CON`, a name ending
  in a dot or space, odd or invisible characters), more than 50000 entries or a path deeper than 64 parts.
- A pulled file can hold secrets that were on the machine. It is saved with your normal file permissions.

Exit status: 0 saved, 2 a mistake in how it was asked, 1 the pull did not work (the message says why).

### Sending a file or a folder to a machine

```sh
maitty machine put build ./report.pdf     # one file, up to 100 MB
maitty machine put build ./my-project     # one folder, up to 1 GB
```

`maitty machine put <machine> <path>` sends one file or one folder from this computer to a machine you added, over
ssh, and prints where it is there. It is kept in that machine's private cache folder (`~/.cache/maitty/pastes`, readable
only by you, cleared after a week) under a name like `paste-<time>-<id>-my-project`, so nothing there is overwritten.
Dropping a file on a session of another machine does the same thing for a file. A pulled or sent file can hold secrets,
so do not send one you would not leave in that cache.

Exit status: 0 sent, 2 a mistake in how it was asked, 1 the send did not work.

In the desktop app, right-click a session that is on another machine and choose **Get a file…**. The box lists the
session's folder: click a folder to open it, **Up** to go back, or a file to get it. Type a few letters of a name to
search the session's folder and everything below it (not `.git`, `node_modules` or build folders; the first 5000
files), and use the arrow keys and Enter. If the file is somewhere else, browse up to it or type the path in the box
under the list. The file is saved in `Downloads/maitty/<machine>/` with a progress toast you can cancel and an
**Open folder** button when it is done. It is the same pull as the command. Names with unusual characters are left
out of the list, and the dialog says how many.

### Moving text between machines

Text needs no special command.

- **From this computer to a machine:** paste into the session as you would anywhere. The text is typed into that
  machine's session. A paste of text never sends a file, even when the text is a path.
- **From a machine to this computer:** copy in the session as you would here (select text, or an agent's `/copy`),
  and it reaches your clipboard. The machine's clipboard switch is on unless you turned it off (Settings › Machines, or
  `clipboard = false` in its entry in `config.local.toml`).
- **A long text, or one you want to keep as a file:** save it as a file. To bring it here, use
  `maitty machine get` or **Get a file…** (see *Getting a file or a folder from a machine*). To send a file there, drop it on the
  session.

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

### Machines in the desktop app

The desktop app uses the same machine list as the terminal app (`config.local.toml`), so a machine added in
one shows in the other the next time that app starts.

With one machine the sidebar looks as it always did. Once you have another, every machine's folders sit under
a header: **this computer** first, then the others. A machine's header carries its state as a shape and a
word: `connected`, `connecting`, `reconnecting #2`, `needs login`, `not installed`, `needs update`, `newer than this app`, `unreachable` or `refused`
(a different computer answers at that name). When you have to act, the fix is written under the header, in
full. A machine that is down keeps its folders and sessions, dimmed, with `disconnected since 2m ago`; its
sessions' needs-you and done marks leave the counts and the Inbox until it is back, and everything returns on
its own. An Inbox row from another machine says `· on build`.

**Settings › Machines** (or *Machines* in the command palette) lists them:

- **Retry** dials a machine now and lets a refused one be looked at again.
- **Remove** asks first, then takes the machine out of the list and out of `config.local.toml`. Its sessions
  keep running on it and the layout you saved is kept.
- **Add a machine** lists the hosts in `~/.ssh/config` that are not machines yet, or takes `user@host`. The
  desktop app first looks at the machine over ssh without asking anything (12 seconds at most; *Cancel* ends
  it) and adds it only when ssh could reach it. If ssh needs you to type a password or accept a host key, the
  machine is added and ssh opens in a session named `ssh <name>`; the machine connects when that session
  closes. On Windows use ssh keys or an agent: password prompts work in that session but are not remembered.
  A machine whose app is missing or old is added too; its header then says `not installed` or `needs update`.

A machine with no maitty says `not installed` under its header in the sidebar, with an **Install** button; one
with another version says `needs update`, with an **Update** button. Its right-click menu has the same as
**Install Maitty** or **Update Maitty**, and Settings › Machines has it as the first button. No command or flag is
needed: Install runs `maitty machine update <name> --install` for you, and Update replaces the version there
(`--yes`). Both ask first. Install says what it puts on the machine; Update says how many sessions are open there
and how many are busy, and offers **Update now** (its daemon restarts and its saved sessions come back with their
agents resumed; a turn in progress is lost) or **Update when idle** (the new version is installed now and the daemon
restarts once every session there is closed; it waits up to half an hour, then leaves the daemon running). A toast says it is running, then how it ended: installed, already up to date, or why it did not (ssh
could not get in, the machine has a newer maitty than this app, which is never downgraded from here, or it is a
Windows machine, which cannot be a remote yet). After a success the machine is dialled again. The exact command
is still shown with a **Copy command** button, to run in a terminal instead.

Each machine on that page has two switches. **Copy to your clipboard** (what that machine's sessions ask to
copy is put on your clipboard) is on until you turn it off. **Notifications** (its notices, and a notice when
its sessions need you or finish, each starting with `[name]`) is off until you turn it on. They are written to
the machine's entry in `config.local.toml`, as `clipboard = false` and `notices = true`. The desktop app plays
no sounds, so there is no switch for them.

A new session (**New session**, ⌘T) starts where the focused session is. On another machine it asks for a
folder there, starting at `~`, because the folder picker is this computer's; a folder that does not exist
opens in the machine's home folder. Files, Changes, worktrees and the other features that read a pane's files
are for this computer's sessions only.

**Dropping files or folders on a session of another machine** sends them there: drag up to 10 files or folders from
your file manager onto the pane. Each goes over ssh (not a custom transport) into `~/.cache/maitty/pastes` on that
machine (private, cleared after a week), and its path there is typed into the pane. A file is sent up to
100 MB, a folder (as one archive) up to 1 GB. A toast shows the file or folder, how far it has got, and a **Cancel** button; if a
file fails, the ones after it are not tried. The machine has to be `connected`. A file stays on the machine for
a week, readable by you only, so do not send a secret you would not leave there. Dropping on a session of this
computer types the file's path, as before.

**Pasting an image** works in the desktop app too: copy a screenshot or an image, click the session and press
Ctrl+Shift+V on Windows and Linux, or ⌘V on a Mac. On a session of this computer the image is saved in the app's private cache folder and its
path is typed; on a session of another machine it is sent over ssh (up to 25 MB, not a custom transport) into
`~/.cache/maitty/pastes` there, with the same progress toast and **Cancel** as a file, and its path there is typed.
If the clipboard holds text as well, the text is pasted and the image is left alone. On Linux and Windows
plain Ctrl+V still goes to the program in the pane as the raw `^V`, so Claude Code and vim see it; use Ctrl+Shift+V
to paste. In the chat box any paste key attaches an image.

## Never leave maitty

| Leader + | Panel | What it does |
|---|---|---|
| `f` | **Files** | The project's tree, with the file you pick beside it. `r` switches to recent files (just appeared in Downloads, Desktop, Documents or the project), `/` filters. Enter types the path into your prompt; `e` your editor, `i` edit it in place (Shift+arrows or the mouse select; Ctrl+C, Ctrl+X, Ctrl+V copy, cut and paste), `o` open it outside, `y` copy the path. `f` again or Esc closes it. |
| `c` | **Changes** | A checkout's changed files and the diff: `c` commit, `p` push and open a pull request, `x` mark reviewed, `r` go to its agent; in a worktree `m` merges into the base branch and `d` throws it away. Each of `c`, `p`, `m` and `d` asks first. |
| `e` | **References** | The project's screenshots and design files, newest first: `p` adds the clipboard image, `n` a file by its path (type it or drop it in), Enter opens one in your image viewer, `y` copies its path, `d` removes it (asked first). `e` again or Esc closes it. See *References* below. |
| `g` | **Branch** | Where the session's checkout stands, in the side panel: its branch, how far it is ahead of and behind its upstream and its base branch, the last 10 commits, the last check result and its snapshots. Its first row, **Switch branch…**, opens the branch switcher (local and remote branches; type `#123` to check out a pull request with `gh`); `f` fetches, `p` pulls (fast-forward only) and `P` pushes; Enter on a snapshot restores it (asked first). `g` again or Esc closes it. |

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
Either way the list of closed agent sessions is kept, as the usage history is.
Closing every pane yourself also starts fresh. A pane or agent that ends on its own is left out
of the saved session for 15 seconds: a shutdown, reboot or logoff kills agents a moment before
maitty, so the session from before it is the one that comes back. Chat conversations that were running
resume by themselves on the same provider session (`conversations = false` leaves them in Recovery
needed until you press Resume). A resumed chat waits for you; with `continue_turn = true` maitty sends
a chat that was mid-turn `Continue where you left off.` once after it resumes, and never more than 3 times in 5 minutes to
one conversation, however often the service restarts. The message shows in the chat like one you
sent. A chat whose turn had already ended, and every terminal agent, never get it. Configure all of this under `[restore]`,
and per agent with `resume` / `resume_last`. Each pane's recent output is kept on disk so it is
there after a restart; it can hold secrets that were on screen, so `output = false` (Settings ›
Sessions › Keep each pane's recent output) stops saving it and deletes what was saved.

## The desktop chat

In the desktop app an agent has a chat view as well as its terminal. For an agent that runs in a
terminal it is the same session: the Terminal and Chat switch never restarts the agent. (A chat that
Maitty's background service runs, see [Worktrees](#worktrees), has no terminal.) This part is about working in the chat and
checking what the agent did. What the status marks mean is in [What a session's status
means](#what-a-sessions-status-means).

A session running a workflow that reports to Maitty (`maitty workflow report`) also gets **Work** in that switch, with
the number of runs going and an amber square when one needs you. Work shows its runs as a board: one column per step,
each ticket in the step it is on. Finished runs are hidden until you choose **Show**. Click a ticket to see its steps in
a drawer, with **Open ticket**; click a step's helper to see what that helper was asked and is doing. Maitty only shows
the runs; the workflow itself decides every step.

### Starting a chat

**New chat** (the top of the sidebar, or **New agent chat…** in the command palette, Ctrl+Shift+T, ⌘T
on a Mac) opens a page that asks "What
should we work on?". Pick a project (or paste a path, or **Browse…**), write the first message and
press Enter. The page opens on your most recent project, and shows the project list only when you
have none. The agent starts in its own terminal and the chat opens. (**New agent chat** without the
dots, Ctrl+Shift+A, skips the page: it starts a chat at once with the default agent in the start
folder.)

Under the project there are a few more choices. **Branch**, **Own copy** and **Also start with**
are left out when the folder is not a git repository, and for a session on another machine.

- **Branch.** Pick a branch, or type a new name to create one.
- **Own copy** is on by default. The agent works in a worktree of its own, on a new branch named
  from your message, so your checkout stays as it is. A line under it says what it starts from, and
  how far behind its upstream it is. Turn it off and the agent edits the checkout. Then, if the
  checkout is behind, a **↓ Pull** button runs `git pull --ff-only`. It is refused while you have
  uncommitted changes.
- **Model, effort and permission mode** sit on the buttons under the message box. The agent is on
  the model button too. If an agent is not installed, the page says so and offers **set it up**.
- **Suggested tasks** under the choices start a chat on that task with one click, and **Resume**
  lists the conversations Claude or Codex saved for the folder. Resuming starts in that folder
  without a worktree.

**Several agents at once.** Under the message box, **Also start with** lists the other agents
installed on this computer. Tick one or more and the same message goes to each, every one in its own
worktree and on its own new branch (`fix-login`, `fix-login-2`, …). That needs Own copy. With it off
the boxes cannot be ticked, and the page says "Each agent needs a copy of its own. Turn on Own copy
to start several." The chat you
see open is the agent on the model button. The others start on their own default model, effort and
permission mode, and they are listed in the sidebar. They start only when you start a new chat, not
when you resume a saved one,.

**Keep drafting.** Press Ctrl+Enter (⌘Enter on a Mac) instead of Enter. The chat starts and is listed
in the sidebar, but it is not opened. The page stays, the message box and its files are emptied, and
every other choice is kept, so you can write the next chat while the first one starts. With Own copy on,
the next one gets a new branch name. If a start fails, a line under the box says why, with **Put back**,
which puts the message and its files back in the box after whatever you typed since, and **Dismiss**.
On another machine Ctrl+Enter is a plain start. Esc closes the page.

### Answering prompts

When an agent stops to ask you something, its session needs you and the chat shows a card over the
message box. The message box is off, and says why, until you have answered.

- **Wants to run a command** (or "Wants to use <tool>"). For Claude the card has **Yes once** and
  **No**, and the keys `y` and `n` do the same from anywhere in the chat except a text field. They
  type Claude's own answer into its terminal.
- **A plan Claude wants to start on** (in plan mode). The card shows the plan, formatted, with
  **Approve** (`y`), which picks the first choice of Claude's plan menu, and **Keep planning** (`n`).
- **A question Claude asks.** Each choice is a button, and its number key picks it. A prompt with several questions (up to 4, each
  with 2 to 4 choices) shows each one with its choices and one **Send answers** button, which stays
  off until every question has an answer. Nothing is typed to the agent before then, so you can still
  change an answer. A question beyond those limits, or "Type something", is answered in the terminal.
- **Codex, and any menu the chat has no buttons for.** Maitty does not press a button for these,
  because it has not checked that those keys are right. The card shows the question as the terminal
  draws it, with a row of **↑ ↓ ← → Enter Esc** buttons. They press those keys in the agent's terminal,
  the way you would. For a session on another machine the screen cannot be read: the card says the
  question is in the terminal and keeps the keys.

A chat that Maitty's background service runs (see [Worktrees](#worktrees)) shows **Yes once**,
**This session** and **No** instead, with **Always in this project** for a command.

### Queue and Send now

While the agent works you can still write. Enter queues the message, and it goes in when the turn
ends. The queue is kept by the daemon, so closing the window loses nothing. Each queued message is a
dashed card above the box that says **Queued**. With several, they stack with the next one in front
and show `1 of 3`; click the stack to spread them out. A card has:

- **Edit**, which takes the message back into the box, with its files, to change it. Nothing has been
  sent.
- **Remove**, which drops it.
- **Send now** (an up arrow), which does not wait for the turn to end.

What Send now does depends on the chat. For an agent in a terminal it types the message into the turn
that is running, as typing there would, so the agent reads it at its next step. For a chat that
Maitty's service runs, Codex takes the message into the running turn where its version supports that.
Otherwise, and for Claude, the turn is interrupted and your message is sent next, ahead of the others
in the queue. If the agent refuses (the turn just ended, or it is waiting on an approval), the reason
shows under the box and the message stays queued. The button is hidden while a question is showing, and
off while Maitty waits for the answer.

### Naming a file with @

Type `@` at the start of the message, or after a space, then a few letters of a name at the end of
what you have written. A list of the
session's files that match opens over the box. The arrow keys move, Enter or Tab inserts, Esc closes,
and a click picks one. The path goes in relative to the session's folder, followed by a space. A path
with a space in it is put in quotes. At most 50 are listed, so keep typing to narrow them. An `@`
inside a word, such as an email address, opens nothing.

### What a turn changed

A turn that edited files ends with a row such as "3 changed files +111 −0" and **Review**. Click it
and the side panel opens on **Changes** with only what that turn changed, headed "This turn's
changes". **All changes** goes back to the whole working tree.

This works from the snapshots Maitty keeps of the checkout. If none from before the turn (or from
after it) is kept, or the turn has no times, or the snapshots cannot be read, the panel shows the whole working tree
instead, and says why: "Whole working tree · no snapshot from before this turn is kept" (or "this
turn has no times to find its snapshots by", or "couldn't read the snapshots"). The turn's diff covers the
checkout, not the agent, so a file you or another agent in the same checkout edited meanwhile is in
it. This is for sessions on this computer.

### Edit from here

Point at a message you sent and open its **⋯** menu, then choose **Edit from here**. Maitty puts the
files back to how they were before that message, and puts the message back in the box so you can change
it and send it again. It asks first. The prompt says how many files change back (**Restore 3
files**), and that what is there now is kept as a "before a restore" snapshot. Nothing is changed
until you confirm. If you had typed something in the box, the message goes above it.

The agent's own conversation is not rewound. The next message you send starts with one line telling
it the files were restored, and a line under the box lets you drop it (**Leave it out**).

It is greyed out, with the reason in the menu, while the agent works, while the service is offline,
for a session on another machine, and, with the reason "Another session is using this folder", when
another open session on this computer, other than a plain shell, is in the same checkout, even if it
is idle. A restore would undo that session's files too. If no snapshot from before the message
is kept, a notice says so and nothing is restored.

### The worktree manager

**New worktree** (Ctrl+Shift+N, ⌘⇧N on a Mac, or the command palette) opens a dialog that makes a branch in its own folder next to the
project. When the project already has other worktrees, the dialog lists them below. Each row says
where that worktree stands, for example "inactive 3 days · not merged anywhere · last commit …" or "2
uncommitted changes", and has:

- **Merge**, which merges its branch into the main checkout's branch (never a fast-forward). It is off,
  with the reason, when the worktree has uncommitted changes, has no branch, or its work is already on
  another branch. After you click, it can still be refused because the main checkout has uncommitted
  changes, or because of a conflict; a conflict aborts the merge and leaves the main checkout as it
  was. Merge leaves the worktree and its sessions in place (the terminal app's `m` removes them).
- **Remove**, which removes the worktree's folder. It does not remove one something still runs in, so
  close its sessions first. The branch stays unless it is merged. When that would lose uncommitted changes, or commits that are on no other branch, the
  button first becomes "Lose … ? Remove" and you press it again.
- **Recreate**, for a worktree whose folder is gone, which makes it again from its branch.

If two worktrees changed the same file, a line at the top says so, so you hear about a merge conflict
before the merge. The list is for projects on this computer. For the rest of how maitty names and
cleans up worktrees, see [Worktrees](#worktrees).

## Worktrees

```sh
maitty worktree feat/login -- claude      # new branch from HEAD, opened with claude running
maitty worktree fix-123 --base origin/main
maitty worktree-remove [--force]          # closes the workspace and removes the checkout
```

New worktrees go to `{repo_parent}/{repo}-worktrees/{branch}` (set `[worktree] dir`). An existing
local or `origin/` branch is checked out instead of created. Set `[worktree] command = "claude"`
to start an agent in every new worktree.

In the desktop app, **New agent on a branch** (the branch button in the sidebar, ⌘K, or a
repository's right-click menu) takes a project, Claude or Codex, an optional first prompt and a
branch (`agent/<words of the prompt>` until you type your own). It makes the worktree the usual way
(same folder naming, same repo setup hook, same clean-up when its last session closes), starts the
agent in it with the prompt, and opens it in a new tab under its project. The prompt is handed to the
agent as one argument, never run by the shell, and cannot start with a dash. It is for projects on
this computer.

The same dialog has a **Session** choice: **Terminal** (the default, as above) or **Chat**. A chat
session is Claude or Codex run by Maitty's background service instead of in a terminal, so the chat
has a message box that works: Enter sends, Stop interrupts, a message sent while the agent is busy
waits in a queue (with Remove), and each permission the agent asks for appears in the chat with
**Yes once (y)**, **This session (a)** and **No (n)**; a command also has **Always in this project**,
which approves that exact command (written the same) in that project from then on. Those are kept
in `chat-allowed-commands.json` in Maitty's data folder, never in the repo. Settings → Agents lists
them with a **Forget** for each, which takes effect at once (a second service running on the same
computer drops it only when it next starts). Nothing else is ever remembered: a file change, an extra permission or a
question always asks. A chat session has no terminal (to use the agent's own screen, start a terminal
session), it keeps running when you close the window, and closing its tab or pane stops the agent a
few seconds later, so Undo can bring it back. A
shell's empty chat card also offers **Start a Claude chat** and **Start a Codex chat** for the shell's
folder. If you typed `claude` or `codex` in a shell yourself, its chat is read-only: it shows what
the agent did and you answer in the terminal.

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
**J** copies the selection as one line: each line's indentation goes and the lines join with a space (with nothing where a line ran to the last column, a word cut by the wrap), for a command that wrapped. With the mouse, hold **Alt** while you let go of a drag to copy the same way.

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
The reason for each status is described under *What a session's status means* above.
