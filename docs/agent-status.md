# Connect agents (status setup)

Seshi works out whether an agent is working, needs you or is done by watching its process and screen. **Connect agents** installs a small adapter into Codex, Pi, OpenCode, OMP, Mastracode, Kilo or Kimi so the agent reports that itself. Claude needs nothing here. It's a one-time step per machine.

Choose **Menu → All commands → Connect agents**, then click **Codex**, **Pi**, **OpenCode**, **OMP**,
**Mastracode**, **Kilo** or **Kimi**.
Setup installs
only Seshi's adapter; it preserves other extensions and refuses to overwrite a
different file named `seshi.js`. It does not interrupt an agent or restart the daemon.

Set up the integration on each machine where the agent runs; it lives in that
machine's agent config, not Seshi's synced config. `seshi doctor` checks the
machine-local adapters and says when an installed setup needs a provider reload.
The check confirms configuration presence, not live reporting: native status is
confirmed only after the running provider sends Seshi an event. Until then,
process and screen detection remain the fallback. Restart Codex and OpenCode
after installation. In Pi, use `/reload` or restart it.
The CLI equivalents, including removal, are:

```sh
seshi integrate pi
seshi integrate codex
seshi integrate opencode
seshi integrate omp
seshi integrate mastracode
seshi integrate kilo
seshi integrate kimi
seshi integrate pi --uninstall
seshi integrate codex --uninstall
seshi integrate opencode --uninstall
```

Codex installs `SessionStart`, `SubagentStart` and `SubagentStop` hooks in
`$CODEX_HOME/config.toml` (or `~/.codex/config.toml`). SessionStart records the
root thread without changing pane status, allowing thread-specific rollout-file
detection to work when Codex does not report child lifecycle events. Native
child events provide immediate helper identity and lifecycle when available.
Windows also gets Codex's `commandWindows` field with PowerShell's call
operator; Linux and macOS retain the normal shell command.
Seshi preserves existing Codex hooks and another program's `notify` command;
turn completion still uses `notify` when that key is available.

Pi loads `~/.pi/agent/extensions/seshi.js` (or
`$PI_CODING_AGENT_DIR/extensions/seshi.js`). Use a current Pi with
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

Both adapters remain inert outside Seshi. Reports carry only lifecycle/status
and session identity/working directory, with the pane's inherited authentication. A reporting
failure surfaces through Pi's warning UI or OpenCode's log and later events retry.

OMP uses `~/.omp/agent/extensions/seshi.js` (`PI_CONFIG_DIR` and
`PI_CODING_AGENT_DIR` follow OMP's own configuration). Pi and OMP must use
separate directories. Nested `OMPCODE=1` agents cannot own the parent pane.
Approvals/ask tools remain blocked until resolved; automatic continuation stays
working and retryable failures have a 2.5-second retry grace before needs-you.
Kilo uses `~/.config/kilo/plugin/seshi.js` (honoring `XDG_CONFIG_HOME`).
Mastracode uses flat command arrays in `~/.mastracode/hooks.json`.
Kimi uses native `[[hooks]]` tables in `~/.kimi-code/config.toml`
(`KIMI_CODE_HOME` overrides it). Setup preserves custom hooks; `--uninstall`
removes only Seshi's entries.

Seshi recognizes all 24 identities in the pinned Herdr registry plus Aider, Goose,
Crush, Auggie and the separate DeepSeek harness. The 22 bundled Herdr manifests
use prioritized live-screen/OSC evidence; OMP and Mastracode rely on their native
integrations. A known executable alone does not guarantee native lifecycle support.
The pin is Herdr commit `143bf5fd4781d164260c63fe28e7dc08054ad32f`; its license and
notices are in `vendor/herdr-detection/`.

Not yet verified, so Seshi does not claim exact Herdr parity:

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

Needs-you uses the configured blocked dot and blocked text color (red in the
standard themes); working keeps its working indicator. Terminal bells never
replace status with a musical note. Codex's collapsed question notice and
expanded question dialog both take priority over a working spinner.
An advancing Codex footer starts automatic follow-up work after a completion;
an unchanged old footer cannot reopen the finished turn. Recognized agent process
exit releases hook authority, so an exited agent cannot leave a permanent spinner.
An explicit idle prompt recognized by a bundled screen detector also reconciles a
missed turn-end event for any supported agent, hooked or screen-detected. It clears
active child rows as unconfirmed while retaining their lifecycle records; it does
not infer completion from silence alone. Hosts without an explicit idle screen
signal rely on their lifecycle completion events.

Questions ring even in the focused pane after remaining blocked for 300 ms;
the red indicator appears immediately. Completions are quiet and remain visible
as status/notice changes. A queued needs-you sound rechecks the exact current
status and cancels when work resumes, the pane disappears or attention changes.
A new working turn clears the previous turn's notification deduplication.
Seshi retains its four-second global sound spacing and two-minute same-news
deduplication; its delivery settings and multiple-client acknowledgements differ
from Herdr.

Codex helpers belong to a specific thread, not a project folder. Native
`SubagentStart` events identify the parent thread and helper before the parent
turn ends. Older Codex setups without those hooks rely on local session files
and the turn-end notifier to identify the thread; Seshi omits helpers rather
than borrowing another session's. If two panes report the same thread ID, disk
fallback is ambiguous and is omitted.

A parent stays working for as long as any helper is listed. Three things must not end
that early: a progress bar that goes off while helpers run (the agent is waiting on them,
not cancelled); a session start in the middle of a turn (compaction); and a stop for a
helper seshi never saw start, which removes nobody else. A stop with no id removes only
the last helper started.

When a trusted integration reports active helpers, Seshi keeps the parent
session working until they finish or go quiet. Codex uses native lifecycle hooks
when installed, with thread-specific local session files as fallback; Claude,
Qwen, Kimi and Mastracode report helper lifecycle events through native hooks;
OpenCode's selected-session bridge reports active child sessions. Provider/model
choice does not change the host integration. Other agents still get their own
working/needs-you/done status, but Seshi cannot list hidden helpers unless the
agent exposes their lifecycle.

Lifecycle reports are retained as child activity records after completion and
restored with the saved pane (wire protocol v41). The sidebar lists active
children only and removes completed, failed, cancelled or unconfirmed children
from the live view. Click an active row to inspect it, or focus the parent in the
sidebar and press `s`, then use Up/Down to browse retained lifecycle records.
The inspector shows the reported identity, task, parent, model, effort, state,
start/elapsed time and latest activity. Missing metadata says **Not reported**.
A parent ending or a child going quiet without an end event is shown as
unconfirmed, never completed.

Codex rollout files currently report child identity, parent thread and lifecycle
events, but not the assigned prompt, child model/effort or result. Native hook
payloads may report task/model/effort fields; Seshi uses them only when supplied.
The sidebar shows the reported task when available. Otherwise it shows a
reported current tool action when available, then a recognizable role stage, a
meaningful reported role name, or `Working` when the runtime provides no useful
label. The inspector marks missing metadata **Not reported**. It does not expose
hidden reasoning or claim transcript access that a runtime does not provide.

Installing a new executable does not replace an already-running window. **Settings → General** shows installed/window/daemon versions and a restart-pending notice. Local open sessions keep running; once no panes remain, Seshi restarts the daemon and reconnects the window automatically. Closing a window alone leaves the daemon running. A remote daemon must be updated on its host.
