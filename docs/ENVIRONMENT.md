# Environment and configuration

This doc covers how the project is configured, how a new machine gets it
running, and how environments differ. `AGENTS.md` is the operating contract;
this doc holds the environment practices in depth.

## Configuration rules

- Configuration comes from the environment. Code is identical across local, CI,
  staging, and production. Only config changes between them.
- **Load once, validate once.** A single config module reads the environment at
  startup, validates it with a schema, and exports a typed object. The app
  fails fast with a clear message when a required value is missing or invalid.
- Nothing else reads environment variables directly (`process.env`,
  `import.meta.env`, `os.environ`). Everything imports the typed config.
- Give optional values their defaults in the config module, not at the call
  site.
- Name variables in `UPPER_SNAKE_CASE`, grouped by prefix (`DATABASE_URL`,
  `STRIPE_SECRET_KEY`, `FEATURE_EXPORT_ENABLED`).
- Variables exposed to a client bundle (`VITE_*`, `NEXT_PUBLIC_*`, and similar)
  are public. Only non-secret values get the public prefix. See
  [SECURITY](SECURITY.md).

## `.env` files

- Commit `.env.example`. It lists every variable the app reads, each with a
  placeholder value and a one-line comment.
- Never commit real `.env` files. They are listed in `.gitignore`.
- Adding, renaming, or removing a variable updates the config schema,
  `.env.example`, and this doc's variable table in the same change.

```bash
# .env.example
# Postgres connection string for the app role (not the admin role).
DATABASE_URL=postgres://user:password@localhost:5432/app
# Public: base URL the client uses for API calls.
VITE_API_URL=http://localhost:3000
```

## Local setup

A new contributor or agent gets from a fresh clone to a running app and a green
test suite by following *Project-specific: setup*, and nothing else. When setup
changes, update those steps in the same change. If a step can be scripted,
script it.

## Project-specific: prerequisites

The optional Tauri desktop build requires the platform tools and webview
runtime listed in `docs/stack/tauri.md`. These are not prerequisites for the
Rust CLI/TUI build or `cargo install --path apps/tui`.

Rust stable (edition 2024, so 1.85+) with `cargo`; for the cross-checks, the
`x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` targets
(`rustup target add …`). Optional at runtime: `git`, `gh` (opening a PR from Changes), the
agent CLIs you want to run, a Nerd Font for icons.

## Project-specific: setup

```sh
git clone https://github.com/CydoEntis/maitty && cd maitty
cargo build
cargo test
cargo install --path apps/tui   # then run: maitty
```

To work on the optional desktop preview, from `apps/gui/` run:

```sh
pnpm install --frozen-lockfile
pnpm desktop:dev       # run the Tauri desktop shell (same as `pnpm tauri dev`)
pnpm desktop:build     # create platform installer packages
```

