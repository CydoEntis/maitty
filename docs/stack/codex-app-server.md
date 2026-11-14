# Codex app-server

> **Use for:** the Codex structured conversation contract and its compatibility fixtures.
> **Requires:** rust; an explicitly selected installed Codex executable.
> **Fills:** ARCHITECTURE § structured conversations; API_PATTERNS § provider boundary.

## Rules

- Keep the provider process daemon-owned. Use app-server JSON-RPC over stdio;
  do not introduce a Node bridge or interpret terminal text as provider events.
- Check the selected executable's version and generated schema before starting or
  resuming a thread. Missing methods or an unverified version must return an
  actionable unsupported error before any session mutation. Never auto-upgrade.
- On Windows, search `PATH`, `NVM_SYMLINK`, `%APPDATA%\npm` and
  `%ProgramFiles%\nodejs` for npm-installed Codex; resolve it to the packaged
  platform executable and do not launch its `.cmd` or `.ps1` wrapper as the
  app-server process.
- On Unix, preserve the invoked executable name through PATH lookup and launch.
  Version-manager shims such as Omarchy's mise `codex` symlink dispatch based
  on that name; canonicalizing the final symlink launches mise as `mise` instead.
- Send `initialize` with `clientInfo`, await its response, then send `initialized`.
  `capabilities` in initialize params declares client opt-ins; it is not a server
  capability list. The response has `codexHome`, `platformFamily`, `platformOs`
  and `userAgent`, not supported method discovery.
- Bind responses, notifications and server requests to the daemon-owned
  connection and current conversation generation. Match approval responses to
  the exact server request ID. On stdio, answer server-initiated requests with
  the JSON-RPC `{id, result}` envelope and keep the approval payload inside
  `result`. Never silently approve an unknown request kind.
- Preserve the effective account, config root, working directory, approval and
  sandbox settings. Refuse an unrepresentable or ambiguous identity/posture.
- Permit only one runtime to write a thread. An idle native CLI still owns its
  runtime; cancellation alone does not release ownership. Confirm process exit
  before starting the target runtime. Concurrent native/app-server writers are
  not a supported handoff mechanism.

## Project-specific: verified baseline

