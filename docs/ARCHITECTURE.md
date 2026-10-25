# Architecture

This doc covers how the code is organised, which way dependencies point, and
which design patterns to use. `AGENTS.md` is the operating contract; this doc
holds the architecture rules in depth.

## Principles

- **Dependencies point inward.** Stable, pure code sits at the centre. Volatile
  code — UI, frameworks, databases, networks — sits at the edge. Inner code never
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
  cli.rs         one-shot subcommands (send, read, wait, hook, allow, doctor, …)
  tidy.rs        what removing a worktree would lose (shared by the server's sweep and the clean-up list)
  snapshot.rs    a checkout's files kept before each agent turn, and put back (the server takes them, the client lists and restores)
  output_line.rs  one line of a check's output: made safe to show, and read as pass or failure (the server picks the first failing line, the client colours with the same rule)
  protocol.rs    wire messages and PROTOCOL_VERSION
  ipc.rs         local sockets and the SSH proxy
  config.rs      config.toml (+ config.local.toml) loading, atomic state files
  keys.rs        actions, default bindings, key parsing
  layout.rs      split trees and neighbour finding
  theme.rs       built-in themes and contrast audit
  clock.rs       time helpers, local clock times
  proc.rs        running other programs: no console window, output or error as text,
                 git, shell command lines
  daemon/        the server
    mod.rs       the event loop, clients, snapshot, spawning panes
    commands.rs  commands from clients and the CLI
    status.rs    hook reports (checked on their own thread) and screen detection
    worktrees.rs making and trusting worktrees, spares, hooks, cleanup
    restore.rs   saving sessions and recovering agents left asleep by older versions
    check.rs     a pane's check result, and the runner: the project's check command run in the session's worktree when a turn ends
    check_source.rs which checks a project has (yours, else the repo's .maitty/checks.toml), where each comes from, whether it may run
    check_stale.rs  a pass gone stale: the worktree's tree when a run starts, compared on the git poll
    helpers.rs   sessions starting helpers: the ask / always / never rule, the request a session's row shows, and the projects you said Always for (helper-projects.json)
    approvals.rs your Always / Never / Once for repo-supplied check commands (kept in Maitty's data folder, read at start), and the ApproveCheck / ForgetCheckApproval commands
    term.rs      one pane: PTY, input thread, vt100 emulator
    scan.rs, git.rs, persist.rs
  client/        the UI
    mod.rs       App state, the event loop, shared types
    input.rs     keys, pastes, the mouse
    actions.rs   what each action does
    background.rs server messages and background results
    view_keys.rs keys inside full-pane views and the remaining popups
    screen/      the layout: mod.rs (state, model, click targets), draw.rs (grid, pane cards
                 with the tabs in their title), side.rs (the sidebar card and its rows), card.rs (cards, pills, fading; the Look from
                 settings), leader.rs (key map, actions list, new tab), popups.rs,
                 dialogs.rs, glyphs.rs (session and check marks), splash.rs, popover.rs (follow-up and why, beside a row), heads.rs (same-file heads-ups,
                 both diffs), sheet.rs (the card docked right of the panes:
                 Inbox, Changes), inbox.rs (the Inbox in it), behaviour.rs (its keys and clicks)
    render.rs, design.rs  drawing entry point and shared helpers
    themes.rs    searchable theme browser, previews, background save
    menu.rs, views.rs, files.rs, find.rs, branch.rs, recipes.rs, overlap.rs,
    markdown.rs (markdown drawn for the Files preview: text, width, theme in; styled lines out),
    motion.rs (slides and glides), …
    tests.rs     rendering and behaviour tests
  mcp.rs, project.rs, gitfs.rs, alert.rs, reveal.rs, update.rs, restart.rs, sync.rs
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
| **Mapper** | data crosses a boundary with a different shape (row → entity, entity → DTO) | the shapes are identical and will stay so |

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

Maitty is one binary that runs in two roles:

- **Daemon (server):** started on demand, one per user (or per `MAITTY_SOCKET`
  name). Owns every pane: spawns shells and agents in pseudo-terminals
  (ConPTY on Windows), parses their output with `vt100`, detects agent status,
  manages worktrees, persists sessions, and restores them after a restart.
- **Client (UI):** the full-screen TUI. Connects over a local socket (named pipe
  on Windows, Unix socket elsewhere), receives snapshots and pane output, sends
  commands and keystrokes. Many clients can attach; any can detach.
- **CLI subcommands** (`maitty send`, `maitty read`, `maitty wait`, `maitty worktree`,
  `maitty handoff`, `maitty doctor`, `maitty mcp`, `maitty proxy`, …) talk to the same daemon.

External systems: the agent CLIs it runs (claude, codex, …) and their hook
callbacks (`maitty hook`), `git` and `gh`, `ssh`
for remote machines (`--remote host` runs `ssh host maitty proxy`), the OS
clipboard and notifications, and an MCP server (`maitty mcp`) agents use to talk to
each other.

## Project-specific: stack

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

## Project-specific: layers

- **Domain / shared types:** `protocol.rs` (every message between client and
  daemon), `layout.rs` (split trees), `keys.rs` (actions and bindings),
  `theme.rs`, `config.rs`. No I/O beyond reading config.
- **Infrastructure:** `ipc.rs` (sockets, SSH proxy), `daemon/term.rs` (PTY +
  emulator per pane), `daemon/scan.rs` (status detection), `daemon/git.rs`,
  `daemon/persist.rs`, `gitfs.rs`, `alert.rs`, `reveal.rs` (notification links,
  bringing the window forward), `update.rs`, `sync.rs`, `mcp.rs`,
  `daemon/usage.rs` (agents' usage, limits, continue after a limit).
- **Application:** `daemon/mod.rs` (the server loop: commands in, state and
  output out) and `client/mod.rs` (the client loop: events in, commands out).
- **Presentation:** `client/screen/` (the layout and its popups),
  `client/design.rs` (shared drawing helpers), `client/render.rs`, and the
  feature views in `client/` (files, changes, branches, …).

`client/spinners.rs` owns the working-spinner picker, previews and background
saves. Presets and frame validation live in `config.rs`; `ui.spinner` remains the
existing persisted array consumed by the shared status-icon renderer.

The client never touches a PTY; the daemon never draws. They meet only through
`protocol.rs` messages.

## Project-specific: directory layout

```text
src/
  main.rs        argument parsing, picks client / daemon / CLI role
  cli.rs         one-shot subcommands (send, read, wait, hook, doctor, …)
  cli/handoff.rs `maitty handoff`: the text brief, and the files a checkout changed
  cli/mcp_config.rs `maitty integrate mcp`: Maitty's entry in Codex, Qwen Code and Gemini CLI config files
  tidy.rs        what removing a worktree would lose (shared by the server's sweep and the clean-up list)
  snapshot.rs    a checkout's files kept before each agent turn, and put back (the server takes them, the client lists and restores)
  output_line.rs  one line of a check's output: made safe to show, and read as pass or failure (the server picks the first failing line, the client colours with the same rule)
  protocol.rs    wire messages and PROTOCOL_VERSION
  ipc.rs         local sockets and the SSH proxy
  config.rs      config.toml (+ config.local.toml) loading
  keys.rs        actions, default bindings, key parsing
  layout.rs      split trees and neighbour finding
  theme.rs       built-in themes and contrast audit
  daemon/        the server: mod.rs (loop), term.rs (PTY + vt100), scan.rs
                 (status), git.rs, persist.rs
  client/        the UI: mod.rs (loop, input), screen/ (layout), design.rs
                 (drawing helpers), render.rs, menu.rs, views.rs, and one file
                 per feature (files, find, branch, recipes, pr, …)
  mcp.rs, project.rs, gitfs.rs, alert.rs, reveal.rs, update.rs, restart.rs, sync.rs
docs/            these docs, the roadmap, design briefs
config.example.toml
```

## Project-specific: patterns and conventions

**The app's name** lives in `Cargo.toml` and nowhere else in the code (ADR 0019): the package
name is the command, `[package.metadata.brand]` holds the name people see and the names it had
before. `build.rs` hands them to the code as `APP_COMMAND`, `APP_NAME`, `APP_ENV` and
`APP_PREVIOUS`; `src/brand.rs` wraps them. Write a name-bearing string as
`concat!(".", env!("APP_COMMAND"), ".toml")`, or name `APP_NAME` in a format string with
arguments of its own. Adapters in `src/cli/*.js` are templates (`__APP_NAME__`,
`__APP_COMMAND__`, `__APP_ENV__`, `__APP_EXE__`) filled in when installed. `tests/naming.rs`
fails on the name written out. To rename the app:

1. In `Cargo.toml`, set the package `name` (the command) and `[package.metadata.brand] name`,
   and put the old command first in `previous`. Build once and commit `Cargo.lock` too (it
   records the package name; releases build `--locked`).
2. In `install.sh` and `install.ps1`, set `APP` / `$app` and the repository (a test checks them).
3. Replace the name in the reference frames by hand (they show it; see
   [TESTING](TESTING.md#project-specific-layout-and-fixtures)).
4. Replace the name in the docs (not the ADRs), then rename the GitHub repository.

`client/screen/keyoverlay.rs` is the one key overlay: the key map, the search over every
command and All keys, all built from `key_groups()` and the active keymap (`Action::Palette`
opens the search, `Action::Help` opens All keys). The Menu and tab renaming are in
`client/screen/leader.rs`. `docs/shortcuts.md` is documentation only; the app does not embed it.

**Agent launchers.** A plain shell pane (no command, not a popup) starts with a launcher per
agent first on its `PATH` (`shim.rs`, written to `<data>/shell/bin/`; only files maitty wrote are
replaced or removed): `claude` there runs `maitty agent-dir claude …`, goes to the folder it
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
  …); each mode has a `draw_*` and an `on_*_key` function.
- **Hit testing.** Drawing registers clickable rects (`hit(app, rect, ScreenHit::…)`);
  mouse handling looks them up, so drawing and clicking can't drift.
  `client/screen/rail.rs` renders the collapsed sidebar from the same session
  model and status glyphs; it shares focus, hit testing and scroll state.
- **Background work.** Slow work runs in `spawn_bg` and returns a `Bg` message.
- **Other programs** go through `proc.rs` (`proc::git`, `proc::run`, `proc::shell`,
  `proc::quiet`), never a hand-built `Command` with its own window flag or error parsing.
- **Filterable popups** use `query_list` + `list_row` (client/screen/popups.rs).
- **Cards and pills** come from `card.rs`: `card()` for every pane, the sidebar and popups
  (title in the border, ✕ on the right), `pill()` / `row_pill()` for tabs, buttons and
  selected rows. Use accent for focus, the working colour for work, red for needs-you,
  sky for leader mode and red for destructive actions.
- **Popups** use the shared `panel()` helper (a lit card with ✕), `dim_all` behind, and
  `hints()` for the key line.


Decisions: [0001 daemon owns sessions](adr/0001-daemon-owns-sessions.md),
[0002 portable-pty + vt100](adr/0002-portable-pty-and-vt100.md),
[0003 msgpack over local sockets](adr/0003-msgpack-over-local-sockets.md),
[0004 one layout](adr/0004-one-layout.md),
[0005 herdr-compatible keys](adr/0005-herdr-compatible-keys.md),
[0011 keep Maitty terminal-first](adr/0011-keep-seshi-terminal-first.md).
