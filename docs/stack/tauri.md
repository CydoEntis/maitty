# Tauri

> **Use for:** Maitty's optional cross-platform desktop GUI host.
> **Requires:** Rust, TypeScript, React
> **Fills:** ARCHITECTURE § stack and layers; ENVIRONMENT § prerequisites; SECURITY § threat model

## Rules

- Keep Tauri as an optional desktop shell. The Rust CLI/TUI build and install
  must continue to work without Node or frontend dependencies.
- The webview is a client of the daemon. It never owns agent processes, PTYs,
  durable sessions, provider credentials or conversation state.
- Keep the command surface narrow and typed. Validate every command input in
  Rust; do not expose generic filesystem, process, shell or IPC commands.
- Bundle the standalone CLI as the daemon sidecar. The native host starts it
  only with the fixed `daemon` argument when no local daemon answers; the
  webview cannot choose an executable or process arguments. The one other
  invocation is `connect_agent`: `integrate <agent>` for an agent on a fixed
  allowlist (`claude`, `codex`), so a chat can connect its status hooks.
- Launch the daemon as a separate detached process. It must keep sessions alive
  after the GUI exits, and must not inherit pane identity, structured runtime
  identity or remote routing variables from the GUI process.
- Use the daemon's authenticated local IPC and existing protocol. The GUI must
  not open a second session store or bypass daemon authorization.
- Keep capabilities scoped to the app's own windows and only the plugins and
  commands the GUI uses. Do not enable remote origins or broad filesystem access.
- Native directory selection uses the Tauri dialog plugin. Grant only
  `dialog:allow-open`; do not grant filesystem read/write access for a folder
  picker.
- Build the frontend as a Vite single-page app. Do not add server-side rendering.
- Use the operating system webview. Document its runtime prerequisites and
  verify packaged builds on Windows, macOS and Linux.

## Platform prerequisites

- Windows: Microsoft C++ build tools and WebView2 runtime.
- macOS: macOS 10.15 or later and Xcode Command Line Tools.
- Linux: WebKitGTK 4.1 and the Tauri system libraries listed by the platform
  setup guide.

For Arch Linux, including Omarchy, install Tauri's native build dependencies
before running `pnpm desktop:build` from `apps/gui/`:

```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl dpkg patchelf \
  appmenu-gtk-module libappindicator-gtk3 librsvg rpm-tools xdotool
```