Verified `codex-cli` versions: `0.164.0`, `0.165.0`, `0.166.0-alpha.5.1`, and
`0.166.0`. Their generated schemas contain every required app-server method
and parameter type. The 13 required parameter definitions in
`0.165.0` and `0.166.0-alpha.5.1` are identical. Schema generation uses an
isolated temporary `CODEX_HOME`; no account, private config, transcript or model
request is required. Do not commit or hand-edit the generated schemas. Baseline source is OpenAI's
[`rust-v0.164.0`](https://github.com/openai/codex/tree/fe3b29455488c2e189e43ad5526a107900c5e1b7)
(`fe3b29455488c2e189e43ad5526a107900c5e1b7`).
Codex CLI 0.166.0's local generated schema was checked on 2026-10-31, and that
version passed a Windows GUI conversation create/send/reply smoke.

Required client request methods from generated `ClientRequest.oneOf`:

| Method | Params definition | Purpose |
| --- | --- | --- |
| `initialize` | `InitializeParams` | Connection handshake |
| `config/read` | `ConfigReadParams` | Effective cwd-scoped environment admission |
| `thread/start` | `ThreadStartParams` | Create exact provider identity |
| `thread/resume` | `ThreadResumeParams` | Reattach exact thread |
| `thread/read` | `ThreadReadParams` | Read metadata by `threadId` |
| `thread/turns/list` | `ThreadTurnsListParams` | Page persisted turns |
| `thread/items/list` | `ThreadItemsListParams` | Page persisted items |
| `turn/start` | `TurnStartParams` | Submit structured input |
| `turn/interrupt` | `TurnInterruptParams` | Cancel exact `threadId`/`turnId` |

Optional client request method, absent from the 0.164.0 and 0.166.0-alpha.5.1 fixtures and never required for
admission: `turn/steer` (`TurnSteerParams`, response `TurnSteerResponse`). Its params are
`threadId`, `expectedTurnId` (Codex fails the request unless that turn is the active one) and
`input` (a list of `UserInput`); the response is `{turnId}`. It was read from the installed
0.166.0 generated schema. Ask `supports_turn_steer` on the connected Codex's own inventory;
do not infer it from the version string. Codex 0.164 and 0.165 were not re-generated for it.
The adapter does not send the optional `clientUserMessageId` yet. It builds the request and
parses the response, but the caller must match the response `id` to the request it sent.

Required server request methods from generated `ServerRequest.oneOf`:

| Method | Params definition |
| --- | --- |
| `item/commandExecution/requestApproval` | `CommandExecutionRequestApprovalParams` |
| `item/fileChange/requestApproval` | `FileChangeRequestApprovalParams` |
| `item/permissions/requestApproval` | `PermissionsRequestApprovalParams` |
| `item/tool/requestUserInput` | `ToolRequestUserInputParams` |

Read thread metadata with `includeTurns: false`; full-history hydration is
deprecated for paginated threads. Preserve pagination cursors and stable native
IDs when reading turns/items. Never infer complete history from one page.
Request `itemsView: "full"` and ascending order for turn replay; the default
summary view does not provide complete history. Interruption acknowledgement is
not turn completion. `serverRequest/resolved` invalidates its exact pending ID.
Codex 0.164 rejects `thread/turns/list` on a thread before its first user
message because the thread is not materialized yet. Mark a newly created,
untouched thread as complete empty history without requesting a page. On an
exact resume, treat only that exact provider error as empty when no turn
submission exists and the binding proves no first user message was sent. Do not
convert other RPC errors or uncertain submissions into empty history.
Pinned source includes experimental `thread/resume` history/path alternatives
that are absent from the stable generated schema. The later adapter uses the
exact `threadId` and explicit preserved settings, never alternate history, an
implicit fork or latest.

Protocol reference:
[`codex-rs/app-server/README.md`](https://github.com/openai/codex/blob/fe3b29455488c2e189e43ad5526a107900c5e1b7/codex-rs/app-server/README.md)
and
[`codex-rs/app-server-protocol/src/protocol/`](https://github.com/openai/codex/tree/fe3b29455488c2e189e43ad5526a107900c5e1b7/codex-rs/app-server-protocol/src/protocol).

## Project-specific: native identity and configuration

Use `codex resume <SESSION_ID>` with the exact native UUID. Never use the picker,
`--last`, session names or a working-directory guess for a transfer. `--all`
changes picker filtering, not session identity; `fork` creates a new identity and
is not resume. Resolve and verify the same installed executable and exact config
root on each side. Use native `resume --no-daemon <SESSION_ID>` to avoid shared
Codex daemon discovery/startup; it cannot be combined with `--remote`. This
selects the native CLI's embedded runtime, not an ownership transfer protocol.
`CODEX_HOME` selects the Codex home (default `~/.codex`), whose
`config.toml` and credential storage participate in effective configuration;
an explicit root must already exist and is canonicalized. In this version,
native `-p`/`--profile` layers `$CODEX_HOME/<name>.config.toml`; it does not
independently isolate an account. App-server has no corresponding `-p` flag:
preserve effective configuration explicitly rather than copying native flags.
Native profile-v2 handoff is unsupported until a verified configuration mapping
exists; `-c profile=...` is not evidence of equivalence to the native overlay.
Preserve explicit `-c key=value` overrides and the effective login/provider
selection. Do not copy credentials into conversation identity or fixtures.
Account sameness was not inspected against a real login; future integration must
verify it without treating a profile name or config path as proof.

Exact source references: native options in
[`tui/src/cli.rs`](https://github.com/openai/codex/blob/fe3b29455488c2e189e43ad5526a107900c5e1b7/codex-rs/tui/src/cli.rs),
embedded/shared selection in
[`tui/src/daemon_startup.rs`](https://github.com/openai/codex/blob/fe3b29455488c2e189e43ad5526a107900c5e1b7/codex-rs/tui/src/daemon_startup.rs),
root loading in
[`core/src/config/mod.rs`](https://github.com/openai/codex/blob/fe3b29455488c2e189e43ad5526a107900c5e1b7/codex-rs/core/src/config/mod.rs)
and profile command selection in
[`cli/src/main.rs`](https://github.com/openai/codex/blob/fe3b29455488c2e189e43ad5526a107900c5e1b7/codex-rs/cli/src/main.rs).

## Project-specific: fixture boundary

`crates/maitty-core/src/daemon/conversation/codex.rs` and its sibling `fixtures/` contain a test-only
compatibility contract. The fixtures are hand-authored synthetic representations
of the inspected method/params-reference schema subset, not generated schemas or
captured sessions. Tests run offline with no provider subprocess or session I/O.
The checker proves required method coverage for the pinned baseline, not arbitrary
JSON Schema validation or runtime protocol support. Production adapter transport,
field validation and live handoff remain later phase-32 work.

## Project-specific: transport foundation

`daemon/conversation/transport.rs` preflights the exact selected executable with
`--version` and `app-server generate-json-schema --out` in an isolated temporary
Codex home. It normalizes generated `ClientRequest.json` and `ServerRequest.json`
into the compatibility inventory. Schema generation never opens or resumes a
conversation. App-server launch uses an argument array, the binding's existing
configuration root/cwd, and the caller's effective environment. Environment
values and provider payloads are never diagnostic text. Refuse profile-v2
bindings and conflicting `CODEX_HOME` before spawning.

The background actor sends initialize ID integer `1`, validates that exact
response and canonical `codexHome`, then sends `initialized` once. An integer
ID and a string containing its digits are different IDs. The framing limit is
1 MiB per newline-delimited UTF-8 frame; daemon events use bounded channels.
Drain stderr in fixed 8 KiB chunks without retaining its contents. Preflight
commands time out after 20 seconds each; initialization times out after 10
seconds. EOF, process exit, invalid envelopes, oversized frames and unsolicited
responses fail visibly without retrying any mutation.

Native server notifications include optional `emittedAtMs` (signed 64-bit,
nullable); accept it only on notifications and never use it to order history.
The actor reads `config/read` with explicit binding cwd and `includeLayers:false`
on that same connection before thread admission. Absent/null shell policy options
use the verified native defaults. Require unfiltered all-environment inheritance,
no custom set/profile/filter policy and no reserved identity conflicts. Freeze
the validated value-free shell policy in start/resume request overrides and append
reserved identity names to each enabled stdio MCP `env_vars`, preserving existing
names. Unsupported server-key/allowlist shapes fail before creating a thread.
No secret value enters request config; HTTP MCP configuration is unchanged.

The credential uses `_CONVERSATION_AUTH`, which survives native default
KEY/SECRET/TOKEN exclusions. Pinned source for these contracts is commit
`6aaac694803004a63c6f393d153c96d22521a6b9` behind the inspected annotated tag:
`config/src/shell_environment_policy.rs`, `rmcp-client/src/utils.rs`,
`app-server/src/config_manager_service.rs`, `app-server/src/config_manager.rs`
and `app-server-protocol/src/protocol/common.rs`.

Explicit disposal kills the owned process group on Unix, or the checked
kill-on-close Job Object on Windows, then asynchronously reaps the root.
Windows starts suspended, assigns the job before resuming its checked primary
thread, and refuses initialization if containment fails. Unix group disposal
is not a guarantee of descendant cleanup after a daemon crash.
Startup, preflight and runtime cleanup failures retain the connection's registry
fence and any owned process guard. Release ownership only after confirmed
cleanup. Retry disposal only on an explicit request; a failed cleanup never
automatically retries initialization or a provider mutation. If a failed launch
task leaves process exit unconfirmed, keep the fence instead of guessing.

Production structured CLI admission uses this actor and transport. Synthetic
fixtures exercise real subprocesses and local IPC, including a descendant spawned
before initialize. There is no production fake-provider/debug route. Explicit
start is required. Command execution, file-change, permission and user-input
requests are answered only from the exact outstanding daemon-owned request ID.
File-change decisions wait for the patch notification so the GUI shows the
proposed changes. User-input answers are validated against the question options
and sent only to the live provider request. Permission requests show the exact
requested profile and can grant only that profile for the current turn; no
session-wide grant is offered.
Cancellation uses the exact acknowledged active turn ID on the owning runtime;
the interrupt acknowledgement is not turn completion. A structured chat stays a
chat: there is no transfer to the native Codex terminal and back.

Exact identity note: the binding's generic `session_id` field means Codex
`Thread.id`, the exact native UUID accepted as RPC `threadId` and native resume
ID. `Thread.sessionId` is the shared session-tree root and is not an exact fork
identity. Event history filters use the exact `threadId`. Start/resume response
validation binds `Thread.id`, effective cwd/model/provider, user approval reviewer
and the requested sandbox before any turn can submit. Stable resume sends
`threadId` and `excludeTurns:true`, never experimental history/path fields.
Native history is paged ascending with full turn items and explicit completeness.
