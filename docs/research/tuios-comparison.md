# TUIOS vs Seshi: what to take, what to leave

Prepared for Cody. Research only; no code was changed. Every claim about TUIOS cites a
numbered source in the list at the bottom, with quotes where it matters. Anything I could not
confirm is marked **[unverified]**.

## Summary

- TUIOS is a Go terminal multiplexer and window manager with a daemon, nine workspaces, BSP /
  master-stack / scrolling tiling, tape scripting, hooks, SSH and a browser terminal [1][2].
  It also has an agent-state tracker and an Inbox [8][9]. It overlaps Seshi more than most
  multiplexers do.
- Its leader is a **prefix menu (which-key)**. After the leader it lists the next keys in
  sections. A key marked `+` opens a further menu [3]. Menus are declared in TOML as named
  tables (`open_menu:NAME`), can nest, and can be `sticky` [3]. This is a real key tree.
- Seshi already has the pieces of a which-key: `ui.which_key` ("Leader waits for next key",
  **off by default**, 600 ms) shows the whole key map after a pause. It has exactly one
  hard-coded second step (Worktrees ›). What it lacks is a **tree that is data**: config-defined
  groups, any depth, the popup narrowing as you press.
- Top recommendations: (1) make the leader's key map a real tree, in-place narrowing, driven by
  config; (2) turn `ui.which_key` on by default with a short delay once the tree exists;
  (3) add `keys doctor` / `keys explain` style diagnostics. Detail and risks are in
  *Shortlist*.

## 1. TUIOS leader and which-key, in depth

### 1.1 The leader

- Default leader is `ctrl+b`. Presets: tmux `ctrl+b`, screen `ctrl+a`, zellij `ctrl+g` [3].
- Press the leader twice in terminal mode to send it to the pane (`keybindings.leader_twice`;
  `send` default, or `last_workspace`) [3].
- `ctrl+c` cancels the leader under every preset; `esc` and `ctrl+c` close a menu and "are never
  sent to the pane" [3].
- TUIOS is **modal**: window management mode (tuios keys) vs terminal mode (keys go to the pane).
  The leader is the way to reach tuios keys from terminal mode [2][3][4]. Seshi has no modes; the
  leader is the only entry. This is a deliberate Seshi difference (no bare-letter actions).

### 1.2 How the key tree is modelled

From the keybindings doc [3]:

- Sections after the leader: Windows, Panes, Sessions, Modes, Menus, Tools, and Agents "once an
  agent has run".
- "A key marked with `+` opens a further menu."
- Custom menus are named TOML tables. Quoted from [3]:

```toml
[keybindings.window_prefix]
"open_menu:split" = ["s"]

[keybindings.menus.split]
title = "Split"

[keybindings.menus.split.keys]
prefix_split_vertical = ["y"]
prefix_split_horizontal = ["x"]
"open_menu:resize" = ["R"]

[keybindings.menus.resize]
title = "Resize"
sticky = true

[keybindings.menus.resize.keys]
resize_pane_left_5 = ["h"]
...
```

- Model in one line: **action name -> list of keys**, and a pseudo-action `open_menu:NAME`
  jumps to another named menu. `open_menu:NAME` "goes in any section: after the leader, in
  another menu, or" (the quote is cut off in my fetch **[unverified]** past "or"). So depth is
  arbitrary.
- Examples in the docs: "`Ctrl+B t s y` splits the pane. `Ctrl+B t s R h h h` narrows it by
  three steps." [3]
- Menu names: "lowercase letters, digits, `-` and `_`" [3].
- Validation: "A config warning names a key that opens a menu that is not declared, and a menu that
  no key opens." [3]
- **Sticky**: "`sticky` keeps the menu open after a key runs, as tmux `bind -r` does, until `esc`
  or a key" (the menu does not bind) [3]. Good for resize.
- **Repeat window** (separate from sticky): "The prefix stays armed for half a second after a
  command worth repeating"; `appearance.prefix_repeat_time` changes it, zero turns it off. By
  default only arrows and resize keys repeat; `keybindings.repeat` replaces the list [3].
  "A letter you type right after `ctrl+b p` goes to the shell." [3]

