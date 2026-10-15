# Plan: Seshi

Source of truth for scope, order, decisions and rules. Tickets hold the detail.
Read this before starting work. If work conflicts with it, stop and say so.
Last reconciled: 2026-10-05 at `9cfde2c` on `dev`.
Verify: `cargo test && cargo clippy --all-targets -- -D warnings && cargo check --target x86_64-unknown-linux-gnu && cargo check --target aarch64-apple-darwin`

Seshi is a terminal multiplexer built for running many coding agents at once: see
which ones need you, jump to them, and keep them running when you leave. See
`AGENTS.md` for the mission and `docs/ARCHITECTURE.md` for how it is built.

## Now

User-requested Codex consistency fix: detect native activity footers with
background-terminal metadata and soft wraps; recognize new turns without
whole-second rounding or a thirty-second startup deadline; distinguish Codex
config/profile flags from Claude continue/print when allocating worktrees.
Acceptance: genuine activity shows working with or without notify hooks,
finished prose and stale footers remain finished, queued questions still need
attention, and new Codex launches with config/profile options get the default
worktree while resume/fork and explicit locations stay put. Status: verified
locally, unreleased. Regressions failed before fixes; the ordered gate passed:
`cargo build`, `cargo test` (279 passed / four ignored), clippy with warnings
denied, and Linux/macOS cross-checks. README and the agent status/worktree contract
are updated. Real-agent live verification remains outstanding; no installed
daemon or agent was replaced or restarted.

User-requested shortcut/documentation audit: All keys must expose every configured
leader/global binding, aliases and contextual controls. Keep the compact leader
overview and add a searchable full reference from Actions, palette and leader + `?`.
Share contextual controls with [shortcuts.md](shortcuts.md); update README for
profiles/workflows, independent selection/hover, status evidence and deferred
update restarts. Status: implemented locally. The ordered gate passed: build,
276 tests passed / four ignored, clippy with warnings denied, Linux and macOS
cross-checks. Rendered reference inspected at 80×24, 100×30 and 160×45; tests cover
all active bindings, aliases, replacements/unbinding, custom commands, key-name
search, contextual search, scrolling and Actions entry without running commands.
Live terminal/clipboard verification remains outstanding as described below.
Release target: v0.30.0.

