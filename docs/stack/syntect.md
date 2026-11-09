# syntect

> **Use for:** Syntax highlighting of file previews in the terminal client and the desktop app.
> **Requires:** Rust 2024 workspace
> **Fills:** ARCHITECTURE § directory layout

## Rules

- Only `crates/maitty-highlight` depends on `syntect` and `two-face`. Everything else calls
  its `highlight(path, text)`, which returns colour roles (`Kind`), never colours.
- Use the pure-Rust regex engine (`regex-fancy` / `syntect-fancy`) and no default features. The
  C `onig` engine breaks the musl, ARM and macOS cross-checks (ADR 0008).
- Take grammars from `two-face::syntax::extra_newlines()`, which adds TypeScript, TSX, TOML and
  more to syntect's defaults. Do not load themes: roles come from scope names, so the TUI and the
  GUI each pick their own colours.
- Highlighting is best effort. `highlight` returns `None` for an unknown type, a file over the
  size cap or a failed parse; the caller then renders plain text. A highlight failure never blocks
  showing a file.
- Highlight a file once when it loads or changes, never per frame. Do the work on a background
  task (non-negotiable 6).
- The desktop app reaches the crate through a Tauri command in the host. It must not link
  `maitty-core`, `maitty-terminal-client` or `maitty-settings` through it
  (`scripts/check-gui-deps.sh`).

## Project-specific: syntect

- Versions: `syntect` 5.3 and `two-face` 0.5 (bat 0.26 grammars). Both are MIT or Apache-2.0,
  maintained, and widely used (syntect ~39M downloads, two-face ~16M; checked 2026-11-09).
- Cost: a few MB of grammar data per binary.
- PowerShell has no grammar in this set; `.ps1` keeps the GUI's regex highlighter.
