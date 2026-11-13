# API and service patterns

This doc covers how requests flow through the system, how boundaries validate
data, and what responses and errors look like. It applies to any entry point:
HTTP, RPC, GraphQL, IPC, CLI, or queue consumer. `AGENTS.md` is the operating
contract; this doc holds the API and service practices in depth.

## Request flow

```text
entry point (handler / route / command / consumer)
  -> validate + parse input into typed command
    -> use case / service (authorize, apply rules, coordinate)
      -> repositories and adapters (ports)
  <- typed result or typed error
<- map to response shape / status
```

- A **handler** parses input, calls one service, and maps the result. It holds
  no business rules and no queries.
- A **service** owns one use case. It authorizes, applies the rules, and owns
  the transaction boundary.
- A **repository or adapter** owns I/O. It returns domain types, never raw rows
  or SDK objects.

## Input validation

- Validate every request at the entry point with a schema: body, params, query,
  headers, and message payloads. Reject the request before it reaches a service.
- Parse into types. After validation, the service receives a typed object, not
  a raw payload.
- Keep schemas next to the feature that owns them. If a client shares them, put
  them in a shared package, never duplicated.
- Treat IDs from clients as untrusted. Check that the resource exists **and**
  that the caller may access it.

## Responses

- Use one consistent success shape and one consistent error shape across the
  whole API. The project records both in *Project-specific: response and error
  shapes*.
- Return DTOs mapped from domain types. Never serialize database rows or
  internal objects directly.
- Use one field-naming convention on the wire, for example camelCase JSON.
- Use timestamps in UTC ISO 8601 and IDs as strings.

## Errors

- Services return or throw **typed domain errors**, such as `NotFound`,
  `Validation`, `Conflict`, `Unauthorized`, `Forbidden`, and `RateLimited`. The
  entry point maps each one to a status or exit code in exactly one place.
- Error responses include a stable machine-readable code, a safe human-readable
  message, and a request id. For validation errors, add field-level detail.
- Unexpected errors are logged with full context and return a generic message.
  Never expose stack traces, SQL, or internal paths.
- A client can act on the error code. Codes are part of the contract; don't
  rename them casually.

## Operations

- **Idempotency:** retryable mutations, such as payments, emails, and
  webhooks, accept an idempotency key or are naturally idempotent.
- **Pagination:** list endpoints are paginated from day one, with a documented
  default and maximum page size. Prefer cursor pagination for data that changes
  often.
- **Concurrency:** updates that can race carry a version or ETag, and stale
  writes are rejected with `Conflict` instead of overwriting.
- **Versioning:** removing or renaming a field, or changing its meaning, is a
  breaking change. Version the API or run old and new side by side, then retire
  the old one on purpose.
- **Timeouts and retries:** every outbound call has a timeout. Retries use
  backoff, and only on idempotent operations.

## Auth

- Authenticate at the edge, in middleware or the handler. Authorize in the
  service, against the resource the call acts on.
- Deny by default. A new endpoint without an explicit auth rule is a defect.
- Never trust a client-supplied role, user id, or tenant id without checking it
  against the authenticated identity.

## Client-side data (when there is a UI)

- Keep **server state**, the data owned by the backend, apart from **UI state**
  such as open panels, form drafts, and selection. Don't mirror server data into
  a global UI store.
- Represent every async view's loading, empty, success, and error states
  explicitly.
- After a mutation, invalidate or update the affected cached data at one place
  per feature.

## Observability

- Assign a request or correlation id at the edge and pass it through logs and
  outbound calls.
- Use structured logs: one event per line, with fields rather than
  interpolated strings.
- Log the decision points and failures. Never log secrets, tokens, or personal
  data.

## Project-specific: API style

Not HTTP. A private IPC protocol between client and daemon over a local socket
(or `ssh host maitty proxy`): length-delimited frames carrying msgpack-encoded
`ClientMsg` / `ServerMsg` enums (`crates/maitty-shared/src/protocol.rs`). The first exchange is
`Hello { version }` → `Welcome { version, compatible_since }`. The client decides whether
the two talk (`protocol::speaks_with`): the same `PROTOCOL_VERSION` always does, and another
one does inside the compatibility window, which is from the newer side's `COMPATIBLE_SINCE`
up to its own version. Outside it the connection is refused. Inside it the newer side adapts:
the daemon does not send a client a message newer than the client (`ServerMsg::introduced_in`)
and passes over a request it cannot decode from a newer client; a client checks
`ClientMsg::introduced_in` before sending an older daemon a request that needs an answer.
One client is stricter: the desktop window's line to another machine (`Dial::newest_only`)
refuses a daemon on an older version even inside the window, so the machine shows
`needs-update` and the window offers its update.
Bump `PROTOCOL_VERSION` when a message shape changes incompatibly: a field removed, renamed
or retyped, or a new enum variant (an old decoder rejects it). An added field with
`#[serde(default)]` is compatible (messages are named msgpack maps and unknown fields
are ignored) and needs no bump, provided a test decodes the old shape. Hooks say Hello
too, but a hook or usage report is also accepted by an older daemon back to
`REPORTS_READ_SINCE` (`ipc::open_for_report`), so an update does not stop hooked panes
reporting before the daemon restarts. When `Hook`, `Usage` or `HookStatus` changes in a way an
older daemon cannot read, set `REPORTS_READ_SINCE` to the new version in the same change; the
test `todays_reports_are_read_by_the_oldest_daemon_they_are_sent_to` holds the shapes. The CLI subcommands and the MCP server (`maitty mcp`, JSON-RPC over stdio)
are thin wrappers that send the same commands.

A conversation that changed is announced to the desktop app's attached clients as
`ServerMsg::ConversationChanged { id, generation }` (`daemon/conversation/news.rs`): after each
batch of events the daemon compares a stamp of what a `ConversationRead` would answer (history
revision, status, error, approvals, queue) with the one it last announced, at most once per
100 ms per conversation, and never for a conversation seen for the first time. It carries no
content: the window reads as before, under the same checks. The host passes it to the webview as
the `conversation-changed` event, for this computer only; another machine's is ignored
(`remote_policy`). The terminal app is sent none.

`Command::SendWhenReady { term, text }` hands the daemon a message for the agent starting in a
pane. The daemon holds one per pane and types it in once the agent is running and at rest (idle or
done) with its hooks reporting, or 20 s after an agent without hooks was first seen; never while
the pane works or asks something. It is typed as a queued follow-up is (one paste when it has
several lines), and is saved with the pane, so a daemon restart keeps it. The desktop command is
`send_when_agent_ready` (at most 256 KiB, not blank).

`Command::Prewarm { cwd, cmd }` starts an agent ahead of its first message (`daemon/warm.rs`). It runs hidden, as a spare
(no client is shown it, it alerts nobody and it is not saved), and the next `NewWorkspace` with the same command in the
same folder is that agent, already running, instead of a new one. One is kept at a time: asking for another stops the
first, asking again for the same one keeps it, `Command::DropPrewarm` stops it, and the daemon stops one nobody asked
for in ten minutes, so a window that closed or crashed leaves nothing running. Only an app window may ask, never a pane
or a structured runtime. Both commands came in protocol 79 (`PREWARM_SINCE`, each with its arm in
`ClientMsg::introduced_in`); `COMPATIBLE_SINCE` stays 78, since an older daemon is simply not asked. The desktop
commands are `prewarm_agent_session` (the agent names and flag rules of `start_agent_session`; it answers false when the
daemon running is older) and `drop_prewarmed_agent`.