User-authorized work: [clear session state and workflow controls](features/session-clarity/intent.md)
([spec](features/session-clarity/spec.md)), extending active phase 9. Keep sidebar
selection independent of hover; distinguish update versions; expose status
evidence; prepare workflow instructions; show stages and Inbox approvals; measure
switching, scrolling and resizing under output. Extends existing status/workflow
surfaces, including the existing phase-8 status popover. Updates install without
restarting the window or daemon. Status: implemented locally; the ordered gate
passed (`cargo build`, 274 tests passed / four ignored, clippy with warnings
denied, Linux and macOS cross-checks). Rendered setup, versions, evidence and
Inbox screens inspected; sidebar checks cover sleeping sessions too. The opt-in
release benchmark passed; measurements and limits are in [TESTING](TESTING.md#project-specific-interaction-performance).
Live nested-server and clipboard checks remain outstanding after automatic
approval review rejected the earlier launch as "blocked by policy" without a
more specific reason. Release target: v0.30.0; no remote tickets created.

User-requested status correction: finished Codex prose mentioning its interrupt
shortcut must not count as active work. Remove terminal-bell indicators from the
sidebar and history; retain each session's normal status icon. Acceptance:
completed prose settles to done/idle, real activity stays working, and unanswered
questions stay needs-you. Verification: status and bell regressions fail before
their fixes; rendered rows/history and the full gate passed (265 tests passed,
three existing ignored; lint and Linux/macOS cross-checks passed). Live UI
verification remains outstanding. Release target: v0.30.0; not in v0.29.1.

User-requested update discovery: show the update dot beside **Settings** in
**Actions**, matching the sidebar indicator. Acceptance: hide it without an
update; clicking the marked row reaches the update controls. Verification:
rendered update flow and the full gate passed (263 tests passed, three existing
tests ignored; lint and both cross-checks passed). The missing-badge regression
fails against the previous behavior. The live nested-server check was blocked
by automatic approval review; no live verification claimed. Status: verified
locally for v0.29.1.

Current user-requested work: [saved profile workflows](features/workflows/intent.md)
([spec](features/workflows/spec.md)), extending phase 9. Select custom planner,
worker and reviewer profiles; approve the plan; run at most three build/review
rounds; keep durable progress; expose a right-side drawer and CLI/MCP entry points.
Status: implemented for v0.29.0. The full gate passed (263 tests passed,
three existing tests ignored; lint and Linux/macOS cross-checks passed). Named
servers verified approval, restart recovery, handoffs, completion and the
three-round limit with controlled stub agents. The live 80×24 UI check covered
mouse tabs, editing, task preview/cancel and detach. Real-agent daily use remains
a follow-up after release.

Also requested: fix top-to-bottom repaint during ordinary session switches and
show Codex's queued unanswered questions as needs-you while its spinner continues.
Verify frame presentation, navigation and question-clearing regressions; retain
the separate sleep/wake animation.
Status: regression checks passed, including failures against the prior behavior;
outer frames use synchronized updates. Subjective smoothness still needs checking
in the user's normal terminal with its real sessions.

User-requested navigation fixes: the sidebar's **+ new** must open the same
plain shell as leader + `n`; **Actions** must include Settings. Acceptance:
mouse and keyboard reach the same actions, including the selected sidebar
project. Verification: regression tests, the full gate, and a nested-server UI
check. Status: verified for 0.28.1 (253 tests passed, three pre-existing tests
ignored; lint, both cross-checks and dependency audit passed). Live mouse clicks
opened a plain shell and Settings from Actions.

Active phase: **9 — one agent runs a fleet for you** (chosen 2026-10-11)
Next: the phase's live check (#32–#38 are on `dev`, released in 0.20.0)

Current user-requested work: expand the built-in themes to cover the 25 requested
families and add a searchable preview browser in Appearance. Acceptance: all
families and documented variants are selectable; search, keyboard paging, mouse
selection, preview, cancel, and saving work; every built-in passes the contrast
audit. Verification: the full AGENTS.md gate and a nested-server UI check.
Status: integrated for 0.28.0 with 63 built-in choices; verified in the release
worktree. Sources: [theme research](theme-sources.md).

Also requested: fix Claude reporting working after its turn ends with a dev
server still running. User decision: working tracks the agent's turn; background
shells do not extend it. Acceptance: Stop and idle-prompt reports remain Done
with an unfinished background command in the transcript; real subagent lifecycle
reports still work. Status: integrated for 0.28.0. The combined release gate
passed (251 tests, three pre-existing tests ignored), including Linux and macOS
cross-checks. Named test daemons showed the completed turn as Done, then Idle
once seen, while its background PowerShell job remained Running.

## Phases

### 1 — Working with agents feels effortless (Batch A) · complete 2026-10-03 at `54855dd`

Quick follow-ups, done-not-seen, smarter status, render speed, find file / search
code, agents moving themselves into worktrees. (Tracked as a checklist before
tickets existed.)

### 2 — Launching and steering agents from one place (Batch B) · complete 2026-10-03 at `54855dd`

Task box, agent presets, `seshi send --wait` / `seshi wait`, live model and
last-prompt per row.

### 3 — Code and environment without leaving Seshi (Batch C) · complete 2026-10-03 at `54855dd`

Branch switcher, review marks in Changes, dev server per worktree with hooks,
prewarmed agents.

### 4 — Reach and extras (Batch D) · complete 2026-10-03 at `54855dd`

SSH remotes, extensions, tabs and splits in the seshi layout, memory view,
notification history, `seshi doctor`, more agent types, BEL tracking.

### 5 — Safe and steady · complete 2026-10-04 at `f3b7ae7`

#1, #2, #3, #4, #5, #6, #7, #8, #9, #10, #11, #12

### 6 — Clean code: one layout's worth of code, in files a person can read · complete 2026-10-04 at `0aba78f`

#13, #14, #15, #16, #17, #18

### 7 — Test-week feedback · in progress

What a week of daily use turns up. Bugs found were fixed straight away (0.3.1–0.6.6). #19 is
done. Chosen 2026-10-05 after comparing with tuios, in this order:

Done: #20 sidebar sections (0.7.0), #21 Inbox (0.7.5), #22 Claude Code agent teams (0.7.7);
#25 teach an agent / Codex setup / DeepSeek built in (0.7.1, 0.7.6); #24 ask-human (0.7.8).

Next, in order:

1. #25 (rest) Hooks for Gemini, opencode, Qwen in one command
2. #24 (rest) Per-pane permissions for what an agent may do through seshi
3. #28 Scrollback: jump between commands (0.8.1), multi-pane copy (0.9.1) — done
4. #26 Layouts: split, grid, main and stack, columns; popups — done (0.9.0); tabs 1-9 are the workspaces

On hold (2026-10-05, the user's call): #31 package managers (AUR name and account to decide),
#23 more machines, #27 graphics, #29 extras, #30 web terminal and SSH server mode.

### 8 — Back, and new: the agent loop from task to merged · waiting on designs

Chosen 2026-10-09. Designs first: `docs/design-brief-v4.md` goes to the designer; nothing here is
built before its design is in. Coming back (cut the same day, wanted after all): quick follow-up,
tickets (GitHub, Linear, Plane), the queue, pull requests and Ship, dev server per worktree,
checkpoints (off by default); SSH remotes and config sync are back already, as they were. New, in this order: two agents touching
the same files, review and merge from the Inbox, why this status, today on the splash, alerts on
your phone (ntfy, off by default).

### 9 — One agent runs a fleet for you · active

[A commander that runs a fleet of agents](features/fleet/intent.md): you give one agent a goal; it starts
helpers, briefs them, checks on them and reports back.

Exit when:
- every ticket below is closed and its change is on `dev`;
- the Verify line above passes on `dev`;
- live check: start a fleet with a goal, the commander starts two helpers, and both sit indented under it in the sidebar.

1. #32 — record which session started a session
2. #33 — `seshi brief` prints the commander's instructions · can run alongside 1
3. #34 — keep the started-by link when a session is replaced · after 1
4. #35 — keep the started-by link across a restart · after 1
5. #36 — helpers sit under the session that started them · after 1
6. #37 — start a fleet from the actions list · after 2
7. #38 — make an agent the commander from its row menu · after 2

### 10 — Borrowed from T3 Code · next

Chosen 2026-10-12, after reading `pingdotgg/t3code`. Done: a worktree's `on_create` finishes before
anything starts in it (0.25.1).

Also built, not yet released: queued follow-ups (the queue, back from the 2026-10-09 cut; sent
when the turn ends, never into a question), named accounts (an agent entry with its own `env`),
snapshots before each turn (checkpoints, back from the same cut; a restore keeps what's there
first), and auto-settle and snooze (decided 2026-10-12: a session idle for `settle_after`, or
whose branch's work is merged, folds into a collapsed `settled` line at the foot of its
project; snooze sends one there until a time you pick; one that needs you always comes back).

Exit check, still owed: each of the four used for real in a day's work, not only in tests.
Known gap: a squash-merged branch isn't seen as merged (it settles on the idle timer).

## In scope

Everything below is shipped on `dev` unless marked.

**Core**
- Daemon owns sessions; detach / reattach; many clients — shipped — `src/daemon/mod.rs`, ADR-0001
- Sessions survive a daemon restart (agents resumed, commands re-run) — shipped — `src/daemon/persist.rs`, `restore` in `src/daemon/mod.rs`
- Windows (ConPTY, named pipes, PowerShell folder tracking), macOS, Linux — shipped — `src/daemon/term.rs`, `src/ipc.rs`
- SSH remotes (`--remote host`) — shipped — `src/ipc.rs` `connect_remote`

**Agents**
- Personal and project agent profiles for Claude Code and Codex, setup UI,
  fresh launches and native workflow exports/imports — integrated for 0.28.0;
  verification gate passed in the release worktree — `src/profiles.rs`, `src/client/profiles.rs`; contract:
  [agent profiles](features/agent-profiles.md). Automatic workflow sequencing is
  outside this slice.
- Status detection: working / needs you / done (until seen) / idle, from screen patterns, OSC progress and agent hooks — shipped — `src/daemon/scan.rs`, `seshi hook` in `src/cli.rs`
- Trusted status reports (process tree or `SESHI_PANE_TOKEN`) — shipped — `src/daemon/term.rs`, `src/cli.rs`
- Agent kinds: Claude, Codex, Gemini, OpenCode, Cursor, Copilot, Amp, Qwen, Aider, Goose, Crush, Droid, Pi, Kiro, Grok, custom `[[agents]]` — shipped — `src/config.rs`
- New agent task box (project, worktree, agent, model, effort), presets and recipes as its choices, prewarm — shipped — `src/client/hydra/dialogs.rs`, `src/client/recipes.rs`
- Agents talk to agents (MCP server `seshi mcp`) — shipped — `src/mcp.rs`
- `seshi send --wait`, `seshi wait`, `seshi read` — shipped — `src/cli.rs`
- At most a set number of helpers working at once (`[mcp] helpers_at_once`) — shipped 0.25.0 — `Daemon::starting` in `src/daemon/mod.rs`
- A commander runs a fleet: one agent starts, briefs and checks helpers, which sit under it in the sidebar — active — [intent](features/fleet/intent.md), #32–#38

**The UI (one layout, ADR-0004)**
- Floating look: rounded cards with gaps, title and state in the border, unfocused cards faded, pill tabs, no app footer; floating or tiled, corners, gap, dim, focus border, pill ends in Settings — done, unreleased — `src/client/hydra/card.rs`, `screen.rs`, ADR-0007
- Sidebar card: projects → sessions, state glyph + name, branch · age, question under it, attention sort, helpers indented under the session that started them, keyboard and mouse, row letter keys, `a actions` / `, settings` foot — shipped — `src/client/hydra/screen.rs` `draw_side`
- Panes: tabs as pills (named in their pill), any number of splits (grid for 3+), drag the gap between them, ✕ in the border — shipped — `src/client/hydra/screen.rs` `draw_session`
- Inbox (what needs you first, type to go anywhere), command palette (plain-word commands), key map (searchable, second steps), actions list — shipped — `draw_goto`, `draw_palette`, `src/client/hydra/leader.rs`
- Right-click menus, confirm before closing, toasts — shipped — `src/client/menu.rs`
- Splash: Resume / New shell here — shipped — `draw_splash`
- Settings popup grouped by section, key rebinding, themes with contrast audit — shipped — `draw_settings`, `src/theme.rs`
- Leader keys (a shell is `n`, only kept features have keys), Alt+arrows without the leader, armed pill in sky — done, unreleased — `src/keys.rs` `DEFAULT_PREFIX_KEYS`, ADR-0009
- Copy on select with toast, copy mode, Ctrl+click paths, paste images — shipped — `src/client/copy.rs`, `src/client/pick.rs`

**Code and git**
- Worktrees per agent, create / move / remove with hooks — shipped — `src/daemon/git.rs`, `src/project.rs`
- Worktree clean-up: ones left behind are swept when nothing lives only there, and a list per repo says why each is still around and removes the ones you tick — shipped — `src/tidy.rs`, `src/daemon/worktrees.rs`, `src/client/hydra/tidy.rs`
- Files (tree, preview, in-place edit, external editor), find file, search code — shipped — `src/client/files.rs`, `src/client/find.rs`, `src/client/views.rs`
- Changes (diff, review marks, commit, open a PR, git init offer), branch switcher — shipped — `src/client/branch.rs`, `src/client/views.rs`
- Worktree hooks (`.seshi.toml` `on_create` / `on_remove`, run once allowed; `on_create` finishes before anything starts in the worktree) — shipped — `src/project.rs`, `Ev::WorktreeCreated` in `src/daemon/mod.rs`
- Tasks (a worktree's review: commit, merge, PR, throw away) — shipped — `src/client/tasks.rs`
- Own worktree for `claude`/`codex` typed into a shell in a repo's main folder — shipped 0.9.12 — `seshi agent-dir`, `daemon/term.rs` `agent_functions`
- Usage and limits meter, context per agent, continue after a limit — shipped 0.10.0 — `seshi statusline`, `src/daemon/usage.rs`
- The wake-up transition (rain, a dissolve from the centre, the screen back line by line; `ui.wake_animation`) — shipped 0.27.0 — `src/client/hydra/wake.rs`, spec in `docs/design/wake/`
- A session's share of the machine (CPU and memory) in its pane's title when it's large — shipped 0.25.0, moved off the sidebar row 0.26.2 — `src/daemon/scan.rs`, `load_note` in `src/client/hydra/screen.rs`
- Merge that cleans up (the worktree and its branch go) — shipped 0.11.0 — `src/client/view_keys.rs`

**Identity**
- Renamed hydra → Seshi: command, folders, `SESHI_*`, `.seshi.toml`, MCP, releases; one-time move of an old install — done, unreleased — `config::migrate_old_names`, ADR-0008
- Seshi Walnut (default), Seshi Night and Seshi Day themes; block SESHI wordmark on the splash — done, unreleased — `src/theme.rs`, `src/client/hydra/splash.rs`

**Extras**
- One alert when an agent needs you or finishes: seshi's own two sounds and a note in the window; a desktop notification only with no window open; notification history — shipped — `src/alert.rs`
- Click a notification to jump to its session — shipped — `src/reveal.rs`, `src/alert.rs` (#19)
- Sidebar groups sessions by where they work now (repo, or folder outside git); no projects to open; splits stay with their session — shipped — `src/client/hydra/mod.rs`
- Releases and installers (Windows, macOS, Linux; checksums), `seshi update` and a daily update check — shipped — `.github/workflows/release.yml`, `install.sh`, `install.ps1`, `src/update.rs`
- CI: test and clippy on Linux and Windows, a macOS check — shipped — `.github/workflows/ci.yml`
- Config sync across machines (local file never synced) — shipped — `src/sync.rs`
- SSH remotes (`--remote host`) — shipped — `src/ipc.rs` `connect_remote`
- `seshi doctor` — shipped — `src/cli.rs`

## Out

- Features cut 2026-10-09 (the user's call: they repeated something else or weren't used): open project, go to, next agent that needs you, arrange panes, undo automatic workspace, the map, ideas, races, run a preset as a list (presets stay in the new-agent dialog), past chats, agent tools, extensions, and the tmux stand-in for Claude agent teams. Bringing one back needs a new decision. Memory per session was on this list and came back 2026-10-12 as the load line under a busy session (the user's call, after a day of finding the heavy agent by hand); races were asked about again the same day and stay cut. (Talk / reply / answer, the quick prompt, tickets, the queue, checkpoints, SSH remotes, config sync, the dev-server runner and the pull-request view with Ship were cut the same day and are coming back in phase 8, redesigned.)
- Desktop notifications while a seshi window is open, the terminal bell on an alert, and choosing among system sounds (or your own sound file) — cut 2026-10-11 (the user's call: three overlapping alerts, two of them the Windows error ding). Bringing one back needs a new decision.
- Other UI layouts (workspaces, tree, dock, sidebar) — declined 2026-10-03: one layout to build and test (ADR-0004, now ADR-0007: floating and tiled are styles of it). Bringing them back needs a new decision.

## Later

- Personas (a saved name, brief, agent, model and permissions) — deferred 2026-10-11: the user wants to think about them more. Comes back when the user decides what a persona holds.
- The designed team view, threads between agents, take over and hand back, pause the fleet, stop all, cost per fleet — deferred 2026-10-11: waiting on the design from [design brief v5](design-brief-v5.md). Comes back when the prototype is in.

## Decisions

- [ADR-0001](docs/adr/0001-daemon-owns-sessions.md) — A daemon owns every session; clients only draw. Rules out: sessions inside the UI process.
- [ADR-0002](docs/adr/0002-portable-pty-and-vt100.md) — portable-pty (ConPTY) and vt100 for terminals. Rules out: a home-grown PTY layer.
- [ADR-0003](docs/adr/0003-msgpack-over-local-sockets.md) — msgpack frames over local sockets, versioned. Rules out: unversioned message changes; network listeners.
- [ADR-0009](docs/adr/0009-leader-keys-after-the-cut.md) — Leader keys: a shell is one key, and only kept features have keys (supersedes ADR-0006). Rules out: keys for features that don't exist; defaults that differ from the key map shown in the app.
- [ADR-0008](docs/adr/0008-rename-to-seshi.md) — The app is called Seshi; an old hydra install is brought over once. Rules out: new code or paths under the old name.
- [ADR-0007](docs/adr/0007-floating-and-tiled-styles.md) — One layout, drawn floating or tiled (supersedes ADR-0004). Rules out: other layouts, styles that move or hide parts of the UI.

## Rules

- The daemon's socket only accepts the user who started it — `src/ipc.rs` — SECURITY.md
- Status reports need the pane's token or its process chain — `src/daemon/mod.rs` hook handling — AGENTS.md non-negotiable 5
- Nothing in a cloned repo runs without the user's approval — `src/project.rs` — SECURITY.md
- Client and daemon refuse to talk across protocol versions — `src/protocol.rs` `PROTOCOL_VERSION`, checked on `Hello` — ADR-0003
- Config always loads the seshi layout — `src/config.rs` `Config::load` — ADR-0007
- Every built-in theme passes the contrast audit — `cargo test` (theme audit test in `src/theme.rs`) — readability
- Both platform sides compile — `cargo check --target x86_64-unknown-linux-gnu` / `aarch64-apple-darwin` — AGENTS.md non-negotiable 3
- clippy is clean — `cargo clippy --all-targets -- -D warnings` — CODE-STANDARDS
- A session's "started by" is only set by the daemon, from a pane whose secret checked out — `src/daemon/mod.rs` `starter`, tests in `src/daemon/logic_tests.rs` — AGENTS.md non-negotiable 5

## Records

Docs: `AGENTS.md`, `docs/` (architecture, standards, testing, security, UI, API patterns), `docs/stack/rust.md`.
Design references: [design brief](design-brief.md), [design brief v3](design-brief-v3.md).
Glossary: none yet.
Feature docs: `docs/features/`. Each live intent is linked from In scope or a phase.
Agent profiles: [usage and contract](features/agent-profiles.md).
Profile workflows: [intent](features/workflows/intent.md),
[spec](features/workflows/spec.md), [usage](features/workflows/usage.md).
Session clarity: [intent](features/session-clarity/intent.md),
[spec](features/session-clarity/spec.md).

## Rework

- none recorded yet.
