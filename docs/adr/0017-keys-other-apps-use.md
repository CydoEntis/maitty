# 0017. Default shortcuts: one key each, no Shift, keys other apps use

- **Status:** Accepted
- **Date:** 2026-10-23

## Context

With shortcut mode (ADR 0016) every default leader key is pressed on its own. The defaults
had grown capitals and Shift symbols (`M`, `S`, `Z`, `H J K L`, `X`, `F`, `B`, `W`, `R`, `V`,
`N`, `?`, `{ }`) and letters that did not say what they did (`j` Inbox, `p` split). The user
asked for keys that are sensible and that many apps use, with no overlaps.

## Decision

The defaults are `DEFAULT_PREFIX_KEYS` in `src/keys.rs`: one key per action, no key bound
twice, none needing Shift, and where another app has a habit, its key: `t` new tab and
`Ctrl+W` close it (browsers), `x` close pane and `1`–`9` tabs (tmux), `v` / `h` split side by
side / stacked (the user's choice: the line between the panes is vertical / horizontal), `c`
copy mode, `p` find a file (editors' Ctrl+P), `/` search, `,` settings (VS Code), `F1` help,
`Ctrl+V` paste, `Ctrl`+arrows resize, `Space` search commands. Hide sidebar, prompt jumps, new
worktree, rename workspace and send-prefix have no default key; search commands finds them.
A test (`default_shortcuts_are_one_key_each_and_none_needs_shift`) holds the rules.

## Alternatives considered

- **vim's `v` / `s` splits:** `s` is Session; the user chose `v` / `h`.
- **tmux's `%` / `"` splits:** both need Shift.
- **Keep the old keys:** capitals and symbols the user found hard to remember.

## Consequences

Old habits break (`j` is now `i`, `p` is now `v`); a user's own `[keys.prefix]` entries still
load and win. Supersedes the key choices in ADR 0009, not its rule that only kept features
have keys.
