# Spec: the inspector drawer

Intent: [intent.md](intent.md) · Status: accepted · Date: 2026-10-24

## Concerns

- **New dependency, pulldown-cmark** (`docs/SECURITY.md` supply chain). Recommend: default
  features off, pinned in `Cargo.lock`, a line in `docs/stack/rust.md` saying why.
- **`b` changes meaning**: it opened the branch switcher; it now opens the Branch view, whose
  first action is that switcher. Recommend: say so in `docs/shortcuts.md` and the release notes.
- **Narrow screens**: below `NARROW` the panel already takes the whole pane column; kept.

## Requirements

- **R1.** The panel's left edge drags to resize; `[` and `]` narrow and widen it 4 columns while
  it has the keys; `=` resets — TestBackend tests.
- **R2.** Each view kind keeps its own width across restarts (saved UI state); defaults: Inbox and
  checks 72 columns, Files 55% of the screen, Changes 50%, Branch 30% — tests.
- **R3.** The panel is never under 32 columns, and the panes keep at least 52 beside it; when
  they can't, the panel takes the whole column — tests at 80×24 and 160×45.
- **R4.** `f` opens Files in the panel (no popup): the tree beside the file; under 60 columns the
  file takes the panel with `‹` back; `f` again or Esc closes — tests.
- **R5.** Ctrl+click on `path:line` in a pane opens Files in the panel at that line — test.
- **R6.** `.md` and `.markdown` files preview pretty by default (headings, emphasis, inline and
  fenced code, lists, quotes, tables, links, rules); `m` toggles raw; `e` edits the raw text —
  tests.
- **R7.** `b` opens Branch: the branch, ahead/behind its upstream and its base branch, the last 10
  commits, the last check result, the snapshots; Enter on "Switch branch…" opens the switcher —
  tests with a temporary repo.
- **R8.** Clicking a sidebar row's `+N −N` opens Changes in the panel for that checkout — test.

## Design

- **Source**: [`docs/design/inspector-v1/README.md`](../../design/inspector-v1/README.md) §2 and
  its `theme.js`. Where the app differs: no sidebar fold, no PR status, keys as above.
- **Data**: `Screen.saved` gains a width per view kind (serde default, so older files load).
- **Reuses**: `sheet.rs` `split_for_sheet` and `SheetKind`; `design.rs` `draw_files`; `actions.rs`
  `open_files`, `open_path_from`, `open_changes`; the `branch.rs` switcher; `snapshot.rs` `list`;
  the `Drag::Side` handling in `input.rs` as the pattern for the edge.

## Out of scope

PR status, folding the sidebar, inline images.

## Verification

The gate, plus a live look at 80×24, 100×30 and 160×45 in Floating and Compact.

## Coverage

- R1, R2, R3 → #172
- R4, R5 → #173
- R6 → #174
- R7 → #175
- R8 → #176
