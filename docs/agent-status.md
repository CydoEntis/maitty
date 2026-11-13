# Connect agents (status setup)

Maitty works out whether an agent is working, needs you or is done by watching its process and screen. **Connect agents** installs a small hook or adapter into Claude Code, Codex, Gemini CLI, Qwen Code, OpenCode, Pi, OMP, Kilo, Kimi or Mastra Code so the agent reports that itself. These are exactly the agents `maitty integrate` sets up. The installer (and every update) runs `maitty integrate` for you unless `MAITTY_NO_INTEGRATE=1` is set.

Search commands (leader, then `Space`) for **Connect agents**, or open **Settings › Agents ›
Connect agents…**, then click the agent.
Setup installs
only Maitty's hooks or adapter; it preserves other hooks and extensions and refuses to overwrite a
different file named `maitty.js`. It does not interrupt an agent or restart the daemon.

Set up the integration on each machine where the agent runs; it lives in that
machine's agent config, not Maitty's synced config. `maitty doctor` checks the
machine-local adapters and says when an installed setup needs a provider reload.
The check confirms configuration presence, not live reporting: native status is
confirmed only after the running provider sends Maitty an event. Until then,
process and screen detection remain the fallback. Restart Codex and OpenCode
after installation. In Pi, use `/reload` or restart it.
The CLI equivalents, including removal, are:

```sh
maitty integrate              # every installed agent, plus MCP
maitty integrate --uninstall  # removes every one
maitty integrate claude
maitty integrate codex
maitty integrate gemini
maitty integrate qwen
maitty integrate opencode
maitty integrate pi
maitty integrate omp
maitty integrate kilo
maitty integrate kimi
maitty integrate mastracode
maitty integrate pi --uninstall
maitty integrate codex --uninstall
maitty integrate opencode --uninstall
```

With no agent named, `maitty integrate` connects each agent that is on PATH or whose config
folder exists, skips the rest without creating anything, then registers MCP. It prints one line
per agent (connected, already connected, skipped: not installed, failed: reason), tries every
agent before failing, and changes nothing on a second run.

