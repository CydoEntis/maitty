# Tauri updater

> **Use for:** Letting the Maitty desktop GUI update itself from GitHub releases.
> **Requires:** Tauri 2
> **Fills:** SECURITY § supply chain, ENVIRONMENT § releases

## Rules
<!-- local: owner approved idle installs on 2026-11-04; the stack default was install on a click only -->

- Install an update only on a click, or when **Install updates when idle** is on (the default) and the
  app is idle: no agent working or waiting on an answer, no keyboard input for 5 minutes, and the window
  unfocused or minimized (`features/updates/idle.ts`). The GUI checks once on launch and when asked in
  Settings › General › Updates; it offers the version and what changed, and the user chooses.
  After the restart a toast and a What's new dialog say which version this is.
- Verify every update with the project's updater key. The public key is in `tauri.conf.json`; the
  private key lives only in the `TAURI_SIGNING_PRIVATE_KEY` repository secret and the owner's
  backup. Never commit it, print it, or put it in a fixture or log.
- An update replaces the GUI only. It must never stop the daemon or end a session. After an install
  the window restarts; the daemon keeps running.
- A failed or offline check is quiet: no banner, only the line in Settings. A failed install says so
  and never claims the app is updated.
- Enable only the operations the GUI calls in its Tauri capability.

## Project-specific: updater

The GUI uses `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` with
`tauri-plugin-updater` and `tauri-plugin-process` 2.x, desktop targets only. The capability allows
`updater:allow-check`, `updater:allow-download`, `updater:allow-install` and
`process:allow-restart`. The app downloads in the background and installs on restart (a click, or the idle rule above), which are two
commands, so the combined `updater:allow-download-and-install` does not cover it.

The app reads `https://github.com/CydoEntis/maitty/releases/latest/download/latest.json`. The release
workflow builds with `src-tauri/tauri.updater.conf.json` (which sets `createUpdaterArtifacts`) and
publishes the signed bundles and `latest.json` beside the installers:

| Platform | Update bundle |
| --- | --- |
| Windows x64 | `Maitty_<version>_x64-setup.exe` |
| macOS Apple Silicon | `Maitty_<version>_aarch64.app.tar.gz` |
| Linux x64 | `Maitty_<version>_amd64.AppImage` |

While `apps/gui/RELEASE-HOLD` exists, a release leaves the desktop app out. The release workflow
builds no installers or update bundles and attaches the newest release's `latest.json` to the new
release unchanged. The app asks the newest release for that file, so desktop users go on being
offered the last version released with the desktop app, and the release notes say where its
installers are. The file says why the app is held; delete it to release the desktop app again.
Don't rebuild the release that manifest points at while the hold is on: its signatures are a copy,
and rebuilt bundles would no longer match them.

CI builds the installers on a push to `dev`, without `createUpdaterArtifacts` because it has no private key. On Windows
the installer runs in `passive` mode and closes the app while it installs. The `.deb`, `.rpm`, `.msi`
and `.dmg` installers are not update bundles; a user who installed one reinstalls to update.
