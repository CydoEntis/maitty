# Plan: Maitty

Source of truth for scope, order, decisions and rules. Tickets hold the detail.
Read this before starting work. If work conflicts with it, stop and say so.
Last reconciled: 2026-10-30 on `feat/chat-gui` (v0.51.0; origin/main merged).
Verify: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo check --target x86_64-unknown-linux-gnu && cargo check --target aarch64-apple-darwin`

Maitty is a terminal multiplexer built for running many coding agents at once: see
which ones need you, jump to them, and keep them running when you leave. See
`AGENTS.md` for the mission and `docs/ARCHITECTURE.md` for how it is built.

## Now

Latest release: **v0.51.0** (2026-10-30), wire protocol 50. Phases 1–33 are shipped (phase 11
declined); the release history is in the GitHub Releases and git tags, not here. v0.44.0 added
[Phase 27](#27--radar-and-sending-failures--shipped-in-v0440) (context on the row, a failed
check into the message box), [Phase 28](#28--references--shipped-in-v0440) (references),
`maitty integrate mcp` for Codex, Qwen and Gemini as well as Claude, settings files with
comments, and the phase 29 follow-ups (the Windows short paste, Files' `d`, the side panel's
title, the guide, two flaky tests).

Built 2026-10-27, not released: [Phase 32](#32--two-looks-and-a-sessions-tabs-in-the-sidebar--built-not-released),
two looks and a session's tabs in the sidebar.

Active next development: [Phase 37](#37--claude-continuity-and-initial-gui-release--active),
Claude support and cross-platform GUI release readiness. The standalone Codex GUI
and its design pass are complete. Live GUI-to-native-TUI handoff acceptance stays
deferred until both providers are usable and packaged; the TUI remains supported.
This ordering was chosen by the user on 2026-10-29 ([ADR 0038](adr/0038-standalone-gui-first.md)).
The daily-use trial from 2026-10-26 and open live-validation items remain outstanding.

Open validation items (need a real session on the named platform):
- Pi full-screen scrolling on Linux: waiting on the failing pane's "Pane info (scrolling,
  mouse)" diagnostic before any fix.
- Codex helper detection on both Windows and Linux, after restarting Codex with the hooks.
- Native status for every `maitty integrate` agent has unit tests, not a live run each.
- Flaky under load: `screen_tests::a_toast_clears_after_its_time_or_with_the_close_or_esc` (time-based).
- References: no live nested-server or real drag-and-drop run yet.

## Phases

### 1 — Working with agents feels effortless (Batch A) · complete 2026-10-03 at `54855dd`

Quick follow-ups, done-not-seen, smarter status, render speed, find file / search
code, agents moving themselves into worktrees. (Tracked as a checklist before
tickets existed.)

### 2 — Launching and steering agents from one place (Batch B) · complete 2026-10-03 at `54855dd`

Task box, agent presets, `maitty send --wait` / `maitty wait`, live model and
last-prompt per row.

### 3 — Code and environment without leaving Maitty (Batch C) · complete 2026-10-03 at `54855dd`

Branch switcher, review marks in Changes, dev server per worktree with hooks,
prewarmed agents.

### 4 — Reach and extras (Batch D) · complete 2026-10-03 at `54855dd`

SSH remotes, extensions, tabs and splits in the maitty layout, memory view,
notification history, `maitty doctor`, more agent types, BEL tracking.

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

*Removed by the user in [Phase 19](#19--the-v8-cleanup--shipped-in-v0410-live-checks-open); the record below is history.*

A commander that runs a fleet of agents: you give one agent a goal; it starts
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
first), and automatic session hiding and snooze. Sessions now remain visible until explicitly closed.

The real-day-use check and the known squash-merge detection gap remain visible in
[Later](#later); neither is on the current work path.

### 11 — Optional desktop client, shared session engine · declined 2026-10-17

The user canceled this direction before implementation and chose to keep Maitty
terminal-first, using T3 Code when they want a chat-first GUI. No code, tickets,
branch, or worktree were created. Reopened by [ADR 0041](adr/0041-chat-gui-shared-conversations.md)
as phases 34–37; the earlier decline is superseded.

### 12 - Flat rendering style - shipped in v0.36.0

Add an optional full-bleed Flat style from the supplied design handoff. Keep the
Cards layout as default and share session state, tabs, splits, sheets, status and
mouse behavior. Acceptance: choose Cards or Flat in Appearance; flat surfaces
have square edges and no gaps, keyboard focus zones,
working mouse targets and sensible narrow-screen behavior; docs and tests cover
the option; the verification gate passes before the phase ships.

### 13 — Borrowed from T3 Code, round two · shipped in v0.40.0

Approved by the user 2026-10-19 after a second read of `pingdotgg/t3code`.
Tickets: #116 one function builds an agent's start command, #117 the handoff brief, #118 `maitty handoff`.

- **Handoff:** `maitty handoff <pane> --to <agent>` starts another agent in the same worktree with a
  short text brief (the first prompt, branch, changed files, a tail of recent output). Text only; it
  does not carry the old agent's native context, and says so. Built from `maitty worktree` and
  `maitty send`, so no wire change is expected.
- Phone alerts are not part of this phase: ntfy alerts already ship (`[notify] phone_topic`). An earlier
  draft of this phase listed them by mistake.
- Acceptance: a ticket, tests, docs and a passing verification gate before it is marked shipped.

Status 2026-10-22: built and released in v0.40.0. The gate passed on the merged tree
(575 tests). A live run on an isolated daemon with stub agents checked the brief, the refusals (no pane, no
agent, unknown agent) and the grants. The brief goes in as the new agent's first prompt on its command line,
not through `maitty send` (avoids racing the agent's start screen); four choices made while planning are
recorded in the intent for the user to confirm. Known gaps: the brief uses the pane's name as its task
(the real first prompt is not on the wire); in a main checkout with `worktree.per_agent` the new agent may
land in a fresh worktree; nushell quoting of a prompt with an apostrophe is unchanged from before.

Done outside the phase: saved pane output no longer races at shutdown (saves run one at a time,
newest wins, fsynced) and restored history starts at a clean line.

### 14 — Design v5 cleanup pass · complete 2026-10-22 at `865998b`

The user supplied the v5 cleanup handoff (`maitty-app.js`
is the spec) on 2026-10-19 and approved following it where it differs from older UI rules, including
dropping model, context and cost from Maitty's own chrome. One commit per step, each with the full gate:

1. One background (terminal default or solid) — done.
2. Spacing (1-column margin and gap; sidebar has no footer) — done.
3. Each fact shows once (pane title, status line holds path/branch/PR) — done, except the PR, server and `⇆` tags, which step 6 moves into the status line.
4. Colour jobs (working is never amber; neutral selection tint and `▎` bar) — done (sidebar, sheet and popup lists).
5. Exactly one lit edge — done.
6. Status line in both layouts — done, without the `#PR` and dev-server tags (no data yet).
7. Settings › General sections — done.

Shipped since v0.35.0. The gate passed on `main` at `865998b`.

### 15 — Checks Maitty runs itself · shipped in v0.37.0

Today an agent saying "done" turns its session done even when the project's checks fail. This phase
makes "done" mean something Maitty observed. Chosen by the user 2026-10-22 (verify slice only).
[Intent](features/checkpoints/intent.md).
Tickets, in order (each carries its own tests and docs):

1. #94 protocol: check state and result on a session
2. #95 daemon: run the project's check after a turn ends (after #94)
3. #96 daemon: hold finished alerts and waiters until the check settles (after #95)
4. #97 daemon: re-run and override a check (after #95)
5. #98 UI: the check result on the sidebar row (after #94)
6. #99 UI: why popover shows the check timeline (after #94)
7. #100 UI: evidence sheet (after #94)
8. #101 UI: failed and could-not-run checks in the Inbox (after #94)
9. #102 UI: settings, check command and timeout (after #94)
10. #103 UI: re-run and override from the menu, sheet and Inbox (after #97, #100, #101, #102)
11. #107 CLI and home screen: show a failed check in `ls`, `maitty_list` and the counters (found by the live check)

Later: stale detection and a command supplied by the repo with its approval, both built in Phase 18.

