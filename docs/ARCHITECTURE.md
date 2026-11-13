# Architecture

This doc covers how the code is organised, which way dependencies point, and
which design patterns to use. `AGENTS.md` is the operating contract; this doc
holds the architecture rules in depth.

## Principles

- **Dependencies point inward.** Stable, pure code sits at the centre. Volatile
  code â€” UI, frameworks, databases, networks â€” sits at the edge. Inner code never
  imports outer code.
- **Organise by feature, then by layer.** A feature's UI, use cases, types, and
  data access live together. Code becomes shared only when a second real
  consumer appears.
- **Boundaries are explicit.** Data crossing a boundary is validated and mapped
  into the receiving side's types. Row shapes, HTTP payloads, and SDK types never
  leak inward.
- **Side effects live at the edges.** Domain logic takes values and returns
  values. I/O happens in adapters that the outer layers call.
- **Choose the simplest structure that holds.** Add a layer, interface, or
  package only when it removes real complexity or has a current second
  consumer.

## Layers

```text
presentation     UI, HTTP handlers, CLI, IPC entry points
  -> application   use cases / services: orchestrate one user intent
    -> domain        entities, value objects, rules; pure and framework-free
application -> ports        interfaces the application needs (repositories, clock, mailer)
adapters    -> implement ports against real infrastructure (DB, HTTP, filesystem, SDKs)
```

- **Presentation** parses input, calls one use case, and renders the result. It
  holds no business rules and does no direct data access.
- **Application** coordinates domain objects and ports to carry out a single
  intent, such as `createInvoice` or `archiveProject`. It owns transactions and
  authorization checks.
- **Domain** holds the rules that would stay the same if the framework changed.
  It has no I/O, no framework imports, and no reads of global state.
- **Ports** are interfaces owned by the application layer. **Adapters** implement
  them, so infrastructure can be swapped or faked in tests.
- **The composition root** is the one place that constructs adapters and wires
  them into services, such as `main`, the app bootstrap, or the DI container. No
  other code calls `new` on an infrastructure class.

A small project can merge application and domain, or skip ports where only one
implementation will ever exist. When you do, say so in *Project-specific:
layers*. Never reverse the direction.

## Where code lives (default)

