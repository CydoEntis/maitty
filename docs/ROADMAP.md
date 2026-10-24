# Plan: Seshi

Source of truth for scope, order, decisions and rules. Tickets hold the detail.
Read this before starting work. If work conflicts with it, stop and say so.
Last reconciled: 2026-10-22 at `97d6d19` on `main` (v0.37.0).
Verify: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo check --target x86_64-unknown-linux-gnu && cargo check --target aarch64-apple-darwin`

Seshi is a terminal multiplexer built for running many coding agents at once: see
which ones need you, jump to them, and keep them running when you leave. See
`AGENTS.md` for the mission and `docs/ARCHITECTURE.md` for how it is built.

## Now

The latest published release on `main` is v0.41.0 (2026-10-23): the v8 cleanup
([Phase 19](#19--the-v8-cleanup--shipped-in-v0410-live-checks-open)). Every action goes through the
leader, one key overlay and a grouped Menu replace the palette and the shortcut screens, sessions are
coloured dots, and a collapsed sidebar becomes a top strip. A new shell is `n`, and an agent typed in a
shell pane always starts in a new worktree. The fleet commander, the Team tab and the sidebar subagent
tree are gone, and helpers ask first. Updating restarts the daemon in one step. It moves the wire protocol
to 45, so a new `seshi` replaces an older daemon by itself (it asks once when agents are working). v0.40.0
(2026-10-23) was the v7 layout
([Phase 18](#18--the-v7-layout-and-checkpoints--shipped-in-v0400-live-checks-open)), with several
named checks per project, stale passes and repo checks you approve; `seshi handoff`
([Phase 13](#13--borrowed-from-t3-code-round-two--shipped-in-v0400)); and team roles for the commander ([Phase 17](#17--team-roles-for-the-commander--shipped-in-v0400-live-trial-open)).
It moves the wire protocol to 44, so restart the daemon after installing it, and update
remote daemons too. v0.37.1 (2026-10-22) added a restart button for
the daemon after an update, and a prompt instead of an error when a new `seshi` meets an
older daemon ([Phase 16](#16--restart-the-daemon-from-the-update-flow--complete-2026-10-22)).
v0.37.0 brought checks that Seshi
runs itself ([Phase 15](#15--checks-seshi-runs-itself--active-chosen-2026-10-22)), with
the sidebar grouping by folder and tool icons. It moves the wire protocol to 43, so
restart the daemon after installing it, and update remote daemons too. Five platform
archives and checksums are published, and the release workflow's install checks passed
on Windows, macOS and Linux. v0.36.13 restarts the local
daemon and reconnects the window automatically after the last pane closes,
without interrupting open sessions. Remote daemons still need to be updated on
their host. It includes the v0.36.12 Codex root-session and helper detection,
plus the v0.36.11 sidebar and session reliability work below. Five platform
archives and checksums are published, and installer checks passed on Windows,
macOS and Linux. Provider reload and live reporting still need confirmation in
sessions on the machines where those providers run. Pi full-screen scroll
behavior remains an open live-validation item below.

Codex helper visibility needs cross-platform live validation. The integration
records the root thread on session start so local rollout-file detection can
work when Codex omits child events; Windows additionally needs a PowerShell
`commandWindows` invocation. Restart Codex after installing the hooks, then
verify active and completed helpers on both Windows and Linux/Omarchy.

User-reported sidebar hiding was removed: idle and completed sessions remain
visible, with no snooze or automatic settling. Background top-level completions
now use the soft alert sound; the focused session and helper completions stay
quiet. Starting an agent in a git project defaults to a new worktree, and a
stale explicit worktree choice now asks the user to choose again rather than
silently starting in the main checkout.

Seshi is terminal-first; direct subagent messaging remains out of scope. The v0.36.5 release includes cross-agent idle-status recovery; v0.36.6 passed Windows, macOS and Linux installer checks.

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
shape, dependency, migration or permission change. Reverted in v0.36.2 (`5c3313d`):
Settings is a centred modal again, as `docs/UI.md` says.

User-requested working-spinner selection and Pi/DeepSeek scrolling correction:
add an Appearance picker with animated presets, Static and validated custom
frames, preserve saved choices, and investigate wheel/keyboard transcript
scrolling in Pi's regular/fullscreen modes. Also scope Codex helpers to exact
owning threads and retain a compact status/session rail when the sidebar hides.
User-reported spurious sounds also add blocked-sound debounce and exact status
checks in the client and headless daemon. Red status remains immediate.
Spinner selection, exact Codex helper ownership, collapsed rail and false-blocker
sound-race correction: merged to `dev`, shipped in v0.31.0. Ordered build, full tests (307 passed,
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
See [status setup and its limits](agent-status.md).

User-requested mouse access and provider-independent status correction: expose
Status setup and all commands through Actions, retain
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
no dependencies, new permissions or wire shapes were added.

The user subsequently chose to remove Seshi's profile and workflow feature so the
terminal multiplexer stays focused. v0.34 removes that UI, its runner and config
surface; this supersedes the earlier v0.31 workflow implementation. Old profile
and workflow config tables remain ignored when loading existing config files.

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

Historical shortcut/documentation audit: All keys expose configured bindings and contextual controls. The README and shortcut reference were rewritten; this removal deletes stale profile/workflow entries while keeping the searchable keycap reference. Status: implemented locally. The ordered gate passed: build,
276 tests passed / four ignored, clippy with warnings denied, Linux and macOS
cross-checks. Rendered reference inspected at 80×24, 100×30 and 160×45; tests cover
all active bindings, aliases, replacements/unbinding, custom commands, key-name
search, contextual search, scrolling and Actions entry without running commands.
Live terminal/clipboard verification remains outstanding as described below.
Release target: v0.30.0.

User-authorized work: clear session state and workflow controls, shipped in v0.30.0. Keep sidebar
selection independent of hover; distinguish update versions; expose status
evidence; prepare workflow instructions; show stages and Inbox approvals; measure
switching, scrolling and resizing under output. Extends existing status/workflow
surfaces, including the existing phase-8 status popover. Updates install without
restarting the window or daemon. Status: implemented locally; the ordered gate
passed (`cargo build`, 274 tests passed / four ignored, clippy with warnings
denied, Linux and macOS cross-checks). Rendered setup, versions, evidence and
Inbox screens inspected; sidebar checks cover legacy recovery states too. The opt-in
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

The saved profile/workflow feature shipped in v0.30.0 and was removed from the current product on 2026-10-18. Its historical requirements are no longer an active roadmap item; Seshi remains focused on session management and status.

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

Active phase: **15 — checks Seshi runs itself** (chosen 2026-10-22)
Next: three follow-ups the user approved on 2026-10-23: `a` on a sidebar row opens the row menu, an
amber for needs-you in every theme (built), and `seshi wait` reporting a check waiting for approval or that could
not run. Then the Phase 17 live trial and the Phase 18 live checks. Then
[Phase 20](#20--borrowed-from-tuios--queued-2026-10-23), borrowed from TUIOS (approved 2026-10-23).

Current user-requested work: expand the built-in themes to cover the 25 requested
families and add a searchable preview browser in Appearance. Acceptance: all
families and documented variants are selectable; search, keyboard paging, mouse
selection, preview, cancel, and saving work; every built-in passes the contrast
audit. Verification: the full AGENTS.md gate and a nested-server UI check.
Status: integrated for 0.28.0 with 63 built-in choices; verified in the release
worktree. Sources: [theme research](theme-credits.md).

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

The user canceled this direction before implementation and chose to keep Seshi
terminal-first, using T3 Code when they want a chat-first GUI. No code, tickets,
branch, or worktree were created. Revisit only through a new proposal.

### 12 - Flat rendering style - shipped in v0.36.0

Add an optional full-bleed Flat style from the supplied design handoff. Keep the
Cards layout as default and share session state, tabs, splits, sheets, status and
mouse behavior. Acceptance: choose Cards or Flat in Appearance; flat surfaces
have square edges and no gaps, keyboard focus zones,
working mouse targets and sensible narrow-screen behavior; docs and tests cover
the option; the verification gate passes before the phase ships.

### 13 — Borrowed from T3 Code, round two · shipped in v0.40.0

Approved by the user 2026-10-19 after a second read of `pingdotgg/t3code`.
Tickets: #116 one function builds an agent's start command, #117 the handoff brief, #118 `seshi handoff`.

- **Handoff:** `seshi handoff <pane> --to <agent>` starts another agent in the same worktree with a
  short text brief (the first prompt, branch, changed files, a tail of recent output). Text only; it
  does not carry the old agent's native context, and says so. Built from `seshi worktree` and
  `seshi send`, so no wire change is expected.
- Phone alerts are not part of this phase: ntfy alerts already ship (`[notify] phone_topic`). An earlier
  draft of this phase listed them by mistake.
- Acceptance: a ticket, tests, docs and a passing verification gate before it is marked shipped.

Status 2026-10-22: built and released in v0.40.0. The gate passed on the merged tree
(575 tests). A live run on an isolated daemon with stub agents checked the brief, the refusals (no pane, no
agent, unknown agent) and the grants. The brief goes in as the new agent's first prompt on its command line,
not through `seshi send` (avoids racing the agent's start screen); four choices made while planning are
recorded in the intent for the user to confirm. Known gaps: the brief uses the pane's name as its task
(the real first prompt is not on the wire); in a main checkout with `worktree.per_agent` the new agent may
land in a fresh worktree; nushell quoting of a prompt with an apostrophe is unchanged from before.

Done outside the phase: saved pane output no longer races at shutdown (saves run one at a time,
newest wins, fsynced) and restored history starts at a clean line.

### 14 — Design v5 cleanup pass · complete 2026-10-22 at `865998b`

The user supplied the v5 cleanup handoff (`seshi-app.js`
is the spec) on 2026-10-19 and approved following it where it differs from older UI rules, including
dropping model, context and cost from Seshi's own chrome. One commit per step, each with the full gate:

1. One background (terminal default or solid) — done.
2. Spacing (1-column margin and gap; sidebar has no footer) — done.
3. Each fact shows once (pane title, status line holds path/branch/PR) — done, except the PR, server and `⇆` tags, which step 6 moves into the status line.
4. Colour jobs (working is never amber; neutral selection tint and `▎` bar) — done (sidebar, sheet and popup lists).
5. Exactly one lit edge — done.
6. Status line in both layouts — done, without the `#PR` and dev-server tags (no data yet).
7. Settings › General sections — done.

