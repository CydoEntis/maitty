# 0014. Bring over the v8 cleanup, keeping what the user already decided

- **Status:** Accepted (shipped in v0.41.0; amended by [ADR 0024](0024-the-mode-lives-in-a-bottom-bar.md))
- **Date:** 2026-10-23

## Context

The v8 handoff (a written spec with prototypes for layouts, strings and keys; its files
are in git history, the current design is in [DESIGN](../DESIGN.md)) cleans up
the TUI: a collapsed-sidebar strip, one key overlay, a redesigned Menu, Settings,
toasts, the message popover, and it cuts the fleet commander, the Team tab, the sidebar's
subagent tree and the helper mode. Before it arrived the user made decisions the handoff
does not know about. Where the two disagree, the user's decision wins.

## Decision

Take every v8 screen and cut, except where this list keeps what the user decided:

- **Session marks stay dots.** A session is a coloured `●` (or `○` at rest) with its word;
  working keeps the spinner (Bars preset) and checking its ring (ADR 0013, `ui.glyphs =
  dots`). The handoff's per-state glyphs (`✕`, `✓`) are not used for session rows. Its
  colour roles, row text, red edge bar for failed, and "amber only for needs you" are taken.
- **The Menu keeps lit letters.** Each row is run by the letter lit in its name, as in the
  right-click menus. The handoff's group headers (NEW, GO TO, VIEW, SET UP), its row set,
  and its `Collapse sidebar` / `Hide sidebar` rows are taken; its right-aligned key column
  is not. `m` (after the leader) opens the Menu. "Connect agents" is found by Search commands, and is also
  reachable from Settings › Agents as the handoff says.
- **Message a session is `M`**, not `r`: leader `f` is Files, `r` runs a failed check from
  the sidebar and renames a tab from the leader. The message popover itself is the v8
  design.
- **One update notice in the working area** (the `↑ Update` pill at the top right of row 0)
  plus the splash. The Menu carries no dot for it.
- **Actions go through the leader.** No bare letter acts in the sidebar, its collapsed strip
  or on a session row (no `n`, `c`, `m`, `M`, `i`, `s`, `x`, `r`, `O`, `u`, `z`, `Z`): the
  user kept opening Changes and closing sessions by hitting a letter. Bare keys there are
  navigation only (arrows, `j`/`k`, Home/End, Enter, Esc, 1–9 while the strip has focus).
  Every action has a leader key (Ctrl+Space, then the key). Letters still work inside a
  popup the user opened on purpose (the Menu, a row menu, a confirm).
- **Tab, Shift+Tab and Esc belong to the pane in both layouts.** The sidebar is reached
  from the Menu/leader. Nothing in v8 changes this.
- **Rounded settings stay** (`ui.corners`, `ui.pill_caps`, `ui.round_tabs`); v8 styles
  take their shapes from them.
- **Needs you is amber in every theme, failed red** (ADR 0013 follow-up).

## Consequences

The cuts change a public contract: `PROTOCOL_VERSION` is bumped once for the whole of v8,
and the config keys for removed settings (`team.*`, fleet) are read and ignored, not
rejected, so an old `config.toml` still loads. Work lands in waves (look and feel, then
key overlay and Settings, then the cuts), each ending with the full gate.

## Notes from the key overlay wave

Where the handoff and the rulings above disagreed, the rulings won:

- **Message is `M`, rename tab stays `r`** (the handoff has `r` reply and `R` rename tab).
  `R` stays Rename session.
- **`n` stays a plain shell where you are** (the user's ruling: you open a shell, type `claude`
  yourself, and the launcher puts you in a worktree; a project that is not a git repo is the only
  case with none). The `+ new` chip, a folder row's `+`, the empty row's action and the splash's
  `n` start a shell too. The handoff's Session `n` and Shell here `s` are swapped to match: **`s`**
  starts an agent session straight away (Settings › Sessions `Agent` and `Where`) and **`c`** opens
  the New session dialog (Menu › Session with options…). The Menu lists Shell here first. The
  setting `sessions.plus_starts` is gone (old files still load; the key is ignored). `c` was the
  fleet's key; the fleet has no key now.
- **The Menu has no key column** (the handoff right-aligns keys); its rows are run by lit
  letters, one per row: `s w h p t` / `i c k` / `a d` / `e`.
- **`Space` stays a leader key** that opens the search (it replaced the palette); the
  handoff's key map does not list it, ours does under Seshi.
- **`S` stops the agent**, a leader key added so the sidebar's old bare `s` has a home.
- **Keys dropped as duplicates:** `v`, `|`, `b`, `:` and `Ctrl+V` after the leader. Entries a
  user wrote in `[keys.prefix]` still load.

## Notes from the cuts wave

What was removed, and where the handoff and the rulings above disagreed:

- **Removed:** `Start a fleet` and its goal box, `Fleet commander` in the row menu, `seshi brief`
  (`brief.rs`, `brief.md`), `team.rs`, the `[team]` structs and `validate_team`, the `job` argument
  of `seshi_start`, the sidebar's subagent tree, its inspector (`Mode::SubagentInfo`) and the
  `Hide subagents` row action. The daemon's subagent tracking and the `· N agents` count stay.
- **Old files load.** `[team]`, `[fleet]` and other removed keys in `config.toml` are ignored (no
  `deny_unknown_fields`); a `[keys.prefix]` entry `start-fleet` reads as unbound; a saved session
  with fleet or commander fields loads and the fields are ignored.
- **Helpers (handoff section 4).** The handoff lets `1`/`2`/`3` answer from the selected row. Here
  they work inside the popover only; Enter on the row (or a click on its line) opens it, and Enter,
  `y` and Space do nothing in it. The popover does not name a base branch ("off dev") because the
  daemon does not know one when it asks.
- **One limit.** `[sessions] helpers_at_once` (2, 3 or 4, per session) replaces `[mcp]
  helpers_at_once` (0 to 16, all helpers together). The old key is read only when the new one is
  not set, held to the nearest of 2, 3 or 4 (0, "no limit", becomes 4). It is no longer a setting.
- **Where the answer is kept.** "Always in this project" is `helper-projects.json` in the data
  folder (not the repo): the same trust boundary as `check-approvals.json`.
- **Kept:** helpers still nest under the session that started them in the sidebar (a helper is a
  session; the handoff's cut is the subagent tree, not this).