```text
src/
  main.rs        argument parsing, picks client / daemon / CLI role
  cli.rs         one-shot subcommands (send, read, wait, hook, allow, doctor, â€¦)
  cli/target.rs  which machine a one-shot command talks to and which pane it means: `PaneAddress` (`4`, `build/4`), `resolve_machine` (`local` or a configured name), `Target` (the machine, whether it was named, whether `$MAITTY_TERM_ID` is its pane, how its failures and replies are shown); the flags that choose it are in ENVIRONMENT "The command line on a machine"
  cli/doctor_machines.rs  the machines section of `doctor`: per machine, reachable, app version, protocol, login PATH and ssh options (a custom transport: the protocol check and its program's name), each one line, all machines at once within one budget each (`machine_probe`, `link_failure`; nothing from a machine is shown uncleaned)
  cli/machines_ls.rs  `ls --all`: this computer and every configured machine asked at the same time, one that does not answer reported on its own line
  cli/mcp/machines.rs  the MCP tools' `machine` argument and `machine/id` ids: routing, the `mcp` opt-in, the calling pane's grants checked here before a machine is dialled, typed errors (`McpError`) and the cleaned, cut, `[machine: name]`-led replies (SECURITY "MCP across machines")
  tidy.rs        what removing a worktree would lose (shared by the close-time removal and the clean-up list)
  snapshot.rs    a checkout's files kept before each agent turn, and put back (the server takes them, the client lists and restores)
  references.rs  a project's reference files (`.maitty/references/`): listed, added and removed (the terminal app edits them, the server lists them for the desktop app)
  prompt.rs      the question and numbered choices an agent is stopped on, read off its screen (the MCP answer tool and the inbox card share it)
  output_line.rs  one line of a check's output: made safe to show, and read as pass or failure (the server picks the first failing line, the client colours with the same rule)
  ipc.rs         local sockets and the `proxy` command; opening a connection to another machine (ssh or a custom transport) is `maitty-machines`, which this module re-exports
  remote_paste.rs (now `maitty-machines`' `remote_paste`, re-exported here) sending a pasted image or a file to another machine ([ADR 0007](adr/0007-multiple-machines.md)): the file name, the check of the path the far side prints, the failure words; the script itself is also in `maitty-machines`, as is `remote_send` (the upload) and `ssh_run` (the runner that counts bytes and takes a cancel flag; the desktop app uses them too)
  remote_install.rs installing the app on another machine, as pure rules: the `uname -sm` parser, the installed-version check, the decision table (nothing is installed without a yes), the destination, the install script's exit-code table (the script itself is in `maitty-machines`)
  machine_probe.rs (now `maitty-machines`' `machine_probe`, re-exported here) a look at a machine before it is added: `ssh ... maitty --version` with no prompt, ended by a limit or a cancel flag, and its result classified (ready, a failure class, cancelled); `locate` asks the same way where the login shell finds the app (`command -v`)
  config.rs      config.toml (+ config.local.toml) loading, atomic state files
  keys.rs        actions, default bindings, key parsing
  theme.rs       built-in themes and contrast audit
  clock.rs       time helpers, local clock times
  proc.rs        running other programs: no console window, output or error as text,
                 git, shell command lines
  daemon/        the server
    mod.rs       the event loop, clients, snapshot, spawning panes
    commands.rs  commands from clients and the CLI
    status.rs    hook reports (checked on their own thread) and screen detection
    worktrees.rs making and trusting worktrees, spares, hooks, cleanup
    warm.rs      an agent a window asked for ahead of its first message: hidden until its session is started
    restore.rs   saving sessions and recovering agents left asleep by older versions
    provider_groups.rs  the process groups of running providers, on disk, and the start-up sweep of those a killed daemon left (Linux, macOS)
    repo.rs      read-only answers about the repository a pane is in (`Query::Repo`), and the one snapshot restore
    search.rs    find across sessions: text in what each pane printed recently
    check.rs     a pane's check result, and the runner: the project's check command run in the session's worktree when a turn ends
    check_source.rs which checks a project has (yours, else the repo's .maitty/checks.toml), where each comes from, whether it may run
    check_stale.rs  a pass gone stale: the worktree's tree when a run starts, compared on the git poll
    closed_sessions.rs the agent sessions that closed for good or crashed, newest 50 (closed-sessions-<socket>.json): what a close or a failed exit records, read at start and written off the loop
    reopen.rs    reopening a closed session: its worktree made again when the folder is gone (recorded path, branch and repository only), then its conversation resumed in a new session
    conversation/allowed.rs  commands you approved "always in this project" for a chat (chat-allowed-commands.json)
    helpers.rs   sessions starting helpers: the ask / always / never rule, the request a session's row shows, and the projects you said Always for (helper-projects.json)
    identity.rs  the daemon's random id, made on first start and kept in the data folder (daemon-id); sent in every snapshot
    secrets.rs   secrets agents ask you for: the pending requests, and what you typed, held in memory as single-use references (ADR 0006)
    pr.rs        each pane's pull request read through `gh`: the poll, the answer parsed, what changed between two readings, and the message queued for the agent
    approvals.rs your Always / Never / Once for repo-supplied check commands (kept in Maitty's data folder, read at start), and the ApproveCheck / ForgetCheckApproval commands
    term.rs      one pane: PTY, input thread, vt100 emulator
    scan.rs, git.rs, persist.rs
  client/        the UI
    mod.rs       App state, the event loop, shared types
    input.rs     keys, pastes, the mouse
    actions.rs   what each action does
    background.rs server messages and background results
    ids.rs       client-only ids: ClientSession, and MachineSnap / Remote (each machine's name, last snapshot and connection state, and when its line went down, so limits, spend, version and the active workspace stay per machine); never on the wire (`MachineId`, `ClientTerm` and the machine-addressed `Outgoing` are in `maitty-machines`, and re-exported here)
    machine_config.rs where this computer's `config.local.toml` is; the `[[machines]]` entry edits (append, remove, the name suggested for a target) are in `maitty-machines`, and re-exported here
    machine_flow.rs what the Machines dialog does with a choice: probe, popup login, start/stop a supervisor, retry, remove (tests in `machine_flow/tests.rs`)
    machine_links.rs what the window does with the supervisors' events: register lines, the toasts, counting typing a down machine dropped
    remote_policy_tests.rs what the window does with each verdict against a built `App` (the policy itself, `remote_policy` in `maitty-machines`: one verdict per `ServerMsg` variant (apply, answer only to a request this window sent, gated by the machine's opt-ins, or ignore) and the cleaning of what is let through; this computer's own daemon bypasses it)
    routing.rs   which machine a message goes to and what a pane's machine lets this window do: the focus machine, `target_machine()`, the unanswered-request count behind reply bookkeeping, the local-only `Feature` gate and `local_cwd`, and `deliver` (the down-machine toast); the table is in API_PATTERNS "client routing"
    remote_snapshot_tests.rs the snapshot cleaning's tests, on a snapshot built from the reference fixture (the cleaning itself, `remote_snapshot` in `maitty-machines`: strings cleaned and cut, lists capped, sizes clamped, by walking the serialized form)
    cli_machine_tests.rs the one-shot commands against a real daemon behind the fake-ssh shim, addressed by configured name (new, split, focus, popup, close, send, wait, ls, `ls --all`, kill-server, the explicit-pane rule)
    doctor_machine_tests.rs `doctor`'s machine checks against the shim: a good machine, another protocol, a login needed, machines that never answer and are ended
    mcp_machine_tests.rs the MCP tools against a real daemon behind the shim: list, read, send, wait with `machine/id` ids, the opt-in, the caller's grants, local-only operations
    machine_ids.rs which file the daemon ids are kept in (`machine-ids.json` in the data folder); the store and the check that refuses a different id are `maitty-machines`' `machine_ids`
    ssh_hosts.rs loads the named hosts in `~/.ssh/config` off the UI thread; the pure parser (Include followed, patterns skipped, read through a file trait) is `maitty-machines`' `ssh_hosts`; the Machines dialog lists its hosts (ADR 0007)
    routing_tests.rs which machine each outgoing message goes to, the reply bookkeeping and the local-only gate
    remote_status_tests.rs a remote agent's row is the status its own daemon reported, against a real daemon behind the shim
    remote_image.rs sends the clipboard image to the machine a pane is on over its ssh line and gives the path it is kept at there; `remote_image_tests.rs` runs it against the fake remote
    machine_update_tests.rs `machine update` against the shim's `remote-sh` mode (the real install script on a temporary filesystem)
    screen/flat_tests.rs the flat sidebar, the collapsed column, the status glyphs and the workspace frames; screen/machines_tests.rs frames of the Machines dialog
    view_keys.rs keys inside full-pane views and the remaining popups
    screen/      the layout: mod.rs (state, model, click targets), draw.rs (grid, the sidebar overlay, the toast),
                 workspace.rs (the lone pane's framed border, a split's lines and pane headers), side.rs (the flat sidebar: one row per pane, in id order), rail.rs (the 10-column collapsed sidebar), status_mark.rs (the status glyph and spinner pace), chips.rs (the chips on a border), card.rs (cards, pills, fading; the Look from
                 settings), leader.rs (key map, actions list, new tab), popups.rs,
                 machine_group.rs (a machine's label, state mark and whether its line is down),
                 machines.rs (the Machines dialog: view, keys, drawing),
                 dialogs.rs, glyphs.rs (session and check marks), splash.rs, popover.rs (follow-up and why, beside a row), heads.rs (same-file heads-ups,
                 both diffs), sheet.rs (the card docked right of the panes:
                 Inbox, Changes), inbox.rs (the Inbox in it), behaviour.rs (its keys and clicks)
    render.rs, design.rs  drawing entry point and shared helpers
    themes.rs    searchable theme browser, previews, background save
    menu.rs, views.rs, files.rs, find.rs, branch.rs, recipes.rs, overlap.rs,
    markdown.rs (markdown drawn for the Files preview: text, width, theme in; styled lines out),
    motion.rs (slides and glides), â€¦
    tests.rs     rendering and behaviour tests
  installed_executable.rs, version.rs, project.rs, gitfs.rs, alert.rs, reveal.rs,
  restart.rs, sync.rs, cli/mcp.rs, cli/update.rs, cli/reveal.rs, cli/restart.rs,
  cli/machine_update.rs (`machine update <name>`: probe, decide, send, swap, ask the daemon; with `machine_update/binary.rs` for this computer's own build or a fetched, verified release; its ssh line runs through `maitty-machines`' `ssh_run`)
  secret_files.rs  a secret handed to an agent as a private file, and clearing the stale ones
  login_path.rs  the daemon host's login-shell PATH, resolved once at start and applied to panes, providers and lookups
  shim.rs        the per-agent launchers on a shell pane's PATH, and the PATH code `maitty agent-launch` shares
docs/            these docs, the roadmap, design briefs
config.example.toml
```