On Omarchy, install the same packages through **Super+Space → Install → Package**
or run `omarchy pkg add <package>` once per package. Keep the system updated
with Omarchy's own update flow; do not run a full `pacman -Syu` for this setup.
Package names follow [Tauri's Linux prerequisites](https://v2.tauri.app/start/prerequisites/).
The Omarchy package and update flows are documented in its
[package guide](https://omarchy.org/manual/other-packages/) and
[update guide](https://omarchy.org/manual/updates/).

Build the GUI on Linux itself so Tauri links against that system's WebKitGTK.
The Rust CLI/TUI can still be cross-checked from other platforms, but that does
not validate the Linux desktop shell.

The `.AppImage` bundle needs FUSE to mount when launched. Arch Linux and Omarchy
can install `fuse2` through **Super+Space → Install → Package** or
`omarchy pkg add fuse2`. If FUSE is unavailable, launch it with
`--appimage-extract-and-run` instead. See the
[AppImage FUSE guide](https://docs.appimage.org/user-guide/troubleshooting/fuse.html).

## Project-specific: tauri

Tauri 2 is the optional desktop host. The GUI lives in `apps/gui/`, uses React,
TypeScript, Vite, shadcn/ui and Tailwind, and connects to the existing daemon.
Keep native packaging and GUI build steps separate from `cargo install --path apps/tui`.
Desktop build scripts compile and package a GUI-specific CLI sidecar for the
current host. Keep its `maitty-gui-daemon` name separate from the terminal CLI
executable so a running daemon cannot lock the file a new GUI build must
replace. That sidecar is only the payload: a packaged app installs it as the
shared `maitty` (ADR 0004) and runs that file. Find the payload in packaged
resources first, then as an adjacent target-suffixed sidecar so release builds
launched from their output directory work too. A development build runs the
sidecar directly.
Expose only fixed commands to the webview. On a protocol mismatch, the GUI may
offer an explicit restart for an older local daemon after showing its saved
session and active-turn counts; recheck the exact protocol in Rust before
stopping it, then reconnect through the bundled daemon. Never downgrade a newer
daemon or restart a remote service from the GUI.
The target OS floor is the existing project support floor; do not raise it
without updating this section and the release workflow.

Tray item (`tray.rs`, Tauri's `tray-icon` feature): one item in the Windows tray or the macOS
menu bar, with the system's own menu. The window says which sessions to list
(`set_tray_sessions`: the ones that need you, then the finished ones, by id and name); the host
only lists them, at most ten a group, each name cut to one short line and an id that is not a
session id left out. Choosing one shows the window and tells it the id (`tray-open-session`); a
left click on the item shows the window. The host decides no status and sends nothing to a
session. The app runs without the item when the system refuses it. Linux builds it too (the
`libayatana-appindicator3` package the workflows already install) but reports no click on the
item there, only the menu; it has been tried on Windows only.

Machines (ADR 0007): the host reads the `[[machines]]` list of `config.local.toml` once at start
through `maitty-machines` (`machines.rs`, command `list_machines`); a hand edit applies on the next
start. The window is told names only, plus the error when the file is bad, and shows it as a toast
without stopping the app. The host may link `maitty-machines`, `maitty-desktop-client`,
`maitty-shared` and `maitty-ipc-transport`, never `maitty-core`, `maitty-settings` or
`maitty-terminal-client` (`scripts/check-gui-deps.sh` checks it).

Connections (ADR 0007): `hub.rs` holds one supervised connection per configured machine through
`maitty-machines` (`Links`, `Connections`), dials with the desktop surface and no pane claim, and
tells the window each machine's state as `machine-state` (`{machine, name, state, why, attempt}`;
this computer's own line is machine 0, named `local`, and is held by the same hub). All output
reaches the window on one raw channel with the machine in the frame header (`streams.rs`). Every message from a machine is vetted by
`remote_policy` and a different daemon answering at a known name is turned away (`machine_ids`,
shared with the terminal app); the web view never connects. Its tests run against the fake `ssh`
shim (`crates/maitty-test-support`).

Routing (`routing.rs`, `terminal_stream.rs`): the pane commands take an optional `machine` and a
command for another machine's pane goes down that machine's line through the hub instead of opening
a connection; this computer's commands are unchanged.

In the web view (`api/machines.ts`, `features/workspace/model/machines.ts`): a session of another machine
has an id `<name>/t<pane>` and is a row of the one flat list, marked with a server icon and its machine's name. The saved
layout is `maitty.gui.layout.v3`; a v2 or v1 tab layout is dropped once, with a notice. Until the machine has listed its
sessions, a layout's panes for it are kept.

The glue's per-update logic is `terminal_stream::react` (no window involved), so the host's behaviour
is tested end to end against real daemons (`hub/tests/end_to_end.rs`).

File sends (ADR 0007): `file_send.rs` takes the paths of files dropped on a pane of another machine and
that machine's name (`send_files_to_machine`), checks them in Rust (a known ssh machine; each path absolute, an
existing regular file of at most 100 MB; at most 10; one send at a time), and sends them in turn on a blocking
worker through `maitty-machines`' `remote_send` and `ssh_run`, the runner the terminal app uses. It reports
`file-send` events (`progress`, `sent`, `failed`, `cancelled`, `done`) and `cancel_file_send` ends the one named by
the id the window chose. The window, not the host, types the far-side paths into the pane.

Pasted images (ticket 19): `image_paste.rs` has two commands that take the image as the **raw body** of the
call (`tauri::ipc::Request`, not base64 in JSON or an event), with the machine and send id in `x-maitty-*`
headers. `save_pasted_image` checks the bytes (PNG, JPEG, GIF or WebP signature, at most 25 MB), keeps them in the
app's cache folder `pastes` (owner-only, never over an existing file, our own old files swept) and returns the
path. `send_pasted_image` runs the same checks, then that the machine is known, ssh, and `connected`
(`MachineStates::is_connected`), stages the bytes as a private file and starts a `file_send` job of kind `Image`:
the runner, events, Cancel and the far-side path checks are the file send's, and the staged copy is removed when
the send ends. A chat composer's attachment calls these two commands.
