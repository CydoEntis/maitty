# Maitty desktop GUI

This is Maitty’s optional desktop chat interface. It uses Tauri, React and
TypeScript, and connects to the same local daemon as the terminal UI. Closing
the GUI does not stop conversations.

## Run locally

Install Node.js, pnpm, Rust and the desktop prerequisites for your platform.
See [`docs/stack/tauri.md`](../docs/stack/tauri.md) for WebView2, macOS and
Linux requirements, including Arch Linux and Omarchy packages.

From this folder:

```sh
pnpm install --frozen-lockfile
pnpm desktop:dev
```

The GUI uses the daemon’s protocol. If an older daemon is already running, the
GUI explains the mismatch and offers a guarded restart after showing what may be
interrupted. Save or finish important work before choosing that action.

Codex conversations use the installed Codex CLI and its local sign-in. On
Windows, Maitty also searches common npm and NVM install locations. On Linux,
keep the Codex command on `PATH`; Unix mise shims are supported.

## Check and package

```sh
pnpm test
pnpm lint
pnpm build
pnpm desktop:build
```

`pnpm dev` and `pnpm preview` run a browser preview with sample data. They do
not connect to the daemon or start a real Codex conversation. Desktop packages
are written to the workspace `target/release/bundle/`; build on the target OS,
especially on Linux, where Tauri links to the installed WebKitGTK.

Version-tagged releases publish the GUI installers beside the TUI archives. The
GUI installer carries a daemon built from that same release. Install the TUI
and GUI from one release when you want both clients to share sessions through
local IPC. The TUI install scripts remain terminal-only.