Codex installs `SessionStart`, `SubagentStart`, `SubagentStop` and the turn lifecycle
(`UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `Stop`, `Interrupt`)
in `$CODEX_HOME/config.toml` (or `~/.codex/config.toml`). SessionStart records the
root thread without changing pane status, allowing thread-specific rollout-file
detection to work when Codex does not report child lifecycle events. Native
child events provide immediate helper identity and lifecycle when available.
Windows also gets Codex's `commandWindows` field with PowerShell's call
operator; Linux and macOS retain the normal shell command.
Maitty preserves existing Codex hooks and another program's `notify` command;
turn completion still uses `notify` when that key is available.

The lifecycle hooks read Codex's own payload (the same dialect as Claude's): a prompt, a
tool or an approved tool is working, `PermissionRequest` is needs you, `Stop` is done and
`Interrupt` is idle. A `PreToolUse` for the `request_user_input` tool, Codex asking you a
question, is needs you. A hook that prints nothing leaves Codex's own approval flow
alone, and `PermissionRequest` does not fire at all where approvals are off (YOLO).

**Codex runs a user hook only once it is trusted.** Review Maitty's hooks once in Codex's
`/hooks`; an untrusted or edited hook is skipped. Maitty never writes the trust itself.
Settings › Agents says `hooks installed · trust them in Codex /hooks` until Codex has
recorded trust for each. A pane switches to hook status the moment its first lifecycle
hook arrives (`Term::lifecycle_hooks`); until then, and for a pane whose hooks are not
trusted, Maitty reads Codex's screen as before.

Checked against Codex 0.166 on Windows (2026-11-01): `SessionStart`, `UserPromptSubmit`,
`PreToolUse`, `PostToolUse` and `Stop` all fire in a plain `codex exec` turn, each hook gets
its event's JSON on stdin (`hook_event_name`, `session_id`, `turn_id`, and `tool_name` or
`last_assistant_message` where they apply), and an untrusted `-c` hook was skipped. Not yet
seen live: `PermissionRequest` (needs an approval) and `Interrupt` (needs an Esc), taken from
Codex's published hook schemas. A hook run through PowerShell costs about 150 ms, so a tool
call that fires `PreToolUse` and `PostToolUse` takes about 0.3 s longer, inside Codex's 3 s
hook limit.

Pi loads `~/.pi/agent/extensions/maitty.js` (or
`$PI_CODING_AGENT_DIR/extensions/maitty.js`). Use a current Pi with
`agent_settled`, `ui_prompt_start` and `ui_prompt_end` events. The adapter reports
working at agent start, needs-you while a blocking extension dialog is open,
and done only after automatic retries/continuation settle. It does not interpret
ordinary assistant prose as a blocking dialog. See Pi's
[native lifecycle and extension contract](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md).

OpenCode loads a server plugin for standalone `run`/`--mini` and a per-TUI bridge
for interactive sessions, including attached/shared servers. Setup writes beneath
`~/.config/opencode` (or `$XDG_CONFIG_HOME/opencode`), registers the V1 bridge in
`tui.jsonc`/`tui.json` and the V2 entrypoint in `cli.json` when doing so will not
skip OpenCode's settings migration. Comments, custom plugins and settings survive
setup/removal. Re-run setup after a deferred V1-to-V2 migration.

The bridge owns the pane's selected session and its subtree; another terminal's
events cannot change it. It follows busy/retry, permission, question, idle and
error events. Several unanswered requests keep needs-you
until all are resolved; a helper finishing cannot finish its parent. Adapter
events are ordered and unrelated conversation events are not forwarded. See
OpenCode's [plugin contract](https://opencode.ai/docs/plugins/) and
[question event schemas](https://github.com/anomalyco/opencode/blob/dev/packages/schema/src/v1/question.ts).

These are terminal-app integrations: selecting z.ai, DeepSeek or another model
inside Pi/OpenCode uses the same status events. Providers do not need separate
process matchers or hooks. Process detection recognizes Pi's current
`@earendil-works/pi-coding-agent` and older `@mariozechner/pi` packages.
The separate `@deepseek-ai/dsh` CLI is also recognized as DeepSeek, but is not
covered by the Pi/OpenCode native adapters.

Both adapters remain inert outside Maitty. Reports carry only lifecycle/status
and session identity/working directory, with the pane's inherited authentication. A reporting
failure surfaces through Pi's warning UI or OpenCode's log and later events retry.

OMP uses `~/.omp/agent/extensions/maitty.js` (`PI_CONFIG_DIR` and
`PI_CODING_AGENT_DIR` follow OMP's own configuration). Pi and OMP must use
separate directories. Nested `OMPCODE=1` agents cannot own the parent pane.
Approvals/ask tools remain blocked until resolved; automatic continuation stays
working and retryable failures have a 2.5-second retry grace before needs-you.
Kilo uses `~/.config/kilo/plugin/maitty.js` (honoring `XDG_CONFIG_HOME`).
Mastracode uses flat command arrays in `~/.mastracode/hooks.json`.
Kimi uses native `[[hooks]]` tables in `~/.kimi-code/config.toml`
(`KIMI_CODE_HOME` overrides it). Setup preserves custom hooks; `--uninstall`
removes only Maitty's entries.

Maitty recognizes all 24 identities in the pinned Herdr registry plus Aider, Goose,
Crush, Auggie and the separate DeepSeek harness. The 22 bundled Herdr manifests
use prioritized live-screen/OSC evidence; OMP and Mastracode rely on their native
integrations. A known executable alone does not guarantee native lifecycle support.
The pin is Herdr commit `143bf5fd4781d164260c63fe28e7dc08054ad32f`; its license and
notices are in `vendor/herdr-detection/`.

What each agent sends at turn end (measured 2026-11-06 on Linux with a one-word turn, no tools,
in a scratch folder, the payload dumped by a stand-in script):

| Agent | Version | Last assistant message | Field |
| --- | --- | --- | --- |
| Claude Code | 2.1.319 | yes | `Stop` hook stdin: `last_assistant_message` (also `transcript_path`, `background_tasks`, `stop_hook_active`) |
| Codex | 0.166.0 | yes | `notify` argument: `last-assistant-message` (also `thread-id`, `turn-id`, `input-messages`, `client`). The Windows `Stop` hook check above saw `last_assistant_message`; the Linux `Stop` hook was not re-run because it needs a trusted hook |
| Gemini CLI | not installed | not measured | |
| OpenCode | not installed | not measured | |

The screen fallback (`daemon/last_lines.rs`) was not run against a live Claude or Codex screen:
that needs a pane inside a running daemon, and Claude and Codex already send the message, so
the fallback is not used for them. Its readability on a real screen stays unmeasured.

Permission-prompt lifetime (ticket 31, `NEEDS_SETTLES`), measured 2026-11-06 on Linux with
interactive Claude Code 2.1.319 in a PTY (`scripts/measure-permission-prompt.py MODE TURNS`;
one "create x.txt" turn each, 24 real turns). The time is from the `PermissionRequest` hook to
the `PostToolUse` hook, which includes the hook's own process start (about 50 ms) and the write:

| Case | n | min | median | p95 | max |
| --- | --- | --- | --- | --- | --- |
| Approved at once by a `PermissionRequest` hook | 8 | 111 ms | 147 ms | 179 ms | 179 ms |
| Auto permission mode (`--permission-mode auto`) | 8 | no `PermissionRequest` event and no prompt on screen | | | |
| Waits for a person (answered after 3.0 s on purpose) | 7 | 3164 ms | 3187 ms | 3215 ms | 3215 ms |

So a prompt that settles itself is gone within about 180 ms, 120 ms under 300 ms, and the
agent never reports one in auto mode. The person row only shows the harness's chosen wait plus
about 165 ms; a real wait is unbounded. `NEEDS_SETTLES` stays 300 ms (the margin is about 1.7x
the slowest sample). Not measured: Codex (the turn budget was spent on Claude), a slow hook
(one that does network work holds the prompt as long as it runs), and an allowlist (the agent
asks nothing, so there is nothing to time). The prompt text itself was never seen on the PTY
for the auto-approved cases, and is drawn in pieces for the person case, so the hooks are the
only timing source here.

**Sounds.** A window rings at most once per `ANY_AGAIN_AFTER` (4 s), whichever kind it was: a
needs-you (two notes) followed by a finish inside that spell is held back, so a burst never
reads as three dings. The same agent saying the same thing again within `SAME_AGAIN_AFTER`
(120 s) stays quiet too. The card and toast still say what happened; only the sound waits. The
spell is kept per window: a daemon with no window open rings for itself, and two windows on one
daemon each keep their own. `maitty test-alert` plays both sounds.

Not yet verified, so Maitty does not claim exact Herdr parity:

- every identity against real process entry points, including npm and native
  wrappers, Windows paths, aliases and helpers that must not match;
- every bundled manifest against current working, idle, blocked, narrow and
  stale screens;
- each native adapter's ordering, reload, retries, nested helpers, cancellation,
  process exit and transport failure;
- OpenCode session switching with two clients attached to one server;
- notifications when focused, in the background, unfocused and detached,
  including stale and duplicate completions;
- live runs against every paid provider and CLI version.

Needs-you uses the theme's blocked colour for its mark and name (amber in the
default Night theme); working keeps its working indicator. Terminal bells never
replace status with a musical note. Codex's collapsed question notice and
expanded question dialog both take priority over a working spinner.
An advancing Codex footer starts automatic follow-up work after a completion;
an unchanged old footer cannot reopen the finished turn. Recognized agent process
exit releases hook authority, so an exited agent cannot leave a permanent spinner.
An explicit idle prompt recognized by a bundled screen detector also reconciles a
missed turn-end event for any supported agent, hooked or screen-detected. It clears
active child rows as unconfirmed while retaining their lifecycle records; it does
not infer completion from silence alone. For a hooked agent the screen must also have
been still for 5 seconds: Claude's input box is on screen all turn, and its title reads
idle while its own Stop hooks still run, so an animating screen waits for the hook. Hosts without an explicit idle screen
signal rely on their lifecycle completion events. The idle prompt reconciles nothing while a
turn end is held on a helper the agent reported that was heard from within the hour: an agent
whose turn ended with a background helper running shows that same prompt. A turn end counts as
held when it was reported with a helper listed, or when a helper starts after it. With no turn
end reported (a turn cancelled while its helper ran), the idle prompt still ends the turn.

Every status change goes to `daemon.log` in the data folder at info level, with the pane,
agent, old and new status and the reason (`hook: Stop`, `screen: claude rule …`), so the
cause of an alert is one search away: `pane 4 (claude): Working -> Done: hook: Stop`.

A working agent with no output and no hook report for 10 minutes is marked quiet
(`quiet_since` on the wire): it may be stuck. Quiet never changes its status, which
stays working. An agent waiting on helpers it reported, or asking to start helpers,
is never quiet. Any output or hook report clears it.

A working agent that is busy but going nowhere is marked repeating (`repeats` on the wire, the
number of calls the repeat has reached): its last tool calls are one call five times in a row, or
two calls taking turns for eight calls. The daemon sees each call only as the fingerprint a
tool-start hook reports (`call`, see [API_PATTERNS](API_PATTERNS.md)), so only Claude, Qwen, Codex
and Gemini can be marked, and a call made inside a subagent does not count. A different call, a new
prompt, the agent leaving work or exiting clears it, and a report with no call leaves the run alone.
Like quiet it never changes the status, rings, or enters the Inbox, and it is not saved with the
session. A change is one `daemon.log` line (`pane 4 (claude): repeating 6x`, or `cleared`) and
never says what the calls were.

A finished turn stays **done** until a window reports it looked: the terminal app sends
`MarkSeen` when its window is focused and the pane is in front, and the desktop app when you
view the session. The daemon never reads "seen" from which pane is active, since a window in
the background is not looking. The daemon tells one window about an alert (the one a person
used last), and that window rings or notifies; with no window attached the daemon rings.

Questions ring after remaining blocked for 300 ms, except in the pane you are
looking at (focused pane, focused window); the needs-you mark appears immediately.
Completions are quiet and remain visible as status/notice changes. A queued
needs-you sound rechecks the exact current status and cancels when work resumes, the
pane disappears or attention changes. A pane that rang needs-you stays quiet for the
same news for 2 minutes, even if it works and asks again; a new working turn clears
only a finished pane's deduplication. A finish read off the screen after under 5
seconds of work is a title blink: the status shows, nobody is told.
A Claude turn that stops with background work it started still running (a shell, a
helper, a monitor: the running entries of its Stop hook's `background_tasks` that were not
running at the previous stop) shows done but is not announced: no sound, desktop or phone
alert, and no check yet. Claude wakes itself when that work ends; the next stop with no new
work in flight announces the finish and runs the checks, even after a wake-up with no tool
use. Entries Claude marks finished don't count. A task still running from an earlier turn
(a dev server) holds nothing: later turns ring as usual, while the turn that starts a
never-ending task stays quiet (decided 2026-10-24). With two tasks, a stop listing `b1` and
`b2` pauses; once `b1` ends, the wake-up that stops with `b2` still running rings, since
`b2` is no longer new.
The desktop notification, phone alert and in-app note name the repository and branch
(`maitty · fix/early-alerts`), so two worktrees of one repository tell apart; a detached
checkout shows its short commit, and a session outside a repository its folder.
Maitty retains its four-second global sound spacing and two-minute same-news
deduplication; its delivery settings and multiple-client acknowledgements differ
from Herdr.

Codex helpers belong to a specific thread, not a project folder. Native
`SubagentStart` events identify the parent thread and helper before the parent
turn ends. Older Codex setups without those hooks rely on local session files
and the turn-end notifier to identify the thread; Maitty omits helpers rather
than borrowing another session's. If two panes report the same thread ID, disk
fallback is ambiguous and is omitted.

A parent stays working for as long as any helper is listed. Four things must not end
that early: a progress bar that goes off while helpers run (the agent is waiting on them,
not cancelled); the agent's idle prompt, which it shows once its own turn has ended and a
background helper carries on; a session start in the middle of a turn (compaction); and a
stop for a helper maitty never saw start, which removes nobody else. A stop with no id
removes only the last helper started. A helper that says nothing for an hour is taken to
have died: it stops holding its parent, which finishes and says so once, and it is kept as
unconfirmed. An interrupt or a new session ends any hold. A Codex helper whose stop was lost
stops holding as soon as its own session file shows it completed or aborted, even in the
same second it started. When a
Claude stop listed the helpers as running, the last one stopping shows done unannounced, and
the stop of the wake-up turn that follows is the finish that is announced.

When a trusted integration reports active helpers, Maitty keeps the parent
session working until they finish or go quiet. Codex uses native lifecycle hooks
when installed, with thread-specific local session files as fallback; Claude,
Qwen, Kimi and Mastracode report helper lifecycle events through native hooks;
OpenCode's selected-session bridge reports active child sessions. Provider/model
choice does not change the host integration. Other agents still get their own
working/needs-you/done status, but Maitty cannot list hidden helpers unless the
agent exposes their lifecycle.

Lifecycle reports are kept as child activity records after completion and
restored with the saved pane. A working session's row shows how many helpers are
running (its subagent rows in the sidebar; Settings › Agents › **Subagents in the sidebar**). A parent
ending or a child going quiet without an end event counts as unconfirmed, never
completed.

Codex rollout files currently report child identity, parent thread and lifecycle
events, but not the assigned prompt or child model/effort. Native hook
payloads may report task/model/effort fields; Maitty uses them only when supplied.
It does not expose hidden reasoning or claim transcript access that a runtime does
not provide.

### Each helper's own record

What a helper leaves behind, for showing its work ([ADR 0010](adr/0010-workflows-and-specialist-agents-stay-outside-maitty.md)).
Measured 2026-11-13 on Linux with Claude Code 2.1.323 and Codex 0.166.

- **Claude.** `SubagentStart` carries `agent_id`, `agent_type`, `session_id`, `prompt_id` and the
  parent's `transcript_path`, but no task. Hooks fired inside the helper carry the same `agent_id` and
  the parent's `transcript_path`. `SubagentStop` adds `agent_transcript_path` and
  `last_assistant_message` (the helper's answer). The helper's record is
  `<parent transcript without .jsonl>/subagents/agent-<agent_id>.jsonl`, in the parent's line format
  (`isSidechain: true`, `agentId`), with `agent-<agent_id>.meta.json` beside it holding `description`
  (the parent's Agent call), `agentType`, `toolUseId` and `requestShape` (`background` for one started
  in the background). A background helper is also listed in the parent's `Stop` as
  `background_tasks[]` (`id` = `agent_id`, `description`, `status`).
- **Codex.** A helper is its own rollout file whose `session_meta` has `thread_source: "subagent"`,
  `parent_thread_id`, `agent_path` (`/root/<task name>`, the name the parent gave `spawn_agent`) and
  a generated `agent_nickname`. It starts with a copy of the parent's history, then its own turn: its
  tool calls, their output and its final message are plain text, but the task it was given is
  encrypted, as is the parent's `spawn_agent` message. `codex exec --json` shows only the parent's
  `wait` calls as collab items, with no receivers, so it is no source for helpers.
- **Codex hook trust.** In a new pane Codex asked to review Maitty's nine hooks ("new or changed")
  and, until they are trusted, sent none. Without `SessionStart` Maitty learns the thread only from
  `notify` at the turn's end, so the rollout fallback cannot list helpers while the turn runs.
- **Linking a workflow's step to its helper.** Both providers carry a name the parent chose: Claude's
  Agent `description` and Codex's task name. A workflow names its helper with that label and gives
  the same label in its report; the daemon links a helper of the same pane whose label is equal.

Installing a new executable does not replace an already-running window. **Settings › General › UPDATES** shows the version, and Settings warns at the top when the installed, window and daemon versions differ. Local open sessions keep running; once no panes remain, Maitty restarts the daemon and reconnects the window automatically. Closing a window alone leaves the daemon running. A remote daemon must be updated on its host.