Shipped since v0.35.0. The gate passed on `main` at `865998b`.

### 15 — Checks Seshi runs itself · active, chosen 2026-10-22

Today an agent saying "done" turns its session done even when the project's checks fail. This phase
makes "done" mean something Seshi observed. Chosen by the user 2026-10-22 (verify slice only).
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
11. #107 CLI and home screen: show a failed check in `ls`, `seshi_list` and the counters (found by the live check)

Later: stale detection and a command supplied by the repo with its approval, both built in Phase 18.

Status 2026-10-22: released in v0.37.0 (tag `v0.37.0` at `97d6d19`). CI passed on Windows, Ubuntu
and macOS (it caught four check tests with `sh`-only commands, fixed before release), so the Windows
runner code now compiles and its tests pass. #94–#103 and #107 are built, with two fixes the live check found
(the Inbox's first failing line; the Flat sheet title). #107 makes `seshi ls`, `ls --json`, MCP
`seshi_list` and the home counters show a failed or running check instead of "done", and the Inbox
and the home screen share one rule for what counts as finished. The gate passed on the merged
tree. A live run on an isolated daemon (before #107) confirmed: no check reads done as before; a
failing check keeps a session from reading done and `seshi wait` exits 4; pass; could-not-run on a
missing command and on a timeout with no child left; the pane token is not passed on; results
survive a daemon restart; re-run, override with confirm, undo and the evidence sheet on a real
screen in Cards and Flat. Validation on Windows, 2026-10-22 (isolated daemons, PowerShell):
pass, fail with `seshi wait` exiting 4, timeout as could-not-run, #107 in `ls` and `ls --json` on a live
daemon, 80×24 frames in all three layouts, and Enter on a row opening the sheet all checked. The ASCII glyph
fallback was broken on the sheet banner, why popover and override (Unicode marks); fixed with a regression
test. Under PowerShell a missing check command exited 1 and read **failed**; Phase 18 starts each
PowerShell check with a not-found trap that exits 127, so it reads could-not-run (other exit codes unchanged). Still not verified: desktop and phone alerts, and a check run by hand on
macOS. On Linux a check's grandchild process can outlive the daemon
(documented in `docs/SECURITY.md`).

- **Scope of the first slice (verify only):** a project may name one check command. When a
  session's turn ends and the session would become done, Seshi runs that command in the
  session's worktree as a background task, records the exit code, the time and the tail of the
  output, and shows the session as done only if it passed. A failure shows `failed: <check>`, appears in
  the Inbox, and keeps the session visible. The user can override a failure; the override is recorded.
- **Opt-in:** a project with no check command behaves exactly as today.
- **Not a workflow runner:** Seshi runs one user-named command and reports the result. It starts no
  role, assigns no task and does not stop an agent from working, so it stays clear of the runner cut
  on 2026-10-18. The user confirmed this reading on 2026-10-22: checkpoints replace the cut
  workflows.
- **Rules it must keep:** the check runs off the UI thread (`spawn_bg`) and survives the UI closing,
  so the daemon owns it; the result is trusted because Seshi ran the command, not because the agent said so; the command runs
  only with the user's approval, like other project-supplied commands (SECURITY.md); it works on
  Windows (PowerShell), macOS and Linux; the saved session and `PROTOCOL_VERSION` change together.
- **Open decisions (ask before building):** where the command is configured (user config or the
  repo); the timeout; whether a re-run happens on every turn end or only when the files changed;
  what the Inbox says for a check that cannot run.
- **Design:** design brief v6 and its handoff, since replaced by [`design/v7/`](design/v7/)
  (`seshi-checks/checks.js` is the spec; drawn in the Flat style only). Checkpoints must work in
  all three layouts (Cards floating, Cards tiled, Flat) at 80×24, 100×30 and 160×45. Flat is built
  first; floating and tiled follow the designer's versions. Decisions taken from the handoff:
  10-minute timeout, run after every turn end, a separate Inbox group for checks that could not
  run, a re-run requested while the agent is working waits for the turn to end, override covers
  one result only. Reuse the existing theme colours (no new theme roles), keep the sidebar's
  `+ new` / `actions` foot and the Appearance spinner choice, and add re-run and override as
  row-menu items. A command supplied by the repo (`.seshi/checks.toml`) and stale detection are
  separate later tickets.
- Acceptance: tickets, tests, docs and a passing verification gate before it is marked shipped,
  plus a live check where a failing check keeps a session from reading done.

### 16 — Restart the daemon from the update flow · complete 2026-10-22

Found while shipping v0.37.0: the protocol moved from 42 to 43 and the only way forward was
`seshi kill-server`. Ticket #110. After an update that leaves the daemon older, Settings and the update
toast show **Restart now** (capital `R` in Settings), which confirms how many sessions stop and how
many are working; with none open it restarts at once. A new `seshi` that finds an older daemon it
cannot talk to shows a full-screen prompt (**Restart and open** / **Quit**) on a real terminal, and
keeps the old error and exit code everywhere else; a remote daemon is never restarted by it. No
protocol change. Live-checked against the published v0.36.13 daemon on an isolated socket: the prompt
(100×30 and 80×24, by key and by mouse), Quit leaving the old daemon and its session alone, Restart
and open bringing the new daemon up with the session back, and no prompt without a terminal.
Not verified live: **Restart now** against a real older daemon of the same protocol (tests use a
faked snapshot), and Windows beyond CI. Known gaps: the prompt shows protocol numbers, not release
numbers (an older daemon reports only its protocol); it installs no panic hook to switch mouse
capture back off; the offer-then-restart glue in `open_client` has no test of its own.

### 17 — Team roles for the commander · shipped in v0.40.0, live trial open

*Removed by the user in [Phase 19](#19--the-v8-cleanup--shipped-in-v0410-live-checks-open); the record below is history.*

A team list for the commander: a Settings › Team list of jobs
(planner, builder, reviewer, or the user's own), each with a model per agent family (Claude, Codex),
one global family switch and a per-project override. A commander reads the list and starts each helper
on its job's model. Handoffs are gated by Phase 15 checks, which the commander is told to wait on.
Seshi runs no pipeline and normal sessions are unchanged.

The user asked on 2026-10-22 to build it now. Tickets: #113 config, #114 start a helper for a job,
#115 the team in the brief, #119 hand on only after a passing check, #120 Settings › Team with the family
switch, #121 add/edit/remove jobs, #122 docs, #123 live trial on the user's models (manual).

Status 2026-10-22: #113–#115 and #119–#122 built and released in v0.40.0; the gate passed on
the merged tree. Not done, so the phase is not complete: #123, a commander running a small ticket set on the
user's models. Known gaps: no key-driven test that saving a job writes the file (covered below that level);
one click on the Family row while a job is being edited leaves the edit open.

### 18 — The v7 layout and checkpoints · shipped in v0.40.0, live checks open

The user supplied the v7 handoff on 2026-10-22 ([`design/v7/`](design/v7/), `checks7.js` is the spec) and
asked for all of it: a new layout for the app, not a pass over the old one. Covers the frame in all three
layouts (Cards floating, Cards tiled, Flat) at 160×45, 100×30 and 80×24 with the drop order below 140 and
100 columns, every check screen, approving a repo's `.seshi/checks.toml` command (Enter, y and Space never
approve; Always needs pressing twice; a changed command asks again), stale passes, and several named check
commands per project. The working state is built too: the "Bars" 3-bar spinner preset (ASCII under the ASCII glyph set) and the name and word shimmer, both frozen when motion is off. Not built: checkpoint kinds
other than a command (commits, plan approval, review), which the design only shapes. Tickets are labelled
`phase-18`. One protocol bump for the whole phase.

Status 2026-10-22: #124–#131 built and released in v0.40.0 (protocol 43→44). The gate
passed on the merged tree (754 + 1 tests). All 26 presets match the handoff's frames in every layout and
size, plus ASCII at Flat 80×24 (`src/client/v7_frames_tests.rs`); colour roles were compared by a reviewer.
Two blind reviews (code and security, design fidelity) ran with two fix rounds; their blocking findings
(the approval modal clipping a long command, approving while another checkpoint runs, the Flat ground)
are fixed. Live checks: Windows at 120×32 on an isolated daemon (fail, pass, stale, two checkpoints, the
approval modal's keys, a missing command reading could-not-run); Always and Never only on Linux in Docker,
because Windows has no way to point the approvals file away from the user's own. Also in this phase: the
unix pane kill signals only the pane's own process group (it could reach every process the user owns).
Open decisions: whether `a` on a sidebar row opens the row menu (handoff) or global Actions (today, ADR
0009); `seshi wait`
exits 0 for a check waiting for approval or that could not run; ADR 0013 is still Proposed. Not built:
`· 1 of 59 tests failed` in the why popover (needs output parsing) and when you chose Never (needs a
protocol change). Not checked live: 80×24, 100×30 and 160×45, ASCII and Nerd glyphs, macOS.

### 19 — The v8 cleanup · shipped in v0.41.0, live checks open

The user supplied the v8 handoff ([`design/v8/`](design/v8/); `app8.js` is the spec) and asked for it with
the rulings in [ADR-0014](adr/0014-v8-cleanup.md) where the two differ. One protocol bump for the whole of
v8: 44 to 45. This phase covers the cuts the user asked for by name: the fleet commander (`Start a fleet`,
`Fleet commander` in the row menu, `seshi brief`, `[team]` jobs and their validation, the `job` argument of
`seshi_start`), the sidebar's subagent tree and inspector (the daemon still tracks subagents and a working
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
end. Open: the live check of the helper flow with a real agent over `seshi mcp`; whether check re-run and
override want leader keys (they work from the check sheet).

### 20 — Borrowed from TUIOS · queued 2026-10-23

The user asked on 2026-10-23 for the parts of TUIOS (tuios.dev) that Seshi lacks, after a
check showed ranked status sources, evidence times, box-only screen rules, saved scrollback,
per-turn checkpoints and hook setup already exist. [Intent](features/borrowed-from-tuios/intent.md).
One protocol bump for the phase: 45 to 46. Tickets are labelled `phase-20`.

1. #142 daemon: mark a hooked agent quiet after 10 minutes with no output
2. #143 UI: a quiet agent on its row and in the why popover (after #142)
3. #144 UI: quiet agents in the Inbox's NEEDS YOU, no sound (after #142)
4. #145 CLI and MCP: quiet in `ls`, `ls --json`, `seshi_list` and `seshi wait` (after #142)
5. #146 MCP: `seshi_answer` answers only the prompt the caller saw
6. #147 refuse text with hidden characters sent to an agent
7. #148 daemon: a setting to stop saving pane output
8. #149 daemon: saved output files readable only by you
9. #150 UI: refuse to restore a checkpoint while its agent works

Declined while planning: a fifth "unknown" status (a quiet agent stays working).

## In scope

Everything below is shipped on `main` unless marked.

**Core**
- Daemon owns sessions; detach / reattach; many clients — shipped — `src/daemon/mod.rs`, ADR-0001
- Sessions survive a daemon restart (agents resumed, commands re-run) — shipped — `src/daemon/persist.rs`, `restore` in `src/daemon/mod.rs`
- Windows (ConPTY, named pipes, PowerShell folder tracking), macOS, Linux — shipped — `src/daemon/term.rs`, `src/ipc.rs`
- SSH remotes (`--remote host`) — shipped — `src/ipc.rs` `connect_remote`

**Agents**
- Personal and project agent profiles / saved workflows — removed from Seshi in
  the v0.34.0 release candidate at the user's direction; legacy config tables are
  ignored, and old saved-run data is tolerated during session restore.
- Status detection: working / needs you / done (until seen) / idle, from screen patterns, OSC progress and agent hooks — shipped — `src/daemon/scan.rs`, `seshi hook` in `src/cli.rs`
- Trusted status reports (process tree or `SESHI_PANE_TOKEN`) — shipped — `src/daemon/term.rs`, `src/cli.rs`
- Agent kinds: Claude, Codex, Gemini, OpenCode, Cursor, Copilot, Amp, Qwen, Aider, Goose, Crush, Droid, Pi, Kiro, Grok, custom `[[agents]]` — shipped — `src/config.rs`
- New agent task box (project, worktree, agent, model, effort), presets and recipes as its choices, prewarm — shipped — `src/client/screen/dialogs.rs`, `src/client/recipes.rs`
- Agents talk to agents (MCP server `seshi mcp`) — shipped — `src/mcp.rs`
- `seshi send --wait`, `seshi wait`, `seshi read` — shipped — `src/cli.rs`
- At most a set number of helpers working at once (`[mcp] helpers_at_once`) — shipped 0.25.0 — `Daemon::starting` in `src/daemon/mod.rs`
- A commander runs a fleet: one agent starts, briefs and checks helpers, which sit under it in the sidebar — shipped in v0.20.0 — #32–#38

**The TUI (one shared session layout; ADR-0007, superseded by ADR-0012)**
- Flat rendering style: square, full-bleed sidebar, tabs, panes and sheets; sidebar/pane focus zones, no global footer; shipped in v0.36.0; `src/client/screen/draw.rs`, ADR-0012
- Floating look: rounded cards with gaps, title and state in the border, unfocused cards faded, pill tabs, no app footer; floating or tiled, corners, gap, dim, focus border, pill ends in Settings — shipped by v0.30.1 — `src/client/screen/card.rs`, `screen.rs`, ADR-0007
- Sidebar card: projects → sessions, state glyph + name, branch · age, question under it, attention sort, helpers indented under the session that started them, keyboard and mouse, row letter keys, `a actions` / `, settings` foot — shipped — `src/client/screen/draw.rs` `draw_side`
- Panes: tabs as pills (named in their pill), any number of splits (grid for 3+), drag the gap between them, ✕ in the border — shipped — `src/client/screen/draw.rs` `draw_session`
- Inbox (what needs you first, type to go anywhere), command palette (plain-word commands), key map (searchable, second steps), actions list — shipped — `draw_goto`, `draw_palette`, `src/client/screen/leader.rs`
- Right-click menus, confirm before closing, toasts — shipped — `src/client/menu.rs`
- Splash: Resume / New shell here — shipped — `draw_splash`
- Settings popup grouped by section, key rebinding, themes with contrast audit — shipped — `draw_settings`, `src/theme.rs`
- Leader keys (a shell is `n`, only kept features have keys), Alt+arrows without the leader, armed pill in sky — shipped by v0.30.1 — `src/keys.rs` `DEFAULT_PREFIX_KEYS`, ADR-0009
- Copy on select with toast, copy mode, Ctrl+click paths, paste images — shipped — `src/client/copy.rs`, `src/client/pick.rs`

**Code and git**
- Worktrees per agent, create / move / remove with hooks — shipped — `src/daemon/git.rs`, `src/project.rs`
- Worktree clean-up: ones left behind are swept when nothing lives only there, and a list per repo says why each is still around and removes the ones you tick — shipped — `src/tidy.rs`, `src/daemon/worktrees.rs`, `src/client/screen/tidy.rs`
- Files (tree, preview, in-place edit, external editor), find file, search code — shipped — `src/client/files.rs`, `src/client/find.rs`, `src/client/views.rs`
- Changes (diff, review marks, commit, open a PR, git init offer), branch switcher — shipped — `src/client/branch.rs`, `src/client/views.rs`
- Worktree hooks (`.seshi.toml` `on_create` / `on_remove`, run once allowed; `on_create` finishes before anything starts in the worktree) — shipped — `src/project.rs`, `Ev::WorktreeCreated` in `src/daemon/mod.rs`
- Tasks (a worktree's review: commit, merge, PR, throw away) — shipped — `src/client/tasks.rs`
- Own worktree for `claude`/`codex` typed into a shell in a repo's main folder — shipped 0.9.12; a launcher per agent first on a shell pane's PATH (any shell) built on the working tree — `seshi agent-dir`, `src/shim.rs`, `daemon/term.rs` `agent_functions`
- Usage and limits meter, context per agent, continue after a limit — shipped 0.10.0 — `seshi statusline`, `src/daemon/usage.rs`
- Automatic idle sleeping and its sleep/wake screen and animation: removed; legacy recovery remains only for sessions saved by older versions.
- A session's share of the machine (CPU and memory) in its pane's title when it's large — shipped 0.25.0, moved off the sidebar row 0.26.2 — `src/daemon/scan.rs`, `load_note` in `src/client/screen/draw.rs`
- Merge that cleans up (the worktree and its branch go) — shipped 0.11.0 — `src/client/view_keys.rs`

**Identity**
- Renamed hydra → Seshi: command, folders, `SESHI_*`, `.seshi.toml`, MCP, releases; one-time move of an old install — shipped by v0.30.1 — `config::migrate_old_names`, ADR-0008
- Seshi Walnut (default), Seshi Night and Seshi Day themes; block SESHI wordmark on the splash — shipped by v0.30.1 — `src/theme.rs`, `src/client/screen/splash.rs`

**Extras**
- Alert when an agent needs you: Seshi's two rising notes and an in-app notice; a desktop notification only with no window open. Completions stay quiet — `src/alert.rs`
- Click a notification to jump to its session — shipped — `src/reveal.rs`, `src/alert.rs` (#19)
- Sidebar groups sessions by where they work now (repo, or folder outside git); no projects to open; splits stay with their session — shipped — `src/client/screen/mod.rs`
- Releases and installers (Windows, macOS, Linux; checksums), `seshi update` and a daily update check — shipped — `.github/workflows/release.yml`, `install.sh`, `install.ps1`, `src/update.rs`
- CI: test and clippy on Linux and Windows, a macOS check — shipped — `.github/workflows/ci.yml`
- Config sync across machines (local file never synced) — shipped — `src/sync.rs`
- SSH remotes (`--remote host`) — shipped — `src/ipc.rs` `connect_remote`
- `seshi doctor` — shipped — `src/cli.rs`

## Out

- Saved role profiles and the planner/approval/worker/reviewer workflow runner were removed by user decision (2026-10-18). Use the agent's own configuration and external workflow tools; Seshi remains the session manager. Narrowed 2026-10-22 by the user: a team list for the commander (jobs with a model per agent family, [Phase 17](#17--team-roles-for-the-commander--queued-2026-10-22)) is allowed. A Seshi-run pipeline, where Seshi starts or orders stages itself, stays cut.

- Features cut 2026-10-09 remain out unless re-approved: open project, go to, next agent that needs you, arrange panes, undo automatic workspace, the map, ideas, races, run a preset as a list (presets stay in the new-agent dialog), past chats, agent tools, extensions, and the tmux stand-in for Claude agent teams. Memory per session later returned as the load line under a busy session. Items explicitly restored since the cut are listed in In scope; races remain cut.
- Desktop notifications while a seshi window is open, the terminal bell on an alert, and choosing among system sounds (or your own sound file) — cut 2026-10-11 (the user's call: three overlapping alerts, two of them the Windows error ding). Bringing one back needs a new decision.
- Separate desktop GUI client — declined 2026-10-17: keep Seshi terminal-first and use T3 Code when a chat-first GUI is wanted. Revisit only through a new proposal (ADR-0011).
- Other structural terminal layouts (workspaces, tree, dock, alternate sidebar) — declined 2026-10-03. Flat is an accepted rendering style of the shared session layout, per ADR-0012; it does not introduce another session model.
- Phase 7 proposals canceled 2026-10-17: named hosts and remote panes/agents (#23); terminal graphics and Kitty keyboard protocol (#27); launcher, templates, recording/replay, screenshots and key display (#29); web terminal and SSH server mode (#30); package-manager distribution (#31). Revisit only through a new proposal. This does not remove shipped SSH remote-client support.

## Later

- Non-blocking validation follow-ups: run the Phase 9 live fleet check with two helpers visible under the commander; use the four Phase 10 features in real work; address the squash-merged-branch detection gap.

- Personas are dropped, not deferred: saved role profiles were removed 2026-10-18 (see [Out](#out)). Phase 15 replaces the idea with checkpoints that Seshi checks itself.
- Checkpoint kinds beyond the first (`commits`, `approval`, `review`) and a project-defined checkpoint list — deferred 2026-10-22 until Phase 15's verify check has been used in real work. `approval` and `review` sit close to the workflow runner cut on 2026-10-18; each needs the user's explicit re-approval before any ticket. Phase 17's handoffs use the verify check only.
- The designed team view, threads between agents, take over and hand back, pause the fleet, stop all, cost per fleet — deferred 2026-10-11: waiting on a design for it. Comes back when the prototype is in.

## Decisions

- [ADR-0001](adr/0001-daemon-owns-sessions.md) — A daemon owns every session; clients only draw. Rules out: sessions inside the UI process.
- [ADR-0002](adr/0002-portable-pty-and-vt100.md) — portable-pty (ConPTY) and vt100 for terminals. Rules out: a home-grown PTY layer.
- [ADR-0003](adr/0003-msgpack-over-local-sockets.md) — msgpack frames over local sockets, versioned. Rules out: unversioned message changes; network listeners.
- [ADR-0009](adr/0009-leader-keys-after-the-cut.md) — Leader keys: a shell is one key, and only kept features have keys (supersedes ADR-0006). Rules out: keys for features that don't exist; defaults that differ from the key map shown in the app.
- [ADR-0008](adr/0008-rename-to-seshi.md) — The app is called Seshi; an old hydra install is brought over once. Rules out: new code or paths under the old name.
- [ADR-0012](adr/0012-flat-rendering-style.md) — Add a flat rendering style over the shared session layout; Cards stays default.
- [ADR-0014](adr/0014-v8-cleanup.md) — Bring over the v8 cleanup, keeping what the user already decided. Rules out: bare letters acting in the sidebar; per-state glyphs on session rows; the fleet, the Team tab, the subagent tree and a helper mode.
- [ADR-0011](adr/0011-keep-seshi-terminal-first.md) — Keep Seshi terminal-first. Rules out: adding a separate desktop GUI client to this project.

## Rules

- The daemon's socket only accepts the user who started it — `src/ipc.rs` — SECURITY.md
- Status reports need the pane's token or its process chain — `src/daemon/mod.rs` hook handling — AGENTS.md non-negotiable 5
- Nothing in a cloned repo runs without the user's approval — `src/project.rs` — SECURITY.md
- Client and daemon refuse to talk across protocol versions — `src/protocol.rs` `PROTOCOL_VERSION`, checked on `Hello` — ADR-0003
- Config selects the Cards or Flat rendering style while sharing the session layout — `src/config.rs` `Ui.layout`, `src/client/screen/card.rs` `Look::of` — ADR-0012
- Every built-in theme passes the contrast audit — `cargo test` (theme audit test in `src/theme.rs`) — readability
- Both platform sides compile — `cargo check --target x86_64-unknown-linux-gnu` / `aarch64-apple-darwin` — AGENTS.md non-negotiable 3
- clippy is clean — `cargo clippy --all-targets -- -D warnings` — CODE-STANDARDS
- A session's "started by" is only set by the daemon, from a pane whose secret checked out — `src/daemon/mod.rs` `starter`, tests in `src/daemon/logic_tests.rs` — AGENTS.md non-negotiable 5
- `seshi_answer` sends a key only for the prompt id the caller passed — `src/mcp.rs` `answer` / `prompt_seen`, test `an_answer_goes_only_to_the_prompt_it_saw`
- Text with invisible characters never reaches an agent through `seshi send`, `seshi_send` or `seshi_start` — `src/hidden.rs`, tests there — not yet: #147

## Records

Docs: `AGENTS.md`, `docs/` (architecture, standards, testing, security, UI, API patterns), `docs/stack/rust.md`.
Design references: the v8 handoff [`design/v8/`](design/v8/README.md) (current) and the v7 handoff [`design/v7/`](design/v7/README.md) (layout and checkpoints; its frames back the tests). Older briefs and handoffs (v1–v6, Flat, wake-up) are in git history.
Glossary: none yet.
Feature docs: `docs/features/`. Each live intent is linked from its phase; shipped or removed intents are in git history.

## Rework

- none recorded yet.