Desktop commands build the Rust CLI sidecar for the current host platform. In dev,
Tauri's `beforeDevCommand` rebuilds it (`pnpm bundle:daemon:dev`) on every start, so
`pnpm tauri dev` never runs a stale service copy. A packaged app instead installs that
file as the shared `maitty` on first start and after each update (see
[ADR 0004](adr/0004-client-split-and-desktop-host.md)).
The packaged GUI uses that sidecar to start the separate daemon when needed;
closing the GUI leaves the daemon and its sessions running. `pnpm dev` opens
the browser preview with sample data and does not start or connect to a daemon.
On Arch Linux, including Omarchy, install the packages listed in
[`docs/stack/tauri.md`](stack/tauri.md#platform-prerequisites) before running
`pnpm desktop:build`. Build on the target Linux host because the desktop shell
links to its installed WebKitGTK.

The dev server and the window use port 1420 (`server.port` in `vite.config.ts`, `devUrl` in
`tauri.conf.json`) with `strictPort`: if something else holds 1420, `pnpm desktop:dev` stops with
an error instead of letting the window load that other server. To try a build without touching
the sessions of a running daemon, give it its own server, as for any nested test:
`MAITTY_DATA_DIR=/tmp/maitty-try-data MAITTY_SOCKET=try pnpm desktop:dev`. On NVIDIA under
Wayland, WebKitGTK can die with `Error 71 (Protocol error) dispatching to Wayland display`; start
with `WEBKIT_DISABLE_DMABUF_RENDERER=1` (verified on Hyprland, RTX 4060, WebKitGTK 2.52.6).

Two things to know when building on Windows while a desktop daemon is running:

- `cargo build` fails with "Access is denied" if a `maitty-gui-daemon.exe` that
  was started from `target\debug` is still running, because Windows will not let
  cargo replace the running file. Don't kill it (it owns your sessions); build into
  another folder instead: `CARGO_TARGET_DIR=C:/dev/maitty-gate-target cargo build`.
- `cargo check --target x86_64-unknown-linux-gnu` (or `aarch64-apple-darwin`) cannot
  build the Tauri crate `maitty-gui` from Windows, because its `libdbus-sys` needs a
  Linux sysroot. Cross-check the rest with `--workspace --exclude maitty-gui`.

Two more, on Linux:

- A fresh checkout or worktree has no `apps/gui/src-tauri/binaries/` sidecar, and `cargo build`
  fails in `maitty-gui`'s build script until there is one. `node apps/gui/scripts/bundle-daemon.ts`
  builds `maitty` and copies it there (the desktop commands do this themselves).
- `cargo check --target aarch64-apple-darwin` cannot build `maitty-gui` without a macOS C compiler
  (its Objective-C does not compile). Cross-check the rest with `--workspace --exclude maitty-gui`.

The release workflow signs the desktop update bundles with a Tauri updater key. Generate one once
with `pnpm exec tauri signer generate -w <path outside the repo>`, put its public half in
`plugins.updater.pubkey` in `apps/gui/src-tauri/tauri.conf.json`, and store the private half as the
repository secret `TAURI_SIGNING_PRIVATE_KEY`: in bash `gh secret set TAURI_SIGNING_PRIVATE_KEY < <key file>`, in
PowerShell `Get-Content -Raw <key file> | gh secret set TAURI_SIGNING_PRIVATE_KEY`.
Keep a backup of the private key: if it is lost, installed apps cannot verify a new key and users
reinstall by hand once. To rotate it, publish a release signed by the old key that carries the new
public key, then switch the secret. Local `pnpm desktop:build` does not need the key; only releases
do. See [`docs/stack/tauri-updater.md`](stack/tauri-updater.md).

Settings live in `config.toml` (`%APPDATA%\maitty\` on Windows,
`~/.config/maitty/` elsewhere), with `config.local.toml` beside it for
machine-only values. `config.example.toml` documents every key.

Your Always / Never answers to a repo's `.maitty/checks.toml` commands live in
`check-approvals.json` in Maitty's data folder (`%LOCALAPPDATA%\maitty\data\` on
Windows, `~/.local/share/maitty/` on Linux, `~/Library/Application Support/maitty/`
on macOS), never in the repo and never synced. The daemon reads it once when it
starts; Settings › Check forgets an answer.

`ui.spinner` is an array of up to 64 single-column frame strings; an empty array
uses `icons.working` without animation. Settings → Appearance → Working spinner
offers presets and a custom editor. Preserve explicit custom frames on reload.

## Project-specific: machines

`[[machines]]` names other computers reached over ssh. It is read only from
`config.local.toml` (beside `config.toml`), which is per computer and never synced. A
`[[machines]]` entry in `config.toml`, the file sync shares, is a load error that says where
to move it; the file is left as it was. With `MAITTY_CONFIG` set, that file is the whole
config and is held to the same rule.

| Key | Meaning |
| --- | --- |
| `name` | required; 1-32 of letters, digits, `-`, `_`, `.`, starting with a letter or digit; unique ignoring case; not `local`; not only digits (those are pane ids) |
| `target` | required; the ssh destination; may not start with `-` or contain whitespace or control characters |
| `ssh_args` | list of extra ssh arguments, one per entry; none empty, none with control characters; never split on spaces |
| `remote_cmd` | the app's command on the far side; unset, `maitty`; not empty, no control characters |
| `via` | optional custom transport: a list, the program then its arguments, run in place of ssh (`["fly", "ssh", "console", "-C", "{command}"]`). It must carry stdin and stdout unchanged, allocate no pty and not prompt. The element `{command}` (alone, at most once) is replaced by the remote command `'<remote_cmd>' proxy --attach-only`; without it the command is the last argument. Not empty; the program is not empty, not `{command}` and does not start with `-`; no control characters; `{command}` inside a longer entry is refused. `ssh_args` beside `via` is refused (it would be ignored). Refused on Windows. It is a program this app runs, so it is read only from `config.local.toml`. `target` stays required: it is the machine's label |
| `clipboard`, `notices`, `sounds` | default `false`; let that machine's panes write the clipboard, raise notices (and bring the window to the front) or play sounds. A machine's own worktree prompts, replies and workspace announcements are never acted on. See [SECURITY](SECURITY.md) |
| `mcp` | default `false`; lets an agent in a pane on this computer reach this machine through its MCP tools (`machine` argument or a `machine/id` id): list, read, send, wait. Off, a call naming the machine is refused with `machine_not_enabled` and the machine is not dialled. On, the agent's own pane still needs the `read` or `write` grant. Starting, answering and interrupting stay local-only. See [SECURITY](SECURITY.md) |

Validation runs when the config loads. `--remote host` is an ad-hoc machine named after the
host, with no table entry: `MAITTY_SSH` is its ssh program and leading arguments (split on
whitespace), `MAITTY_REMOTE_CMD` its remote command. A `[[machines]]` entry uses `ssh_args` and
`remote_cmd` instead, and does not read those two variables.

A machine with a `via` is connected, redialled and checked like any other, with these differences.
`maitty machine update <name>` is refused (`installing over a custom transport is not supported;
install maitty on <name> yourself`) and nothing is run. The Machines dialog only adds ssh machines, so
it has no probe or first-login step for a `via` entry; the entry connects when the app starts. `maitty
doctor` shows `custom transport` with the program's name (never its arguments), runs the protocol
check, and marks the steps that need ssh `not available over a custom transport`. `--on <name>` works.
A failure is `NeedsInstall` on exit 127, `Unreachable` with the program's last stderr line otherwise,
never `NeedsLogin` (that reads ssh's words).

Each `[[machines]]` entry is connected when the app starts, and kept connected: a line that
drops is redialled at once and then at growing intervals (states and timings:
[API_PATTERNS](API_PATTERNS.md#project-specific-connections-to-other-machines)). The machine's daemon must
already be running only after the first connection: maitty starts it there on a machine's first
connection and when you retry, because it exits when nothing is attached; a redial after a lost
connection only attaches and never starts it (`--remote host` always starts it). The window says, in a toast, when a machine
is reconnecting, connected again, unreachable (with why), needs a login (make plain `ssh` to it
work without a prompt) or needs maitty updated or installed there. Typing sent to a machine while
it was down is dropped, and the toast on reconnect says how much. A machine added or removed in the
Machines dialog (leader `w`) takes effect at once: the dialog appends or deletes the `[[machines]]` entry
in `config.local.toml` (`name` and `target` only, comments and the rest kept, written atomically) and
starts or stops that machine's connection. A change made by hand to a `[[machines]]` entry takes effect
after a restart; only a refusal (a different machine answering at the name) is re-checked when the
config reloads.

### The command line on a machine

`--on <machine>` (global, before or after the subcommand) runs a one-shot command on a configured
machine, and a pane there is written `machine/id` (`--pane build/4`, `read build/4`). `local`, in any
case, is this computer. Names compare ignoring case, as the config check does. `--on` works with `ls`,
`read`, `send`, `wait`, `split`, `handoff`, `new`, `focus`, `close`, `grant`, `teach`, `popup` and
`kill-server`; any other command refuses it rather than act here. The machine of a command is the
address's, else `--on`, else `--remote` (and `MAITTY_REMOTE`), else this computer. `--on X` with
`--pane Y/4` for another machine, `--on` with `--remote`, and an address with `--remote` are usage
errors (exit 2), as are `ls --all` beside `--on`, `--remote` or a pane address, and an unknown name, which lists the names that work. `--remote host` is the ad-hoc
route and behaves as it always did; a configured machine is only attached to by `--on` (it never starts a remote daemon).

`$MAITTY_TERM_ID` and `$MAITTY_PANE_TOKEN` name a pane of the daemon on this computer. They stay the
default pane and the identity of a command that talks to this computer (`--on local` included). A
command given `--on build` or `build/..` never takes them: it needs `--pane build/<id>`, and the other
machine is told no pane id or token. `kill-server --on build` stops every session on that machine and
needs `--yes`.

### Installing the app on a machine

`maitty machine update <name> [--install] [--yes] [--force]` puts this version on a configured machine
(see the guide). The destination is `remote_cmd` when it is an absolute path or begins with `~/` (the `~`
is expanded on the far side, from its `$HOME`, never here); a `remote_cmd` that is only a name (the
default, `maitty`) installs to `~/.local/bin/maitty`. Only Linux and macOS on x86_64 or aarch64 are
supported as a remote; a Windows machine is refused. The release assets are named
`maitty-<target>.tar.gz` (see `cli/update.rs`) and the fetch uses the same `gh` or `curl` path as
`maitty update`, on this computer only.

## Project-specific: PATH on the daemon's host

On unix the daemon resolves a login shell's PATH once, at start, before it takes its first
pane (`crates/maitty-core/src/login_path.rs`). A daemon started by a non-interactive ssh
session keeps a minimal PATH, and a plain pane is not a login shell, so an agent installed
through `.profile` or `.zprofile` would otherwise be missing. The client never resolves it.

- It runs `$SHELL -ilc` with a printed PATH between two marker words; what the rc files print
  around them is ignored. Fish prints `string join : $PATH`. `$SHELL` must be an absolute path
  to an existing bash, zsh, sh, dash, ksh or fish; anything else (unset, relative, nu, ...)
  uses `/bin/sh`.
- The probe has a fixed timeout (5 seconds, not configurable). On a timeout, a failure to
  start, or output without the markers it logs one warning (shell name and reason, never the
  PATH) and the daemon keeps its inherited PATH. Start never fails because of it.
- The result is the login entries first, then any inherited entries it lacks, without empty
  or repeated entries. The daemon's own lookups (`proc::on_path`, `proc::resolve_executable_path`,
  `gh`), every pane's `PATH` and the environment handed to conversation providers use it. The
  agent launchers stay first on a plain shell pane's `PATH`; the resolved PATH replaces only the
  inherited part after them. `[env]` in `config.toml` still overrides a pane's `PATH`.
- Windows keeps today's behaviour; the Windows remote is out of v1 (ADR 0007).

## Project-specific: variables

No `.env` file: Maitty is configured through `config.toml`. Environment variables
it reads:

| Variable | Purpose | Set by |
| --- | --- | --- |
| `MAITTY_CONFIG` | use this config file instead of the default | user |
| `MAITTY_DATA_DIR` | absolute directory for saved state, logs and approval settings; validated and captured at startup | developer / user |
| `MAITTY_SOCKET` | name of a separate server (like `tmux -L`); used for nested test servers | user |
| `MAITTY_REMOTE` | the SSH host this client works on (from `--remote`) | maitty |
| `MAITTY_SSH`, `MAITTY_REMOTE_CMD` | replace `ssh` / the remote `maitty` command (`MAITTY_REMOTE_CMD` is one program path); removed from every pane's environment, like `MAITTY_REMOTE` | user |
| `MAITTY_TERM_ID`, `MAITTY_PANE_TOKEN` | the pane's id and secret, set inside every pane | daemon |
| `MAITTY_SHOW` | tests print rendered frames | developer |
| `MAITTY_NO_INTEGRATE` | `1`: the installers and `maitty update` leave agents' config alone instead of running `maitty integrate` | user |
| `PI_CODING_AGENT_DIR` | Pi/OMP native agent configuration directory; do not share between the two apps | user / agent app |
| `PI_CONFIG_DIR` | OMP config folder below the user's home, default `.omp` | user / OMP |
| `KIMI_CODE_HOME` | Kimi config directory, default `~/.kimi-code` | user / Kimi |
| `XDG_CONFIG_HOME`, `XDG_STATE_HOME` | native OpenCode configuration/state roots; Kilo's config also uses the config root | user / OS |
| `EDITOR`, `VISUAL`, `SHELL`, `COMSPEC` | editor and shell defaults | OS / user |

For isolated tests, set `MAITTY_DATA_DIR`, `MAITTY_CONFIG` and a unique
`MAITTY_SOCKET` together. Keep the data directory outside the test repository:
it contains approval decisions that repository files must not control. The data
override skips legacy-name migration; it does not redirect config files or
provider-native history. Empty or relative paths and existing non-directories
fail startup. A missing absolute directory is created by the private state writers.

## Project-specific: settings both apps share

The terminal app and the desktop app read and write the same `config.toml`. The desktop app
reaches it only through the daemon (`Query::Settings`, `Command::SetSetting`; ADR 0004), and
only for these keys. Secrets, `[[machines]]` and `[keys]` are not reachable that way.

| Setting | On disk (written) | Also read as | Values |
| --- | --- | --- | --- |
| default agent | `sessions.agent` | | `last used`, `claude`, `codex`, `gemini` |
| new session view | `ui.new_session_view` | | `chat` (default), `terminal` |
| sidebar side | `ui.sidebar_position` | `ui.sidebar_side` | `left`, `right` |
| dark / light theme | `theme_dark`, `theme_light` | | a theme name (letters, digits, `-`, `_`, `.`; at most 64) |
| appearance | `appearance` | | `dark`, `light` |
| spinner | `ui.spinner` | | frames, as in the terminal app (at most 64) |
| start folder | `ui.start_dir` | | a path or `~...`; at most 4096 bytes, no control characters |
| reduce motion | `ui.motion` (inverted) | `ui.reduce_motion` | true when `motion = false`; `motion` wins if both are set |
| window style | `ui.window_style` | | `flat` (default), `glass` |

A change edits the one key in place (comments and other keys stay), swaps the file in
atomically, reloads, and sends `ServerMsg::SettingsChanged` to attached windows. A write
removes the alias spelling of the same key, so the file keeps loading. A value outside the
table is refused before the file is opened. A key set in `config.local.toml` still overrides
what is written to `config.toml`.

## Project-specific: removed config keys

Removed `[ui]` keys. `pane_colors`, `tab_gap` and `tool_rail` no longer exist: the TUI draws one accent focus border,
the tabs follow the v27 tab row and the tools open from the Project chip. A `config.toml` that still names them
loads without error and the keys are ignored (the file is not rewritten). `layout = "grid"` now reads as Floating
without pane colours.

## Project-specific: environments

Local only: Maitty is a desktop tool. Development builds use `cargo install --path apps/tui`;
release archives and installers are published through the GitHub release workflow.
`dev` is the working branch.

## Project-specific: usage store

The daemon keeps `usage-<socket>.json` in the data folder beside `session-<socket>.json`: 90 days
of provider-reported usage per session per day ([DATABASE](DATABASE.md#project-specific-usage-store),
[ADR 0009](adr/0009-usage-is-provider-reported-never-estimated.md)). It adds no setting or variable.
Deleting the file starts the history over; nothing else depends on it.

## Project-specific: structured history retention foundation

The structured cache derives `session-<socket>.history/` beside the
existing private session file, with numeric conversation/page directories. It adds
no settings, environment variables or dependencies. Its cache owner accepts the
existing `restore.enabled` and `restore.output` policy at initialization and
reload: either false removes/prevents all Maitty structured history content.
Cleanup errors surface; native provider history is untouched. Attachment-scoped
memory hydration remains available while disk retention is disabled.
Structured mutations require `restore.enabled=true` so delivery metadata can be
durably recorded. `restore.output=false` still permits content-free identity and
operation receipts. A selected executable and existing absolute Codex home/cwd
are explicit CLI launch inputs; the actor sets `CODEX_HOME` to that root. No new
provider credential setting is introduced.

Structured provider children receive four internal, runtime-scoped variables:
`MAITTY_CONVERSATION_ID`, `MAITTY_CONVERSATION_GENERATION`,
`MAITTY_CONVERSATION_CONNECTION`, and secret `MAITTY_CONVERSATION_AUTH`.
The daemon owns these values; do not set or log them. All four are required when
any is present. They identify a live runtime without a terminal pane. Provider
launch strips inherited pane/structured claims, and configured MCP subprocess
allowlists receive only the variable names through transient request overrides.
