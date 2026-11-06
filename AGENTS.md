# AGENTS.md

The operating contract for AI coding agents and human contributors. Read it before you plan or
edit anything. A nested `AGENTS.md` may add narrower rules for its directory; the closest file wins.

## Documentation map

Read the docs that match your change when the work touches their area, not all up front. Each
rule lives in exactly one doc; if two disagree, the doc that owns the topic wins. Report the
conflict.

| Doc | Read before | Owns |
| --- | --- | --- |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | adding modules, crossing layers | layers, dependency direction, where code lives |
| [`docs/CODE-STANDARDS.md`](docs/CODE-STANDARDS.md) | writing or reviewing code | naming, types, errors, **comments**, prohibited shortcuts |
| [`docs/TESTING.md`](docs/TESTING.md) | writing tests or fixing bugs | test levels, determinism, what to assert |
| [`docs/API_PATTERNS.md`](docs/API_PATTERNS.md) | touching IPC, services, client data flow | request flow, validation, response and error shapes |
| [`docs/DATABASE.md`](docs/DATABASE.md) | touching persistence or migrations | persistence rules, data safety |
| [`docs/SECURITY.md`](docs/SECURITY.md) | input, secrets, permissions, new dependencies | threat checklist, supply chain |
| [`docs/ENVIRONMENT.md`](docs/ENVIRONMENT.md) | adding config or env vars | config loading, local setup |
| [`docs/UI.md`](docs/UI.md) | changing any interface | component structure, UI state, accessibility |
| [`docs/DESIGN.md`](docs/DESIGN.md) | changing how the app looks | the current visual language |
| [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) | animations, timers, rendering, polling, output path | measured causes of lag, how to re-measure |
| [`docs/COMMIT-STANDARDS.md`](docs/COMMIT-STANDARDS.md) | committing, branching, opening a PR | message format, git safety |
| [`docs/stack/`](docs/stack/) | touching a technology with a stack doc | per-technology rules |
| [`docs/adr/`](docs/adr/README.md) | changing a recorded decision | the nine decisions that are expensive to reverse |
| the Plane project (see *Planning*) | starting, finishing, scoping work | what is in, out, and in what order |

## How the docs are structured

Rules come in three layers: the generic base, stack docs in `docs/stack/`, and
`## Project-specific: …` sections. Precedence, highest first: the user's instruction for the
task, the nearest nested `AGENTS.md`, project-specific sections, stack docs, the generic base.
An unfilled section holds a `<!-- FILL: … -->` comment: the project has not decided, so follow
the generic base and ask the user before inventing a rule (`grep -rnE "^<!-- FILL:" AGENTS.md CLAUDE.md docs/`
lists them). If following an instruction would break a non-negotiable, stop and say so.

## Maintaining these docs

The docs are part of the codebase. In the same change, update the doc that owns the topic when
you add or move a module, change a public contract, add a dependency, establish a convention, or
find a doc wrong. Write rules and current contracts, not history. Link to a rule; never copy it.
Add an ADR for a decision that is expensive to reverse, not for how something looks.

Ask the user first before changing or removing a rule in the generic base or a filled
project-specific section, or filling a `FILL` section with a decision the code does not show. Never
loosen a rule so a change passes, delete a `Project-specific` heading, or let this file and
`CLAUDE.md` drift apart.

A technology with no stack doc yet: stop before writing code that depends on it, and suggest
`/starter add <tech>` (or write `docs/stack/<tech>.md` by hand and record it in `docs/stack/README.md`).

## Planning

Scope and order live in **Plane**, not a repo file (see *Project-specific: planning*).

- Work only on the ticket you were given (`MAITTY-N`), claim it first (`plan-pull start`), and
  respect "blocked by" links.
- Mark a ticket Done only after its acceptance criteria are met and the verification gate passed
  in the real worktree; the `done` comment names the commands and results.
- File follow-ups as new Backlog tickets. Reordering, adding or cutting planned work is the
  user's decision; propose it.
- A bug fix needs no ticket or plan to start.

## Workflow

1. **Before changing code:** read this file and the docs your change touches; inspect the code and
   `git status`; find the existing pattern first; keep user-authored and unrelated changes; stop and
   ask if the request conflicts with the docs or is destructive.
2. **While changing code:** one concern per change; no speculative abstractions; reuse existing
   dependencies; never hand-edit generated files; never reformat code you did not change.
3. **Before declaring done:** run the verification gate. Never report a check as passing unless it
   ran and passed in this worktree.
4. **Handoff:** the outcome in plain language; important files changed; the commands you ran and
   their real results; docs updated; new dependencies, migrations or config; known limits and the
   next step. Separate verified facts from proposals.

## Non-negotiables