`Command::BranchWorktree { project, branch }` makes a worktree for a window with no pane to ask from (the desktop
app's new chat page, where "New worktree" used to run a plain `git worktree add` beside the project, skipping the
trust copy, the repo's setup hook and the tidy-up list). The daemon checks `branch` as `NewWorktree`'s branch agent
does, requires `project` to be one the app shows and a git repository, then makes the checkout with
`create_trusted_worktree` (the `worktree.dir` template, an existing local or remote branch else a new one from HEAD,
trusted like its repo), runs the repo's `on_create` setup, lists the folder in `made_worktrees` and replies
`Reply::Text(folder)`. The window starts its agent in that folder. It came in protocol 81 (`BRANCH_WORKTREE_SINCE`, with
its arm in `ClientMsg::introduced_in`); `COMPATIBLE_SINCE` stays 78 because an older peer is never sent it. The desktop
command is `add_branch_worktree` (`maitty_desktop_client::branch_worktree`); it waits up to eleven minutes, since the
setup hook may take ten, and says the service is from an older release instead of asking a daemon that would never
answer. It is this computer's daemon only. Structured runtimes may not ask for it.

A pane's effort (`TermInfo.effort`, an optional field: no `PROTOCOL_VERSION` bump, a test decodes the old shape) is read by the
daemon from the agent's own files every five seconds off the loop (`usage::poll_effort`): for Claude the level in the newest request
line of its transcript, present only when a level was set; for Codex the level its thread settings name, else its model's default
from Codex's `models_cache.json`. Nothing is guessed: a pane whose files say none has none, and the window then says "agent's own".

A pane's follow-up queue (`TermInfo.queued`) is the daemon's: it types the oldest into the agent when a turn ends, never
into a question, and keeps the rest across a restart. `Command::QueueFollowUp { term, text, first }` adds one (`first`,
a defaulted field, puts it ahead of the ones waiting, for an agent's own command picked while it works),
`Command::ClearQueue` drops them all, and `Command::DropQueued { term, index, text }` drops one: the one at `index`, and
only while it still reads `text`, since the queue moves when a turn ends. `DropQueued` came in protocol 80
(`DROP_QUEUED_SINCE`, with its arm in `ClientMsg::introduced_in`); `COMPATIBLE_SINCE` stays 78. The desktop commands are
`queue_follow_up` and `drop_queued_follow_up`; the second says the service is from an older release instead of asking a
daemon that would never answer. A daemon on another machine that is older is asked anyway and does not answer.

A state change reaches attached clients as `ServerMsg::State` (the whole `Snapshot`) or, when only
some panes differ from what that surface's clients last received, as `ServerMsg::TermsChanged`
(the panes to replace and the ids to drop); the daemon compares encoded bytes and sends nothing
when nothing visible changed, and a client that has just attached gets the next change in full. A
client merges a delta into the snapshot it holds and treats the result as a state update. For a pane delta from another machine the snapshot is that machine's own,
and the delta is cleaned first (SECURITY, "remote daemon"). Replays follow `Hello.replays`: a terminal client takes every pane's replay on attach, the desktop window
attaches without and sends `ClientMsg::WantReplay { term }` on its stream when it shows a pane; a
desktop replay carries the last mebibyte of the ring and then the screen as it is, which is all the
window's scrollback holds. Any replay of a pane on the alternate screen repaints the main screen,
switches, then repaints the alternate one, so the view is on the right screen even when the
program's own switch is older than the ring.

Over ssh, `maitty proxy` writes `STREAM_MARKER` (`maitty-ipc-transport`) to stdout once, after it
reaches the daemon and before it relays a byte. The marker sits outside the frames, so it is
not a protocol change. The client sends its Hello, then discards stdout up to the marker (a
remote login that prints text, such as `.bashrc`, would otherwise corrupt the frames) and
reads frames from the first byte after it. At most 64 KiB is discarded and the wait is 30 s,
longer than ssh's `ConnectTimeout` so a slow login is not cut off. If text arrived and the
marker did not, the connection fails with "the remote login printed text before maitty
started (or it runs an older maitty: update it)". If nothing arrived, the error is what ssh
said. A remote from before the marker is not refused: when the first four bytes of its
output are a plausible frame length (up to `MAX_FRAME_BYTES`), the client reads them as the
first frame and the usual version check applies. Text never passes as a length. An old remote
whose login also prints text can't be told from noise, and gets the error above, hint included.

The optional Tauri GUI invokes only fixed desktop commands from
`crates/maitty-desktop-client/`; validate webview input in both the TypeScript boundary and
the Rust host. Never expose a generic process or shell command. Between the Tauri host and the
webview (`apps/gui/src-tauri/src/terminal_stream.rs`, `streams.rs`, `apps/gui/src/api/`): pane output
arrives, from every machine, as raw frames over the one channel the webview opens once with
`watch_terminal_output` (the machine as 2 big-endian bytes, 0 for this computer, the pane as 4, the
sequence number as 8, one replay byte, then the output; this header is the host-to-webview format
only and the daemon protocol is unchanged), one frame per pane per flush (the host gathers reads for
up to 12 ms or, for this computer, 64 KiB; a replay flushes what came before it; another machine gets
at most 256 KiB into a flush, and if 4 MiB of its output waits, that pane's waiting output is dropped
and its daemon is asked for a fresh replay, so one machine cannot slow this computer's panes);
history, sequence numbers and the last snapshot are kept per machine, and `terminal_history` and
`terminal_input` take an optional `machine` (this computer when left out), as do the pane commands
(resize, rename, pin, archive, seen, close): a pane of another machine is asked over that machine's
line (`PaneCommand` in `maitty-desktop-client` makes each message for both), a reply or error from it
is taken only while a request of the host's is out to it (30 s, 64 kept, as in the terminal app), and
typing or a command for a machine with no line is refused naming it, the typing counted and reported
as `unsent` on the machine's next `machine-state` `connected`;
the view uses the sequence number to catch up from `terminal_history`; `terminal_history` and the
replay commands answer with raw bytes
(`tauri::ipc::Response`), the history prefixed with its sequence number as 8 big-endian bytes and
empty once the host has asked the daemon for the pane's replay over the stream (it arrives as that
pane's next frame) and an error when there is no stream, so the view fetches a replay itself; the
daemon's `Snapshot` goes out as a `sessions-changed` event
whenever it changes, with `null` when the stream dropped, and the webview sets its session query
from it instead of polling (a 15 s poll remains as a safety net); another machine's goes out as
`machine-sessions-changed` (`{machine, snapshot}`), after the host has cleaned it, and its connection
state as `machine-state` (`connecting`, `connected`, `reconnecting`, `needs-login`, `needs-update`, `unreachable`, or `refused` when a different daemon answers at its name), with `since` (epoch ms) once it is not connected and the machine's `clipboard`, `notices` and `sounds` opt-ins from its entry (all off unless it says); a notice from a machine that opted in goes to the window as `machine-notice` (`{machine, text}`, cleaned, cut and led by `[name]`) and the window shows it as a toast or, away from it, a system notification; `retry_machine(name)` dials a machine now and clears a refusal; `remove_machine(name)` deletes its entry from `config.local.toml` first and only then stops it, forgets its daemon id and tells the window with `machine-removed` (its sessions and saved layout are left alone); the commands that read a pane's files or repository (`repo_query`, `restore_snapshot`, `new_worktree_session`, `pane_terminal_screen`) take the pane's `machine` and refuse another machine's with the terminal app's words (`<Feature> is local-only in v1: the files are on <machine>, not on this computer.`), as the window does before it asks (`apps/gui/src/lib/localOnly.ts`); `set_machine_grant(name, grant, on)` allows or stops a machine's `clipboard` or `notices` (written to its entry in `config.local.toml` first, then applied at once; the new state follows as `machine-state`); text an allowed machine asks to copy arrives as `machine-clipboard` (`{machine, text}`); `start_session_on_machine(machine, folder)` starts a terminal session in a typed folder of another machine (one that is not there opens in its home folder); `list_ssh_hosts` lists the `~/.ssh/config` hosts that are not machines yet; `add_machine(target, name?)` checks the target, looks at the machine over ssh without asking anyone anything (12 s at most; `cancel_add_machine` ends it), writes the entry only when that held up, and starts dialling it (a login that needs a person opens ssh in a session on this computer, and the window dials again when it closes); `list_machines_overview` gives a window that starts late each machine's last state and
snapshot. The window names a machine's terminal sessions `build/t12` (the machine's name, so a saved
layout survives a restart; this computer's stay `t12`) and passes the number the host gave the
machine (`machine`) with each command, only for a pane of another machine. Files, Changes, Find,
worktrees, file drops and system notices are for this computer's panes only. A desktop
protocol-mismatch restart must recheck the exact local protocol before stopping
an older daemon and must refuse to downgrade a newer one. It stops an older daemon that
owns sessions only after the user confirms, and only when those sessions are saved on disk
(the next daemon restores them); one that isn't saving them is left running.

## Project-specific: response and error shapes

Commands are fire-and-forget; the daemon's answer is the next `Snapshot` (full
state) and pane output frames. Failures come back as `ServerMsg::Error(String)`,
a human-readable message the client shows as a toast; CLI subcommands print it
and exit non-zero.

## Project-specific: auth model

Socket access is the auth (see `docs/SECURITY.md`); pane status reports also need
the pane's process tree or `MAITTY_PANE_TOKEN`.

## Project-specific: feature/module structure

Saved role profiles and sequenced planner/worker/reviewer workflows are not part
of Maitty's interface, and neither is a fleet commander (`maitty brief`, `[team]`): old
config keys for them are read and ignored. Sessions, ordinary agent launches, helpers, and
MCP session controls remain supported. The wire protocol version is `PROTOCOL_VERSION` in `crates/maitty-shared/src/protocol.rs`.
Conversation tool-history entries carry a bounded provider-supplied `label` for
display; `#[serde(default)]` keeps older cached entries readable without a
protocol bump. A tool entry may also carry `input` (the tool's own input JSON,
bounded: strings cut with a `[truncated]` marker, a value too deep or too big left
out whole), `exit_code` and `duration_ms` (Codex only), and every entry may carry
`started_at_ms` / `completed_at_ms` (Unix milliseconds). All are optional and skipped
when absent, so old shapes decode and no bump is needed. `ConversationRead` also
returns `answered_approvals`, the last 64 approvals a person answered (request,
decision, time), in daemon memory only. A Claude approval's `item_id` is the
tool-use id when Claude names it, so it links to the tool step; an edit tool is a
`FileChange` request with its diff in `changes`.

`SwapPanes { a, b }` trades the places of two panes of one tab (the split ratios stay with
their slots). Panes in different tabs, or one that isn't there, are an error.

`maitty_answer` takes the `prompt` id that `maitty_list` (`prompt` on a session that needs you) or
`maitty_wait` (`prompt <id>`) gave. It hashes the question and its choices, whitespace aside. If the
prompt on screen has a different id, nothing is sent and the error shows the current question,
choices and id.

**Helpers.** A start that comes from an agent's pane (`NewWorkspace`, `NewTab`, `Split`,
`NewWorktree` over a connection that proved it is that pane by its secret) is a helper
start. Under `[sessions] helpers = "ask"` the first one is refused with an error telling the
agent the user was asked, and the daemon records the task on the pane. `TermInfo.last_lines`
(`String`, `#[serde(default)]`, added without a protocol bump) is the last three lines of text on
the pane above its input line when its turn ended or it stopped to ask, for an agent whose hooks
reported no last message; it is empty while `TermInfo.said` has one, and cleared when a turn
starts (`daemon/last_lines.rs`). `TermInfo.helper_ask`
lists the recorded tasks (not empty: the pane needs you, and `TermInfo.status` reads Blocked
whatever its hooks report), and the request is announced like any needs-you. Only you answer
it, with `Command::AnswerHelpers { term, choice }`: `Allow` (this session may from now on),
`Always` (and every session in its project; kept in `helper-projects.json` in Maitty's data
folder), or `No` (its starts are refused for the rest of its run). A pane may not send it, and
it is an error unless the pane is asking. `always` and `never` skip the question; `never`
refuses with an error the agent can read. `sessions.helpers_at_once` (2, 3 or 4) bounds the
helpers one session has working at once; a start past it is refused with what to do. A
person's shell (no agent in the pane) is never asked. Sessions saved before a restart keep
`started_by`; any fleet or commander field in a saved file is ignored on load.

**Pull requests.** The daemon polls `gh` off the event loop for each pane's branch, or the URL
set by `Command::LinkPr { term, url }`; `None` returns to branch discovery. A pane may only link
its own PR. `TermInfo.pr` holds the latest checks, review, conflict, title and URL. When a new
reading has a failed check, review request or merge conflict, Maitty queues a short follow-up
for the pane's next idle turn when `[sessions] pr_wake` is enabled. It never types into a question.

**Secrets.** A pane asks through `Command::RequestSecret { term, label, reason }` on its
authenticated connection. The request appears in `Snapshot.secret_requests`; the value is never
part of a snapshot. Only the user can answer with `Command::AnswerSecret { id, value }`. The
waiting pane receives a single-use, 24-hour reference, then requests its value once with
`Command::SpendSecret { reference }`. The reference belongs to the pane that asked, and a daemon
restart forgets all held secrets. Disconnecting the asking pane withdraws its request.

A pane's check results travel as `CheckInfo` summaries on `TermInfo.checks`, one per
checkpoint in project order; `TermInfo::check()` is the headline one (the worst state wins:
Failed > CouldNotRun > NeedsApproval > Checking > Pending > Stale > Never > Overridden >
Passed, the first in order on ties), and `reported_check` applies to it. A summary holds the
command, state, exit code, start, duration, the turn-end it followed, the first failing line (set by the server when it fails: the first failure-looking line of the output, else its last line, cut to `FIRST_FAILING_LINE_MAX_CHARS`; empty otherwise), whether a re-run
is queued, the override time and reason, the checkpoint `name`, the folder it ran in, the turn
it checked, the full output's line count, `stale_since` and `changed_files`, who overrode it,
its `source` (`User`, or `Repo` with the file and commit) and, on `NeedsApproval`, the
`previous_command`. The output tail is never in the snapshot:
`Query::CheckOutput { term, name }` returns it as `Reply::Text`, under the read grant (`name`
None: the headline checkpoint). Each result's
tail is capped at `CHECK_OUTPUT_MAX_BYTES` (1 MiB), cut on a character boundary. The latest
result and the overridden one of each checkpoint are saved in `SavedPane` (`checks`,
`checks_overridden`; a pane saved with the single `check` / `check_overridden` loads them as
one checkpoint); their tails are saved beside the pane's output files as
`<term>-<name>-<slot>.txt`, and a tail under the old `<term>-<slot>.txt` name is read once at
restore and dropped by the next save.

Three commands act on a check, each needing `Grant::Write` on the target pane and each naming
a checkpoint (`name` None: all of them for a re-run, the headline for the others). `RerunCheck`
runs it again at once, or, while the agent is working, sets `rerun_queued` on the result on
show; the turn end then runs it (a pane never checked has nothing to queue, since its turn end
checks it). `OverrideCheck { reason }` works only on a `Failed` result: it records the time and
the trimmed reason (at most `OVERRIDE_REASON_MAX_CHARS`), sets `Overridden`, and keeps the
output. `UndoOverride` works only on `Overridden` and restores `Failed`. Any other state
answers `ServerMsg::Error`. The next run replaces the result on show; `check_overridden` keeps
the overridden one with its output. A pane may not override or undo its own check.

`ApproveCheck { term, name, choice, command_sha256 }` (`Once`, `Always`, `Never`) answers a
repo-supplied check command and `ForgetCheckApproval { project, name }` drops a remembered
answer. No pane may send either, whatever its grants. `command_sha256` (`protocol::command_hash`)
is of the command the client showed; an answer for a checkpoint that isn't waiting, or whose
waiting command has another hash, is a `ServerMsg::Error`. The answered checkpoint then runs
again; while another checkpoint of the pane runs, it is queued (`rerun_queued`) and runs when
that run settles, in the same turn.

Subagent lifecycle is carried as structured `SubagentActivity` records in each
pane snapshot and persisted with its saved pane. Hook reports use the owning
pane's authenticated process/token boundary. Codex disk records are accepted
only for the exact thread ID reported by that pane; discard background polling
results if the pane changes thread or agent. Merge by stable child ID and event
time so duplicate or stale reports cannot replace newer lifecycle state.

A new capability adds: a `Command` variant (and any new `ServerMsg` /
snapshot field) in `protocol.rs`, bumping `PROTOCOL_VERSION` when the change is
incompatible and deciding `COMPATIBLE_SINCE` with it (see *API style*); handling in
`daemon/mod.rs` (or a `daemon/<feature>.rs`); the client side as an `Action` in
`keys.rs`, a `Mode` and `draw_*` / `on_*_key` pair in `client/`, and a CLI
subcommand in `cli.rs` when scripts should reach it.

## Project-specific: agent turns and background shells

A `Hook` report's `call` is a fingerprint of the tool call that is starting: 16 hex characters, the
first 8 bytes of SHA-256 over the tool's name and its input. Only a tool-start event from the
session's own agent carries one (`PreToolUse` for Claude, Qwen and Codex, `BeforeTool` for Gemini),
and only when the payload has both the name and the input. Nothing else does, and the name and the
input are never sent.

Honor a trusted turn-finished hook even when its transcript lists a background
shell or dev server. Background commands do not turn `Done` back into `Working`.
A Claude Stop that lists running background work new since its last stop stays `Done`
but is not announced ([agent-status](agent-status.md)).
Continue tracking actual subagents through their own lifecycle reports. Prepare
the hook message separately from socket I/O so its effective status is testable.

Codex's queued or expanded unanswered question takes precedence over its working spinner,
including with turn-end-only hooks. Clearing it restores ongoing work detection.

Pi/OpenCode native adapters retain blocked status until their reply/dismissal
events; quiet-screen heuristics must not clear native dialogs. Pi finishes on
`agent_settled`, after automatic continuation. OpenCode preserves pending asks
across busy events and resolves session ancestry so a helper finishing cannot
finish its parent. Status payloads include event evidence, not conversation text;
explicit status overrides retain that evidence. Existing pane authentication applies.

Full lifecycle authority applies to Pi, OMP, Mastracode, OpenCode, Kilo and Kimi.
For other hook sources, a fresh agent-owned blocker can override stale hook state.
The bundled Herdr manifest rules preserve status in transcript/history viewers.
OpenCode interactive TUI V1/V2 bridges report only the selected session/subtree;
shared serve/web/attach processes must not claim another pane's status.
`--status same` records session metadata without inventing a status transition.

Bind Codex disk usage/helpers to the exact thread ID reported by the pane's
authenticated integration. Never infer helper ownership from a shared working
directory. Unknown or multiply-owned thread IDs show no disk helpers. Discard
background results after a pane changes thread or agent; clear helpers when a
new thread is accepted. This uses internal daemon events, not a wire change.

For Codex's turn-end-only hooks, compare subsequent pane input to the accepted
turn end using monotonic instants. Keep that boundary when done becomes seen/idle;
do not impose a startup deadline or let a stale activity footer reopen the turn.
Screen patterns read logical lines across soft wraps, retaining hard newlines.
Codex activity matching accepts trailing background-terminal metadata.
An advancing activity footer after an accepted end also starts automatic follow-up
work without keyboard input; an unchanged completed footer stays finished.

Shell `agent-dir` classification is agent-specific: Codex `-c`/`-p` configure
new sessions; Claude's continue/print flags remain exclusions. Consume option
values before checking Codex subcommands. Resume/fork and explicit Codex
directory/worktree options retain the requested location. Worktree allocation
still requires the authenticated pane shell and the existing per-agent setting.

Snapshots include the daemon package version. Term evidence includes the latest
process observation time; screen evidence refreshes on repeated observations
without changing the status start time or repeating transition alerts.

Publish attention status immediately, but debounce its sound: blocked waits
300 ms and done waits one second. Queue the expected status with its deadline
in both client and daemon. Cancel on a different snapshot/status or pane removal;
recheck the exact status before ringing. Transient auto-approved asks and stale
attention messages must not ring while the agent is working. Background top-level
completion uses the soft completion sound; helper completion stays quiet. Helper
questions may ring after the same blocked delay.

A session whose project has a check is not finished until the check settles. While it
runs, no finished alert (desktop or phone) rings and `maitty wait`, `maitty send --wait` and
MCP `maitty_wait` / `maitty_send` with `wait` stay blocked, bounded by their timeout. A
passed check lets the normal finished alert ring. A failed check raises a blocked
attention alert and a notice saying "check failed", rings no finished alert (desktop or
phone), and the waiter returns with the result: `Waited::Turn` carries the check's
`CheckInfo`, MCP replies end with its one-line result, and `maitty wait` prints that line
to stderr and exits 4 (2 is timed out, 3 needs you). A check that could not run gives a
notice and then the normal finished alert. Queued follow-ups still send when the turn
ends, and `sessions.helpers_at_once` still counts only working sessions. A session with no check
alerts and waits exactly as before.

A project with several checkpoints runs them in order, each whatever the one before it
returned. The session is reported by its headline checkpoint (`protocol::headline`, the
worst). A waiter stays blocked while any checkpoint is checking or pending and exits 4 when
any failed. A checkpoint waiting for approval, one you said never to, one that could not
run and a stale pass all exit 0. Waiting for approval raises a blocked attention alert and
the notice `<place>: approve check: <command>` once a turn; it is never a finish.

A working agent the daemon marks quiet (`TermInfo::quiet_since`) reads `[claude: working · quiet
12m]` in `maitty ls`; `ls --json` and `maitty_list` add `quiet_since` (unix seconds) only then. A
`maitty wait` or `maitty_wait` that times out on one says `still working, but quiet for 12m`.
A working agent the daemon marks repeating (`TermInfo::repeats`, the calls the repeat has reached)
reads `[claude: working · repeating 6x]` in `maitty ls`; `ls --json` and `maitty_list` add `repeats`
only then; and a wait that times out on one says `still working, but repeating itself (6x)`. A quiet
agent says so first. The count is a hint, never a status: nothing waits on it or rings for it.

`maitty ls`, `maitty ls --json` and MCP `maitty_list` report a check next to the status, never in
place of it: `status` keeps its value (`done`) and `TermInfo::reported_check` decides whether a
result is shown (not while the agent is working or asking, as in the sidebar). `cli::check_note`
is the one wording for both text surfaces: `check failed (exit 1)`, `checking`, `check passed`,
`check could not run`, `check overridden`. `ls` prints it in the agent tag
(`[claude: done · check failed (exit 1)]`); `maitty_list` adds it as a `check` string. `ls --json`
adds a `check` object (`state` as `checking`, `passed`, `failed`, `overridden` or
`could_not_run`; `command`; `exit_code`; `duration_ms`; `first_failing_line`; `rerun_queued`;
`overridden_at`). A pane with no check has no `check` key and its output is unchanged.
The newer states read `pending`, `approve check: npm test`, `stale pass: test · 3 files
changed` and `not checked: never for this repo`. With several checkpoints the note ends in
`· n checkpoints`, and `ls --json` also lists every one under `checks` (`name`, `state`,
`command`, `exit_code`, `duration_ms`, `first_failing_line`, `line_count`, `turn`,
`rerun_queued`, `overridden_at`, `overridden_by`, `stale_since`, `changed_files` as
`{kind, path}`); `check` stays the headline.

## Project-specific: structured display history foundation

`daemon/conversation/history.rs` owns a display projection, separate from native
executable context. Stable identities are conversation + exact native turn/item
IDs; turn status has a separate row (empty item ID reserved internally).
Lifecycle events upsert the existing row in place. Codex deltas have no sequence
or offset, so equal chunks are appended within one connection; completed native
items win over replayed deltas. Reconnect drops uncertain drafts before ordered
authoritative hydration. Unknown items/events make completeness explicit.

Every entry is passed through `redact_entry` (`redact.rs`) before it is stored, so a
history page never carries a token-shaped string (shown as `[redacted]`) or the text of a
secret file (a tool input, diff or command output holding `[content of a secret file
omitted]`, with `omitted_bytes` in the input). Both are plain strings in existing fields,
so the wire shape and `PROTOCOL_VERSION` are unchanged; clients show them as hidden, not as
errors. See [SECURITY](SECURITY.md#conversation-history-and-secrets).

Internal reads accept a conversation/revision/position cursor, at most 128 entries
and 256 KiB of serialized JSON including the page envelope. Content/order changes
invalidate cursors; oversized entries fail explicitly. `next = None` means cached
exhaustion only. Completeness is independently Unknown, Loading, Complete or
Incomplete(reason). Ordinary snapshots contain summaries only; explicit
`ConversationRead` replies carry bounded history pages.

**Following a long history.** Entries are added at the end and changed in place (a tool step
finishing, a message growing), never moved or removed until the history is rebuilt. Every page
carries `resume`, a `delta` cursor (`epoch`, `revision`): reading with it returns only the entries
added or changed after that revision, in order, paged with `next` like any read, so a client that
applies entries by their source id follows a conversation of any length with small reads. The
daemon keeps the revision of each entry's last change (`Track` in `history.rs`). A delta read pages
against the revision its first page began at, and its last page's `resume` starts there too, so a
change made between its pages is returned again rather than missed. A history that is rebuilt
(hydration, or a reconnect that drops drafts) gets a new `epoch`; a `delta` cursor from another
epoch, from the future (another run of the daemon) or from another generation is not an error: the
page starts from the beginning and sets `reset`, and the client discards what it held. A plain
paging cursor still goes stale on any change (`StaleCursor`) and the client starts that read over.
`epoch`, `delta`, `snapshot`, `resume` and `reset` are all defaulted, so an older client or daemon
reads the rest as before (a test decodes the old shapes); no `PROTOCOL_VERSION` bump.

Daemon notifications reach
attached projections only after current generation/connection acceptance, and
client disconnect drops that attachment's memory. Production admission pages
`thread/turns/list` ascending with `itemsView: full`, checks every continuation
cursor and refuses partial/unsupported history before enabling writes. Hydration
is capped at 100,000 entries and 32 MiB. Cached completeness is never fresh native
completeness.

`Cache::load` describes only its committed historical revision. Attachment
`hydrate_cached` demotes it to disconnected/incomplete; only `hydrate_native`
after authoritative reconciliation may establish current completeness. Structured
admission must use this boundary when a provider continued during an attachment
gap; cached exhaustion/Complete never proves fresh native completeness.

Native hydration preserves running item prefixes as connection drafts: following
deltas append to them, and durable snapshots omit them with incomplete status.
Empty deltas do not mutate the projection. Attachment hydration advances beyond
both imported and existing revisions; cache commits reject older or equal
revisions so competing attachment snapshots cannot replace an equal revision.

The cache accepts one daemon-owned authoritative projection/writer per
conversation. Attachment read views are not independent cache writers. A
`commit` result of `false` explicitly rejects a stale/equal revision or obsolete
policy epoch; it is not a save acknowledgement. The caller must retain dirty
state and refresh/reconcile before retrying. The daemon retains dirty revisions
until a matching commit succeeds, with at most one cache write in flight per
conversation. Reload fences the policy synchronously; deletion runs in the shared
background storage lock. Attachments never submit independent cache revisions.

## Project-specific: structured conversation IPC

See [stack/claude-code-cli.md](stack/claude-code-cli.md) for the Claude CLI
process boundary and event/approval handling rules.

**An agent in a pane has a read-only conversation** ([ADR 0003](adr/0003-status-and-conversation-trust.md)),
with no protocol change. For an owned desktop pane whose hook-verified agent is `claude` or `codex` and
whose hook gave a session id (and, for Claude, its transcript path), the snapshot's `conversations` holds
a `ConversationSummary` with `execution: Native`, `native_term: <pane>`, the pane's `status` and no
checks; `Query::ConversationRead` serves its history. The id is the daemon's, stays while the pane runs
the agent, and its `generation` rises when the pane's session id changes. It is in memory only (never in
the session file or history cache). `ConversationSend`, `Approve`, `AnswerInput`, `Cancel` and `End` on it
fail with a "read-only" error; `Rename` and `Archive` fail as for an unknown id (the pane is named and
closed as a pane). A client the pane does not belong to (ADR 0004) gets "No such conversation". The
history is the provider's own transcript projected through the same entries a structured runtime
produces (`conversation/follow_transcript.rs`); nothing read from the file sets any status.

`ConversationSend` carries an optional `choice` (`TurnChoice { model, effort }`, `#[serde(default)]`, no
protocol bump; a send from before it decodes as no choice). The daemon validates it (a plain model name that
cannot read as an option, an effort in `low | medium | high | xhigh | max`) and keeps it with a queued
message. For Claude every message starts its own `claude` process, so the choice becomes `--model` and
`--effort` on that process; for Codex it rides on `turn/start` as `model` and `effort`, which Codex applies
to that turn and the ones after. The choice never touches the permission arguments or the thread's sandbox
(ADR 0005). The window sends the picked values with each message, so the daemon stores nothing.

Protocol 71 adds the daemon-held send queue. A `ConversationSend` for a conversation that is
mid-turn (or whose earlier messages are still waiting, or with a send in flight) is not refused: the
daemon holds it and answers `Reply::ConversationQueued { conversation, generation, operation }`.
`ReadResult.queued` (`#[serde(default)]`) lists what it holds, oldest first (`operation`, `text`).
On its tick the daemon delivers the oldest message of each conversation whose turn has ended and
that has no unanswered request, no cancel in flight and no uncertain earlier send, through the same
receipt and durable-barrier path as a direct send, so a delivered message is one `Receipt` with its
own operation number. The queue is per conversation, in order (an operation number must exceed the
last one held), bounded (32 messages, 1 MiB), in memory only, and dropped when the conversation is
ended or has started over (a new generation). Retries keep their meaning: the same operation and text
while queued answers `ConversationQueued` again without a second copy, once delivered it answers the
`Receipt`, and the same number with other text is `stale or reused`. `Command::ConversationDequeue
{ id, generation, operation }` takes a waiting message back out (human clients only, write grant;
`Ok`, or an error that it is no longer waiting). A new command and a new reply variant, so
the version was raised to 71; tests assert an older peer cannot read them and that an older read reply
still decodes.

Protocol 72 adds `branch: Option<String>` (`#[serde(default)]`) to `Command::ConversationStart`, for a
chat session on a branch ([ADR 0003](adr/0003-status-and-conversation-trust.md)).
The daemon, before anything is made, refuses a `branch` beside anything but a new conversation
(`StartTarget::Create`), checks the name with the branch agent's rules (`branch_agent::branch_problem`),
and requires the target's folder to be a project the app shows and a git repository
(`conversation/branch_start.rs`). It then makes the worktree with `create_trusted_worktree`, runs the
repo's setup hook, notes it in `made_worktrees`, and starts the conversation in the new folder; the reply
is the usual `ConversationStarted`. The field is defaulted, so the old shape decodes (a test), but a
peer that predates it would drop the field and start in the main checkout, so the version was raised
to 72 and the two refuse to talk. The window's Tauri command `start_chat_session` asks for
workspace write with approval on request, or, with its optional `full_access` flag, for `Sandbox::FullAccess`
with `ApprovalPolicy::Never` ([ADR 0005](adr/0005-provider-adapters-and-permissions.md)). It adds no wire field: the choice rides in the existing
`LaunchSettings`, so the protocol version is unchanged. It is local-only.

**Shared settings.** `Query::Settings` answers
`Reply::Settings(SharedSettings)`; `Command::SetSetting { key: SettingKey, value: SettingValue }`
(human clients only) edits `config.toml` in place and reloads, and every attached local window
gets `ServerMsg::SettingsChanged`. Keys, values and limits are in
[ENVIRONMENT](ENVIRONMENT.md#project-specific-settings-both-apps-share). A value of the wrong kind or
out of range answers `ServerMsg::Error` and leaves the file as it was. A peer that predates the
variants cannot decode them (a test), and another machine's `SettingsChanged` is ignored.

Protocol 76 adds `Provider::Gemini` (a conversation's provider on the wire). A Gemini CLI agent in a pane gets the same read-only, transcript-following conversation as Claude and Codex (`follow_transcript.rs`, `Format::Gemini`); the file is the one its hook's `transcript_path` names, and its name must start `session-` and end with the first 8 characters of the hook's session id. Gemini has no structured runtime: starting one is refused. A peer that predates the variant cannot decode it (a test decodes the old provider names).

Protocol 75 adds `Command::NewFleet { name, cwd, colors }` and `Command::NewShip { ws, cwd, cmd }`. A fleet is a
workspace on a machine's daemon: `NewFleet` names it (trimmed, 1 to 80 bytes, no control characters), starts it
with one shell and takes the first unused colour from the list the client wants; `NewShip` adds a shell to an
existing fleet. `SetWorkspaceColor` now answers an error for an index past the palette.

Protocol 74 adds `Command::CloseSession { term }` and `Command::UndoClose { term }`, the shared settings
messages above, and, without a bump of their own, the defaulted `TermInfo` fields `diff_added`,
`diff_removed` and `exit`. `CloseSession` hides a session from the snapshot and its layouts for six seconds
(`UNDO_WINDOW`) with its process still running, then closes it as `ClosePane` does; `UndoClose` inside the
window brings back the same session in the same place. `ClosePane` still closes at once. A session ids only
count up, so id order is creation order. `exit` is set from the shell's own exit mark when a command ends
non-zero, and is only a signal for the red "exited with error" mark.

Protocol 73 adds `Query::ListFolders { path, tag }` and `Reply::Folders { tag, listing }`: the folders
directly inside `path` on the daemon's computer (`FolderListing`, `FolderEntry` in
`crates/maitty-shared/src/protocol.rs`), for a client choosing where to start a session (the terminal
app's New session dialog, the desktop app's folder dialog). `path` is absolute or starts with `~`
(the daemon's home folder), at most 4,096 bytes; anything else answers `exists: false`. Only folder
names are listed, never file names or contents. Each entry carries the git branch when it is a
checkout (read from `.git/HEAD`, no git run) and whether it holds a folder worth browsing into
(hidden folders do not count). At most 1,000 entries, with `truncated` set past that. Human
clients only; the reads run off the event loop. A listing from another machine is cleaned the way a
snapshot is (`remote_policy::vet`): control characters out, names that are paths dropped, the count
cut, and it is passed on only while a request to that machine is waiting.

Protocol 85 adds `Query::CloneRepo { address, parent, tag }` and `Reply::Cloned { tag, outcome }`
(`CLONE_SINCE`; `COMPATIBLE_SINCE` stays): `git clone` of `address` into a new folder, named after the
repository, inside `parent` on the daemon's computer (`crates/maitty-core/src/daemon/clone.rs`).
`outcome` is the new folder's path with `/` separators, or one line saying why nothing was cloned; a
refusal is an answer, not `ServerMsg::Error`, so the client that asked can match it by `tag`. The
address rules are in [SECURITY](SECURITY.md#project-specific-auth-and-permissions). `parent` follows the
rules of `ListFolders` and must be a folder that is there; a file or folder of the new name already
there is a refusal. Human clients only. git runs off the event loop, cannot ask a question
(`GIT_TERMINAL_PROMPT=0`, no input), and is stopped after ten minutes; a clone that fails or is stopped
leaves no folder. An answer from another machine is cut and cleaned to one line (`remote_policy::vet`).
The desktop host sends it through `clone_repository(machine, address, parent, tag)` and raises a
`repository-cloned` event with the tag and either `folder` or `refused` (`apps/gui/src/api/cloneRepository.ts`
waits on it). The host refuses only what could not be a request (empty, over-long, a control
character); the daemon is the judge of the address. A reply from another machine normally counts as an
answer for 30 seconds after a request; a clone's is expected for the daemon's limit plus 30 seconds
(`routing::answer_window`), and a quick answer to something else never uses up that wait.

Protocol 86 adds `Query::GitHubAccounts { tag }` and `Reply::GitHubAccounts { tag, report }`
(`GITHUB_ACCOUNTS_SINCE`; `COMPATIBLE_SINCE` stays): who is signed in to github.com on the daemon's
computer, as its GitHub command line says (`gh auth status --json hosts`,
`crates/maitty-core/src/daemon/github.rs`). `report` is `installed` (false when the machine has no
`gh`), `accounts` (login and whether it is the active one, active first, at most
`GITHUB_ACCOUNTS_MAX`, only sign-ins that still work) and `problem` (one line when `gh` could not be
read: too old to answer in JSON, or no answer in 15 seconds). Nobody signed in is an empty list, not a
problem. Human clients only; `gh` runs off the event loop and cannot ask a question. A report from
another machine keeps only logins GitHub could have given out and one clean `problem` line
(`remote_policy::vet`). A program run with a time limit goes through `proc::run_within`. The desktop
host sends it through `github_accounts(machine, tag)` and raises a `github-accounts` event with the tag
and the report (`apps/gui/src/api/githubAccounts.ts` waits on it for 20 seconds).

Protocol 87 adds `Query::GitHubRepos { owner, search, tag }` with `Reply::GitHubRepos { tag, answer }`,
and `Query::GitHubOwners { tag }` with `Reply::GitHubOwners { tag, answer }` (`GITHUB_REPOS_SINCE`;
`COMPATIBLE_SINCE` stays). Each `answer` is a `GitHubAnswer`: `NotInstalled`, `NotSignedIn`,
`Failed(one line)` or `Listed(…)`; having no `gh` and nobody signed in are answers, not
`ServerMsg::Error`. Repositories are `owner`'s, or the signed-in account's when none is named
(`gh repo list`), narrowed by `search` when there is one (`gh search repos --owner`): at most
`GITHUB_REPOS_MAX` (200), each with `name` (`owner/name`), a one-line `description` cut at 300
characters, `private`, `pushed_at` and `clone_url`, and `more` when there were more. Owners are the
signed-in login, then its organisations (`gh org list`), at most `GITHUB_OWNERS_MAX`. Human clients
only; `gh` runs off the event loop and is stopped after `GITHUB_LIST_LIMIT_SECS` (30). What a login,
a repository name and a time may look like is in `maitty_shared::github`; `clone_url` is always built
from the checked name (`clone_address`), by the daemon and again when the answer comes from another
machine (`remote_policy::vet`), never copied from what was printed or sent. The desktop host sends
them through `github_repos(machine, owner, search, tag)` and `github_owners(machine, tag)` and raises
`github-repos` and `github-owners` events: the tag, a `state` (`listed`, `not-installed`,
`not-signed-in` or `failed`), `why` for a failure and `listed` for a list
(`apps/gui/src/api/githubRepos.ts`). Their answers are expected for 75 seconds
(`routing::answer_window`), which covers the daemon's two `gh` runs. In the window, a question whose
answer is an event with the same tag goes through `apps/gui/src/api/taggedAnswers.ts`.

Protocol 70 adds `Query::UsageReport { period }` (`day`, `week` or `month`; rolling windows
ending today, UTC) and `Reply::UsageReport`, answered from the daemon's usage store
([ADR 0009](adr/0009-usage-is-provider-reported-never-estimated.md)): totals, one entry per
session and one per day with usage, as `UsageReport` in `crates/maitty-shared/src/usage_report.rs`
(camelCase when it reaches the desktop app as JSON). Human clients only. A figure no provider
reported is `null`, never zero. `Usage` also gains the defaulted `tokens_in` and `tokens_out`
(the halves of `tokens`), so older peers still decode it. The desktop app calls it through the
`usage_report` command.

Protocol 67 adds, for the desktop app's Inspect panel, find, quick-look links and worktrees:

- `Query::Repo { term, tag, request }` and `Reply::Repo { tag, reply }`: read-only looks at the
  repository a pane is in (`RepoRequest`: status, tree, file, `Locate`, working-tree diff, range
  diff, branches, log, worktrees, snapshots, references; `crates/maitty-shared/src/repo.rs`,
  answered by `daemon/repo.rs`). Human clients only, with the pane's read grant. The daemon roots
  every path at the top of the repository the pane's folder is in, refuses symbolic links and
  paths that climb out, refuses a revision that starts with `-`, and runs git on a blocking
  thread. `Locate` turns a path seen in the pane's output (absolute, or relative to the pane's
  folder) into a repository-relative one, and refuses anything that is not a file inside the
  repository. The answer echoes `tag`, so a client can have several in flight.
- `Command::RestoreSnapshot { term, tag, commit }`: put the pane's checkout back as it was in one
  of the daemon's own snapshots (the commit must be one `RepoRequest::Snapshots` lists), after
  saving what is there now. Only a human can, and not while the pane's agent works. It answers
  `Reply::Repo { Restored(n) }`. This is the one request that changes the repository; every other
  change stays in the pane's PTY.
- `Snapshot.daemon_id` (`Option<String>`, `#[serde(default)]`) is the daemon's random id, kept
  across its restarts. A daemon built before it sends none, and a window still connects to it. A
  window records the id per machine name the first time it sees one and refuses a connection
  whose id differs (see [SECURITY](SECURITY.md)). Added without a `PROTOCOL_VERSION` bump; a
  test decodes the old shape.
- `TermInfo` carries four more fields, each defaulted so an older peer's snapshot still reads:
  `asking` (what a stopped agent is waiting on, read off its screen: the exact command or file
  and its numbered choices; empty unless the pane needs you), `interrupted` (an agent that was
  mid-turn when the daemon restarted, with the command that resumes it), `timeline` (the latest
  turn events, oldest first, from status reports and the shell's OSC 133 prompt marks, 60 kept in
  memory) and `usage.tokens` (what the provider reported, never an estimate). The daemon never
  resumes an `interrupted` agent: a pane saved mid-turn comes back as a plain shell, whichever app
  owns it, and the window offers the command.
- `Query::PaneSearch { query }` and `Reply::Search`: where a word appears in what the app's own
  panes printed recently, case-insensitively, at most 100 hits and 20 from one pane. Human clients
  only, over the panes the client may use. The text is the pane's replay with escape sequences
  removed, so it is what was printed, not what is on screen now.

`NewWorktree` also carries `agent` and `prompt` (`Option<String>`, `#[serde(default)]`; added without
a `PROTOCOL_VERSION` bump, and a test decodes the old shape): the desktop app's "New agent on a
branch", which the revised v18 removed from the window; the daemon keeps accepting them. When either is set the daemon, before anything is made, refuses
a `cmd` beside them, checks `branch` (`[A-Za-z0-9_./-]+` plus git's ref rules, at most 200
characters; `daemon/branch_agent.rs`), requires `from` to be a project the app shows (a repository some
workspace or pane is in), requires the agent to be a configured preset or built-in agent, and bounds the
prompt (8000 characters, no control characters but line breaks and tabs, no leading dash so the agent
cannot read it as an option). The prompt goes in the agent's command line quoted for the pane's shell
(`Config::agent_command`), as the MCP `start` tool does, never interpolated. The worktree itself is the
usual one (`create_trusted_worktree`, the repo's setup hook, the `[worktree] dir` naming, clean-up when
its last session closes). Local projects only: the window refuses another machine's project.

Protocol 65 removes the handoff between chat and terminal
(`ConversationHandoffToNative`, `ConversationHandoffToChat`): a session is a chat
or a terminal session from the moment it starts. It adds the human-only
`ConversationRename` and `ConversationArchive` commands, and `title` and `archived` on
`ConversationSummary` and the saved conversation record (both defaulted, so older files
still load). A name and a closed flag belong to the conversation, so every client reads the
same ones: a rename in the terminal app shows in the desktop app and back, and closing a
chat in either puts it away in both. Closing a chat is not stopping it: an archived
conversation keeps running in the daemon, and a client lists it only under its own
"archived" view. A title is trimmed, stripped of control characters and capped at 80
characters (`clean_title`); an empty one clears it, and each client then words the session
itself. Clients keep no copy of either in browser or file storage, except the browser
preview, which has no daemon.
It also adds terminal access keyed by the pane, for terminal sessions that have no
conversation: the human-only `TerminalInput` and `TerminalResize` commands and the
`TerminalScreen` and `TerminalReplay` queries. They follow the same limits as the
conversation-bound ones (16 KiB of input per message, sizes from 2x2 to 500x300, write
and respond grants, and read for the queries) and resolve nothing but the pane number
they are given.
Protocol 59 adds the human-only `ConversationTerminalResize` command. It
requires the exact conversation generation, validates bounded terminal
dimensions, and resizes only the daemon-resolved native pane. Terminal replay
queries return the pane's bounded raw output history for initial scrollback;
screen queries return a formatted ANSI snapshot for terminal emulation.
Protocol 56 added conversation-bound native terminal screen and input messages.
They require the exact conversation generation and resolve the attached pane
inside the daemon; input is human-only, requires write (and respond while the
pane is blocked), and is limited to 16 KiB per message. Protocol 55 added a
read-only conversation diff query. Protocol 54 added Codex permission approval
requests. Protocol 53 added file-change approval requests. Protocol 52 added
`ConversationStart`, `ConversationSend`, `ConversationApprove`,
`ConversationCancel` and `ConversationRead` over the existing authenticated
socket. Start needs the start grant, send and cancel the write grant, approval
answers the respond grant and read the read grant. Conversation IDs grant no
pane self-exemption.
Programmatic prompts use the same invisible-character refusal as ordinary send.
Pane-originated starts also use existing helper consent and concurrency limits.
Pending structured launches and live structured helpers count alongside native
helpers; the verified initiating pane remains attached to the runtime's state.

Daemon-owned structured processes receive a separate generation/connection-bound
identity through four reserved environment variables and `Hello.structured`.
The runtime mints its own secret; it never copies a pane token. Partial/malformed
environment claims and rejected/stale handshakes cannot become human callers.
Every incoming request rechecks the live owner and configured MCP grants;
structured callers have no pane self exemption or human-only approval authority.
Granted list/read/send workflows remain available under their respective grants.
Structured descendants are refused until conversation-owned helper consent is
supported; ordinary native helper behavior is unchanged.

Before creating/resuming a thread, the actor reads effective provider config for
the selected cwd on its own initialized connection. Supported shell policy retains
the four identity variables; unsupported custom filtering/set/profile policies
fail before thread creation. Runtime-only, value-free config overrides preserve
existing stdio MCP inherited-variable lists and append the identity variable
names. Explicit conflicting MCP environment values are refused. No authentication
secret is inserted into provider config, persisted thread settings or metadata.
MCP references revalidate authenticated read access before filesystem listing.

Start targets are Create with explicit launch settings, Session with an exact
Codex `Thread.id`, or Conversation with a Maitty ID and expected generation.
Paths are resolved in background before admission. Explicit resume never uses
latest, a picker, a fork, alternate history or a missing-thread fallback. Native
profile-v2 remains unsupported. Effective cwd/model/provider/approval reviewer
and sandbox are validated against the requested posture before identity commits.
The exact native binding commits before start acknowledgement or turn submission.

Send carries conversation ID, generation, a nonzero increasing operation ID and
text. One current receipt and content digest are retained per conversation;
prompt text stays in memory. Persist `SubmissionUncertain` before the RPC and
persist the acknowledged native turn ID before replying. Same-operation retries
return a durable receipt without sending again. Changed/reused/older IDs, stale
generations and busy/blocked runtimes fail. EOF, partial writes, timeout, crash or
failed acknowledgement persistence leave delivery uncertain and prohibit automatic
resend. Request correlation IDs do not establish provider idempotency.
Explicit reads expose an uncertain receipt while acknowledgement persistence is
pending; the staged in-memory native acknowledgement is not a durable receipt.

Command execution, file-change, permission and user-input requests retain the exact provider request ID, method,
generation and connection. File-change approval is enabled only after the
matching patch notification arrives, so the GUI can show the requested files and
diff before a decision. A human with the respond grant may answer only the
currently pending request with `accept`, `acceptForSession`, `decline` or `cancel`.
The request ID is removed only after its response is queued on the owning runtime;
stale generations, duplicate answers, and structured runtime callers are refused.
Only the command, requested write root, permission profile and reason needed for a decision are exposed to the client;
invisible Unicode characters are displayed as `\\u{...}` escapes. Raw provider
params are not retained in conversation history or the metadata ledger.
User-input answers must cover each displayed question, use offered choices or an
explicit free-form option, and contain no hidden Unicode. Secret answers are
forwarded only in the live provider response and are not stored in conversation
history or metadata. Permission requests show the exact requested profile and
allow only that profile for the current turn; declining returns an empty profile.
Session-wide grants are not offered. Other provider request kinds remain visibly
blocked and unanswered. Turn
Cancellation names only the acknowledged active turn ID and exact native thread;
uncertain submissions cannot be canceled by guessing. The interrupt response
means the request was accepted, not that the turn stopped; the GUI waits for the
matching `turn/completed` event. Only human clients can cancel. Native ownership
transfer remains a later ticket contract. A completed
native turn enters the existing checkpoint resolver/approval/worker path once;
generation/run fences reject obsolete results. Provider completion cannot turn a
failed or pending checkpoint into a pass.

`ConversationDiff` accepts only a conversation ID and generation. The daemon
resolves its checkout from the stored conversation binding, requires the read
grant, and runs Git on a blocking worker. It includes tracked and non-ignored
untracked changes, returns at most 100 files and 2 MiB of patch text, and marks
the result truncated when large or symlinked untracked files or excess diff data
are omitted. `ConversationFiles` and `ConversationFileRead` use the same exact
conversation binding and read grant. Paths are relative to its checkout; parent
traversal and symbolic links are refused. Listings are bounded to 500 entries,
and previews are UTF-8 files no larger than 256 KiB. Filesystem work runs on a
blocking worker, and results from obsolete generations are discarded.

Snapshot conversation summaries expose stable ID/generation, provider, cwd,
worktree, recovery/status/checkpoint context and the optional real native pane.
They carry no transcripts, launch environment, configuration roots or credentials.
Explicit read replies carry the same checkpoint-gated status and checkpoint
summaries. Provider error notifications and failed/interrupted native turns stay
blocked with an explicit error reason; they cannot report successful completion.
Failures answer the owning pending request and notify attached observers, without
injecting unrelated errors into other one-shot clients.

Revision advances use checked arithmetic before changing content, order or
completeness. Exhaustion returns typed `RevisionExhausted`, preserves the prior
view/cursor revision and surfaces at the daemon boundary. Cache loading/preparing
rejects an exhausted manifest/projection revision; imported attachment revisions
are checked before replacing the existing view. Revisions never wrap or saturate.

## Project-specific: connections to other machines

One supervisor task per `[[machines]]` entry (`link/` in `maitty-machines`) keeps that machine's line up.
It talks to the window only through `LinkEvent`s (a new line; a new state) and `LinkControl`
commands (line ended, refuse, retry), so a dead machine cannot hold a lock or block drawing.
With no `[[machines]]` entry nothing starts. `--remote host` is unchanged: the window's one
machine, `MachineId::LOCAL`, connected by `open_or_spawn`.

**Ids.** `MachineId` 0 is this computer. Configured machines get 1, 2, ... in name order
(case-insensitive), so a run's ids follow from its config. Ids are never saved (layout is kept
under the machine's name), so adding a machine renumbers the ones after it without harm.

**States** (`ConnectionState`; `link::next(state, event, now)` is pure and takes the time):

| State | Meaning | Left by |
| --- | --- | --- |
| `Connecting` | first dial of this run | success, or a classified failure |
| `Connected` | a line is up | the line dropping (`LineDropped`, `WatchdogFired`) |
| `Reconnecting { attempt, since }` | a line that was up dropped; the first `RECONNECT_DIALS` (3) failed dials still read as this | success, or a failure that needs a person, or the dials running out |
| `NeedsLogin { why }` | ssh could not log in without a prompt | success |
| `NeedsUpdate { why }` | the far side runs another protocol version, or has no app | success |
| `Unreachable { since, why }` | no answer (also: no daemon running there; refused) | success; `Retry` shows `Reconnecting` |

A failed dial in `NeedsLogin`, `NeedsUpdate` or `Unreachable` changes nothing a person sees
except `why`: retries are quiet. `Retry` (a person asks, or the machine's config entry changed)
dials at once. A machine's agents stay in the sidebar model through every state (the snapshot
is kept, not cleared), and the next `State` replaces it.

**Backoff** (`Backoff::DEFAULT`): no wait before the first dial and before the redial of a line
that was up; after `n` failures in a row, 250 ms doubled `n - 1` times, never more than 30 s
(250, 500, 1000, ..., 16000, 30000, 30000). A success resets it.

**Failure classes** come from `link_failure::classify`, from structured signals: the error's
type (`OtherVersion` is `NeedsUpdate`), ssh's exit status, and whether the far side ever sent
its stream marker or a frame. Exit 127 is `NeedsUpdate` with the install text. Exit
`NO_DAEMON_EXIT` (74) is `Unreachable` with the start-it text. Exit 255 with nothing received
and one of three fixed tokens in stderr (`Permission denied`, `Host key verification failed`,
`Too many authentication failures`) is `NeedsLogin`: the only place ssh's words are read.
Everything else is `Unreachable` with ssh's last stderr line, control characters removed and
cut to 200 characters. The fix text of each class is one constant in `maitty-machines` (`link_failure.rs`)
(`FIX_LOGIN`, `FIX_UPDATE`, `FIX_INSTALL`, `FIX_NO_DAEMON`); `FIX_UPDATE` and `FIX_INSTALL` name
`maitty machine update <name>` (and `--install`), with `<name>` as written, one text for every machine.

**Custom transport.** A machine with `via` is reached by running the user's program
(`Route::Command`, `ssh_command::command_argv`, both in `maitty-machines`) in place of ssh: the same pipes, stream marker skipping,
handshake and `SshEnd` (with `custom` set) as ssh. Text before the marker with no marker is the same login
error. Its failures classify without ssh's words: exit 127 is `NeedsInstall`, `NO_DAEMON_EXIT` is
`NoDaemon`, anything else is `Unreachable` with the program's last stderr line, never `NeedsLogin`. On
Windows connecting fails with `a custom transport (via) is not supported on Windows in v1; use ssh`.

**Attach only, and when the daemon may start.** A daemon exits ~30 s after it has no session and
no attached window, so a fresh machine has nothing to attach to. The supervisor therefore decides
per dial (`MayStart` in `maitty-machines`' `link/supervisor.rs`): the dial runs `<remote_cmd> proxy` (the daemon is
started if none runs, as `--remote host` does; `Machine::starting`) on the machine's first dial,
on every retry of a failed first dial, and after an explicit `Command::Retry` (the Machines
dialog's retry, also after a refusal). The redials after an established line ended (stalled or
closed) run `<remote_cmd> proxy --attach-only` (built in `maitty-machines`' `ssh_command.rs`): if no daemon runs
there, the proxy exits `NO_DAEMON_EXIT` before writing anything, and the machine is `Unreachable`
with `FIX_NO_DAEMON` ("retry to start it there") until a person retries, so a dropped link never
brings back a daemon the user stopped. `Machine::from_def` is attach-only by default, which is
what the CLI's `--on`, MCP and `doctor` use: none of them starts a remote daemon.

**Liveness.** An idle line is kept honest by ssh itself (`ServerAliveInterval=15`,
`ServerAliveCountMax=3`: ssh exits after 45 s of silence, which closes the line). The client's
own watchdog is for the other failure: a frame that began and stopped arriving. The reader waits
`STALL` (15 s) per window for a whole frame; two windows in a row with the same non-zero part of
a frame buffered, and nothing more received, drop the line (`Burst.stalled`). A frame that
trickles in grows the buffer every window, so a big frame on a slow link is never dropped; an
idle line has no part of a frame buffered, so it is never dropped by this. (A line that stops
right after a frame's four length bytes has nothing buffered; ssh's ServerAlive ends it.) A dial
that never answers is dropped after `CONNECT_BUDGET` (ssh's connect timeout plus 5 s).

**Teardown.** A line's ssh child is owned by a background task that kills it when the writing
half is dropped and reaps it there; nothing awaits ssh's end on the way out. Ending a lost line
drops its tasks at once, so a stalled line's ssh does not linger.

**On reconnect** the daemon sends `State` and a `Replay` per pane on attach: `State` replaces
the machine's snapshot and each `Replay` replaces that pane's screen. Typing sent while the
machine was down is dropped, never queued: replaying keystrokes into a pane that moved on could
run commands the person no longer meant. The window counts it and says, once the machine is
back, "N keystrokes to <name> were not sent while it was disconnected".

**Refusal.** A different daemon answering at a machine's name (`machine_ids`) parks the
supervisor (`Refuse`): no dial until a person asks. A new line re-checks the id, so the same
wrong daemon is refused again and a fixed one is let in; editing or removing the machine's
`[[machines]]` entry and reloading the config clears the refusal and dials. A running
supervisor keeps the entry it started with until the app restarts.

## Project-specific: client routing across machines

Every message the window sends is an `Outgoing { machine, msg }`; `client/routing.rs` and
`App::{send, send_input, cmd_on}` decide `machine`. The rule:

1. Anything addressed to a pane (a `ClientTerm`) goes to `term.machine`.
2. Anything addressed to a project, session, tab or workspace goes to the machine that owns it:
   the `ClientSession` / `Proj.machine`, or the focus machine for a workspace or tab id read from
   the active workspace (`active_ws`, `active_tab`).
3. A new session, pane, tab or workspace goes to `target_machine()`: the machine of the session
   selected in the sidebar, else the focused one (ADR 0007); this computer when neither is anywhere else.
   A project's own "new" (its row, a recipe, "+ New") goes to `Proj.machine`.
4. Things with no owning machine (settings, themes, reload config, kill server, UI state) and
   anything that acts on this computer's disk stay local.

`focus_machine()` is the machine whose active workspace and tab hold the window's focus. It
follows the last command that moved a focus (`routing::moves_focus`) and falls back to this
computer when that machine is gone. `focused()`, `active_ws()`, `active_tab()`, `popup()` and
`target_ws()` / `target_term()` read that machine's snapshot. A popup on another machine than the
focus never takes the keys.

| Outgoing path | Machine before | Machine after |
| --- | --- | --- |
| `Input` (typing, paste, mouse and focus reports, `send_input`) | `term.machine` | `term.machine` |
| `Resize` (`sync_sizes`) | `term.machine` | `term.machine` |
| `Hook`, `Usage` | sent by the CLI, never the window | unchanged |
| `FocusPane` from pane clicks (`input.rs`), `MarkSeen` in the Inbox, `ConversationArchive` when a pane ends | this computer | `term.machine` |
| `FocusPane`, `ClosePane`, `RenamePane`, `SetTabOf`, `SwapPanes`, `TeachAgent`, `ClearQueue`, check commands, `AnswerHelpers`, `AnswerSecret` | `term.machine` | unchanged |
| `Query::CheckOutput` | `term.machine` | `term.machine`; the reply is taken only for an open sheet or waiting failure on that machine |
| `Query::Worktrees` | this computer | focus machine (worktrees are local-only for other machines) |
| `NewWorkspace` from new session, shell here, split, recipes, new-agent dialog | this computer | `target_machine()`, or `Proj.machine` for a project's own |
| `NewWorkspace` from a split menu | this computer | `term.machine` of the pane split |
| `NewTab` (`SpawnTab`) | this computer | `target_machine()` with that machine's active workspace |
| `RenameTab`, `CloseWorkspace` (prompts carrying `WsId` / `TabId`) | this computer | `focus_machine()` (the ids came from its active workspace) |
| `ConversationRename`, `ConversationArchive` | this computer | `conversation_machine(id)`: the selected session's machine if it lists the chat, else the first machine that does |
| `NewWorktree` (also with `agent` and `prompt`), `BranchWorktree`, `RemoveWorktree`, `TidyWorktrees`, `CloseWorktree`, `AgentWorktree` | this computer | this computer; refused for another machine's project or pane |
| `ForgetCheckApproval`, `ReloadConfig`, `KillServer`, first-state `SelectWorkspace` / `NewWorkspace` | this computer | this computer (no owning machine) |

**Local-only features.** `Feature` (Files, Changes, Find, Branches, References, image paste, agent
status setup, Worktrees) names what reads or writes a pane's files or runs git there. One gate,
`App::guard_local(machine, feature)`, decides per pane: allowed when the pane's files are on this
computer, otherwise a toast and nothing else. Every entry point calls it: `act()` for actions
(with the selected pane's machine), the project menu for worktrees, `open_path_from`,
`paste_image`, the Inbox's Changes / merge / discard keys, and new worktrees. A window attached
to one remote daemon (`--remote`) keeps its own worktrees; every other feature there is gated too.
The message is one format, `routing::local_only_message`: "<Feature> is local-only in v1: the
files are on <machine>, not on this computer."

A pane's folder becomes a path on this computer only through `App::local_cwd(term)`, which is
`None` for a pane on another machine. Git runs only in `local_git_projects` (overlap and
changed-line counters), never in a project whose machine is not this one.

**Replies.** The wire is unchanged. For each `Command` or `Query` sent to another machine the
window keeps a timestamp (`Routing::asked`); a `Reply` or `Error` from that machine is applied
only while one is waiting (`awaiting`), and consumes the oldest (`answered`). A timestamp older
than 30 s is dropped and at most 64 are kept, because most commands are never answered. An
unsolicited reply or error is ignored (`remote_policy::Verdict::Answer`). This computer's own
daemon is not subject to it.

**A machine that is down.** `App::deliver` hands a message to `Connections::send`. On
`NotConnected`, typing is counted (`unsent_input`, told when the machine is back); any other
command or query raises one toast, "<machine> is disconnected; <what> was not sent."; resizes and
the rest are dropped silently. Nothing blocks and nothing is queued.

**One-shot commands.** `cli/target.rs` picks the machine of a command once, before it runs: a
pane address `machine/id`, else `--on`, else the `--remote` adapter (`ipc::Machine::current()`),
else this computer. Each command is handed that `Target` and opens its connection with
`ipc::open_to` (for another machine, `maitty_machines::open_remote`); nothing reads the environment for the machine after that. A configured machine only
attaches. The pane a command runs in (`$MAITTY_TERM_ID`) is a pane of this computer's daemon, so it is
the default pane only when the target is this computer or the ad-hoc `--remote` route; a command
aimed at a named machine needs an explicit `--pane machine/id`. `ls` is unchanged for this computer;
named at a machine it heads the list with `machine <name>`, writes panes as `name/id`, and
`ls --json` adds `"machine"` to each workspace. `ls --all` asks this computer and every configured
machine at the same time and prints those that answer; each that does not gets a line
`<name>: unreachable (<why>)` (or `needs a login`, `is another version`, ...) on stderr, and the command
fails only when none answered.

**MCP tools.** `maitty_list`, `_read`, `_send`, `_wait` and `_start` take an optional `machine`; a
session of another machine is written `machine/id` in a list and accepted as an `id` by every pane
tool (the `machine` argument may then be left out; if both are given and differ, the call is an
error). Omitted, a call is exactly as before. `machine = "all"` is for the list only. Errors that
name a machine are typed, with `structuredContent.error.code` and the text `code: message`:
`machine_not_enabled`, `unknown_machine`, `permission_denied`, `unsupported_for_remote`,
`unreachable`. Answering, interrupting and starting are `unsupported_for_remote`. See SECURITY "MCP
across machines" for who may reach what.