Features never import from inside another feature. If feature A needs
something from feature B, B exposes it through its public entry file, or the
shared piece moves to `domain/` or `lib/`.

## Pattern catalog

Use a pattern when its trigger applies. A pattern with no trigger is only
ceremony.

| Pattern | Use when | Avoid when |
| --- | --- | --- |
| **Repository** | a use case needs to load or save aggregates without knowing storage | it only forwards to an ORM that is already a repository |
| **Port / Adapter** | an external system (DB, API, clock, filesystem) must be swappable or faked | only one implementation will exist and tests do not need a fake |
| **Service / Use case** | an operation coordinates several steps, rules, or ports | it only forwards a call. Call the repository directly instead. |
| **Value object** | a primitive carries rules (email, money, id, date range) | the value has no invariant |
| **Result / typed error** | a failure is expected and the caller must handle it | the failure is a bug. Throw and let it surface. |
| **Strategy** | behaviour varies by a known set of cases chosen at runtime | there are two cases and an `if` is clearer |
| **Factory** | construction has rules, defaults, or picks among implementations | a constructor or object literal is enough |
| **Dependency injection** (constructor or parameter) | a unit needs collaborators that tests must control | the dependency is a pure function. Import it. |
| **Domain events** | several independent reactions follow one state change | there is one reaction. Call it directly. |
| **Mapper** | data crosses a boundary with a different shape (row â†’ entity, entity â†’ DTO) | the shapes are identical and will stay so |

