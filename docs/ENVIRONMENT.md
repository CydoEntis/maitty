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
pnpm desktop:dev       # run the Tauri desktop shell
pnpm desktop:build     # create platform installer packages
```

Desktop commands build the Rust CLI sidecar for the current host platform.
The packaged GUI uses that sidecar to start the separate daemon when needed;
closing the GUI leaves the daemon and its sessions running. `pnpm dev` opens
the browser preview with sample data and does not start or connect to a daemon.
On Arch Linux, including Omarchy, install the packages listed in
[`docs/stack/tauri.md`](stack/tauri.md#platform-prerequisites) before running
`pnpm desktop:build`. Build on the target Linux host because the desktop shell
links to its installed WebKitGTK.

Two things to know when building on Windows while a desktop daemon is running:

- `cargo build` fails with "Access is denied" if a `maitty-gui-daemon.exe` that
  was started from `target\debug` is still running, because Windows will not let
  cargo replace the running file. Don't kill it (it owns your sessions); build into
  another folder instead: `CARGO_TARGET_DIR=C:/dev/maitty-gate-target cargo build`.
- `cargo check --target x86_64-unknown-linux-gnu` (or `aarch64-apple-darwin`) cannot
  build the Tauri crate `maitty-gui` from Windows, because its `libdbus-sys` needs a
  Linux sysroot. Cross-check the rest with `--workspace --exclude maitty-gui`.

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

## Project-specific: variables

No `.env` file: Maitty is configured through `config.toml`. Environment variables
it reads:

| Variable | Purpose | Set by |
| --- | --- | --- |
| `MAITTY_CONFIG` | use this config file instead of the default | user |
| `MAITTY_DATA_DIR` | absolute directory for saved state, logs and approval settings; validated and captured at startup | developer / user |
| `MAITTY_SOCKET` | name of a separate server (like `tmux -L`); used for nested test servers | user |
| `MAITTY_REMOTE` | the SSH host this client works on (from `--remote`) | maitty |
| `MAITTY_SSH`, `MAITTY_REMOTE_CMD` | replace `ssh` / the remote `maitty` command | user |
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

## Project-specific: environments

Local only: Maitty is a desktop tool. Development builds use `cargo install --path apps/tui`;
release archives and installers are published through the GitHub release workflow.
`dev` is the working branch.

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
