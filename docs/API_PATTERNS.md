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
(or `ssh host seshi proxy`): length-delimited frames carrying msgpack-encoded
`ClientMsg` / `ServerMsg` enums (`src/protocol.rs`). The first exchange is
`Hello { version }` → `Welcome { version }`; mismatched `PROTOCOL_VERSION`s are
refused. The CLI subcommands and the MCP server (`seshi mcp`, JSON-RPC over stdio)
are thin wrappers that send the same commands.

## Project-specific: response and error shapes

Commands are fire-and-forget; the daemon's answer is the next `Snapshot` (full
state) and pane output frames. Failures come back as `ServerMsg::Error(String)`,
a human-readable message the client shows as a toast; CLI subcommands print it
and exit non-zero.

## Project-specific: auth model

Socket access is the auth (see `docs/SECURITY.md`); pane status reports also need
the pane's process tree or `SESHI_PANE_TOKEN`.

## Project-specific: feature/module structure

`seshi profile list|run|export|import` is the local profile interface; see
[agent profiles](features/agent-profiles.md). Launches use the existing
`NewWorkspace` / worktree commands, so the wire protocol is unchanged.

`seshi workflow` and `seshi_workflow_*` MCP tools expose saved workflow execution;
see [usage](features/workflows/usage.md). Workflow commands and snapshots use
protocol 39. Only the authenticated active pane may submit a stage report;
approve/retry/cancel are human controls. Validate and freeze profiles before launch.

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
attention messages must not ring while the agent is working. Helper completion
stays quiet; helper questions may ring after the same blocked delay.