### Anti-patterns

- God services or "manager" classes that own unrelated use cases.
- Generic `BaseRepository<T>` or `CrudService<T>` built before two concrete
  repositories need it.
- Service locators, or globals reached from deep inside logic.
- Layers that only forward calls and add nothing.
- Business rules in UI components, HTTP handlers, SQL, or ORM hooks.
- Circular imports between features or layers.

## Cross-cutting concerns

- **Configuration** is read and validated once at startup into a typed object,
  then passed in. Nothing reads environment variables directly outside the
  config module. See [ENVIRONMENT](ENVIRONMENT.md).
- **Logging** is structured, carries a correlation or request id, and never
  includes secrets or personal data.
- **Errors** are typed at the domain and application layers. The presentation
  layer maps them to user-safe output. See [API_PATTERNS](API_PATTERNS.md).
- **Authorization** is checked in the application layer. Checks in the UI alone
  never count.
- **Time and ids** come from injected providers wherever determinism matters.

## Recording decisions

Record a decision that is expensive to reverse in `docs/adr/`, then summarise it
in the relevant project-specific section below with a link to the ADR. Examples
include the architecture style, framework, persistence engine, sync model, and
monorepo split.

## Project-specific: system context

The terminal `maitty` binary runs as a daemon or as a client:

- **Daemon (server):** started on demand, one per user (or per `MAITTY_SOCKET`
  name). Owns every pane: spawns shells and agents in pseudo-terminals
  (ConPTY on Windows), parses their output with `vt100`, detects agent status,
  manages worktrees, persists sessions, and restores them after a restart.
- **Client (UI):** the full-screen TUI. Connects over a local socket (named pipe
  on Windows, Unix socket elsewhere), receives snapshots and pane output, sends
  commands and keystrokes. Many clients can attach; any can detach.
- **CLI subcommands** (`maitty send`, `maitty read`, `maitty wait`, `maitty worktree`,
  `maitty handoff`, `maitty doctor`, `maitty mcp`, `maitty proxy`, â€¦) talk to the same daemon.

The optional desktop GUI is a separate Tauri application. It connects to the same
daemon through the local protocol and does not link the terminal runtime. The
daemon remains the owner of sessions, conversations, persistence and agent
processes.

