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
`ClientMsg` / `ServerMsg` enums (`src/protocol.rs`). The first exchange is
`Hello { version }` → `Welcome { version }`; mismatched `PROTOCOL_VERSION`s are
refused. The CLI subcommands and the MCP server (`maitty mcp`, JSON-RPC over stdio)
are thin wrappers that send the same commands.

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
MCP session controls remain supported. The wire protocol is version 46.

`maitty_answer` takes the `prompt` id that `maitty_list` (`prompt` on a session that needs you) or
`maitty_wait` (`prompt <id>`) gave. It hashes the question and its choices, whitespace aside. If the
prompt on screen has a different id, nothing is sent and the error shows the current question,
choices and id.

**Helpers.** A start that comes from an agent's pane (`NewWorkspace`, `NewTab`, `Split`,
`NewWorktree` over a connection that proved it is that pane by its secret) is a helper
start. Under `[sessions] helpers = "ask"` the first one is refused with an error telling the
agent the user was asked, and the daemon records the task on the pane. `TermInfo.helper_ask`
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
snapshot field) in `protocol.rs` with a `PROTOCOL_VERSION` bump; handling in
`daemon/mod.rs` (or a `daemon/<feature>.rs`); the client side as an `Action` in
`keys.rs`, a `Mode` and `draw_*` / `on_*_key` pair in `client/`, and a CLI
subcommand in `cli.rs` when scripts should reach it.

## Project-specific: agent turns and background shells

Honor a trusted turn-finished hook even when its transcript lists a background
shell or dev server. Background commands do not turn `Done` back into `Working`.
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