Status 2026-10-22: released in v0.37.0 (tag `v0.37.0` at `97d6d19`). CI passed on Windows, Ubuntu
and macOS (it caught four check tests with `sh`-only commands, fixed before release), so the Windows
runner code now compiles and its tests pass. #94–#103 and #107 are built, with two fixes the live check found
(the Inbox's first failing line; the Flat sheet title). #107 makes `maitty ls`, `ls --json`, MCP
`maitty_list` and the home counters show a failed or running check instead of "done", and the Inbox
and the home screen share one rule for what counts as finished. The gate passed on the merged
tree. A live run on an isolated daemon (before #107) confirmed: no check reads done as before; a
failing check keeps a session from reading done and `maitty wait` exits 4; pass; could-not-run on a
missing command and on a timeout with no child left; the pane token is not passed on; results
survive a daemon restart; re-run, override with confirm, undo and the evidence sheet on a real
screen in Cards and Flat. Validation on Windows, 2026-10-22 (isolated daemons, PowerShell):
pass, fail with `maitty wait` exiting 4, timeout as could-not-run, #107 in `ls` and `ls --json` on a live
daemon, 80×24 frames in all three layouts, and Enter on a row opening the sheet all checked. The ASCII glyph
fallback was broken on the sheet banner, why popover and override (Unicode marks); fixed with a regression
test. Under PowerShell a missing check command exited 1 and read **failed**; Phase 18 starts each
PowerShell check with a not-found trap that exits 127, so it reads could-not-run (other exit codes unchanged). Still not verified: desktop and phone alerts, and a check run by hand on
macOS. On Linux a check's grandchild process can outlive the daemon
(documented in `docs/SECURITY.md`).

- **Scope of the first slice (verify only):** a project may name one check command. When a
  session's turn ends and the session would become done, Maitty runs that command in the
  session's worktree as a background task, records the exit code, the time and the tail of the
  output, and shows the session as done only if it passed. A failure shows `failed: <check>`, appears in
  the Inbox, and keeps the session visible. The user can override a failure; the override is recorded.
- **Opt-in:** a project with no check command behaves exactly as today.
- **Not a workflow runner:** Maitty runs one user-named command and reports the result. It starts no
  role, assigns no task and does not stop an agent from working, so it stays clear of the runner cut
  on 2026-10-18. The user confirmed this reading on 2026-10-22: checkpoints replace the cut
  workflows.
- **Rules it must keep:** the check runs off the UI thread (`spawn_bg`) and survives the UI closing,
  so the daemon owns it; the result is trusted because Maitty ran the command, not because the agent said so; the command runs
  only with the user's approval, like other project-supplied commands (SECURITY.md); it works on
  Windows (PowerShell), macOS and Linux; the saved session and `PROTOCOL_VERSION` change together.
- **Open decisions (ask before building):** where the command is configured (user config or the
  repo); the timeout; whether a re-run happens on every turn end or only when the files changed;
  what the Inbox says for a check that cannot run.
- **Design:** design brief v6 and its handoff, since replaced by the v7 handoff
  (`maitty-checks/checks.js` was the spec; drawn in the Flat style only). Checkpoints must work in
  all three layouts (Cards floating, Cards tiled, Flat) at 80×24, 100×30 and 160×45. Flat is built
  first; floating and tiled follow the designer's versions. Decisions taken from the handoff:
  10-minute timeout, run after every turn end, a separate Inbox group for checks that could not
  run, a re-run requested while the agent is working waits for the turn to end, override covers
  one result only. Reuse the existing theme colours (no new theme roles), keep the sidebar's
  `+ new` / `actions` foot and the Appearance spinner choice, and add re-run and override as
  row-menu items. A command supplied by the repo (`.maitty/checks.toml`) and stale detection are
  separate later tickets.
- Acceptance: tickets, tests, docs and a passing verification gate before it is marked shipped,
  plus a live check where a failing check keeps a session from reading done.

### 16 — Restart the daemon from the update flow · complete 2026-10-22

Found while shipping v0.37.0: the protocol moved from 42 to 43 and the only way forward was
`maitty kill-server`. Ticket #110. After an update that leaves the daemon older, Settings and the update
toast show **Restart now** (capital `R` in Settings), which confirms how many sessions stop and how
many are working; with none open it restarts at once. A new `maitty` that finds an older daemon it
cannot talk to shows a full-screen prompt (**Restart and open** / **Quit**) on a real terminal, and
keeps the old error and exit code everywhere else; a remote daemon is never restarted by it. No
protocol change. Live-checked against the published v0.36.13 daemon on an isolated socket: the prompt
(100×30 and 80×24, by key and by mouse), Quit leaving the old daemon and its session alone, Restart
and open bringing the new daemon up with the session back, and no prompt without a terminal.
Not verified live: **Restart now** against a real older daemon of the same protocol (tests use a
faked snapshot), and Windows beyond CI. Known gaps: the prompt shows protocol numbers, not release
numbers (an older daemon reports only its protocol); it installs no panic hook to switch mouse
capture back off; the offer-then-restart glue in `open_client` has no test of its own.

### 17 — Team roles for the commander · shipped in v0.40.0, removed in phase 19

*Removed by the user in [Phase 19](#19--the-v8-cleanup--shipped-in-v0410-live-checks-open); the record below is history.*

A team list for the commander: a Settings › Team list of jobs
(planner, builder, reviewer, or the user's own), each with a model per agent family (Claude, Codex),
one global family switch and a per-project override. A commander reads the list and starts each helper
on its job's model. Handoffs are gated by Phase 15 checks, which the commander is told to wait on.
Maitty runs no pipeline and normal sessions are unchanged.

The user asked on 2026-10-22 to build it now. Tickets: #113 config, #114 start a helper for a job,
#115 the team in the brief, #119 hand on only after a passing check, #120 Settings › Team with the family
switch, #121 add/edit/remove jobs, #122 docs, #123 live trial on the user's models (manual).

Status 2026-10-22: #113–#115 and #119–#122 built and released in v0.40.0; the gate passed on
the merged tree. Not done, so the phase is not complete: #123, a commander running a small ticket set on the
user's models. Known gaps: no key-driven test that saving a job writes the file (covered below that level);
one click on the Family row while a job is being edited leaves the edit open.

### 18 — The v7 layout and checkpoints · shipped in v0.40.0, live checks open

The user supplied the v7 handoff on 2026-10-22 and
asked for all of it: a new layout for the app, not a pass over the old one. Covers the frame in all three
layouts (Cards floating, Cards tiled, Flat) at 160×45, 100×30 and 80×24 with the drop order below 140 and
100 columns, every check screen, approving a repo's `.maitty/checks.toml` command (Enter, y and Space never
approve; Always needs pressing twice; a changed command asks again), stale passes, and several named check
commands per project. The working state is built too: the "Bars" 3-bar spinner preset (ASCII under the ASCII glyph set) and the name and word shimmer, both frozen when motion is off. Not built: checkpoint kinds
other than a command (commits, plan approval, review), which the design only shapes. Tickets are labelled
`phase-18`. One protocol bump for the whole phase.

Status 2026-10-22: #124–#131 built and released in v0.40.0 (protocol 43→44). The gate
passed on the merged tree (754 + 1 tests). All 26 presets match the handoff's frames in every layout and
size, plus ASCII at Flat 80×24 (`crates/maitty-runtime/src/client/reference_frames_tests.rs`); colour roles were compared by a reviewer.
Two blind reviews (code and security, design fidelity) ran with two fix rounds; their blocking findings
(the approval modal clipping a long command, approving while another checkpoint runs, the Flat ground)
are fixed. Live checks: Windows at 120×32 on an isolated daemon (fail, pass, stale, two checkpoints, the
approval modal's keys, a missing command reading could-not-run); Always and Never only on Linux in Docker,
because Windows has no way to point the approvals file away from the user's own. Also in this phase: the
unix pane kill signals only the pane's own process group (it could reach every process the user owns).
Open decisions: whether `a` on a sidebar row opens the row menu (handoff) or global Actions (today, ADR
0009); `maitty wait`
exits 0 for a check waiting for approval or that could not run; ADR 0013 is still Proposed. Not built:
`· 1 of 59 tests failed` in the why popover (needs output parsing) and when you chose Never (needs a
protocol change). Not checked live: 80×24, 100×30 and 160×45, ASCII and Nerd glyphs, macOS.

### 19 — The v8 cleanup · shipped in v0.41.0, live checks open

The user supplied the v8 handoff and asked for it with
the rulings in [ADR-0014](adr/0014-v8-cleanup.md) where the two differ. One protocol bump for the whole of
v8: 44 to 45. This phase covers the cuts the user asked for by name: the fleet commander (`Start a fleet`,
`Fleet commander` in the row menu, `maitty brief`, `[team]` jobs and their validation, the `job` argument of
`maitty_start`), the sidebar's subagent tree and inspector (the daemon still tracks subagents and a working
row still says `· N agents`), and the helper mode, which becomes a question on the row (`wants to start N
helpers`, Enter or a click opens it; `1` Allow, `2` Always in this project, `3` No, Esc later). Settings ›
Sessions `Sessions may start helpers` and `Helpers at once` now act: `[mcp] helpers_at_once` is folded into
`[sessions] helpers_at_once` and no longer a setting. The collapsed strip, key overlay, Menu, Settings, toasts
and the message popover are the other waves of this phase (see the ADR).

Status 2026-10-23: all of v8 is built and released in v0.41.0 (protocol 44 to 45): the key overlay, Menu,
Settings, toasts, collapsed strip, message popover, instant sessions, the cuts and the helper question
(which now also opens from the Inbox), the PATH launchers that put an agent typed in a shell pane in a new
worktree, one-step update and restart, and the Round session rows setting. The gate passed on the merged
tree (794 tests, including one daemon test, `a_pass_goes_stale_when_the_files_change_and_a_failure_does_not`,
that failed about one full run in four and passes alone). Not checked live: any of it in a real session,
Windows (never built), zsh, fish and PowerShell launchers, and the old-binary to new-binary restart end to
end. Open: the live check of the helper flow with a real agent over `maitty mcp`; whether check re-run and
override want leader keys (they work from the check sheet).

### 20 — Quiet agents and safety guards · shipped in v0.42.0, live checks open

The user asked on 2026-10-23 for guards against a hung agent, an answer to the wrong prompt,
hidden characters in agent-to-agent text, and saved output that can't be turned off, after a
check showed ranked status sources, evidence times, box-only screen rules, saved scrollback,
per-turn checkpoints and hook setup already exist.
One protocol bump for the phase: 45 to 46. Tickets are labelled `phase-20`.

1. #142 daemon: mark a hooked agent quiet after 10 minutes with no output
2. #143 UI: a quiet agent on its row and in the why popover (after #142)
3. #144 UI: quiet agents in the Inbox's NEEDS YOU, no sound (after #142)
4. #145 CLI and MCP: quiet in `ls`, `ls --json`, `maitty_list` and `maitty wait` (after #142)
5. #146 MCP: `maitty_answer` answers only the prompt the caller saw
6. #147 refuse text with hidden characters sent to an agent
7. #148 daemon: a setting to stop saving pane output
8. #149 daemon: saved output files readable only by you
9. #150 UI: refuse to restore a checkpoint while its agent works

Declined while planning: a fifth "unknown" status (a quiet agent stays working).

Status 2026-10-23: #142–#150 built on `dev` (protocol 45 to 46); the gate passed on the merged
tree (814 tests) after each ticket. `ls --json` and `maitty_list` add `quiet_since` only for a
quiet agent rather than `null` on every pane, so other panes' output is unchanged. Not checked
live: a real agent going quiet, an MCP answer refused for a changed prompt, the output setting
in Settings, and the unix file modes (tested on Linux in CI only).

### 21 — Layout names and the Grid style · shipped in v0.42.0, live checks open

The user asked on 2026-10-23 for clearer layout names and the window look from their "Windows"
handoff, without replacing the existing looks. One Layout setting: Floating, Tiled, Compact and
Grid ([ADR 0015](adr/0015-four-named-layouts-and-grid.md)); Corners applies to each look that
draws boxes; Floating stays the default. Grid colours each pane by its position from theme
roles. Tickets are labelled `phase-21`; no protocol change.

1. #153 config: one Layout setting named floating, tiled and compact
2. #154 UI: the Grid layout, each pane a box in its own colour (after #153)
3. #155 UI: Grid's top border, number and state left, name tag right (after #154)

Declined while planning: the handoff's top bar, right-hand sidebar, fixed 2×2 grid, zoom and
bare keys (ADR 0015). Later: its scratch shell, as its own proposal (needs a daemon-owned shell).
The keybind system is to be discussed with the user before any key work.

Status 2026-10-23: #153–#155 built on `dev`; the gate passed on the merged tree (819 tests)
after each. Grid's colours come from `Theme::grid_colors`, which skips a role too like one
already taken (Walnut's accent and violet are one teal), so every built-in theme gives
four. Not checked live: Grid in a real terminal at 160×45, 100×30 and 80×24, light themes by
eye, and an old config file loading under its new name.

### 22 — Shortcut mode · shipped in v0.42.0, live checks open

The user asked on 2026-10-23 for an input mode and a shortcut mode, so a run of actions is one
key each rather than the leader every time, with the mode shown
([ADR 0016](adr/0016-sticky-shortcut-mode.md)). The leader enters shortcut mode and it stays
until Esc, the leader again, a click into a pane, or an action that gives you something to type
into. Shown by a `SHORTCUT` pill and a sky border; input mode shows nothing. Tickets are
labelled `phase-22`; no protocol change.

1. #156 keys: a shortcut mode that stays until you leave it
2. #157 UI: show shortcut mode with a SHORTCUT pill and a sky border (after #156)

Status 2026-10-23: #156–#157 built on `dev`; the gate passed (826 tests). Ctrl+Space after the
leader now leaves shortcut mode, so `send-prefix` lost its default key; it is in search
commands and can be bound. Not checked live: the mode in a real terminal with a working agent.

Also 2026-10-23, at the user's request: new default keys (ADR 0017), one per action with no
Shift (`v` / `h` splits, `i` Inbox, `y` why, `k` stop, `Enter` message, `o` session with
options, `c` copy mode, `p` find file, `g` worktrees, `b` branches, `l` history, `F1` all keys,
`Ctrl+W` close tab, `Ctrl+V` paste image, `Ctrl`+arrows resize, `1`–`9` tabs).

### 23 — Rename to Vibecasso · shipped in v0.42.0 as Maitty

The user renamed the app to Vibecasso on 2026-10-23, a clean break with no `vibecasso` command
([ADR 0018](adr/0018-rename-to-vibecasso.md)), with a paint-stroke splash and the repository
renamed to CydoEntis/vibecasso. Tickets are labelled `phase-23`.

1. #160 rename the app to Vibecasso, carrying a vibecasso install over
2. #161 release and install Vibecasso from CydoEntis/vibecasso (after #160)
3. #162 a VIBECASSO splash with a paint-stroke sweep (after #160)
4. #163 a product README and a shortcuts reference under the new name (after #160)

Declined while planning: the Starry Night art background (the user's call).

Status 2026-10-23: #160–#163 built on `dev` (834 tests and the rest of the gate passed); the
repository is now CydoEntis/maitty. The v7 design source names the product anew and its
frames are regenerated; its sample project is `orbit`. The README is a short product page;
the detailed feature notes moved to [the guide](guide.md). Not checked live: installing over a
real seshi install (migration, Claude hooks, `maitty integrate mcp`), the release workflow
under the new archive names, and the splash at 80×24 and 160×45 by eye.

Then, outside a phase (2026-10-23): the app was renamed again, to Maitty
([ADR 0020](adr/0020-rename-to-maitty.md), #168), with a new splash (#170); Night and Day
replaced the Seshi-era themes and became the defaults (#169); rounded shapes reached every chip
(#171). Shipped in v0.42.0.

### 24 — The name in one place · shipped in v0.42.0

The user asked on 2026-10-23 for a rename to mean changing the name, not the code
([ADR 0019](adr/0019-the-name-lives-in-one-place.md)): the name is set in `Cargo.toml`, the code
takes it from `brand`, and a test keeps it so. Tickets are labelled `phase-24`.

1. #164 the name in brand.rs for paths, env, themes and splash
2. #165 hooks, MCP, shims and adapters take the name from brand (after #164)
3. #166 no code or text writes the app's name out (after #165)
4. #167 scripts name the app once, and the docs say how to rename (after #166)

Status 2026-10-23: #164–#167 built on `dev`; the gate passed (836 tests, 3 naming tests, 10
adapter tests). A trial rename to `testrename` by the four steps in ARCHITECTURE (Cargo.toml,
the installers' first lines, `regen.sh`, docs) passed the whole gate with no code change; it
first found the splash font and two tests that assumed the name's length, now fixed. Not
checked live: a release built by the workflow under a new name.

### 25 — Rows, rounding and padding · shipped in v0.43.0, live checks open

The user asked on 2026-10-24 for padding round the screen, one rounding setting with three
choices, a round hover, shortcut mode that stays until left, and one-line session rows with the
agent and the uncommitted lines ([ADR 0021](adr/0021-one-line-rows-one-rounding-sticky-mode.md)).
No protocol change; config keeps reading `corners` and a `true`/`false` `rounded`.

1. `ui.padding` (0, 1, 2) round the screen in every layout
2. `ui.rounded` is `full`, `corners` or `off`; Corners leaves Settings
3. the pointer's hover rounds its ends with `full`
4. shortcut mode ends only on Esc, the leader or a click; closing a popup returns to it
5. one-line session rows: the agent's mark, `+N −N` uncommitted, the name in its state's colour
6. the version and update check move to General › UPDATES › Version; no version line over every tab

Status 2026-10-24: built on `dev` (e48ed40..e2b1eab, one commit per change, each green); the gate passed. Not checked live: rows, hover and
padding in a real terminal at 80×24, 100×30 and 160×45. Also fixed here: Backspace on the last letter of an empty Inbox's search crashed the window
(the selection outlived the matches), and tests wrote the real clipboard.

### 26 — The inspector drawer · shipped in v0.43.0, live checks open

The user asked on 2026-10-24 for the file tree, branch and diff in a native, resizable drawer,
after reviewing the inspector design handoff. It grows the
existing side panel rather than adding a second one; the sidebar keeps its state; markdown reads
pretty with pulldown-cmark. Intent: [inspector-drawer](features/inspector-drawer/intent.md),
spec: [spec](features/inspector-drawer/spec.md). Tickets are labelled `phase-26`; no protocol
change.

1. #172 resize the side panel and remember its width per view
2. #173 Files opens in the side panel (after #172)
3. #174 markdown previews pretty, `m` shows raw (after #173)
4. #175 a Branch view in the side panel (after #172)
5. #176 a row's `+N −N` opens its Changes (after #172; needs phase 25 on `dev`)

Exit: every ticket closed, the gate green on `dev`, and a live look at 80×24, 100×30 and
160×45 in Floating and Compact.

Status 2026-10-24: #172–#176 built on `dev` (9463fb3..de25486), each reviewed independently and
approved after one round of fixes; the gate passed at de25486 (912 tests, clippy, Linux and macOS
checks). Also on `dev`: 39a3a2e, a view opened from shortcut mode takes the keys (a phase 25 bug
the #175 review found: `k` in Changes stopped the agent). Open before the exit holds: the live look.
Then, at the user's call: a click into a pane gives it the keys beside an open view (07d9fe6).
Also 2026-10-24, at the user's request: Settings › Appearance › Cursor (terminal, block, underline, bar) and
Cursor blink, used where a pane's program picks no cursor of its own; a program's choice still wins.
Declined while planning: a smooth, animated cursor (a terminal app can only place it on whole cells;
the glide belongs to the terminal, as in Ghostty or kitty).

### 27 — Radar and sending failures · shipped in v0.44.0

A session row shows its context as `72%` once past half full (red when nearly full), reversing
phase 14's "context is left to the agent's status line"; and a failed check's command, exit code
and last 40 lines can be put into the agent's message box with one key, sent only on Enter.
Intent: [radar-and-failures](features/radar-and-failures/intent.md). Tickets when phase 26 is
done. Decided 2026-10-24: red from 85% full; `m` on a failed check opens the message box with the
failure in it (no new key).

### 28 — References · shipped in v0.44.0

Per project, `.maitty/references/` holds screenshots and design files: a References view in the
side panel lists, adds (paste or pick), removes and opens them in the system viewer, and an MCP
tool lists them for agents. Intent: [references](features/references/intent.md). Tickets when
phase 27 is done. Decided 2026-10-24: the folder is gitignored by default.

### 29 — Polish from the 0.43 trial · shipped in v0.43.2–v0.43.5, live checks open

The user tried v0.43 on 2026-10-24 and asked for: everything in the theme and uniform, the side
panel's views as tabs with keys, `c` for Changes, Settings on `,` and every key on `?`
([ADR 0022](adr/0022-question-mark-all-keys-c-changes.md)), modals framed like panes, a folder row
rounded at both ends, and a way to tell shortcut mode is still on under a popup. No protocol change.

1. leader `c` opens Changes, `?` lists every key; F1, `d` and copy mode lose their default keys
2. a Files · Changes · Branch strip across the side panel (click, Tab, Shift+Tab); a commit asks first
3. modals framed like panes: the title as a chip on the border, `Esc close` as a chip
4. a folder row rounds both ends; no stray rule beside a padded Compact sidebar
5. ` SHORTCUT · paused ` stays, dimmed, while a popup has the keys; shortcut mode comes back when a
   popup closes by itself
6. Grid's number, name tag and `+` as chips; toasts and corner pills follow `ui.rounded`
7. a narrow title keeps the current tab and drops the state's glyph last of all

Also: the mode made unmistakable ([ADR 0023](adr/0023-the-mode-is-always-shown.md)): a sky frame
round the window in shortcut mode, a `TYPING` / `SHORTCUT` tag always in the corner, and a hint
toast on a key that does nothing. Built on `feat/loud-mode`; not checked live.
Also: Grid titles: the state glyph sits in the tab; no state words; no repeated name tag. Built
on `feat/grid-title-status`; not checked live.

Also: Background: solid is the default (user decision 2026-10-24), so a plain terminal shows the
theme whole; `background = "terminal"` still reads for transparent or image terminals.

Status 2026-10-24: built on `dev` (d8da65e..95b5c91), three parts each reviewed independently and
approved after fixes; the gate passed at 95b5c91 (959 tests, clippy, Linux and macOS checks). Also
fixed: the `Esc close` chip on a confirm did nothing when clicked. Not checked live: the new
frames at 80×24, 100×30 and 160×45 in each look.
Follow-ups: in Files, `d` still opens Changes; the side panel's title repeats its lit tab;
`docs/guide.md` still describes the older Files panel; the paused pill gets no room in a
right-hand sidebar's header; below about 60 columns a narrow pane's name chip can shrink to `…`;
`screen_tests::tiled_square_and_faded` failed once in a parallel run; under about 16 rows the
paused frame can cover a popup drawn on the ring (its snapshot is taken after `screen::draw`);
Compact at padding 0 has no frame (offer one on its header and foot strips if wanted); on
Windows a short one-line paste (under 16 characters) arrives as separate keys, so while the
Files tree or preview has the keys its letters run Files' commands (`e`, `y`, `o`).
Also: alerts that rang mid-turn fixed (a Claude stop with background work in flight no longer
rings finished; screen-idle recovery waits for a still screen; every status change is logged to
`daemon.log`); notifications name the branch. Not checked live yet.
Also: Files' editor selects and copies (user report 2026-10-24: no way to select, copy or paste):
Shift-selection, Ctrl+A and the mouse; Ctrl+C, Ctrl+X, Ctrl+V and the terminal's paste as text;
Ctrl+Z / Ctrl+Y; the file shown (not edited) selects with the mouse and Ctrl+C copies. Built on
`feat/editor-selection`; not checked live.
Also: polish from the 0.43.3 trial (user decisions 2026-10-24): Grid panes lose their number chip
(no key uses it; the top-left read `1  [1  name]`), and the sky frame and corner tag give way to
one mode bar along the bottom with key suggestions from the keymap; Update and Restart move to
its right end ([ADR 0024](adr/0024-the-mode-lives-in-a-bottom-bar.md)). Built on
`feat/mode-bar`; not checked live.
Also: the bar says who has the keys (user decision 2026-10-25, after ` SHORTCUT · paused `
showed with only the sidebar focused): `SIDEBAR · ↑↓ move · Enter open · Esc back`, `SETTINGS`,
`FILES`, `INBOX`, a confirm's `y yes · n no` and so on, held or not, with `shortcut mode waits`
while held; "paused" is gone. Built on `feat/bar-says-who`; not checked live.

### 30 — The Hairline layout · shipped in v0.45.0, live checks open

The user asked on 2026-10-25 for a new layout, Hairline, from a design handoff (option 1c, "split,
one needs you"), added beside Floating, Tiled, Compact and Grid without changing them. No boxes:
single `│` / `─` hairlines in `line`; one row per session in the sidebar with a `▎` marker; each
pane a title row and a rule coloured by state (teal focused, amber needs you); a status strip of
one segment per session above a single bottom bar (mode, layout · panes, hints, `● N needs you`).
Hairline is never rounded (`ui.rounded` doesn't apply). Every menu, popup and side panel gets
the same look in Hairline. Decided 2026-10-25: behaviour stays as today (alt+1–9, digits go to
the focused agent); the handoff's open questions (auto-open a waiting session in pane B, a quick
answer key) wait for the user's call after trying it.

1. the workspace chrome: config, sidebar, pane title and rule, status strip, bottom bar
2. every menu, popup and side panel in the Hairline style (after 1)

### 31 — Panes you can tell apart · shipped in v0.46.0, live checks open

Asked by the user on 2026-10-25 while trying splits, and built the same night at their call. Also in
v0.46.0: panes swap places (drag a title onto another pane, leader Alt+arrows, or the pane menu;
protocol 47), a click on a pane or a strip segment gives that pane the keys, the strip marks your
session, and Hairline's sidebar foot reads just `new` and `menu`. Decided 2026-10-25: every pane gets a number in all five layouts, reversing
UI.md's "no number no key uses" rule for Grid and phase 30's removal in Hairline.

1. every pane of a split shows its number (its place in reading order) in all layouts; a lone
   tab no longer shows a tab number
2. closing a pane names it: the confirm says "Close pane 2 · <name>" and the pane being closed is
   lit while the confirm is up
3. the focused pane is unmistakable in every layout while a menu or confirm is open
4. the pane right-click menu shows the real keys (leader `v`, `h`, `x`), not first free letters
5. open question for the user: swap the split defaults so `v` stacks and `h` goes side by side?
   Today `v` is a vertical divider (side by side) and `h` a horizontal one, as in Vim and tmux.

### 32 — Two looks, and a session's tabs in the sidebar · 1–8 shipped in v0.47.0–v0.47.2, 9 built, not released

Asked by the user on 2026-10-27: a second tab working out of view did not show anywhere, and five
looks were too many to keep linked up. Decided the same day
([ADR 0027](adr/0027-two-looks-and-tabs-in-the-sidebar.md)): keep Floating and Hairline; Grid's
colours become the Pane colours option on Floating; Floating keeps square, corners and full.

1. a session with two tabs or more lists them under its row, one line each with its own state;
   a click shows the tab
2. two looks: Tiled loads as Floating with no gaps, Grid as Floating with pane colours, Compact
   as Hairline
3. `ui.pane_colors`: each Floating pane's border in its own colour by its place
4. Floating's tabs get a row of their own as Hairline's do; a tab's only pane is titled
   `project · ⎇ branch`; a tab is named the same in the tab row and the sidebar
   ([ADR 0028](adr/0028-tab-rows-and-popups-drawn-like-panes.md))
5. every popup, menu and toast boxed as its look boxes a pane; Gap and Pane colours greyed in
   Hairline as Rounded is; the two sidebars share their row bookkeeping
6. a tab out of view shows its state on its tab (it showed none until focused); the ✕ is on
   the tab; `ui.sidebar_tabs` (all, busy, off); a retired look is said once and written back
7. Settings regrouped (General, Appearance, Sessions, Alerts, Agents, Check, Keys) with updates
   first and a banner on every tab; the splash says a release is out in a banner, `u` installs
8. fewer steps: Alt+n/a/x/s/d (shell, session, close, split side by side, split down) run without the leader
   (`ui.alt_keys`; Settings › Keys lists and rebinds them under "Without the leader"), and
   every close asks first ([ADR 0029](adr/0029-alt-letters-and-closing-without-asking.md))
9. panes titled like an editor's (asked 2026-10-28): a lone pane has no title, a split pane's is
   number, glyph and name with no state word or ✕, every tab has a ✕; sidebar tab lines skip the
   session's own tab; padding defaults to a row above only; a restart keeps a session's tabs its
   own (the daemon keeps `tab_of`); the toast a round outline in Floating and a solid
   block with a coloured left edge in Hairline
   ([ADR 0030](adr/0030-panes-titled-like-an-editor.md))
10. Alt+r renames the tab (`ui.alt_keys`), shipped in v0.50.5; and (asked 2026-10-28) Floating's
    tab row sits in the header over the pane boxes, not inside them
    ([ADR 0035](adr/0035-the-tab-row-sits-outside-the-pane-boxes.md))

### 33 — Secrets and PR tracking · shipped in v0.51.0

Maitty stores secrets outside source files and chat history, delivers them only to
the owning daemon process, and tracks pull request state. See
[the feature intent](features/secrets-and-pr/intent.md) and
[ADR 0041](adr/0036-secrets-never-enter-chat.md).

### 34 — Shared conversations and Codex handoff · foundation in progress

Requested 2026-10-27. [Intent](features/chat-gui/intent.md) ·
[Spec](features/chat-gui/spec.md) · [ADR 0041](adr/0041-chat-gui-shared-conversations.md).
Codex first. The daemon owns structured execution and exact native conversation
identity. The ticket list below tracks that foundation. Standalone desktop work
now proceeds in phase 35; native GUI-to-TUI handoff acceptance is deferred.

The daemon conversation foundation and standalone Codex execution support are
required by the GUI. The optional GUI-to-native-TUI handoff was implemented early,
but its live roundtrip and platform acceptance move to [Later](#later) and do not
block the standalone GUI.
1. [#197](https://github.com/CydoEntis/maitty/issues/197) — test(agents): define the supported Codex session contract
2. [#198](https://github.com/CydoEntis/maitty/issues/198) — feat(daemon): persist exact conversation identities; after [#197](https://github.com/CydoEntis/maitty/issues/197)
3. [#199](https://github.com/CydoEntis/maitty/issues/199) — feat(daemon): own Codex app-server connections; after [#197](https://github.com/CydoEntis/maitty/issues/197), [#198](https://github.com/CydoEntis/maitty/issues/198)
4. [#200](https://github.com/CydoEntis/maitty/issues/200) — feat(daemon): retain ordered conversation history; after [#198](https://github.com/CydoEntis/maitty/issues/198), [#199](https://github.com/CydoEntis/maitty/issues/199)
5. [#201](https://github.com/CydoEntis/maitty/issues/201) — feat(cli): start and continue structured Codex turns; after [#199](https://github.com/CydoEntis/maitty/issues/199), [#200](https://github.com/CydoEntis/maitty/issues/200)
   Windows build, test, clippy, Linux, and macOS cross-checks passed in the isolated verification target; live acceptance and independent review remain pending.
6. [#202](https://github.com/CydoEntis/maitty/issues/202) — feat(agents): answer exact structured approval requests; after [#201](https://github.com/CydoEntis/maitty/issues/201)
   Command execution, file-change, permission and tool-input requests now have exact-ID daemon and desktop controls. File changes show the matching patch before enabling approval; permission grants are limited to the requested profile for the current turn.
7. [#203](https://github.com/CydoEntis/maitty/issues/203) — feat(agents): cancel a structured Codex turn explicitly; after [#202](https://github.com/CydoEntis/maitty/issues/202)
   Exact acknowledged-turn interruption and GUI Stop control are implemented with offline provider coverage; platform/user acceptance is still pending.
8. [#204](https://github.com/CydoEntis/maitty/issues/204) — feat(daemon): hand idle conversations to native Codex; after [#202](https://github.com/CydoEntis/maitty/issues/202), [#203](https://github.com/CydoEntis/maitty/issues/203)
9. [#205](https://github.com/CydoEntis/maitty/issues/205) — feat(daemon): reconcile native history before chat takeover; after [#200](https://github.com/CydoEntis/maitty/issues/200), [#204](https://github.com/CydoEntis/maitty/issues/204)
10. [#206](https://github.com/CydoEntis/maitty/issues/206) — feat(client): expose explicit conversation handoff controls; after [#204](https://github.com/CydoEntis/maitty/issues/204), [#205](https://github.com/CydoEntis/maitty/issues/205)
11. [#207](https://github.com/CydoEntis/maitty/issues/207) — test(agents): prove Codex conversation handoff live; after [#206](https://github.com/CydoEntis/maitty/issues/206) · human acceptance
The forward and reverse handoff paths and GUI controls are implemented with
offline provider coverage, including exact thread identity, native history
reconciliation and refusal while the terminal is busy. The isolated Windows
live roundtrip now proves exact Codex continuity through both handoffs;
process-cleanup failure recovery now retries only while the exact native pane
process remains alive and idle. Maitty keeps native ownership after cleanup
fails, refuses retry while busy or after process exit, and confirms process-tree
cleanup again before chat resumes. Tests cover retry refusal while busy or after
exit and a successful retry while the original pane process remains alive. A
real-PTY regression test injects a cleanup failure through the daemon handoff
command, confirms native ownership remains, and retries cleanup before chat
resumes. OS-level cleanup-failure and platform acceptance checks remain open.
The isolated Windows GUI smoke conversation completed the GUI-to-native handoff
and resumed the exact Codex session. It exposed a reverse-handoff dead end: the
GUI disabled return while its cached native history was incomplete, even though
the daemon reconciles history during the return. The GUI now permits that daemon
reconciliation path while keeping chat input gated on complete history. GUI
regression tests pass; the daemon-level live reverse handoff now passes.
The reverse-handoff failure was traced to the daemon's process scan: Codex can
appear as a generic `node` process, and that scan erased the provider identity
assigned when Maitty launched the native conversation. Conversation-owned pane
identity is now preserved when process detection cannot match it; a regression
test exercises reverse handoff after such a scan. An isolated Windows live
smoke using the desktop runtime started Codex, completed a prompt, handed the
exact thread to its native terminal, returned to chat with the original history,
and completed a second prompt. Direct user-driven GUI acceptance and platform
handoff checks remain open.

### 35 — A usable standalone Codex chat GUI · complete

Make the GUI independently usable for Codex: start a conversation in a chosen
folder, send and receive messages, see tool activity, resolve approvals, cancel
and recover turns, inspect changes, reconnect, and reopen the GUI while daemon
work continues. The GUI and TUI remain separate clients of the same daemon; native
terminal transfer is not part of this acceptance.
Decision recorded: Tauri 2 with React, TypeScript, Vite and shadcn/ui over the
daemon's authenticated local IPC ([ADR 0037](adr/0037-tauri-react-desktop-shell.md),
[stack rules](stack/tauri.md)). Platform prerequisites are Windows 10 version
1803 or later with WebView2, macOS 10.15 or later, and Linux with WebKitGTK 4.1.
The functional states and visual brief are in
[the GUI design brief](features/chat-gui/design-brief.md). Use a working layout
in this phase; review its final visual design in phase 36.

Prototype work began before phase 34 exits, at the user's explicit request to
keep the GUI moving. The Windows prototype bundles the standalone CLI and starts
the daemon in a separate process; it can list and read sessions and submit a
message. Command execution, file-change and tool-input requests and exact acknowledged-turn cancellation
now flow through the daemon to the desktop UI under human-only authority. Codex
permission requests now show the exact requested profile and allow only a
turn-scoped grant. The Changes panel now reads tracked and untracked diffs from
the conversation's daemon-owned checkout. The desktop now embeds xterm.js for
native terminal emulation, with initial scrollback replay, ANSI screen snapshots
and generation-bound input and resize commands resolved to the pane inside the
daemon. The GUI can hand an idle conversation to its
native terminal and return it to chat after the native process stops and full
history is reconciled. Terminal resize requests now preserve the latest
dimensions when an earlier daemon resize is still in flight; the regression is
covered by the GUI terminal tests. Windows now has live evidence for creating a
conversation, chatting, canceling and continuing, and preserving daemon work
across GUI close/reopen; Windows installer bundles build successfully. Phase 35
now has live Windows and Arch Linux/Xvfb evidence for Codex chat, approvals,
recovery and diffs. Windows MSI/NSIS bundles and Arch deb/rpm/AppImage packages
build. A real Omarchy desktop check and macOS desktop build remain open for
release verification.
The GUI reads every native-history page and validates Tauri replies at the
frontend boundary. Incomplete or failed history stays visible and disables
return to chat until a complete read succeeds.
When the daemon disconnects after sessions were loaded, the GUI labels cached
statuses as disconnected with their last known state and blocks daemon actions
until reconnect succeeds.
The release desktop host resolves the daemon from bundle resources or an
adjacent build-output executable. A fresh Windows release app reached its empty
daemon session list on an isolated socket. On the default socket, the current
GUI found an existing daemon on protocol v49 while it requires v59. It correctly
refused the incompatible connection; the disconnected screen explains both
versions. It now offers an explicit restart for an older local service, displays
the saved-session and active-turn counts when available, and reconnects through
the GUI's bundled daemon after confirmation. Rust rechecks the protocol and
refuses remote services, compatible services and newer daemons. The confirmation
warns that active turns stop and output since the last save may be lost. Failed
session queries stop automatic polling and wait for Reconnect, so the error
appears instead of an indefinite Connecting state. GUI tests cover the mismatch,
cancellation of the confirmation, and restart/reconnect. The updated first-run
and mismatch screens were inspected from the running Windows app; the earlier
captured mismatch is in `target/gui-windows-version-mismatch-final.png`.
Live Windows acceptance on 2026-10-29 used an isolated daemon and the selected
worktree folder: the GUI created a new Codex conversation, sent a no-tools
prompt, showed the exact reply, and returned the complete history from the
daemon. Closing the GUI left the daemon running; reopening restored the same
conversation and reply. A later live run canceled a running PowerShell command,
showed the tool as interrupted and the composer as ready, then sent a follow-up
and received the exact reply in the same conversation. History polling now
continues while any tool remains running, even if the conversation summary has
already become idle. A live Codex 0.165 approval initially exposed an invalid
stdio response envelope; Maitty now replies with JSON-RPC `{id, result}`. On
2026-10-29, the Windows GUI approved a real out-of-workspace PowerShell write,
Codex completed it, and the exact file content was verified. Codex's private
reasoning items are now omitted from user-visible history so they do not mark a
completed conversation incomplete. On 2026-10-29, the live Changes panel showed
the exact diff for a tracked README edit and a new untracked file. Killing the
isolated Codex app-server put the conversation into Recovery needed; Resume
session reconnected the exact thread without replaying its completed prompt,
restored complete history, and cleared the old disconnect error. Closing and
reopening the GUI preserved the recovery state until it was resumed.
On 2026-10-29, the Windows GUI created a new Codex conversation through the
normal New conversation flow, resolved Codex from the installed NVM npm package,
and returned the exact no-tools reply `GUI_NEW_CONVERSATION_OK`. A second run
removed Node and NVM directories from the GUI's `PATH`, kept `NVM_SYMLINK`, and
still launched the NVM Codex executable and returned `GUI_NVM_FALLBACK_OK`.
Unix Codex discovery now preserves the invoked PATH name through daemon validation,
so Omarchy's mise `codex` shim remains named `codex` when Maitty launches it.
Arch tests cover both shim lookup and launch binding. The AppImage rebuilt
with this fix stayed open under Xvfb and D-Bus for 15 seconds and started its
bundled daemon on an isolated socket. The container has no Codex install, so this
checks package startup only; real Omarchy desktop acceptance remains open.
The current source also built Linux `.deb`, `.rpm`, and AppImage packages in
Arch; package metadata was readable. The AppImage stayed open under Xvfb with
GTK/WebKitGTK installed. On 2026-10-29, the AppImage created a conversation in
an Arch container, launched Codex 0.165 from its npm package, and returned the
exact live GUI reply `GUI_ARCH_LIVE_OK`. This verifies the Linux package and
Codex chat path in Arch under Xvfb; a real Omarchy desktop check remains open.
The same Arch AppImage run also verified Codex approval, a tracked README diff
in Changes, and recovery after killing the isolated app-server: Resume session
restored the exact conversation and its completed history without replaying the
turn. Arch's container kernel could not create Codex's user namespace, so its
workspace commands required individual GUI approvals; both approved commands
completed. The live Linux checks now cover new conversations, chat, approval,
changes and recovery in Arch under Xvfb. A real Omarchy desktop check remains
open.
Codex 0.164 does not materialize a new thread until its first user message.
Empty new threads therefore skip history listing; exact unmaterialized-thread
errors normalize to empty history only when no uncertain submission exists.
Offline regressions cover fresh empty threads, returning from native terminal,
and refusing to hide uncertain submissions. Live Codex checks used version
0.165; the exact 0.164 unmaterialized-thread error remains covered only offline.
The new-conversation form uses a native folder picker and has no OS-specific
default project path. A desktop UI test covers picking a folder, creating the
conversation through the daemon command, and selecting it in the GUI.
On Windows, inherited drive pseudo-variables such as `=C:` are now filtered
from provider launch environments; otherwise Codex conversation creation fails
daemon validation. A live isolated Windows GUI run created a conversation and
returned the exact no-tools reply `MAITTY_GUI_SMOKE_OK` after this fix.
Composer drafts persist locally per conversation and are restored when the GUI
reopens; tests cover restoration and malformed saved data.
The selected conversation is saved locally and restored when the GUI reopens;
missing saved conversations fall back to an available session.
The Midnight/Paper theme choice also persists locally.
In-app toasts offer to open another active conversation when it needs attention
or finishes; archived and selected conversations are excluded.
Conversation tool history preserves the daemon's running, completed, failed and
interrupted states so the GUI does not present failed or active work as complete.
The conversation header and Ctrl+` use the daemon's explicit generation-bound
chat/native handoff commands. The GUI blocks handoff while the conversation is
working or needs attention; the daemon remains the final authority.
Structured recovery now offers an explicit Resume session action through the
daemon's exact conversation identity; it does not replay the interrupted turn.
The native terminal replaces the chat column, leaving Changes and Context in the
side panel. Sidebar projects have collapsible Settled and Archived sections;
conversation rows can be pinned, renamed or archived. Those labels are saved
locally, and archiving a working conversation is blocked. Press `?` for the
keyboard shortcut list. The responsive layout was checked at 1440×900, 1024×700, and 800×600, including the sidebar's 900px breakpoint.

### 36 — Design the GUI using the working app · complete, after 35

The user-approved v2 desktop handoff is now the visual and interaction source of
truth; see the [design brief](features/chat-gui/design-brief.md) and its preserved
reference files. Validate the running Codex GUI against it, finish its responsive
layout and keyboard/accessibility details, and exercise empty/loading/error/
reconnect states. Preserve conversation continuity. Phase 36 acceptance passed:
the running app was reviewed at 1440×900, 1024×700 and 800×600, and the GUI tests
cover keyboard, accessibility, first-run, loading, disconnect and recovery
behavior. Phase 35's live Codex chat, recovery and diff requirements are complete.
Windows and Linux frontend checks and the Rust verification gate pass on the
current worktree. Omarchy desktop and macOS package checks remain release
verification.
The working GUI supports Ctrl+N, Ctrl+Shift+A, Ctrl+Shift+D, Ctrl+I, Ctrl+`,
Ctrl+. and Ctrl+K, plus J/K and Alt+arrow conversation navigation, Y/A/N/Shift+N
approval actions, and Ctrl+\\ to collapse or expand the desktop sidebar. Shortcut
hints now show Command on macOS and Ctrl on Windows and Linux. The v2 design
review caught a shortcut hint overflowing the 52px compact sidebar rail; the hint
is now hidden while the rail is collapsed. The updated Arch AppImage was checked
at 1440×900, 1024×700 and 800×600. At 1024px the review panel overlays chat; at
800px the sidebar becomes a rail and the New conversation icon stays inside it.
Visual review found overlapping approval actions at 1024px and a clipped New
conversation label at 800px. The approval controls now wrap into their own row,
and the compact rail hides the visible label while retaining the accessible
button name; fresh 1024×700 and 800×600 screenshots confirm both fixes.
The sidebar opens as a drawer on compact windows. Approval shortcuts invoke only
choices offered by Codex and stay inactive while typing, in dialogs, or in the
terminal. The must-list shortcuts are available in the keyboard map: rename is
available while a conversation row is focused; file-save remains later Files
scope. The quick switcher now opens its focused conversation in
the terminal with Ctrl/Command+Enter and uses the existing guarded handoff.
Context panel tabs use arrow keys and keep only the active
tab in the Tab sequence. A first-tab-stop skip link moves keyboard focus past the
conversation sidebar to the main conversation; it is covered in the GUI tests.
The Changes panel now lists every changed file, displays one selected diff at a
time, and supports file selection through the list, previous/next buttons, and
`[` / `]`. Keyboard navigation stays inactive in text entry, dialogs and the
native terminal. Both shortcut maps document these keys; behavior has dedicated
GUI tests.
Screen readers now receive polite announcements for daemon connection changes
and when a conversation finishes, needs attention or needs recovery; streaming
text is not announced. Settings now opens from the gear, command palette, or
Ctrl/Command+comma and has Appearance, Notifications, Daemon & providers, and
Keyboard sections. Its section selector is a tablist with one Tab stop, arrow
navigation matching its vertical or compact horizontal layout, and a visible
focus ring. Midnight/Paper selection and notification preferences
persist; toast position and needs-you/finished alerts affect the running GUI.
The empty desktop workspace now keeps Settings available before the first
conversation, checks Codex readiness, explains current daemon/provider support,
and offers a project-folder action. Ctrl/Command+O opens the folder picker; the
new-conversation dialog retains the selected folder. Both keyboard shortcut
maps include this action. Desktop-flow tests cover an empty session list, Codex
readiness, a Linux-style project path, and Settings before the first
conversation. Another desktop-flow test covers an initially disconnected empty
workspace, Settings access, and recovery after Reconnect. Windows and Linux
frontend checks pass.
The current Arch AppImage was launched under Xvfb: its first-launch screen
reported the daemon connected and Codex missing in the isolated container, then
Settings opened and ArrowDown selected Notifications with visible focus. The
first-launch and Settings screenshots are in the local `target/linux-omarchy-current/screenshots/` output.
Daemon reconnect is available while offline, and provider support is stated
plainly. On 2026-10-29, all 100 GUI tests, lint, typecheck, and production build
passed on Windows; the current GUI source also rebuilt in Arch Linux. Linux
checks ran from a container-local source copy because Vite stalls on this Windows
worktree's Docker bind mount. Arch `pnpm desktop:build` rebuilt the Rust sidecar
and GUI packages from current source; `.deb` and `.rpm` metadata were checked,
and the AppImage stayed open for 15 seconds under Xvfb with WebKitGTK installed.
The latest Linux packages, including the Unix mise-shim Codex discovery fix, are
in `target/linux-omarchy-mise-shim-2026-10-29/`.
Windows MSI/NSIS installers were rebuilt after the first-launch, keyboard-map,
compact-layout, Settings-tab, changed-file navigation, and quick-switcher
terminal shortcut and disconnected-feedback updates. The current Windows
MSI/NSIS packages are in
`gui/src-tauri/target/gui-current-windows-inspect/release/bundle/`. The GUI was
launched from that build with a fresh isolated protocol-v59 daemon; the older
user daemon was left running. The latest Arch `.deb`,
`.rpm`, and AppImage packages from the full `desktop:build` run are in
`target/linux-omarchy-current/quick-switcher/`. GUI
checks cover folder selection, conversation creation/selection, and preserving the
current conversation when startup fails. A fresh Arch container produced Linux
desktop packages from the current source. The branch-view test fixture runs its
captured background jobs inline, avoiding host thread scheduling while still
checking the returned facts and reloads; production work remains asynchronous.
On 2026-10-29, the default `cargo test` passed with 1,344 library tests, 3 naming
tests and 5 ignored. Build, clippy, and Linux/macOS target checks passed on this
source.
Linux `cargo test --lib` passed 1,325 tests, with 4 ignored. After changing the
Claude adapter to request the documented host permission channel, the Windows
gate passed again on 2026-10-29: 1,345 library tests and 3 naming tests passed,
5 tests were ignored, and build, clippy, Linux target check and macOS target
check all passed. A real Omarchy desktop check remains a release check.

### 37 — Claude continuity and initial GUI release · active, after 36

Keep the GUI as an optional client package over the daemon protocol. Before
merging the feature into `main` or `dev`, complete the package separation in
[ADR 0042](adr/0042-decouple-desktop-clients-and-providers.md): extract the
headless core, TUI/CLI, shared protocol, GUI host and provider adapters so the
Tauri host no longer links the core runtime. Preserve the current integrated GUI
branch as the rollback and comparison point while the refactor is verified.

Refactor progress: `crates/maitty-shared` owns the versioned protocol,
conversation contracts, split-tree types, clock helpers, hidden-text
validation and shared check-output rules. `crates/maitty-ipc-transport` owns
bounded length-delimited framing, message encode/decode and the shared Welcome
handshake, socket identity formatting, private Unix socket-directory check and
platform socket-name conversion for both clients; each client keeps its own
protocol mismatch error, socket connection and daemon lifecycle.
`crates/maitty-desktop-client`
now owns GUI daemon IPC, lifecycle operations and Codex discovery; the Tauri
host depends on it and `maitty-shared`, not the `maitty` runtime. Codex
compatibility preflight, Codex session metadata, usage parsing and effective
configuration policy, and Claude protocol parsing, session IDs, launch
arguments, hook policy and turn-event projection, are in their provider packages;
live transports, daemon and TUI/CLI remain to be extracted.
The original terminal implementation has moved into `crates/maitty-runtime`;
the root `maitty` package preserves the installed command and re-exports the
runtime library for compatibility. The daemon, TUI and CLI remain together in
that runtime package and still need to be split.
The daemon no longer imports the CLI to refresh Claude hooks. `app.rs` supplies
that startup action to daemon initialization, after the existing-daemon check.
On 2026-10-30, the Codex parser and effective-config policy extractions passed
the full workspace suite (1,325 root tests passed, 5 ignored, and every
workspace crate suite passed), workspace build and clippy, plus Linux and macOS
target checks. Independent reviews found no boundary or behavior issue.
The runtime package move also passed the workspace build and clippy, Linux and
macOS target checks, the full workspace suite (1,325 runtime tests passed,
5 ignored, and the remaining package suites passed), and `maitty --version` and
`maitty --help` smoke checks. An independent review confirmed app branding and
relocated fixture paths.
The shared `maitty-settings` package now owns key actions, default bindings,
key parsing and built-in themes; the runtime keeps its terminal mouse-event
encoding wrapper. It currently has 19 tests covering these preferences and
validators. The all-workspace Linux cross-check still requires a configured
Linux pkg-config/sysroot for the Tauri host's D-Bus dependency.
The UI preference model has also moved into `maitty-settings`, including its
defaults, rounding deserialization, legacy look mapping, cursor validation and
spinner presets. The `Sessions` preferences and pure choice validation have
also moved; old `[mcp] helpers_at_once` TOML migration remains in runtime.
`maitty-runtime::config` re-exports the old names while keeping file loading
and TOML migration orchestration. Restore, worktree and MCP settings schemas
and defaults have moved as well. Check definitions and pure validation now
live there too, along with recipe, preset and quick-agent schemas/defaults and
preset prompt projection. Repository check-file discovery and TOML parsing
stay in the runtime. Icon, key, detection, snapshot and notification settings
schemas/defaults have now moved too; legacy icon normalization stays in runtime.
The pure `AgentDef` schema has moved as well; command detection and built-in
agent policy remain in runtime. The workspace suite passes 1,317 runtime tests,
19 settings tests, 31 desktop-client tests, 3 IPC transport tests, all remaining
package suites, and 5 ignored tests. Strict clippy, workspace build, and
Linux/macOS package checks all pass.
The bundled agent screen matcher and its manifest fixtures have now moved into
`maitty-agent-detection`; daemon status still owns process-tree and lifecycle
signals. Socket identity formatting is shared in `maitty-ipc-transport`; the
clients retain their OS connection and lifecycle. The private Unix socket
directory permission check is shared too. The Windows workspace suite passes
1,314 runtime tests, 3 detection tests, 19 settings tests, 31 desktop-client
tests, 5 IPC transport tests, all remaining package suites, and 5 ignored tests.
The IPC package passes 6 tests in the Linux Rust container, including its
owner-only directory check. Strict clippy, workspace build, and Linux/macOS
package checks all pass.
Linux and macOS target checks passed for the installed `maitty` package. The
all-workspace Linux cross-check still needs a configured D-Bus
pkg-config/sysroot for the Tauri host.
The native status integration catalog is now a pure `maitty-agent-integrations`
package shared by the TUI and CLI installer. Filesystem discovery and adapter
installation remain in the runtime. This removes the TUI's metadata-only import
from the CLI module while keeping status setup behavior unchanged.
Codex-specific native hook TOML projection now lives in `maitty-provider-codex`;
the runtime retains path checks, backups and atomic file writes. The provider's
completion notification is also mapped there to the shared hook status.
Claude hook events and compatible status event mapping now live with the
Claude provider's hook ownership and stale-hook policy; other provider event
mappings remain in the runtime. The provider also builds the shared tagged hook
group used by Claude, Gemini and Qwen settings; JSONC editing and file writes
stay in the runtime, while the provider owns replacement of its tagged groups.
Claude status-line configuration policy also lives in the provider; the runtime
keeps the saved user command and executes it after reporting usage.

Claude meets the same standalone desktop chat contract as Codex. Native terminal
transfer remains deferred. Package the optional GUI for Windows, macOS and Linux
while retaining the standalone TUI install.
Desktop GUI CI is configured to build packages on Windows, macOS, Ubuntu and
Arch Linux; Arch and Omarchy build prerequisites are documented in the Tauri
stack guide. Each desktop job now runs the GUI test suite and lint before
building its package. On 2026-10-29, Windows `pnpm test` passed all 102 tests
across 16 files, and `pnpm lint` passed. The hosted CI workflow has not yet run
on this worktree.
After the Claude host-channel argument update, a fresh Windows
`pnpm desktop:build` passed TypeScript checking, the Vite production build and
Tauri packaging. Updated MSI and NSIS installers are in
`target/gui-host-channel-windows/release/bundle/`.
After clarifying the disabled-Claude launch error, a clean Windows target at
`target/gui-release-current-20261030/` passed `pnpm desktop:build` again. The
fresh MSI SHA-256 is
`17C0C71C9204DBAB44579C7AC522CE8EC55FA669FDCC4A105CA1D2CDC70E0062`; the NSIS
installer SHA-256 is
`F5CD243635FF61632C3D0E00CAC20DEB28C053B4BBB3E21E7737F0CE36B945F2`. Running
the release executable with an isolated data directory, config path and socket
showed an empty workspace with the daemon connected and Codex CLI ready. This
checks first-launch startup only; no prompt was sent and the installer was not
installed.
The new-conversation flow now blocks duplicate creation while a start is
pending and shows a pending indicator. A regression test failed before the fix
and passes afterward. On 2026-10-29, all 103 GUI tests, lint, typechecking and
the production desktop build passed. Fresh Windows MSI and NSIS packages are in
`gui/src-tauri/target/gui-release-create-guard/release/bundle/`; SHA-256: MSI
`1780B8BEDACD9C09B7078F4861E7F338AF0F182EED562FB3752B83C407AFF05A`, NSIS
`20670EEBBC9988CD5184C3FFC64DB1A6D29DB7E8CAE40C6389F32FF296A5A4D2`. The
rebuilt executable opened against an isolated data directory and connected to
its daemon; no agent prompt was sent and the installer was not installed.
On 2026-10-29, the NSIS installer was installed into an isolated folder under
`target/gui-roadmap-nsis-install-20261029/`. The installed executable started
with its data directory, config path and daemon socket isolated under `target/`;
the window showed the daemon connected and Codex CLI ready. This verifies the
NSIS install and first launch, but not upgrades, uninstall, MSI installation,
or a live Codex prompt.
The same 103-test, lint and production-package checks passed in the existing
Arch Linux container using a clean source copy with this change. Fresh packages
are in `target/gui-release-create-guard-linux/packages/`; SHA-256: deb
`ECD8C6E9F58DB8BA9A22AE0E68D7F93062032EFA52C69A2FA94CB93CB1410558`, rpm
`8DD67FC7193D13A2C6B416AEAD0E1DBE2FCAAC98B62104B9CA903C93BAB59777`, AppImage
`C428C130F2C55F7CC35458F20A5F9AE2C6860750467FF6ED7787EE84B81B42D5`. The
AppImage opened under Xvfb with its bundled daemon listening on a unique socket;
both processes stopped after the check. This is Arch container verification,
not real Omarchy desktop acceptance.
The Windows NSIS installer was subsequently installed to an isolated folder
under `target/`; its packaged GUI opened with an isolated config, data directory
and socket, and showed the daemon connected and Codex ready. The Linux `.deb`
and `.rpm` packages were installed in disposable Debian Bookworm and Fedora
containers and each GUI stayed running for 15 seconds under Xvfb and D-Bus with
isolated state. The containers were removed after the checks. These checks cover
NSIS, `.deb` and `.rpm` installation and launch, but not native desktop behavior,
MSI installation, upgrades or uninstall.
The MSI's administrative image was extracted with `msiexec /a` to
`target/gui-roadmap-msi-admin-image-20261029/`; the extracted executable opened
with isolated state and showed the daemon connected and Codex ready. This checks
MSI payload extraction and first launch, not a registered MSI install, upgrade
or uninstall. A host audit found only an Ubuntu VirtualBox VM; Windows Sandbox
was not found, and checking its optional-feature state requires elevation. The
MSI is per-machine and reads a shared Maitty install path, so registered MSI
install/upgrade/uninstall checks need a disposable Windows environment to avoid
changing the existing NSIS install.
A clean Arch source copy with the same daemon update also passed
`pnpm desktop:build` and produced fresh deb, rpm and AppImage packages in
`target/gui-linux-host-channel-20261030/packages/`. SHA-256: deb
`4744345C6E3E5CEA8A4BA85D0016A078983A5831D928E150034948DD2B89A99D`, rpm
`09B9A52DFB5C70611AD031C1A295910951459D103B8CA904A82A5A856459894C`, AppImage
`903F59D9BE6A396F3A0F078AD107F932CD49B589C12DB45C2D16BC0292301430`.
The current GUI
sources rebuilt in an Arch Linux container on 2026-10-29 and produced deb, rpm
and AppImage packages. The current AppImage also launched under Xvfb with the
first-launch and Settings keyboard navigation checked. An earlier AppImage stayed running for 15 seconds under
Arch WebKitGTK, Xvfb and D-Bus with `--appimage-extract-and-run`. The current
Windows GUI suites and desktop packages pass; Arch container packaging passes.
On 2026-10-29, the current Windows GUI source passed 102 frontend tests, lint,
typecheck and production build. `pnpm desktop:build` produced MSI and NSIS
installers from an isolated Cargo target because the running GUI held the default
release binary open. The artifacts are in
`target/gui-final-release-20261029/release/bundle/`.
After the Windows provider-environment and visible-error fixes, the current GUI
source and Rust daemon source were refreshed in the Arch container. All 100 GUI+tests passed there, and `pnpm desktop:build` rebuilt the Linux sidecar and GUI,
producing `.deb`, `.rpm`, and AppImage packages. Package metadata was checked;
the AppImage stayed open for 15 seconds under Xvfb and D-Bus and started its
daemon on an isolated socket. The artifacts are in
`target/linux-omarchy-final-2026-10-29/`. This remains container verification;
a real Omarchy desktop check is still open.
On 2026-10-29, a clean source copy passed all 102 GUI tests, lint, TypeScript
checking, the Vite production build and `pnpm desktop:build` in Arch Linux.
Fresh deb, rpm and AppImage packages plus build and launch logs are in
`target/gui-linux-release-audit-20261030/`. SHA-256: deb
`FF6C0128C4A2EC4DAA395C366A7B90554E88BA502707743951702735308CFDBF`, rpm
`D59F254A87CC9571608A5B714B132B6EDF23FC54FB07CB7CA385CB074BDF5B5C`, AppImage
`4B83A6CCC976F2A1A325BE32DC34595B893C3332BA5136E00717E90C052EE85C`. The
AppImage stayed open for 15 seconds under Xvfb and D-Bus, showed a `Maitty`
window, and started its bundled daemon on an isolated socket; both processes
were stopped after the check. AT-SPI exposed an eight-node tree without labels,
so this confirms launch and daemon startup, not content-level accessibility.
Real Omarchy desktop acceptance remains open.
After the native-history return-path fix, Arch GUI tests passed again (100/100),
and `pnpm desktop:build` rebuilt the current desktop and daemon sidecar. The
review packages in that directory were refreshed: AppImage
`A3AB9412AC468EA4321F28788A57DD2EC90608CB925334C95286840609BD082F`, deb
`A86AB4A7EC51F9590F83418585EBC708DABBFDB08D3A220AB656FE93243F8BD6`, rpm
`A148CD3931F83348258BDF04F9729E2ABE8182621DEEEAA48945BEC9C0D8F062` (SHA-256).
The reverse handoff now passes an isolated Windows live Codex roundtrip through
the desktop runtime: structured chat replies, the exact thread moves to native
Codex, native history reconciles on return, and chat accepts a second prompt.
The daemon process scan regression is covered by both a focused logic test and
the native handoff integration test. The rebuilt Windows MSI and NSIS installers
include the GUI-specific daemon sidecar and are in
`target/gui-handoff-fix/release/bundle/`.
Arch rebuilt the current sidecar and GUI into deb, rpm and AppImage packages in
`target/gui-handoff-fix-linux/`. The AppImage opened a visible window under
Xvfb, and its bundled daemon started on an isolated socket. SHA-256: AppImage
`AB89A1A3FF870A4B5A327A7C50D475E5A4DA66FFA4C66E510BB3DBB2F4BB6FB2`, deb
`0CE94FE59F1E348A7FF7EC4E9620719CFDAD6445EEBF3C86E23B6FA188046E43`, rpm
`C3269EB213ADE730BE9E414A1851E5246F87F4FE59159C5B9D22364E27BC21A3`.
Real Omarchy desktop and macOS package checks remain open.
The Windows Rust gate passes on this source, including the default-parallel test
run after branch-view fixtures stopped depending on host thread scheduling. CI
has not run on this worktree yet, and installing and launching on a real Omarchy
desktop remains part of Linux release verification.
On 2026-10-29, the Windows GUI check passed again: `pnpm test` (103 tests),
`pnpm lint`, and `pnpm typecheck`. The installed NSIS app showed the empty
workspace, connected daemon, and Codex-ready state; Claude remains disabled.
The GUI now says Claude needs a verified CLI protocol and supported sign-in
instead of the generic "not supported yet" message.
`pnpm build` also produced the production frontend. Vite reported a 628 kB main
JavaScript chunk, above its 500 kB advisory threshold; the xterm chunk is split
out at 331 kB. The installed Claude Code `2.1.314 --help` lists streaming input
and output, host permission prompts, session IDs and resume flags; this confirms
the CLI exposes those options, not that Maitty's live protocol or authentication
is verified.
The latest `pnpm desktop:build` completed in the isolated target
`gui/src-tauri/target/gui-roadmap-tui-build/release/` and built the Windows GUI,
MSI and NSIS packages. MSI SHA-256:
`88E49F85D854367C9456D8BB0667C9FF570A67D34B6EC4980DA65839B35C0BDB`; NSIS
SHA-256: `0B26C8DC49D6EEB68AE79CA5DF31FD74FF4CE9D7A050AB116602D262C4DE345C`.
The new executable started with `MAITTY_DATA_DIR` and `MAITTY_CONFIG` under
`target/gui-roadmap-final-inspect/` and socket `gui-roadmap-final-inspect`; its
window state showed the daemon connected, Codex ready, and the updated Claude
message. The GUI window could not be brought to the foreground: Windows
automation returned `failed to activate captured window` for both activation and
the window's Raise action. No prompt was sent, so package-level interaction
remains unverified.
A clean Arch source copy with the same GUI changes passed its 103 tests, lint,
typecheck and production build, then produced Linux packages in
`target/gui-linux-status-latest-20261029-packages/`. `dpkg-deb -I` and `rpm -qpi`
read the package metadata, and `AppImage --appimage-help` ran. SHA-256: deb
`8F44B4F2B5B2DD6C30954CC45A6C8C522EC734BB9612B556055F991EC2B6EF42`, rpm
`2BD2EBBAB4EC5C8014F3CD02DF75017CB7AEE31D15B89B2C411299F3E13B6F37`, AppImage
`B17B15C1CE72C6468C9B197B28D4842A65B602A07616C1B201CECA85480F81BD`. The
AppImage then stayed running for 15 seconds under Xvfb and D-Bus with its
bundled daemon on the isolated socket; both test processes were stopped after
the check. This is Arch container startup evidence, not Omarchy desktop
acceptance.
The full Windows gate passed after adding fake-Claude-CLI initialization,
streaming, approval and interrupt tests: `cargo fmt --check`, `cargo build`,
`cargo test` (1,349 passed, 5 ignored, and 3 naming tests),
`cargo clippy --all-targets -- -D warnings`, and checks for
`x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`. The Codex lifecycle test
fixture now passes only its declared fake-provider environment instead of
snapshotting the machine environment; the latter could race with tests that
change process environment variables. This does not verify Claude's live
protocol or authentication.
After moving shared clock helpers, inverting the daemon-to-CLI startup hook,
and extracting Claude turn-event projection, the current Windows workspace passed
`cargo build --workspace --locked`, `cargo test --workspace --locked
-- --test-threads=2` (1,384 passed, 5 ignored), workspace clippy with warnings
denied, formatting and whitespace checks. The core and all extracted packages
cross-check for Linux and macOS. A full workspace Linux cross-check from Windows
is blocked by the GUI's `libdbus-sys` requiring a configured Linux sysroot;
the full macOS cross-check is blocked because this host has no target C compiler.
These host limitations do not replace native desktop builds.
The terminal-only Windows install also passed with an isolated Cargo target:
`cargo install --path . --root target/gui-roadmap-tui-install-smoke --locked
--target-dir target/gui-roadmap-tui-build`; the installed binary printed
`maitty 0.50.6` for `--version` and opened `--help`. This confirms the Rust CLI
install path without building the React/Tauri frontend. It does not replace the
Linux/Omarchy desktop acceptance check.
Exit: Codex passes real roundtrip/recovery/permission checks on each supported
OS; Claude passes the same checks with a supported authentication path; the
full gate and GUI-specific checks pass; and release/update/install behavior is
verified. Native-TUI transfer is not a release requirement. No provider is
advertised on compile checks alone.
Claude's structured transport is the installed Claude Code CLI's headless
`stream-json` protocol, following PandaCode's provider-native pattern; see
[ADR 0039](adr/0039-claude-code-cli-streaming.md) and [the Claude stack rules](stack/claude-code-cli.md). Current research is in
[Claude Code chat transport](research/claude-code-gui-transport.md). The approved
authentication path is the user's existing local Claude Code sign-in; Maitty
does not collect credentials or offer a separate login. Anthropic's current Help
Center says third-party app use can draw on Claude plan limits. Claude remains
disabled until the installed CLI protocol and live provider behavior pass the
same recovery, approval and cancellation checks as Codex. The initial
GUI provider scope is Codex and Claude, and the OS floors are recorded in the
Tauri stack guide. Verified Claude CLI versions and the packaging/update
approach remain open.
The daemon now contains an initial provider adapter for streamed turns, approvals,
interrupts and exact-session identity. Its launch guard remains closed. Parser
and projector tests plus a local fake CLI exercise JSONL initialization, a
synthetic streamed text turn, acknowledgement, history notifications, host
approval request/reply, an active-turn interrupt request/reply, and clean
shutdown through the adapter. These tests do not cover daemon dispatch, a real
Claude CLI turn, authentication, or actual CLI cancellation/recovery behavior.
No live end-to-end CLI protocol or supported authentication path has been
verified, so Claude is not selectable in the GUI yet.
The locally installed Claude Code CLI now reports `2.1.314`, while the adapter
only accepts `2.1.312`. Following the CLI reference, the adapter now routes
approval prompts through `--permission-prompts host`; a focused test verifies
the generated arguments. PandaCode's `--permission-prompt-tool stdio` is outside
the public parameter contract, which defines that flag for a named MCP tool.
The host message flow still needs live verification. Keep the provider disabled
until the CLI version, permissions, authentication and turn lifecycle are
verified end to end.
Existing TUI support for all other agents remains; more structured providers and
GUI remotes need later scope.
Phase 37 remains active. Real Omarchy desktop acceptance, macOS package/build/run,
hosted CI, live Claude protocol/authentication, and installer upgrade/uninstall
checks remain open. The Debian and Fedora container runs are package smoke tests,
not substitutes for the user's Omarchy desktop.
On 2026-10-30, Windows GUI tests passed (103 tests across 16 files), lint passed,
and `pnpm desktop:build` produced MSI and NSIS packages using an isolated Cargo
target because an existing Maitty process held the default release binary open.
The packages are in `gui/target/gui-refactor-release-20261030/release/bundle/`.

## In scope

Existing features below are shipped unless marked. Optional chat GUI and native
conversation continuity are planned in phases 34–37:
[intent](features/chat-gui/intent.md) · [spec](features/chat-gui/spec.md).

**Core**
- Daemon owns sessions; detach / reattach; many clients — shipped — `crates/maitty-runtime/src/daemon/mod.rs`, ADR-0001
- Sessions survive a daemon restart (agents resumed, commands re-run) — shipped — `crates/maitty-runtime/src/daemon/persist.rs`, `restore` in `crates/maitty-runtime/src/daemon/mod.rs`
- Windows (ConPTY, named pipes, PowerShell folder tracking), macOS, Linux — shipped — `crates/maitty-runtime/src/daemon/term.rs`, `crates/maitty-runtime/src/ipc.rs`
- SSH remotes (`--remote host`) — shipped — `crates/maitty-runtime/src/ipc.rs` `connect_remote`

**Agents**
- Personal and project agent profiles / saved workflows — removed from Maitty in
  the v0.34.0 release candidate at the user's direction; legacy config tables are
  ignored, and old saved-run data is tolerated during session restore.
- Status detection: working / needs you / done (until seen) / idle, from screen patterns, OSC progress and agent hooks — shipped — `crates/maitty-runtime/src/daemon/scan.rs`, `maitty hook` in `crates/maitty-runtime/src/cli.rs`
- Trusted status reports (process tree or `MAITTY_PANE_TOKEN`) — shipped — `crates/maitty-runtime/src/daemon/term.rs`, `crates/maitty-runtime/src/cli.rs`
- Agent kinds: Claude, Codex, Gemini, OpenCode, Cursor, Copilot, Amp, Qwen, Aider, Goose, Crush, Droid, Pi, Kiro, Grok, custom `[[agents]]` — shipped — `crates/maitty-runtime/src/config.rs`
- New agent task box (project, worktree, agent, model, effort), presets and recipes as its choices, prewarm — shipped — `crates/maitty-runtime/src/client/screen/dialogs.rs`, `crates/maitty-runtime/src/client/recipes.rs`
- Agents talk to agents (MCP server `maitty mcp`) — shipped — `crates/maitty-runtime/src/mcp.rs`
- `maitty send --wait`, `maitty wait`, `maitty read` — shipped — `crates/maitty-runtime/src/cli.rs`
- At most a set number of helpers working at once (`[sessions] helpers_at_once`; an old `[mcp]` value is read as a fallback) — shipped 0.25.0 — `Daemon::starting` in `crates/maitty-runtime/src/daemon/mod.rs`
- A commander running a fleet (shipped in v0.20.0, #32–#38): removed in phase 19 (ADR 0014); old `[fleet]` and `[team]` tables are read and ignored

**The TUI (one shared session layout; ADR-0007, superseded by ADR-0012)**
- Flat rendering style: square, full-bleed sidebar, tabs, panes and sheets; sidebar/pane focus zones, no global footer; shipped in v0.36.0; `crates/maitty-runtime/src/client/screen/draw.rs`, ADR-0012
- Floating look: rounded cards with gaps, title and state in the border, unfocused cards faded, pill tabs, no app footer; floating or tiled, corners, gap, dim, focus border, pill ends in Settings — shipped by v0.30.1 — `crates/maitty-runtime/src/client/screen/card.rs`, `screen.rs`, ADR-0007
- Sidebar card: projects → sessions, one-line rows (mark, name in its state's colour, agent, uncommitted lines, age), a second line only when it says more, attention sort, keyboard and mouse, `+ New` / `Menu` foot — shipped — `crates/maitty-runtime/src/client/screen/side.rs` `draw_side`
- Panes: tabs as pills (named in their pill), any number of splits (grid for 3+), drag the gap between them, ✕ in the border — shipped — `crates/maitty-runtime/src/client/screen/draw.rs` `draw_session`
- Inbox (what needs you first, type to go anywhere), command palette (plain-word commands), key map (searchable, second steps), actions list — shipped — `crates/maitty-runtime/src/client/screen/inbox.rs` `draw_inbox`, `crates/maitty-runtime/src/client/screen/keyoverlay.rs` (key map and search commands), `crates/maitty-runtime/src/client/screen/leader.rs`
- Right-click menus, confirm before closing, toasts — shipped — `crates/maitty-runtime/src/client/menu.rs`
- Splash: Resume / New shell here — shipped — `draw_splash`
- Settings popup grouped by section, key rebinding, themes with contrast audit — shipped — `crates/maitty-runtime/src/client/screen/dialogs.rs` `draw_settings`, `crates/maitty-runtime/src/theme.rs`
- Leader keys (a shell is `n`, only kept features have keys), Alt+arrows without the leader, armed pill in sky — shipped by v0.30.1 — `crates/maitty-runtime/src/keys.rs` `DEFAULT_PREFIX_KEYS`, ADR-0009
- Copy on select with toast, copy mode, Ctrl+click paths, paste images — shipped — `crates/maitty-runtime/src/client/copy.rs`, `crates/maitty-runtime/src/client/pick.rs`

**Code and git**
- Worktrees per agent, create / move / remove with hooks — shipped — `crates/maitty-runtime/src/daemon/git.rs`, `crates/maitty-runtime/src/project.rs`
- Worktree clean-up: closing the last pane of a worktree Maitty made removes it only when it is clean and everything in it is merged (clean with commits found nowhere else: it asks, and Enter keeps the folder; uncommitted changes: it stays and says why; every removal or keep is in `daemon.log`), and a list per repo says why each is still around and removes the ones you tick — shipped — `crates/maitty-runtime/src/tidy.rs`, `crates/maitty-runtime/src/daemon/worktrees.rs`, `crates/maitty-runtime/src/client/screen/tidy.rs`
- Files (tree, preview, in-place edit, external editor), find file, search code — shipped — `crates/maitty-runtime/src/client/files.rs`, `crates/maitty-runtime/src/client/find.rs`, `crates/maitty-runtime/src/client/views.rs`
- Changes (diff, review marks, commit, open a PR, git init offer), branch switcher — shipped — `crates/maitty-runtime/src/client/branch.rs`, `crates/maitty-runtime/src/client/views.rs`
- Worktree hooks (`.maitty.toml` `on_create` / `on_remove`, run once allowed; `on_create` finishes before anything starts in the worktree) — shipped — `crates/maitty-runtime/src/project.rs`, `Ev::WorktreeCreated` in `crates/maitty-runtime/src/daemon/mod.rs`
- Tasks (a worktree's review: commit, merge, PR, throw away) — shipped — `crates/maitty-runtime/src/client/tasks.rs`
- Own worktree for `claude`/`codex` typed into a shell in a repo's main folder — shipped 0.9.12; a launcher per agent first on a shell pane's PATH (any shell) built on the working tree — `maitty agent-dir`, `crates/maitty-runtime/src/shim.rs`, `daemon/term.rs` `agent_functions`
- Usage and limits meter, context per agent, continue after a limit — shipped 0.10.0 — `maitty statusline`, `crates/maitty-runtime/src/daemon/usage.rs`
- Automatic idle sleeping and its sleep/wake screen and animation: removed; legacy recovery remains only for sessions saved by older versions.
- A session's share of the machine (CPU and memory) in its pane's title when it's large — shipped 0.25.0, moved off the sidebar row 0.26.2 — `crates/maitty-runtime/src/daemon/scan.rs`, `load_note` in `crates/maitty-runtime/src/client/screen/side.rs`
- Merge that cleans up (the worktree and its branch go) — shipped 0.11.0 — `crates/maitty-runtime/src/client/view_keys.rs`

**Identity**
- Renamed hydra → Seshi (v0.30.1, ADR-0008), Seshi → Vibecasso (ADR-0018), then Vibecasso → Maitty (ADR-0020): the name lives in `Cargo.toml` (ADR-0019); one-time move of an old install — `config::migrate_old_names`
- Night (default) and Day themes among 170 built-in themes (62 by v0.42.0); block MAITTY wordmark on the splash — shipped in v0.42.0 — `crates/maitty-runtime/src/theme.rs`, `crates/maitty-runtime/src/client/screen/splash.rs`

**Extras**
- Alert when an agent needs you: Maitty's two rising notes and an in-app notice; a desktop notification only with no window open. Completions stay quiet — `crates/maitty-runtime/src/alert.rs`
- Click a notification to jump to its session — shipped — `crates/maitty-runtime/src/reveal.rs`, `crates/maitty-runtime/src/alert.rs` (#19)
- Sidebar groups sessions by where they work now (repo, or folder outside git); no projects to open; splits stay with their session — shipped — `crates/maitty-runtime/src/client/screen/mod.rs`
- Releases and installers (Windows, macOS, Linux; checksums), `maitty update` and a daily update check — shipped — `.github/workflows/release.yml`, `install.sh`, `install.ps1`, `crates/maitty-runtime/src/update.rs`
- CI: test and clippy on Linux and Windows, a macOS check — shipped — `.github/workflows/ci.yml`
- Config sync across machines (local file never synced) — shipped — `crates/maitty-runtime/src/sync.rs`
- SSH remotes (`--remote host`) — shipped — `crates/maitty-runtime/src/ipc.rs` `connect_remote`
- `maitty doctor` — shipped — `crates/maitty-runtime/src/cli.rs`

## Out

- Saved role profiles and the planner/approval/worker/reviewer workflow runner were removed by user decision (2026-10-18). Use the agent's own configuration and external workflow tools; Maitty remains the session manager. The team list for the commander that phase 17 allowed (2026-10-22) was removed with the commander in phase 19. A Maitty-run pipeline, where Maitty starts or orders stages itself, stays cut.

- Features cut 2026-10-09 remain out unless re-approved: open project, go to, next agent that needs you, arrange panes, undo automatic workspace, the map, ideas, races, run a preset as a list (presets stay in the new-agent dialog), past chats, agent tools, extensions, and the tmux stand-in for Claude agent teams. Memory per session later returned as the load line under a busy session. Items explicitly restored since the cut are listed in In scope; races remain cut.
- Desktop notifications while a maitty window is open, the terminal bell on an alert, and choosing among system sounds (or your own sound file) — cut 2026-10-11 (the user's call: three overlapping alerts, two of them the Windows error ding). Bringing one back needs a new decision.
- Other structural terminal layouts (workspaces, tree, dock, alternate sidebar) — declined 2026-10-03. Flat is an accepted rendering style of the shared session layout, per ADR-0012; it does not introduce another session model.
- Phase 7 proposals canceled 2026-10-17: named hosts and remote panes/agents (#23); terminal graphics and Kitty keyboard protocol (#27); launcher, templates, recording/replay, screenshots and key display (#29); web terminal and SSH server mode (#30); package-manager distribution (#31). Revisit only through a new proposal. This does not remove shipped SSH remote-client support.
- Claude Code's own fork list showing forks that are not running (seen 2026-10-28): the `main / fork …` block with token counts is drawn by Claude inside the pane; Maitty only shows it. Not fixed, the user's call. Not checked: whether the sidebar's `N agents` count can stay stuck after a missed subagent stop (`crates/maitty-runtime/src/daemon/status.rs`); look there first if it is reported.
- Fleet work declined 2026-10-24 (the fleet was removed in phase 19): the phase 9 live fleet check, the designed team view, pause the fleet, stop all, cost per fleet.

## Later

- Non-blocking validation follow-ups: use the four Phase 10 features in real work; address the squash-merged-branch detection gap.
- Phase 18 follow-ups, kept 2026-10-24: `maitty wait` should exit non-zero when a check needs approval or could not run, and approving a check has no default key. Comes back after phase 28, or sooner if a script relies on `wait`.

- Personas are dropped, not deferred: saved role profiles were removed 2026-10-18 (see [Out](#out)). Phase 15 replaces the idea with checkpoints that Maitty checks itself.
- Checkpoint kinds beyond the first (`commits`, `approval`, `review`) and a project-defined checkpoint list — deferred 2026-10-22 until Phase 15's verify check has been used in real work. `approval` and `review` sit close to the workflow runner cut on 2026-10-18; each needs the user's explicit re-approval before any ticket.
- Threads between agents, take over and hand back — deferred 2026-10-11: waiting on a design for it. Comes back when the prototype is in.
- Live GUI-to-native-TUI Codex transfer and platform acceptance — deferred until the standalone Codex/Claude GUI is usable and packaged; see [ADR 0038](adr/0038-standalone-gui-first.md). Existing TUI support remains required.

## Decisions

- [ADR-0001](adr/0001-daemon-owns-sessions.md) — A daemon owns every session; clients only draw. Rules out: sessions inside the UI process.
- [ADR-0002](adr/0002-portable-pty-and-vt100.md) — portable-pty (ConPTY) and vt100 for terminals. Rules out: a home-grown PTY layer.
- [ADR-0003](adr/0003-msgpack-over-local-sockets.md) — msgpack frames over local sockets, versioned (added default fields need no bump, amended 2026-10-24). Rules out: incompatible message changes without a bump; network listeners.
- [ADR-0009](adr/0009-leader-keys-after-the-cut.md) — Leader keys: a shell is one key, and only kept features have keys (supersedes ADR-0006). Rules out: keys for features that don't exist; defaults that differ from the key map shown in the app.
- [ADR-0008](adr/0008-rename-to-seshi.md) — The app was called Seshi (the name superseded by ADR-0018, then ADR-0020); an old hydra install is brought over once.
- [ADR-0012](adr/0012-flat-rendering-style.md) — Add a flat rendering style over the shared session layout; Cards stays default.
- [ADR-0014](adr/0014-v8-cleanup.md) — Bring over the v8 cleanup, keeping what the user already decided. Rules out: bare letters acting in the sidebar; per-state glyphs on session rows; the fleet, the Team tab, the subagent tree and a helper mode.
- [ADR-0041](adr/0041-chat-gui-shared-conversations.md) — Optional chat GUI over daemon-owned native conversations. Rules out: replacing the native TUI, client-owned sessions, concurrent runtime writers and most-recent-session handoff.
- [ADR-0038](adr/0038-standalone-gui-first.md) — Make the standalone GUI usable before native terminal transfer; Windows, macOS and Linux release checks remain required.
- [ADR-0020](adr/0020-rename-to-maitty.md) — The app is called Maitty, "All agents on deck"; a vibecasso or seshi install carries over once.
- [ADR-0019](adr/0019-the-name-lives-in-one-place.md) — The app's name lives in `Cargo.toml`; code takes it from `brand`. Rules out: writing the app's name out in code, adapters or comments.
- [ADR-0018](adr/0018-rename-to-vibecasso.md) — The app was called Vibecasso (the name superseded by ADR-0020), a clean break; a vibecasso install carries over once. Rules out: a `vibecasso` command, new code or paths under the old name.
- [ADR-0017](adr/0017-keys-other-apps-use.md) — Default shortcuts are one key each, none needing Shift, keys other apps use. Rules out: capital or Shift-symbol defaults (but `?`, ADR-0022); two keys for one action.
- [ADR-0016](adr/0016-sticky-shortcut-mode.md) — The leader opens a shortcut mode that stays until Esc, the leader, a click into a pane or a typing action. Rules out: letters acting outside that shown mode.
- [ADR-0013](adr/0013-v7-frame.md) — Draw the v7 frame: tabs in the pane title (the "no margin" part replaced by `ui.padding`, ADR-0021; the tabs moved to a row of their own by ADR-0028).
- [ADR-0015](adr/0015-four-named-layouts-and-grid.md) — One Layout setting: Floating, Tiled, Compact, Grid; Grid is a look over the shared layout (the four cut to Floating by ADR-0027). Rules out: Grid's own sidebar, top bar, fixed grid, zoom and bare keys.
- [ADR-0035](adr/0035-the-tab-row-sits-outside-the-pane-boxes.md) — Floating's tab row sits over the pane boxes, level with the sidebar's top, not inside them (amends ADR-0028, ADR-0041). Rules out: tabs or a plate inside a pane's border.
- [ADR-0029](adr/0029-alt-letters-and-closing-without-asking.md) — Alt+letter shortcuts for the everyday actions (`ui.alt_keys`); every close asks first (amends ADR-0017). Rules out: bare keys acting outside shortcut mode; Alt+[ and Alt+].
- [ADR-0030](adr/0030-panes-titled-like-an-editor.md) — Panes titled like an editor's: no title on a lone pane, no ✕ or state word on a split pane's; sidebar tab lines skip the session's own tab; padding 0; toasts per look (amends ADR-0027, ADR-0028). Rules out: tabs back in the pane border.
- [ADR-0028](adr/0028-tab-rows-and-popups-drawn-like-panes.md) — A tab row over the panes in both looks; every popup and toast boxed as its look boxes a pane (amends ADR-0013). Rules out: tabs in a pane's title; frameless filled popups.
- [ADR-0027](adr/0027-two-looks-and-tabs-in-the-sidebar.md) — Two looks, Floating and Hairline; Grid's colours an option on Floating; a session's tabs listed under its sidebar row. Rules out: a look per idea; a tab's state shown only on the pane title.
- [ADR-0021](adr/0021-one-line-rows-one-rounding-sticky-mode.md) — One-line session rows, one rounding setting (full, corners, off), padding round the screen, and shortcut mode that stays until Esc, the leader or a click (amends ADR-0016). Rules out: a second line that only names the state; separate corner and pill-end switches.
- [ADR-0022](adr/0022-question-mark-all-keys-c-changes.md) — `?` after the leader lists all keys, `c` opens Changes, copy mode has no default key (amends ADR-0017). Rules out: `F1` and `d` as defaults.
- [ADR-0023](adr/0023-the-mode-is-always-shown.md) — The mode is always shown in the corner (`TYPING` or `SHORTCUT`), the window is framed in sky in shortcut mode, and a key with no action says how to type (amends ADR-0016 and ADR-0021). Rules out: an unlabelled input mode; keys dropped without a word.

## Rules

- Structured conversation trust, ownership and history invariants are target contracts in the [chat GUI spec](features/chat-gui/spec.md); not yet implemented, tracked by phase 34.

- The daemon's socket only accepts the user who started it — `crates/maitty-runtime/src/ipc.rs` — SECURITY.md
- Status reports need the pane's token or its process chain — `crates/maitty-runtime/src/daemon/mod.rs` hook handling — AGENTS.md non-negotiable 5
- Nothing in a cloned repo runs without the user's approval — `crates/maitty-runtime/src/project.rs` — SECURITY.md
- Client and daemon refuse to talk across protocol versions — `crates/maitty-shared/src/protocol.rs` `PROTOCOL_VERSION`, checked on `Hello` — ADR-0003
- Config selects one of two looks (Floating, Hairline) over the one shared session layout; older looks fold into them — `crates/maitty-runtime/src/config.rs` `Ui.layout` / `layout_kind` / `fold_retired_layout`, `crates/maitty-runtime/src/client/screen/card.rs` `Look::of` — ADR-0027
- Every built-in theme passes the contrast audit — `cargo test` (theme audit test in `crates/maitty-runtime/src/theme.rs`) — readability
- Both platform sides compile — `cargo check --target x86_64-unknown-linux-gnu` / `aarch64-apple-darwin` — AGENTS.md non-negotiable 3
- clippy is clean — `cargo clippy --all-targets -- -D warnings` — CODE-STANDARDS
- A session's "started by" is only set by the daemon, from a pane whose secret checked out — `crates/maitty-runtime/src/daemon/mod.rs` `starter`, tests in `crates/maitty-runtime/src/daemon/logic_tests.rs` — AGENTS.md non-negotiable 5
- `maitty_answer` sends a key only for the prompt id the caller passed — `crates/maitty-runtime/src/mcp.rs` `answer` / `prompt_seen`, test `an_answer_goes_only_to_the_prompt_it_saw`
- Text with invisible characters never reaches an agent through `maitty send`, `maitty_send` or `maitty_start` — `crates/maitty-shared/src/hidden.rs` `check`, tests there and `send_refuses_hidden_characters_before_reaching_any_pane`

## Records

Chat GUI: [intent](features/chat-gui/intent.md), [spec](features/chat-gui/spec.md),
[ADR 0041](adr/0041-chat-gui-shared-conversations.md),
[source research](research/gui-agent-continuity.md).

Docs: `AGENTS.md`, `docs/` (architecture, standards, testing, security, UI, API patterns), `docs/stack/rust.md`.
Design: [`DESIGN.md`](DESIGN.md) (the current design). The design handoffs (v1–v8, Flat, wake-up, inspector, splash) are in git history.
Glossary: none yet.
Feature docs: `docs/features/`. Each live intent is linked from its phase; shipped or removed intents are in git history.

## Rework

- none recorded yet.