External systems: the agent CLIs it runs (claude, codex, â€¦) and their hook
callbacks (`maitty hook`), `git` and `gh`, `ssh`
for remote machines (`--remote host` runs `ssh host maitty proxy`), the OS
clipboard and notifications, and an MCP server (`maitty mcp`) agents use to talk to
each other.

## Project-specific: stack

The shared protocol, conversation contracts, split-tree types, clock helpers,
hidden-text validation and check-output classification live in the
`maitty-shared` workspace package. `maitty-settings` owns UI, session, restore,
worktree, MCP, check, recipe, preset, quick-agent and agent config schemas,
icon, key, detection, snapshot and notification preference schemas and
defaults, pure validation and appearance compatibility rules, terminal key actions,
default bindings, key parsing and built-in themes; terminal-specific mouse
encoding stays in the terminal client. Config loading and filesystem migrations
are core-owned. `maitty-machines` owns the machine definition (`[[machines]]`) and its
validation (`maitty-settings` re-exports them from `config`), the ssh line and its hardening
(`ssh_command.rs`), the failure classes and their fix texts (`link_failure.rs`), the fixed remote
scripts and the dial to another machine (`machine.rs`), the connection registry (`connections.rs`: one
bounded reader and writer task per machine, one fair intake, sends routed by machine, a lost connection
marked and never ending the window), the connection state machine and its supervisors (`link/`: `Links`,
`LinkControl`, the pure state table and backoff) and the ids `MachineId`, `ClientTerm` and `Outgoing`, for both apps; `maitty-core` re-exports
them as `ipc::Machine` and `link_failure`.
`maitty-ipc-transport` owns bounded,
length-delimited protocol framing, Welcome handshake, the stream marker and login-noise
skipping for ssh (`stream_marker.rs`), socket identity formatting,
private Unix socket-directory checks and platform socket-name conversion shared
by the TUI and GUI; socket connection and daemon startup stay in their respective
clients. The Tauri host
uses `maitty-desktop-client`
for daemon IPC and GUI operations; it does not link `maitty-terminal-client`. The
`maitty-agent-detection` package owns the bundled agent-screen matcher used by
daemon status; process-tree and lifecycle status remain daemon-owned. The
`maitty-agent-integrations` package owns the catalog of agents with native
status integrations; config discovery and installation remain terminal-client
owned. The `apps/tui` `maitty` package preserves the installed command and re-exports
the terminal client library for compatibility. Codex compatibility preflight,
session metadata, usage extraction, effective-config policy, native hook
configuration projection and completion-event mapping live in
`maitty-provider-codex`;
Claude stream parsing, version checks, session IDs, launch arguments, frame
limit, hook event definitions, command-group projection and replacement,
ownership, stale-hook projection, native event mapping and status-line config
policy live in `maitty-provider-claude`. JSONC configuration editing remains in
the terminal client; live process transports and session ownership are in core.
Completing the package split is tracked by
[ADR 0004](adr/0004-client-split-and-desktop-host.md).

Rust, edition 2024, stable toolchain. Key libraries: `tokio` (async runtime),
`ratatui` 0.30 + `crossterm` (TUI), `portable-pty` (PTYs / ConPTY), `vt100`
(terminal emulation), `interprocess` (local sockets), `rmp-serde` (msgpack wire
format) over `tokio-util` length-delimited frames, `serde`/`toml`/`toml_edit`
(config), `sysinfo` (process trees, memory), `ignore` + `regex` (file search),
`arboard` (clipboard), `clap` (CLI), `anyhow` (errors), `tracing` (logs).
`jsonc-parser` 0.33.1 with CST/serde support edits OpenCode's native JSONC plugin
lists while retaining comments and settings; serde_json alone cannot preserve them.
`vt100` is a patched copy in `vendor/vt100` (see its `MAITTY.md`; wired in with
`[patch.crates-io]`): keep its changes there and listed in that file.
Rules for the language live in `docs/stack/rust.md`.
The optional desktop client uses Tauri 2, React, TypeScript, Vite, shadcn/ui
and Tailwind; its rules live in `docs/stack/tauri.md`, `typescript.md`,
`react.md` and `shadcn-tailwind.md`.

## Project-specific: layers