### 1.3 What the popup shows

- "The prefix menu (which-key) shows the keys after the leader in sections", "in as many columns as
  the screen holds" [3].
- "On a screen too narrow for every column, the descriptions are cut before any key is left
  out." [3] (Keys survive; labels shrink. A good rule.)
- Each menu has a `title` [3]. Whether the title, a breadcrumb of keys pressed so far, or
  the section layout changes on screen when you enter a submenu is **[unverified]**: the docs say
  "opens a further menu" but do not describe the transition, and I did not see screenshots.
- A leader indicator is separate: `appearance.dock_prefix_indicator` is `pill` ("The mode pill
  shows the armed leader. This is the default") or `badge` ("A `PREFIX` badge shows in the corner
  across from the dock") [5]. v0.10.1 notes list "a mode pill that shows when the leader key is
  waiting" [10].

### 1.4 Timing

- "Press Ctrl+B and wait about half a second. 'a panel lists every key the prefix accepts
  next.'" [4] So the popup is **delayed by about 500 ms**, and a fast typist never sees it.
- The README names the toggle `appearance.whichkey_enabled` [2]. Its default, and whether the
  delay is configurable, is **[unverified]**: the configuration doc [5] fetched did not mention
  it (I read the first 100,000 of 125,379 characters, then the tail returned nothing).
- There is no leader timeout: "The page doesn't define a prefix timeout." [3] Only the
  repeat window above. Esc or ctrl+c cancels.
- `Ctrl+B ?` opens the full help; `Ctrl+P` searches all commands [4]. So: pause -> tree popup,
  `?` -> everything, palette -> search. Seshi has the same three-way split.

### 1.5 Discoverability and customisation

- Presets (`tmux`, `screen`, `zellij`) change what is under the leader [3]; `U` in window mode
  opens a preset picker (v0.10.1) [10].
- In-app **Keybind Manager** on `Ctrl+B k` [2][3]; writes "to the file that sets the action" [3].
- CLI: `tuios keybinds list` (one table per scope), `tuios keybinds explain <key>` ("everything
  one key does"), `tuios keybinds doctor` ("for conflicts": dead duplicates, preset keys your
  bindings take; `list --json` has a `shadowed` field) [2][3].
- Conflict rule: "When two entries have the same key, the first entry in `config.toml` runs, and
  the doctor shows the other as dead." [3]
- Scopes: Global, window mode, terminal mode, sidebar rows, agents view, mailbox, copy mode,
  the prefix and its menus, tape playback [3].

### 1.6 How this differs from Seshi today

What Seshi does now (read from source and docs):

| Topic | Seshi now | TUIOS |
| --- | --- | --- |
| Leader | `Ctrl+Space`, one key then done; `Mode::Prefix { since }` in `src/client/input.rs` | `ctrl+b`, multi-key sequences |
| Binding model | Flat: `Keymap.prefixed: HashMap<KeySpec, Action>` in `src/config.rs`; user entries `[keys.prefix] C = "spawn-right:claude"` | Tree of named menus; `open_menu:NAME` |
| Second step | One, hard-coded: `Step::Worktrees` in `keyoverlay.rs` (`step_items`), keys `n s m d`, `Backspace` returns | Any depth, user-declared |
| Pause popup | `ui.which_key` (default **off**) and `which_key_delay_ms` (600); shows the *entire* key map (`keymap_shown` in `keyoverlay.rs`) | ~500 ms, shows the *next level only* [4] |
| After popup shows | Unbound letter starts a search; `Tab` swaps map/search; `?` all keys | `?` full help, palette for search |
| Narrowing as you press | No. A key runs and closes, except `›` which opens the Worktrees step | Yes, by menu |
| Sticky / repeat | `Action::repeats()` re-arms the leader with no popup (`since` set 60 s back) | `sticky` menus and a 500 ms repeat window [3] |
| Diagnostics | Settings > Keys page; `warnings` on `Keymap` | `keybinds list / explain / doctor` |
| Presets | None | tmux / screen / zellij [3] |

Read together: Seshi shows "the map, all at once" (docs/UI.md "One key overlay"); TUIOS shows
"the next level, as you go". What Cody is asking for is the TUIOS behaviour.

## 2. Other TUIOS features, briefly

All from the cited pages; where a feature is not in Seshi it is only a candidate if listed in §5.

- **Daemon and persistence.** Daemon owns shells and a terminal emulator per pane; detach keeps
  them running; a dropped client counts as detach [6][7]. After a daemon restart or crash:
  layout, cwd, scrollback restored, **running programs are not**: "Every window gets a fresh
  shell." [6] On Windows, "working directories are not recovered" [6]; daemon needs "Windows
  build 17063 or later" [6]. Multiple clients can attach, "every attached client has full
  control" [7].
- **Workspaces.** Nine; `alt+N` switches, `alt+shift+N` moves and follows, `ctrl+alt+N` sends [3].
  Layout is kept per workspace [11]. (Seshi's sidebar of projects/sessions replaces this.)
- **Tiling.** BSP (default), master-stack, scrolling (niri-style), floating [11].
- **Tape scripting.** Plain text, one command per line, `WaitFor` / `Expect`, `tape validate`,
  `tape exec` against a running session; project `.tuios.tape` "won't run until you review it and
  trust it. Trust is tied to the file's content hash" [12]. (Compare Seshi's repo checks approval,
  ROADMAP.)
- **Hooks.** `[hooks]` shell commands on events (`after-agent-state`, `after-workspace-switch`,
  ...) with `TUIOS_*` env vars; "No timeout"; edits need a restart [13].
- **Control protocol.** JSON lines over the daemon's Unix socket, 115 verbs, no token, access by
  socket permissions; verbs that act as the person need a `human_nonce` that pane callers are
  refused [14].
- **SSH / web.** `tuios ssh`, remote hosts (`tuios hosts add`, `attach --host`) [6][1];
  `tuios-web` serves a browser terminal; "A browser that opens `tuios-web` gets a shell as the
  user who runs it", non-loopback needs TLS and a password [15].
- **Agents.** Seven states (`working`, `needs_input`, `idle`, `done`, `errored`, `unknown`, `none`)
  with a ranked trust model: "A source may write over a claim ranked at or below its own and
  never over one ranked above it" [8][9]. Process detection polls every two seconds; a `working`
  pane silent for 30 s may become `unknown` [8]. Desktop/sound/shell alerts, `suppress_focused` [8].
  State "survives a detach but not a daemon restart" [8].
- **Inbox.** One list of approvals, questions, mail, errors, finished turns, resumable
  conversations, across sessions and machines; `Ctrl+B o` goes to the oldest item; answer with
  digits/`a`/`d`; held approvals for Claude Code and a few others; "A failure can never approve";
  risky commands need a second press within 3 seconds [9][16].
- **Hints** (`Ctrl+B F`): short labels on URLs, paths, hashes, IPs; type a label to copy or open
  [17].
- **Agent messaging.** Agents mail each other; bodies fenced as untrusted data; a pane cannot send
  as `human` [18].
- **Release cadence.** v0.9.1 to v0.10.1 in about nine days (8 to 17 Oct) [10]. Windows items in
  those notes are small; one says some fixes are "not yet tested on a Windows desktop" [10].

### Pain points (issues)

The open-issues list [19] shows no which-key or leader issues. Open items I could see: #1058 slow
mouse handling; #1048 extra block cursors flash during Neovim redraws; #1045 Cmd-scroll overlay;
#151 keyboard focus adds empty lines on macOS and no keyboard path resizes on Linux; #1016 paste
from the buffer chooser can land after keys typed right after Enter; #585 make review-overlay keys
bindable; #895 wrap directional focus. I read titles only, not threads, and the list may be
partial **[unverified]**. No Windows-specific bug was listed.

## 3. What Seshi already does better

Judged from the docs above versus Seshi's AGENTS.md / UI.md / ROADMAP.md. These are Seshi's
stated properties, not measured comparisons.

- **Built for agents first.** Seshi's sidebar groups sessions by project with status as the
  organising idea; TUIOS has agents as a feature among many (workspaces, tiling, floats).
- **Sessions survive, including programs.** Seshi's daemon keeps agents *running* when the UI
  closes and resumes supported agents. TUIOS after a daemon restart gives "a fresh shell" [6].
  (TUIOS also keeps running programs across *detach*; the difference is restart/crash.)
- **Trust boundary.** Seshi accepts status "only from the pane itself: its process tree, or its
  `SESHI_PANE_TOKEN`" (AGENTS.md non-negotiable 5). TUIOS's control socket has "no token or
  password" and relies on file permissions plus pane grants [14]; its sender identity for messages
  "is not verified" [18]. Seshi's posture is stricter.
- **One tidy surface.** Seshi has one key overlay, one Menu, no modes and no bare-letter actions
  (ADR 0014). TUIOS has a modal split, nine scopes of bindings [3], and a command palette,
  launcher and keybind manager as separate entries.
- **Checks, worktrees, review.** Seshi's per-session worktrees, Changes/merge/PR flow and repo
  checks with approval have no equivalent in the TUIOS docs I read. (TUIOS has `review-diff`,
  `ship-commit/merge/pr` verbs in its protocol [14]; I did not see how they look in the UI
  **[unverified]**.)
- **Windows is first-class** in Seshi's non-negotiables with a cross-check gate. TUIOS publishes
  Windows binaries and daemon support but documents a cwd gap [6] and has untested fixes [10].

## 4. Things to avoid

- **Modes** (window vs terminal). They are the root of "which keys am I in?" and conflict with
  Seshi's "no bare-letter actions" rule. Do not import.
- **Nine scopes of keybinds** [3]. Seshi's single leader namespace is simpler; keep it.
- **A control socket with no token** [14]. Not compatible with non-negotiable 5.
- **Browser terminal that gives "a shell as the user"** [15]. Out of scope; large attack surface
  against `docs/SECURITY.md`. (Terminal-first rule also says no separate GUI client, ADR 0011.)
- **Hooks with no timeout and shell strings** [13] if ever added: spawn off the UI thread
  (non-negotiable 6) and cap time.
- **Many layout engines** (BSP, master-stack, scrolling, floating) [11]. Not Seshi's job.
- **Letting prefix-repeat swallow shell typing.** TUIOS has to document "A letter you type right
  after `ctrl+b p` goes to the shell." [3] Keep Seshi's repeat opt-in and short.

## 5. Shortlist (prioritised)

Effort: S under a day, M a few days, L a week or more. Flags are against AGENTS.md
non-negotiables: (1) daemon owns sessions, (2) bump `PROTOCOL_VERSION` on message-shape change,
(3) Windows first-class, (4) terminal-first, (5) status trust, (6) no UI-thread blocking.
Everything here is client-only, so 1, 2, 5 and 6 are untouched unless noted.

### P1. Key tree as data, with in-place narrowing (the ask)

- **What.** Replace the flat `prefixed` map plus hard-coded `Step::Worktrees` with a tree of
  nodes: leaf = action, branch = titled group with its own keys. After the leader, a pause shows
  only the first level; pressing a branch key swaps the popup to that branch (title = breadcrumb,
  e.g. `Ctrl+Space › worktrees`); a leaf key runs and closes; `Backspace` goes up one level, `Esc`
  closes. Optional `sticky = true` for groups like resize.
- **Config.** Extend `[keys.prefix]`, which today is `letter = "command"`. Candidate: a sequence
  key `"w n" = "new-worktree"` (flat and easy) or TUIOS-style named tables. Recommend the
  sequence form first: it keeps existing files loading and avoids a second naming scheme. A branch
  title could come from `[keys.groups] w = "Worktrees"`.
- **Defaults.** Make today's Worktrees step (`n s m d`) the first real branch. Candidates for
  grouping (Cody's call, not mine): Tabs, Project (Files/Changes/Find/Branches), resize.
- **Files.** `src/config.rs` (`Keymap`, parse of `[keys.prefix]`), `src/client/input.rs`
  (`Mode::Prefix`: carry a path of keys, not just `since`), `src/client/screen/keyoverlay.rs`
  (`Step` becomes a node path; `step_items` becomes a tree lookup; `key_groups()` stays the
  source for the full map), `docs/UI.md` (One key overlay), `docs/shortcuts.md`, README `[keys.prefix]`.
- **Effort/risk.** M-L. Risk is in conflicts (a key that is both a leaf and a prefix of a longer
  sequence) and in not breaking old `[keys.prefix]` entries. Settings > Keys page writes
  single-letter bindings (`view_keys.rs` `write_at(["keys","prefix",...])`) and needs to learn
  sequences.
- **Flags.** None of the six. It touches UI.md's rule "one key per action (no alias keys)": a
  sequence is still one key path per action, so the rule holds. Changing a filled
  project-specific/UI rule needs Cody's approval first (AGENTS.md "Maintaining these docs").

### P2. Show the next level only, after a short pause; consider default on

- **What.** The pause popup shows the *current node's* keys (§1.3), not the whole map. Keep `?`
  for everything and Space for search. Drop `ui.which_key` from "off by default" to on at
  about 400-500 ms, since TUIOS does the same [4] and a fast typist never sees it.
- **Files.** `src/config.rs` (`which_key` default, `which_key_delay_ms`), `keyoverlay.rs`
  (`keymap_shown`), `src/client/tests.rs` (the test asserting the default is off, ~line 3884),
  `docs/UI.md`.
- **Effort/risk.** S once P1 exists. Risk: the default flip changes behaviour for everyone; the
  history shows the setting was deliberately renamed and left off, so ask why (open question 1).
- **Flags.** None.

### P3. Narrow-screen rule: cut descriptions before keys

- **What.** In the popup, truncate labels, never drop a key [3]. Check Seshi's two-column
  fallback and 80x24 minimum.
- **Files.** `keyoverlay.rs` (column layout, `THREE_COLUMNS_FROM`).
- **Effort/risk.** S, low. **Flags.** None (UI.md says popups clamp to the screen).

### P4. `seshi keys doctor` / `explain`

- **What.** CLI that lists duplicate keys, keys shadowed by sequences (important after P1), keys a
  terminal cannot deliver, and what one key does. Reuse `Keymap.warnings`.
- **Files.** `src/config.rs`, CLI entry in `src/main.rs` (not read), new section in docs.
- **Effort/risk.** S-M, low. **Flags.** Windows (3): the "keys a terminal cannot deliver" part
  (Ctrl+Space on Windows, Caps Lock behaviour already in `docs/shortcuts.md`) must be right on all
  three OSes.

### P5. Sticky groups and a short repeat window

- **What.** `sticky` for resize/focus groups. Seshi already re-arms for repeating actions
  (`Action::repeats()`), without showing anything. Make that a visible group.
- **Files.** `input.rs` (`Mode::Prefix`), `keyoverlay.rs`.
- **Effort/risk.** S after P1. Risk: swallowing typed letters meant for the pane (§4).
- **Flags.** None.

### P6. Key presets (tmux / screen)

- **What.** Optional `keys.preset = "tmux"` for people arriving from tmux [3].
- **Effort/risk.** M, medium: needs a mapping table and docs; many Seshi actions have no tmux
  equivalent. Lower priority; Seshi's audience is agent users first.

### P7. Hints (label URLs/paths in a pane, then copy/insert)

- **What.** [17]. Fits "never leave seshi" and the files flow (insert a quoted path into the
  agent prompt).
- **Files.** New client mode, `src/client/screen/`; reads the pane's screen from the client's
  `vt100` copy.
- **Effort/risk.** M-L. **Flags.** Clipboard and "open" actions need care on Windows (3) and under
  `docs/SECURITY.md` (URLs opened from untrusted pane output).

### Considered and not recommended now

- Tape scripting: only worth it if Cody wants scripted session layouts; overlaps the roadmap's
  checks work. Would need an ADR.
- Hooks: Seshi has alerts and agent status already; hooks would be a new public contract with
  security review.
- Workspaces, tiling variants, web terminal, agent mail: see §4.

## 6. Open questions for Cody

1. `ui.which_key` is off by default and was renamed "Leader waits for next key". Was it turned off
   on purpose (annoying when typing fast, or a Windows issue), or just never revisited? This
   decides P2.
2. Sequence keys in `[keys.prefix]` (`"w n"`) or named menu tables like TUIOS? I lean sequences.
3. Should the pause popup show the next level only, or keep the full map and highlight the next
   level? Full map plus highlight is closer to today's overlay and lower risk.
4. Which branches belong in the shipped defaults besides Worktrees (Tabs, Project, resize)? ADR
   0014 says "one key per action"; do you want to move existing single-letter keys under branches,
   or only add branches for new things?
5. Should Backspace go up a level, as in today's Worktrees step, or Esc? (Esc closes today.)
6. Is a visible breadcrumb (`Ctrl+Space › w`) enough of a leader indicator, given the pill at the
   top right of row 0 (UI.md)?
7. Do you want any preset support (tmux), or is that out of scope for Seshi's audience?
8. Is the Inbox/needs-you work already ahead of TUIOS's for your taste, or do you want the "oldest
   item that needs you" jump key (`Ctrl+B o` there [16])? Seshi's Inbox is at leader `j`.

## 7. What I could not verify

- Default value of TUIOS `appearance.whichkey_enabled`, and whether the which-key delay is
  configurable (not in the pages fetched).
- What the popup looks like when moving between nested menus (no screenshots; only text).
- The truncated end of the `open_menu:NAME` sentence in the keybindings doc [3].
- Whether TUIOS's key handling has Windows-specific problems (none listed in the issue titles; I
  did not open issue threads).
- The last ~22,000 characters of the releases page and the last ~25,000 of CONFIGURATION.md;
  large parts of AGENT_STATE.md (only the first 100,000 of 316,824 characters read).
- `src/client/screen/keys.rs` was named in the brief but does not exist; key bindings live in
  `src/config.rs` (`Keymap`) and `src/client/input.rs`. I did not read all of those files; claims
  about them are from the lines quoted in §1.6.
- tuios.dev/docs/agent-inbox and the docs site pages were read via a page summariser (not the raw
  HTML), so wording in quotes is as returned and could differ slightly from the live page.

## Sources

Fetched 2026-10-23 with the WebFetch tool.

1. https://tuios.dev/docs (introduction)
2. https://github.com/Gaurav-Gosain/tuios (README)
3. https://raw.githubusercontent.com/Gaurav-Gosain/tuios/main/docs/KEYBINDINGS.md
4. https://tuios.dev/docs/getting-started
5. https://raw.githubusercontent.com/Gaurav-Gosain/tuios/main/docs/CONFIGURATION.md
6. https://tuios.dev/docs/sessions
7. https://raw.githubusercontent.com/Gaurav-Gosain/tuios/main/docs/SESSIONS.md
8. https://tuios.dev/docs/agents
9. https://raw.githubusercontent.com/Gaurav-Gosain/tuios/main/docs/AGENT_STATE.md
10. https://github.com/Gaurav-Gosain/tuios/releases
11. https://tuios.dev/docs/layout-modes
12. https://tuios.dev/docs/tape-scripting
13. https://tuios.dev/docs/hooks
14. https://tuios.dev/docs/control-protocol
15. https://tuios.dev/docs/web
16. https://tuios.dev/docs/agent-inbox
17. https://tuios.dev/docs/hints
18. https://tuios.dev/docs/agent-messaging
19. https://github.com/Gaurav-Gosain/tuios/issues

Seshi (this worktree): `AGENTS.md`, `docs/UI.md` (One key overlay, lines 136-142; General settings
line 124), `docs/shortcuts.md`, `docs/adr/0014-v8-cleanup.md`, `README.md` (Configure),
`src/client/screen/keyoverlay.rs`, `src/client/screen/leader.rs`, `src/client/input.rs`,
`src/config.rs` (`Keymap`, `which_key`).