1. Follow the [core principles](docs/CODE-STANDARDS.md#core-principles): readability over
   cleverness, KISS, YAGNI, separation of concerns, DRY for knowledge, small pure functions,
   explicit over implicit. Named constants replace magic values.
2. Dependencies point inward. Domain logic never imports frameworks, I/O or UI.
3. Validate all external input at the boundary. Run the
   [security checklist](docs/SECURITY.md#per-change-checklist) on every change.
4. No secrets in source, fixtures, logs, snapshots or command output.
5. Never swallow an error: handle it, rethrow it with context, or convert it to a typed result.
6. Never weaken a type, lint rule or test to make a change pass.
7. Comments explain *why*, not *what* ([CODE-STANDARDS](docs/CODE-STANDARDS.md#comments)).
8. Commits follow [COMMIT-STANDARDS](docs/COMMIT-STANDARDS.md) and carry no AI or tool attribution.
9. Never run destructive git commands, rewrite history, commit, push, or change remote resources
   unless asked.

State assumptions early; when blocked, say what blocks you and never change the requirement to
route around it; when the docs and the request disagree, name the rule and ask.

## Project-specific: mission

Maitty is a terminal multiplexer built for running many coding agents at once
(Claude Code, Codex, Gemini, OpenCode, …): see which ones need you, jump to them,
and keep them running when you leave. It serves developers who run several agents
across several projects, on Windows, macOS and Linux. The one thing it must do
well: never lose or misreport an agent; status (working, needs you, done, idle)
has to be right, and sessions survive the UI closing.

## Project-specific: non-negotiables

1. **The daemon owns every session.** The client is a view; closing, crashing or
   detaching the client must never kill a pane. Anything that touches PTYs or
   process lifetime lives in `crates/maitty-core/src/daemon/`. Only one daemon runs per socket
   (ADR 0001).
2. **Bump `PROTOCOL_VERSION`** (`crates/maitty-shared/src/protocol.rs`) when a
   message shape changes incompatibly: a field removed, renamed or retyped, or a new enum variant. An
   added `#[serde(default)]` field needs no bump when a test decodes the old
   shape. A client and daemon that can't read each other must refuse to talk, not
   misparse. See [API_PATTERNS](docs/API_PATTERNS.md#project-specific-api-style) and ADR 0002.
3. **Windows is a first-class target.** Every change builds and behaves on
   Windows (ConPTY, named pipes, PowerShell) as well as Linux and macOS; the
   cross-checks in the gate are not optional (ADR 0008).
4. **Preserve the TUI alongside the optional chat GUI.** Both clients use the
   daemon's sessions. Keep native agent terminal interfaces and terminal-only
   installation supported (ADR 0004).
5. **Trust status only from the owning runtime, and let one writer decide it.** Pane reports need
   its process tree or `MAITTY_PANE_TOKEN`; structured provider events need the daemon-owned
   connection and current conversation generation. Clients display status; they never derive it.
   Unverified or superseded sources cannot change status (ADR 0003).
6. **No blocking work on the UI thread.** Git, file walks, network and process
   spawns go through background tasks (`spawn_bg`) and report back as messages.

## Project-specific: verification gate

Run from the repo root, in order; all must pass:

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
scripts/check-gui-deps.sh   # the desktop app must not link maitty-core, maitty-terminal-client or maitty-settings
cargo check --target x86_64-unknown-linux-gnu
cargo check --target aarch64-apple-darwin
```

For UI changes, also look at the screen: tests render frames with `TestBackend`
(`MAITTY_SHOW=1 cargo test <name> -- --nocapture` prints them), and a live check
runs a nested server with `MAITTY_SOCKET=<name> maitty`.

## Project-specific: workflow and tooling

- **Plan:** Plane, project `MAITTY` (see *Project-specific: planning (Plane)*). The shipped, declined and deferred feature lists are in [`docs/FEATURES.md`](docs/FEATURES.md). `docs/ROADMAP.md` is the legacy phase history, to be removed once the sessions editing it have landed; don't add new planned work to either. The current design is [`docs/DESIGN.md`](docs/DESIGN.md).
- **Branches:** work lands on `dev` at https://github.com/CydoEntis/maitty; one
  commit per logical change, pushed after the gate passes.
- **Install locally:** `cargo install --path apps/tui` (stop a running `maitty` first on
  Windows, or the binary is locked).
- **Releases:** run `scripts/release.sh X.Y.Z` (`--dry-run` prints what it would do). It refuses
  unless `dev`'s head passed CI, then sets the version in `apps/tui/Cargo.toml` and
  `apps/gui/src-tauri/tauri.conf.json`, brings `main` up to `dev`, tags `vX.Y.Z` on `main` and
  pushes the three together. `.github/workflows/release.yml` builds Windows, macOS
  (Apple Silicon and Intel) and Linux (x64, ARM; static musl) archives with checksums and
  publishes the GitHub Release that `install.sh` / `install.ps1` download from. While
  `apps/gui/RELEASE-HOLD` exists a release leaves the desktop app out; see
  [`docs/stack/tauri-updater.md`](docs/stack/tauri-updater.md#project-specific-updater).
- **Tickets:** Plane work items `MAITTY-N` (see the next section). Older GitHub issues on CydoEntis/maitty (labelled `phase-5`, …) are history; don't file new planned work there.

## Project-specific: planning (Plane)

- **Server:** `http://192.168.1.244`, workspace `ideayard`, project key `MAITTY`.
  The project is on the user's home network, so it is only reachable from there.
- **Structure:** a *module* is a feature and acts as its phase (cycles are off). A
  *label* is an area (`ssh`, `client`, `daemon`, `docs`, `adr`, `windows`,
  `tooling`) or a type (`feature`, `bug`, `chore`, `spike`). States are Backlog,
  Todo, In Progress, Done, Cancelled.
- **Access:** the `plan-pull` skill (`~/.claude/skills/plan-pull/`). It reads
  `PLANE_URL`, `PLANE_WORKSPACE` and `PLANE_TOKEN_FILE` from the environment.
  The token never goes in the repo, fixtures, logs or output.
- **Commands:** `brief MAITTY-N`, `start MAITTY-N`, `done MAITTY-N --gate "<results>"`,
  `list MAITTY [--state …]`. If the skill isn't installed, say so and ask the
  user; don't call the API by hand with a guessed token.
- **Dependencies:** a ticket's "blocked by" links are real. Finish blockers first.