- **Core:** `maitty-core` owns the headless daemon, PTY/process lifetime,
  filesystem and config I/O, persistence, provider process transports, and
  daemon-owned conversation state. It depends on shared contracts and adapters,
  never on a terminal or desktop client.
- **Terminal client:** `maitty-terminal-client` owns the existing CLI and TUI,
  including terminal rendering, input, command parsing, and terminal-only
  integrations. It calls the daemon through the shared IPC contract.
- **Desktop client:** `maitty-desktop-client` owns desktop IPC and GUI operations.
  The Tauri host consumes it and does not link `maitty-core` or the terminal
  client.
- **Shared packages:** `maitty-shared`, `maitty-settings`, `maitty-machines`, and
  `maitty-ipc-transport` hold the wire types, preferences, and transport framing
  used across those boundaries. Provider packages own provider-specific
  protocol mapping; core owns their daemon-managed processes and sessions.

The daemon owns every session and PTY. Both clients communicate with it over
IPC, so closing either UI cannot end an agent session.

## Project-specific: directory layout

```text
apps/
  tui/                            installed `maitty` command and TUI entry point; `examples/fake-ssh.rs` stands in for ssh in tests and `tests/fake_ssh.rs` drives the remote path through it (TESTING)
  gui/                            Tauri host and React interface
crates/
  maitty-core/                    daemon, PTYs, persistence and process lifetime
  maitty-terminal-client/         terminal CLI and TUI implementation
  maitty-desktop-client/          desktop IPC client
  maitty-shared/                  wire protocol and shared contracts
  maitty-settings/                preference schemas, key actions and themes
  maitty-ipc-transport/           shared IPC framing and handshake
  maitty-machines/                the `[[machines]]` definition and its checks and edits, the ssh line, failure classes, dial, connection registry and link supervisors, the cleaning and policy for what another machine sends, the daemon-id store, the `~/.ssh/config` parser, the reader of this computer's machine list (`local_machines`: `config.local.toml`, as both apps read it), and the atomic state-file helpers (`write_atomic`, `read_state`, which `maitty-core` re-exports), shared by both apps
  maitty-highlight/               syntax highlighting for file previews, shared by both apps (STACK: syntect)
  maitty-test-support/            test-only helpers shared across crates: building and locating the fake-ssh shim, running a command with a time limit
  adapters/                       Codex, Claude and future provider adapters
scripts/                          repository checks, e.g. `check-gui-deps.sh` (the desktop app's dependency rule)
docs/                             architecture, roadmap and design specs
```

## Project-specific: patterns and conventions

**The app's name** lives in `apps/tui/Cargo.toml` and nowhere else in the code (ADR 0008): the package
name is the command, `[package.metadata.brand]` holds the name people see and the names it had
before. `build.rs` hands them to the code as `APP_COMMAND`, `APP_NAME`, `APP_ENV` and
`APP_PREVIOUS`; `crates/maitty-core/src/brand.rs` wraps them. Write a
name-bearing string as
`concat!(".", env!("APP_COMMAND"), ".toml")`, or name `APP_NAME` in a format string with
arguments of its own. Adapters in `crates/maitty-terminal-client/src/cli/*.js` are
templates (`__APP_NAME__`,
`__APP_COMMAND__`, `__APP_ENV__`, `__APP_EXE__`) filled in when installed. `tests/naming.rs`
fails on the name written out. To rename the app:

1. In `apps/tui/Cargo.toml`, set the package `name` (the command) and `[package.metadata.brand] name`,
   and put the old command first in `previous`. Build once and commit `Cargo.lock` too (it
   records the package name; releases build `--locked`).
