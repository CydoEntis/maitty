# Herdr detection source

The 22 TOML screen manifests in this directory are an unmodified snapshot from
[herdrdev/herdr](https://github.com/herdrdev/herdr/tree/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests),
commit `143bf5fd4781d164260c63fe28e7dc08054ad32f`, retrieved 2026-10-16.
They are distributed under the included Apache-2.0 license. Herdr copyright
notices and individual source comments are retained. No upstream NOTICE file
was present in this snapshot.

`src/detection/regions.rs` adapts Herdr's screen-region extraction helpers from
`src/detect/manifest.rs` at the same commit; its header records the local changes.
`src/cli/opencode-tui.js` adapts Herdr's OpenCode TUI selection bridge;
`src/cli/opencode_config.rs` adapts its JSONC plugin configuration editor.
`src/cli/omp.js` and `src/cli/kilo.js` adapt the pinned lifecycle event handling.
Their headers identify local transport, authentication and behavior changes.
Seshi's matcher and status arbitration are maintained locally. No remote
manifest downloads or Herdr runtime are executed.

When updating these rules, pin the new upstream commit, review the diff, verify
every supported region and regex compiles, and run Seshi's status regressions.
Keep this notice and LICENSE in distributed binary archives.
