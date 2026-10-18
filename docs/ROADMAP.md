# Plan: Seshi

Source of truth for scope, order, decisions and rules. Tickets hold the detail.
Read this before starting work. If work conflicts with it, stop and say so.
Last reconciled: 2026-10-18 at `4167f40` on `dev`.
Verify: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo check --target x86_64-unknown-linux-gnu && cargo check --target aarch64-apple-darwin`

Seshi is a terminal multiplexer built for running many coding agents at once: see
which ones need you, jump to them, and keep them running when you leave. See
`AGENTS.md` for the mission and `docs/ARCHITECTURE.md` for how it is built.

## Now

The published base is v0.32.0. The next release integrates flat rendering, manual-only session recovery, broader helper visibility and native lifecycle hooks, directional splits, and keycap-style shortcut help. The work is on the release branch and must pass the full gate before it is marked shipped.

User-requested pane padding polish: preserve the pane insets while matching the
card and padding to a terminal app's solid surface when at least three screen
corners share an explicit background. Isolated colored panels keep their theme
treatment. Implemented and verified on `dev`: focused UI regressions pass and the
full gate passes (309 tests passed, four ignored; build, clippy and both
cross-checks passed). Shipped in v0.31.0.

User-requested Settings drawer: move Settings from its centered modal into the
existing right-hand sliding sheet. Keep all categories, setting editors, update
controls, mouse access and keyboard navigation. Implemented and verified on
`dev`, shipped in v0.31.0. The rendered UI and mouse category navigation
were checked at 80×24, 100×30 and 160×45. The ordered gate passed: build, 308 tests (four
ignored), strict clippy, Linux x64 check and macOS ARM check. This adds no wire
shape, dependency, migration or permission change.

User-requested working-spinner selection and Pi/DeepSeek scrolling correction:
add an Appearance picker with animated presets, Static and validated custom
frames, preserve saved choices, and investigate wheel/keyboard transcript
scrolling in Pi's regular/fullscreen modes. Also scope Codex helpers to exact
owning threads and retain a compact status/session rail when the sidebar hides.
User-reported spurious sounds also add blocked-sound debounce and exact status
checks in the client and headless daemon. Red status remains immediate.
Spinner selection, exact Codex helper ownership, collapsed rail and sound-race
correction: merged to `dev`, shipped in v0.31.0. Ordered build, full tests (307 passed,
four ignored), strict all-target clippy, Linux x64 and macOS ARM cross-checks
passed; ten Node adapter tests passed. The transient/stale blocker regression
failed before the fix. Client and headless daemon tests prove cancellation on
resumed work and stable focused/helper questions. Rendered frames were inspected
at 80×24, 100×30 and 160×45, including left/right rail, hover/focus, off-screen
attention and custom frames. An isolated live server verified rail expansion
and saved the ASCII spinner through Settings; the saved array was checked on
disk. Its owned processes stopped and reveal registration stayed unchanged.
README, shortcut, UI, config, architecture and status contracts are current.
These additions introduce no dependency, migration, permission or wire change;
no FILL decisions were filled. No production session was restarted or real
provider job requested.

Pi scrolling remains open: regular/fullscreen Pi 0.87.1 and default fullscreen
Pi 1.2.0 scrolled with wheel and Page Up using a saved synthetic transcript on
Windows. The user reports the failure on Linux with Seshi 0.30.0/Pi 1.2.0.
Await the failing pane's copied “Pane info (scrolling, mouse)” diagnostic before
claiming a fix there; no speculative scroll-routing change was applied.

User-requested Herdr status audit and Codex working correction: use the pinned
Herdr source to cover its 24 identities and 22 screen manifests, add the missing
OMP/Mastracode/Kilo/Kimi native setup, and scope OpenCode TUI V1/V2 reports to the
selected session. Preserve native question lifecycles, recognize Codex automatic
follow-up work and release hook authority on recognized process exit. Questions
ring in the focused pane; completion waits one second and new turns reset dedup.
Status: merged to `dev`, shipped in v0.31.0. Ordered build, tests
(299 passed / four
ignored), strict clippy and Linux/macOS cross-checks passed; ten Node adapter
tests passed. Rendered status frames were inspected. A named isolated live
server showed the screenshot's wrapped footer working, expanded question blocked,
answer working, completion idle, and process exit with no agent status. Its
owned daemon/panes were stopped; the reveal registration remained unchanged.
No paid provider job or production session was started/restarted. The source
audit records remaining differences instead of claiming identical behavior.
New dependency: pinned jsonc-parser 0.33.1 for comment-preserving OpenCode config.
Apache source notices ship with release archives. No wire shape, migration or
permission changes; no documentation decision markers were filled.
See [audit and verification limits](herdr-status-audit.md).

User-requested mouse access and provider-independent status correction: expose
Profiles, Workflows, Runs, status setup and all commands through Actions, retain
Actions without the sidebar/sessions, make palette commands clickable and
wheel-scrollable, and include configured commands. Recognize current Pi/DeepSeek
packages; install Pi/OpenCode native lifecycle adapters through mouse or CLI.
Keep native questions blocked until answered, retain pending OpenCode asks across
busy events and helper completion, and recognize Codex's expanded question dialog.
Needs-you retains the red blocked dot/text; bells do not replace state. Scope
includes the user's z.ai/DeepSeek models inside Pi/OpenCode. Status: merged to
`dev`, shipped in v0.31.0. The ordered build/test (290 passed / four ignored), clippy,
Linux and macOS cross-checks pass, as do three Node adapter tests. Native adapter
tests run in CI on Windows/Linux. An initial Windows run hit an access violation
alongside a PowerShell out-of-memory error; isolated failing cases and complete
subsequent gates passed. Rendered mouse flows fit 80×24, 100×30 and 160×45.
A named isolated live server verified mouse entry, section changes, palette
scrolling, hidden-sidebar access and adapter installation into test directories.
A Pi adapter probe through the real authenticated hook socket remained blocked
beyond the old quiet timeout and became idle on settlement, with event evidence.
No real provider task was launched; existing authenticated Pi/OpenCode sessions
were not restarted. The owned test process tree was stopped. README, shortcut,
UI/API/architecture contracts and [status setup](agent-status.md) are updated;
no dependencies, new permissions or wire shapes were added. Proposed wording
follow-up, awaiting user direction: rename "shared files" to "overlapping files".

User-requested workflow usability correction: navigate Profiles, Workflows and
Runs with mouse/keyboard, retain drafts between sections, open/edit saved items,
choose role profiles with a picker, and read full run reports before explicit
controls. Separate Enter/open from Launch/Run/Open pane; provide Back, cursor
editing, multiline instructions and report scrolling at 80×24. Status: merged to
`dev`, shipped in v0.31.0. Navigation regressions fail before fixes; the ordered build,
test (284 passed / four ignored), clippy and Linux/macOS cross-check gate passed.
Rendered flows inspected at 80×24, 100×30 and 160×45. A named isolated server at
80×24 verified palette entry, keyboard/mouse section changes, role selection,
profile/workflow saves and draft retention; saved values were checked on disk.
Report browsing and controls bound to exact run ids are covered by render/input
tests. The test server was cleaned up; no real agent run was launched, no remote
change was made, and the user's daemon was not restarted. README, usage, UI,
architecture and the embedded shortcut reference are updated.

User-requested Codex consistency fix: detect native activity footers with
background-terminal metadata and soft wraps; recognize new turns without
whole-second rounding or a thirty-second startup deadline; distinguish Codex
config/profile flags from Claude continue/print when allocating worktrees.
Acceptance: genuine activity shows working with or without notify hooks,
finished prose and stale footers remain finished, queued questions still need
attention, and new Codex launches with config/profile options get the default
worktree while resume/fork and explicit locations stay put. Status: verified
locally for v0.30.1. Regressions failed before fixes; the ordered gate passed:
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
([spec](features/session-clarity/spec.md)), shipped in v0.30.0. Keep sidebar
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
([spec](features/workflows/spec.md)), shipped in v0.30.0. Select custom planner,
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

### 7 — Test-week feedback · complete 2026-10-16 at `63b1a72`

All accepted Phase 7 issues are closed and their changes are on `dev`. The five
remaining proposals (#23, #27, #29, #30 and #31) were canceled on 2026-10-17
and their issues closed. They are out of scope, not unfinished Phase 7 work.

### 8 — Agent loop and design v4 · reconciled

The v4 design handoff is implemented and shipped. The later proposed bundle mixed
work that is now shipped, features explicitly cut in [Out](#out), and team/persona
design deferred to v5 in [Later](#later). Phase 8 is historical and does not gate
new work; any remaining item must be re-approved and planned as a new scope.

### 9 — One agent runs a fleet for you · complete 2026-10-16 at `63b1a72`

[A commander that runs a fleet of agents](features/fleet/intent.md): you give one agent a goal; it starts
helpers, briefs them, checks on them and reports back.

Tickets #32–#38 are closed, their changes are on `dev`, and the feature shipped
in v0.20.0. The full verification gate passes on the reconciled base. The live
two-helper sidebar check remains an unverified, non-blocking follow-up in
[Later](#later).

### 10 — Borrowed from T3 Code · complete 2026-10-16 at `63b1a72`

Chosen 2026-10-12, after reading `pingdotgg/t3code`. Done: a worktree's `on_create` finishes before
anything starts in it (0.25.1).

Shipped by v0.26.0: queued follow-ups (the queue, back from the 2026-10-09 cut; sent
when the turn ends, never into a question), named accounts (an agent entry with its own `env`),
snapshots before each turn (checkpoints, back from the same cut; a restore keeps what's there
first), and auto-settle and snooze (decided 2026-10-12: a session idle for `settle_after`, or
whose branch's work is merged, folds into a collapsed `settled` line at the foot of its
project; snooze sends one there until a time you pick; one that needs you always comes back).

The real-day-use check and the known squash-merge detection gap remain visible in
[Later](#later); neither is on the current work path.

### 11 — Optional desktop client, shared session engine · declined 2026-10-17

The user canceled this direction before implementation and chose to keep Seshi
terminal-first, using T3 Code when they want a chat-first GUI. No code, tickets,
branch, or worktree were created. Revisit only through a new proposal.

### 12 — Flat rendering style · in progress on `feat/flat-layout`

Add an optional full-bleed Flat style from the supplied design handoff. Keep the
Cards layout as default and share session state, tabs, splits, sheets, status and
mouse behavior. Acceptance: choose Cards or Flat in Appearance; flat surfaces
have square edges and no gaps, a bottom status line, keyboard focus zones,
working mouse targets and sensible narrow-screen behavior; docs and tests cover
the option; the verification gate passes before the phase ships.

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
- A commander runs a fleet: one agent starts, briefs and checks helpers, which sit under it in the sidebar — shipped in v0.20.0 — [intent](features/fleet/intent.md), #32–#38

**The TUI (one shared session layout; ADR-0007, superseded by ADR-0012)**
- Flat rendering style: square, full-bleed sidebar, tabs, panes and sheets; bottom status line and sidebar/pane focus zones — in progress in Phase 12 — `src/client/hydra/screen.rs`, ADR-0012
- Floating look: rounded cards with gaps, title and state in the border, unfocused cards faded, pill tabs, no app footer; floating or tiled, corners, gap, dim, focus border, pill ends in Settings — shipped by v0.30.1 — `src/client/hydra/card.rs`, `screen.rs`, ADR-0007
- Sidebar card: projects → sessions, state glyph + name, branch · age, question under it, attention sort, helpers indented under the session that started them, keyboard and mouse, row letter keys, `a actions` / `, settings` foot — shipped — `src/client/hydra/screen.rs` `draw_side`
- Panes: tabs as pills (named in their pill), any number of splits (grid for 3+), drag the gap between them, ✕ in the border — shipped — `src/client/hydra/screen.rs` `draw_session`
- Inbox (what needs you first, type to go anywhere), command palette (plain-word commands), key map (searchable, second steps), actions list — shipped — `draw_goto`, `draw_palette`, `src/client/hydra/leader.rs`
- Right-click menus, confirm before closing, toasts — shipped — `src/client/menu.rs`
- Splash: Resume / New shell here — shipped — `draw_splash`
- Settings popup grouped by section, key rebinding, themes with contrast audit — shipped — `draw_settings`, `src/theme.rs`
- Leader keys (a shell is `n`, only kept features have keys), Alt+arrows without the leader, armed pill in sky — shipped by v0.30.1 — `src/keys.rs` `DEFAULT_PREFIX_KEYS`, ADR-0009
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
- Renamed hydra → Seshi: command, folders, `SESHI_*`, `.seshi.toml`, MCP, releases; one-time move of an old install — shipped by v0.30.1 — `config::migrate_old_names`, ADR-0008
- Seshi Walnut (default), Seshi Night and Seshi Day themes; block SESHI wordmark on the splash — shipped by v0.30.1 — `src/theme.rs`, `src/client/hydra/splash.rs`

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

- Features cut 2026-10-09 remain out unless re-approved: open project, go to, next agent that needs you, arrange panes, undo automatic workspace, the map, ideas, races, run a preset as a list (presets stay in the new-agent dialog), past chats, agent tools, extensions, and the tmux stand-in for Claude agent teams. Memory per session later returned as the load line under a busy session. Items explicitly restored since the cut are listed in In scope; races remain cut.
- Desktop notifications while a seshi window is open, the terminal bell on an alert, and choosing among system sounds (or your own sound file) — cut 2026-10-11 (the user's call: three overlapping alerts, two of them the Windows error ding). Bringing one back needs a new decision.
- Separate desktop GUI client — declined 2026-10-17: keep Seshi terminal-first and use T3 Code when a chat-first GUI is wanted. Revisit only through a new proposal (ADR-0011).
- Other structural terminal layouts (workspaces, tree, dock, alternate sidebar) — declined 2026-10-03. Flat is an accepted rendering style of the shared session layout, per ADR-0012; it does not introduce another session model.
- Phase 7 proposals canceled 2026-10-17: named hosts and remote panes/agents (#23); terminal graphics and Kitty keyboard protocol (#27); launcher, templates, recording/replay, screenshots and key display (#29); web terminal and SSH server mode (#30); package-manager distribution (#31). Revisit only through a new proposal. This does not remove shipped SSH remote-client support.

## Later

- Non-blocking validation follow-ups: run the Phase 9 live fleet check with two helpers visible under the commander; use the four Phase 10 features in real work; address the squash-merged-branch detection gap.

- Personas (a saved name, brief, agent, model and permissions) — deferred 2026-10-11: the user wants to think about them more. Comes back when the user decides what a persona holds.
- The designed team view, threads between agents, take over and hand back, pause the fleet, stop all, cost per fleet — deferred 2026-10-11: waiting on the design from [design brief v5](design-brief-v5.md). Comes back when the prototype is in.

## Decisions

- [ADR-0001](docs/adr/0001-daemon-owns-sessions.md) — A daemon owns every session; clients only draw. Rules out: sessions inside the UI process.
- [ADR-0002](docs/adr/0002-portable-pty-and-vt100.md) — portable-pty (ConPTY) and vt100 for terminals. Rules out: a home-grown PTY layer.
- [ADR-0003](docs/adr/0003-msgpack-over-local-sockets.md) — msgpack frames over local sockets, versioned. Rules out: unversioned message changes; network listeners.
- [ADR-0009](docs/adr/0009-leader-keys-after-the-cut.md) — Leader keys: a shell is one key, and only kept features have keys (supersedes ADR-0006). Rules out: keys for features that don't exist; defaults that differ from the key map shown in the app.
- [ADR-0008](docs/adr/0008-rename-to-seshi.md) — The app is called Seshi; an old hydra install is brought over once. Rules out: new code or paths under the old name.
- [ADR-0007](docs/adr/0007-floating-and-tiled-styles.md) — Cards layout, drawn floating or tiled (supersedes ADR-0004); superseded by ADR-0012.
- [ADR-0012](docs/adr/0012-flat-rendering-style.md) — Add a flat rendering style over the shared session layout; Cards stays default.
- [ADR-0010](docs/adr/0010-optional-desktop-client.md) — Offer an optional desktop client. Superseded by ADR-0011.
- [ADR-0011](docs/adr/0011-keep-seshi-terminal-first.md) — Keep Seshi terminal-first. Rules out: adding a separate desktop GUI client to this project.

## Rules

- The daemon's socket only accepts the user who started it — `src/ipc.rs` — SECURITY.md
- Status reports need the pane's token or its process chain — `src/daemon/mod.rs` hook handling — AGENTS.md non-negotiable 5
- Nothing in a cloned repo runs without the user's approval — `src/project.rs` — SECURITY.md
- Client and daemon refuse to talk across protocol versions — `src/protocol.rs` `PROTOCOL_VERSION`, checked on `Hello` — ADR-0003
- Config selects the Cards or Flat rendering style while sharing the session layout — `src/config.rs` `Ui.layout`, `src/client/hydra/card.rs` `Look::of` — ADR-0012
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
