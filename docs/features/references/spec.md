# Spec: references

Type: spec · Status: built on `feat/references` (phase 28), not yet released · Date: 2026-10-25 · Intent: [intent.md](intent.md)

## Concerns

- **Worktrees.** Agents run in per-agent worktrees; a folder per worktree would give each agent
  its own set. Decided: the folder lives at the repo's **main checkout**
  (`references::project_root`, the same `gitfs::head` resolution the daemon uses for
  `TermInfo::root`), so every worktree and every agent sees one set.
- **Gitignored without touching tracked files.** `.gitignore` is the user's tracked file.
  Decided: the line `.maitty/references/` goes in the repo's `info/exclude` (the path git
  reports for `--git-path info/exclude`), once, when Maitty first makes the folder.
- **Remote daemons.** References are files on the machine the client runs on; over `--remote`
  the view says it isn't available over ssh yet, like Files and Branch. The MCP tool runs where
  the agent runs, so it works on a remote host.
- **No wire change.** Everything is client-side file work and the MCP server; `PROTOCOL_VERSION`
  is unchanged.

## Requirements

- **R1.** `<main checkout>/.maitty/references/` holds a project's references; a folder outside
  git uses `<folder>/.maitty/references/`. A linked worktree resolves to the main checkout —
  test with temporary repos (`references.rs`, `client/tests.rs`).
- **R2.** When Maitty makes the folder in a repo, `.maitty/references/` is added to
  `info/exclude` once (never twice; an anchored `/.maitty/references/` counts); no tracked file
  changes; `git add -f` still commits one — tests.
- **R3.** Leader + `a` opens **References**, the fourth tab of the side panel's strip
  (`Files f · Changes c · Branch b · References a`); `a` again or Esc closes it. It lists name,
  kind, size and age, newest first; hidden files are skipped; folders inside are listed to three
  levels — tests.
- **R4.** Empty: "No references yet · paste an image or add a file", where they are kept, and
  that agents find them through MCP — TestBackend at 80×24 and 160×45.
- **R5.** Keys: Enter / `o` open in the system viewer (the Ctrl+click opener), `p` / Ctrl+V add
  the clipboard image as `screenshot-YYYY-MM-DD-HHMMSS.png` (UTC), `n` add a file by its typed or
  pasted path (quotes from a drop are taken off; copied, never moved; numbered when the name is
  taken; 200 MB at most), `d` / Delete remove (asked first, danger), `y` copy the absolute path,
  ↑↓ / `j` `k` choose. A paste while the view has the keys stays in it: one line naming an
  existing file is a drop and is added, an empty paste (a terminal's paste of an image) adds the
  clipboard image, and anything else shows "References has the keys: …" without repeating the
  pasted text (it may be a secret). A name starting with a dot is refused (it would be hidden
  and unremovable). New files are made create-new, at the next free `-N` name, so an add never
  writes through a link or over another add; the size cap holds while copying. No image on the
  clipboard: a toast, and no folder is made — tests.
- **R6.** Every file operation (resolve, list, copy, write, remove, exclude) runs in `spawn_bg`;
  the view opens at once with the project's root from the snapshot and settles it when the
  background read lands.
- **R7.** MCP `maitty_references` lists the caller's project (the MCP server's cwd mapped to
  the main checkout) or the project of an optional `path`: `folder` and, per file, absolute
  `path`, `name`, `kind` (image, pdf, design, text, file), `bytes`, `modified` (UTC ISO 8601).
  The server's instructions tell agents to call it before UI, front-end or design work and open
  the images — tests.
- **R8.** Safety: `path` must be a string naming an existing folder; under `[mcp] scope =
  "project"` it must be in the caller's project, else refused. Nothing outside the references
  folder is listed or removed: no `..`, a link is listed only when it points inside the folder,
  and a references folder (or `.maitty`) that is a link out of the project is refused; `ensure`
  refuses a `.maitty` or `references` that is already a link or a file before making anything.
  The scope is checked on the path's words (`.` and `..` worked out) before anything on disk is
  touched, and network paths (`\\server\share`, `\\?\UNC\…`, `//server/share`) are refused up
  front. Entries left out of a listing (unreadable, links out) are counted and the first reason
  shown (the view's last list row, `skipped` in the MCP reply). `info/exclude` is read as bytes
  and only appended to — tests.

## Out of scope

- Previews in the terminal (images open in the system viewer).
- Notes or tags per reference; the file name is its description.
- Over `--remote`: the view refuses with a message.