2. In `install.sh` and `install.ps1`, set `APP` / `$app` and the repository (a test checks them).
3. Replace the name in the reference frames by hand (they show it; see
   [TESTING](TESTING.md#project-specific-layout-and-fixtures)).
4. Replace the name in the docs (not the ADRs), then rename the GitHub repository.

The release version also comes from `apps/tui/Cargo.toml`. Build scripts expose it as
`APP_VERSION` to the TUI and daemon crates. Use `APP_VERSION` for the app's version output,
update checks, daemon identity and provider metadata; `CARGO_PKG_VERSION` identifies an internal
crate and may differ from the released app version.

`client/screen/keyoverlay.rs` is the one key overlay: the key map, the search over every
command and All keys, all built from `key_groups()` and the active keymap (`Action::Palette`
opens the search, `Action::Help` opens All keys). The Menu and tab renaming are in
`client/screen/leader.rs`. `docs/shortcuts.md` is documentation only; the app does not embed it.

**Agent launchers.** A plain shell pane (no command, not a popup) starts with a launcher per
agent first on its `PATH` (`shim.rs`, written to `<data>/shell/bin/`; only files maitty wrote are
replaced or removed): `claude` there runs `maitty agent-dir claude â€¦`, goes to the folder it
prints, and execs the next `claude` on `PATH` with the launcher folder taken off (a `.cmd`
calling `maitty agent-launch` on Windows). The agents are the built-in four plus the programs the quick
agents run (the first word of a plain-named `command`, so an agent named `work` that runs `claude` adds nothing), and only when `worktree.per_agent` is on. `daemon/term.rs` also defines a shell
function per agent (bash `--rcfile`, zsh `ZDOTDIR`, fish `-C`, PowerShell's startup script) and
keeps the launcher folder first on `PATH` before each prompt or command, since a manager like
mise puts its own shims in front. The function sets `MAITTY_AGENT_DIR_DONE` for the one command
so the launcher does not ask a second time. The daemon decides (`Command::AgentWorktree`): only
a pane with no agent yet, only in a git repo's main checkout, and a caller without the pane's
secret gets an empty answer.

`client/restart_prompt.rs` is the one screen outside the app loop: before a window opens, it
asks once to replace a running server of another version that has agents working or waiting (`main.rs` calls it). `restart.rs` owns the rest of the one-step update: the activity file a server publishes beside its pid file (`Activity`, so a newer maitty can decide without asking an older server anything), the plan for a mismatch (`plan_for_mismatch`), waiting for the old server's socket to close, and re-opening the window on the installed maitty (`relaunch`). The stop itself is `cli::kill_server`, which reaches servers of any version by process and only ever the server of this `MAITTY_SOCKET`.
offers to replace a running server of another version (`main.rs` calls it; the restart itself is
`cli::kill_server`, which reaches servers of any version).

`detection/` evaluates the pinned 22 Herdr screen manifests in
`vendor/herdr-detection/` using agent-owned screen regions, OSC fields, priorities
and nested gates. It performs no I/O. `daemon/status.rs` arbitrates that evidence
against trusted lifecycle reports; custom screen regex overrides remain supported.
Source provenance and redistribution terms are in the vendor NOTICE and LICENSE.

`cli/integrations.rs` installs Pi/OMP extensions, OpenCode server/TUI bridges,
Kilo's plugin and native Kimi/Mastracode hook configurations. OpenCode configuration
editing lives in `cli/opencode_config.rs` and preserves its V1-to-V2 migration.
Actions invokes installation through
`spawn_bg`; adapters send authenticated `maitty hook` reports and remain inert
outside Maitty. No provider-specific status adapter is needed.

- **Commands and snapshots.** The client sends `Command`s; the daemon answers
  with full `Snapshot`s plus per-pane output. The client derives its view model
  (`screen_model`) from the snapshot each frame instead of caching parallel state.
- **Modes.** Client UI state is one `Mode` enum (popups, sidebar focus, settings,
  â€¦); each mode has a `draw_*` and an `on_*_key` function.
- **Hit testing.** Drawing registers clickable rects (`hit(app, rect, ScreenHit::â€¦)`);
  mouse handling looks them up, so drawing and clicking can't drift.
  `client/screen/rail.rs` renders the collapsed sidebar (a narrow column) from the same session
  model and status glyphs; it shares focus, hit testing and scroll state.
- **Background work.** Slow work runs in `spawn_bg` and returns a `Bg` message.
- **Other programs** go through `proc.rs` (`proc::git`, `proc::run`, `proc::shell`,
  `proc::quiet`), never a hand-built `Command` with its own window flag or error parsing.
- **Filterable popups** use `query_list` + `list_row` (client/screen/popups.rs).
- **Cards and pills** come from `card.rs`: `card()` for every pane, the sidebar and popups
  (title in the border, âœ• on the right), `pill()` / `row_pill()` for tabs, buttons and
  selected rows. Use accent for focus, the working colour for work, red for needs-you,
  sky for leader mode and red for destructive actions.
- **Popups** use the shared `panel()` helper (a lit card with âœ•), `dim_all` behind, and
  `hints()` for the key line.


Decisions are the nine in [`adr/`](adr/README.md): the daemon owns every session (0001), the
terminal substrate and wire protocol (0002), status trust (0003), the client split (0004),
provider adapters (0005), secrets (0006), multiple machines (0007), Windows and the product
name (0008), and provider-reported usage (0009).

## Project-specific: planned structured conversations

[Phases 32â€“35](ROADMAP.md#32--shared-conversations-and-codex-handoff--active)
add daemon-owned structured agent runtimes and an optional chat GUI.
[The spec](features/chat-gui/spec.md) owns the target conversation/history/handoff
contract. Structured Codex CLI admission is implemented; the current terminal path remains supported.
Codex is first, Claude follows before release. Desktop framework selection and
final GUI design were selected in phases 35â€“37.

Structured provider integrations follow the provider-appropriate pattern
documented by [T3 Code's provider guide](research/t3code-provider-integrations.md):
use a provider's supported native runtime/protocol when it carries capabilities
Maitty needs; use ACP for compatible agents; add a thin provider-specific flavor
only for behavior ACP does not express. Normalize events and truthful capability
flags at the daemon adapter boundary. Keep each provider's identity, account and
configuration isolated, and preserve its native approval and session semantics.
Do not infer that another product's authentication choices are permitted for
Maitty. This rule guides future provider integrations; it does not expand the
current GUI release scope beyond Codex and Claude.

See [stack/claude-code-cli.md](stack/claude-code-cli.md) for the Claude CLI
adapter's authentication, lifecycle and platform rules.

The Codex transport foundation lives in
`crates/maitty-core/src/daemon/conversation/`: `codex.rs` checks
compatibility, `transport.rs` validates launch configuration and stdio
envelopes, `process.rs` owns platform process-tree cleanup, and `runtime.rs`
owns background connection actors. `daemon.rs` admits bindings from the daemon's
conversation registry and fences every incoming event by conversation,
generation and connection identity. Runtime ownership never belongs to a client
or PTY. Disposal remains pending until the root is reaped; disconnect does not
dispose it. Synthetic fixtures and real fake-provider child tests live beside it.
`commands.rs` owns authenticated start/send/read admission, operation receipts,
durability barriers and native paging. Actors correlate typed requests with exact
JSON-RPC IDs on the existing connection; closing a client leaves its runtime alive.
Only an explicit conversation start launches a selected provider.
The verified provider baseline is in [codex-app-server](stack/codex-app-server.md).

Durable conversation metadata lives in pure `crates/maitty-shared/src/conversation.rs`, separate from
`TermId`. The daemon keeps its registry independently of PTYs and saves it in the
same session snapshot as optional pane associations. Restore fences running and
transferring records as recovery-required and withholds linked panes from legacy
resume fallback. Unresolved creation intents retain a separate `PendingStart`;
they never fabricate a native identity or automatically create another thread.
Native/structured transfer commands remain later work.

The display-history foundation adds `history.rs` (typed projection and bounded
reads), `history_provider.rs` (Codex event normalization) and `history_storage.rs`
(private immutable pages and atomic manifests) beside the transport. Wire DTOs
live inward in `maitty-shared::conversation`. No transcript enters the ordinary
snapshot; attachment content is released on disconnect. See API_PATTERNS and
DATABASE for retention and durability contracts. One daemon canonical projection
owns cache writes; attachments only read. With output retention disabled, native
content lives only during hydration or in an attached view and drops on disconnect.

Session snapshots use capture-time tickets and one serialized writer per resolved
destination. Autosave, forced saves, shutdown and conversation barriers share it.
Only a confirmed durable commit advances `last_saved`; a stale or failed write
cannot authorize a provider mutation. The sidebar uses typed pane/conversation
keys and binds a real native terminal without manufacturing a PTY.

## Project-specific: optional desktop GUI

`apps/gui/` is an optional Tauri frontend and host. It communicates with the daemon
through the host's narrow typed command boundary and the daemon's authenticated
local IPC. The Rust daemon remains the sole owner of sessions, providers,
conversation identity and persistence. The GUI build must not become a
prerequisite for building or installing the CLI/TUI.
