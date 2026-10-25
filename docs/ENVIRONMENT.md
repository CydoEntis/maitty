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

Rust stable (edition 2024, so 1.85+) with `cargo`; for the cross-checks, the
`x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` targets
(`rustup target add …`). Optional at runtime: `git`, `gh` (opening a PR from Changes), the
agent CLIs you want to run, a Nerd Font for icons.

## Project-specific: setup

```sh
git clone https://github.com/CydoEntis/maitty && cd maitty
cargo build
cargo test
cargo install --path .   # then run: maitty
```

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

## Project-specific: environments

Local only: Maitty is a desktop tool. Development builds use `cargo install --path .`;
release archives and installers are published through the GitHub release workflow.
`dev` is the working branch.
