# Features: shipped, declined, later

What Maitty has, what was cut or declined on purpose, and what is deferred. A cut item stays
out until the user re-approves it. Planned work lives in Plane, not here. Moved from the old
roadmap; the phase history stays in `docs/ROADMAP.md` until that file is retired.

## In scope

Existing features below are shipped unless marked. Optional chat GUI and native
conversation continuity are planned in phases 34–37:
[intent](features/chat-gui/intent.md) · [spec](features/chat-gui/spec.md).

**Core**
- Daemon owns sessions; detach / reattach; many clients — shipped — `crates/maitty-core/src/daemon/mod.rs`, ADR 0001
- Sessions survive a daemon restart (agents resumed, commands re-run) — shipped — `crates/maitty-core/src/daemon/persist.rs`, `restore` in `crates/maitty-core/src/daemon/mod.rs`
- Windows (ConPTY, named pipes, PowerShell folder tracking), macOS, Linux — shipped — `crates/maitty-core/src/daemon/term.rs`, `crates/maitty-core/src/ipc.rs`
- SSH remotes (`--remote host`) — shipped — `crates/maitty-core/src/ipc.rs` `connect_remote`

**Agents**
- Personal and project agent profiles / saved workflows — removed from Maitty in
  the v0.34.0 release candidate at the user's direction; legacy config tables are
  ignored, and old saved-run data is tolerated during session restore.
- Status detection: working / needs you / done (until seen) / idle, from screen patterns, OSC progress and agent hooks — shipped — `crates/maitty-core/src/daemon/scan.rs`, `maitty hook` in `crates/maitty-terminal-client/src/cli.rs`
- Trusted status reports (process tree or `MAITTY_PANE_TOKEN`) — shipped — `crates/maitty-core/src/daemon/term.rs`, `crates/maitty-terminal-client/src/cli.rs`
- Agent kinds: Claude, Codex, Gemini, OpenCode, Cursor, Copilot, Amp, Qwen, Aider, Goose, Crush, Droid, Pi, Kiro, Grok, custom `[[agents]]` — shipped — `crates/maitty-core/src/config.rs`
- New agent task box (project, worktree, agent, model, effort), presets and recipes as its choices, prewarm — shipped — `crates/maitty-terminal-client/src/client/screen/dialogs.rs`, `crates/maitty-terminal-client/src/client/recipes.rs`
- Agents talk to agents (MCP server `maitty mcp`) — shipped — `crates/maitty-terminal-client/src/cli/mcp.rs`
- `maitty send --wait`, `maitty wait`, `maitty read` — shipped — `crates/maitty-terminal-client/src/cli.rs`
- At most a set number of helpers working at once (`[sessions] helpers_at_once`; an old `[mcp]` value is read as a fallback) — shipped 0.25.0 — `Daemon::starting` in `crates/maitty-core/src/daemon/mod.rs`
- A commander running a fleet (shipped in v0.20.0, #32–#38): removed in phase 19; old `[fleet]` and `[team]` tables are read and ignored

**The TUI (one shared session layout)**
- The redesign: a 34-column sidebar of machines and their sessions (colour-only status marks, `■ project branch`, the current turn's subagents), boxed panes with title chips and `files` / `changes` / `×`, Files and Changes sliding in inside the pane, a `PANE` / `KEYS` status bar, `Ctrl+Space` key mode, a three-column keys help, Usage and cost, Settings with sections down the left, menus with their keys, "No pane open"; it replaced the fleets and ships sidebar, the tab row and the collapsed column — built on `tui-redesign` — `crates/maitty-terminal-client/src/client/screen/side.rs`, `workspace.rs`, `mode_bar.rs`, `usage.rs`; see `FLAT-SESSIONS-PLAN.md` section 9
- Flat rendering style: square, full-bleed sidebar, tabs, panes and sheets; sidebar/pane focus zones, no global footer; shipped in v0.36.0; `crates/maitty-terminal-client/src/client/screen/draw.rs`
- Floating look: rounded cards with gaps, title and state in the border, unfocused cards faded, pill tabs, no app footer; floating or tiled, corners, gap, dim, focus border, pill ends in Settings — shipped by v0.30.1 — `crates/maitty-terminal-client/src/client/screen/card.rs`, `screen.rs`
- Sidebar card: projects → sessions, one-line rows (mark, name in its state's colour, agent, uncommitted lines, age), a second line only when it says more, attention sort, keyboard and mouse, `+ New` / `Menu` foot — shipped — `crates/maitty-terminal-client/src/client/screen/side.rs` `draw_side`
- Panes: tabs as pills (named in their pill), any number of splits (grid for 3+), drag the gap between them, ✕ in the border — shipped — `crates/maitty-terminal-client/src/client/screen/draw.rs` `draw_session`
- Inbox (what needs you first, type to go anywhere), command palette (plain-word commands), key map (searchable, second steps), actions list — shipped — `crates/maitty-terminal-client/src/client/screen/inbox.rs` `draw_inbox`, `crates/maitty-terminal-client/src/client/screen/keyoverlay.rs` (key map and search commands), `crates/maitty-terminal-client/src/client/screen/leader.rs`
- Right-click menus, confirm before closing, toasts — shipped — `crates/maitty-terminal-client/src/client/menu.rs`
- Splash: Resume / New shell here — shipped — `draw_splash`
- Settings popup grouped by section, key rebinding, themes with contrast audit — shipped — `crates/maitty-terminal-client/src/client/screen/dialogs.rs` `draw_settings`, `crates/maitty-terminal-client/src/theme.rs`
- Leader keys (a shell is `n`, only kept features have keys), Alt+arrows without the leader, armed pill in sky — shipped by v0.30.1 — `crates/maitty-terminal-client/src/keys.rs` `DEFAULT_PREFIX_KEYS`
- Copy on select with toast, copy mode, Ctrl+click paths, paste images — shipped — `crates/maitty-terminal-client/src/client/copy.rs`, `crates/maitty-terminal-client/src/client/pick.rs`

**The desktop app**
- Redesign ("Maitty Desktop" handoff): an agents-only sidebar grouped by machine, most urgent first, with project and branch on each row and an in-place rename; subagents under their session, each with its own chat; one pane header for agents and shells that gives up controls as the pane narrows; mixed right and down splits with draggable dividers; an agent opens as its chat; a three-layer message box (question tray, glass box, settings tray); glass menus, dialogs and drawer — shipped — `apps/gui/src/features/workspace/components/Sidebar.tsx`, `PaneCard.tsx`, `apps/gui/src/features/chat/components/Composer.tsx`, `QuestionTray.tsx`
- Layout 2e, sidebar (`docs/features/chat-gui/design-handoff/v19`): New chat and Search rows, an All projects filter, and sessions grouped by status (Needs you, Working, Finished, Idle; the default), project or machine; a row glides to its new section when its status changes and says how long it has been in its state; Mark as done on a finished agent; no Inbox, since the status sections list what it listed, and jump-to-next has a key on every platform. Kept from before: every theme, the four window styles, the chat and message box, up to four panes with right and down splits, and plain shells as rows — shipped — `apps/gui/src/features/workspace/components/Sidebar.tsx`, `apps/gui/src/features/workspace/model/sidebar.ts`
- Layout 2e, panes and side panel: an icons-only pane header (view switch, split right, side panel with a dot for changes; split down and maximize in the menu), a side panel of Changes, Files and Terminal at three-quarters of the window, a click on a diff line to send the agent a comment about it, Commit N files and Create PR (through `gh`) for a checkout on this computer, and an agent's own terminals as tabs there or pinned in a drawer under its chat. Branches, History, Timeline and References are hidden, undecided — built 2026-11-10, not yet tried in the packaged app — `apps/gui/src/features/inspect/InspectPanel.tsx`, `apps/gui/src/features/workspace/components/AgentTerminals.tsx`, `PaneCard.tsx`
- Fleets, ships, the tab bar and the Crewmates switch are gone from the desktop app (the redesign supersedes them); `features/crew/` stays in the tree, unmounted, until Crewmates is designed

**Code and git**
- Worktrees per agent, create / move / remove with hooks — shipped — `crates/maitty-core/src/daemon/git.rs`, `crates/maitty-core/src/project.rs`
- Worktree clean-up: closing the last pane of a worktree Maitty made removes it only when it is clean and everything in it is merged (clean with commits found nowhere else: it asks, and Enter keeps the folder; uncommitted changes: it stays and says why; every removal or keep is in `daemon.log`), and a list per repo says why each is still around and removes the ones you tick — shipped — `crates/maitty-core/src/tidy.rs`, `crates/maitty-core/src/daemon/worktrees.rs`, `crates/maitty-terminal-client/src/client/screen/tidy.rs`
- Files (tree, preview, in-place edit, external editor), find file, search code — shipped — `crates/maitty-terminal-client/src/client/files.rs`, `crates/maitty-terminal-client/src/client/find.rs`, `crates/maitty-terminal-client/src/client/views.rs`
- Changes (diff, review marks, commit, open a PR, git init offer), branch switcher — shipped — `crates/maitty-terminal-client/src/client/branch.rs`, `crates/maitty-terminal-client/src/client/views.rs`
- Worktree hooks (`.maitty.toml` `on_create` / `on_remove`, run once allowed; `on_create` finishes before anything starts in the worktree) — shipped — `crates/maitty-core/src/project.rs`, `Ev::WorktreeCreated` in `crates/maitty-core/src/daemon/mod.rs`
- Tasks (a worktree's review: commit, merge, PR, throw away) — shipped — `crates/maitty-terminal-client/src/client/tasks.rs`
- Own worktree for `claude`/`codex` typed into a shell in a repo's main folder — shipped 0.9.12; a launcher per agent first on a shell pane's PATH (any shell) built on the working tree — `maitty agent-dir`, `crates/maitty-core/src/shim.rs`, `daemon/term.rs` `agent_functions`
- Usage and limits meter, context per agent, continue after a limit — shipped 0.10.0 — `maitty statusline`, `crates/maitty-core/src/daemon/usage.rs`
- Automatic idle sleeping and its sleep/wake screen and animation: removed; legacy recovery remains only for sessions saved by older versions.
- A session's share of the machine (CPU and memory) in its pane's title when it's large — shipped 0.25.0, moved off the sidebar row 0.26.2 — `crates/maitty-core/src/daemon/scan.rs`, `load_note` in `crates/maitty-terminal-client/src/client/screen/side.rs`
- Merge that cleans up (the worktree and its branch go) — shipped 0.11.0 — `crates/maitty-terminal-client/src/client/view_keys.rs`

**Identity**
- Renamed hydra → Seshi (v0.30.1), Seshi → Vibecasso, then Vibecasso → Maitty: the name lives in `Cargo.toml`; one-time move of an old install — `config::migrate_old_names`
- Night (default) and Day themes among the built-in themes (62 by v0.42.0, later 170, later cut to 16, now 50: 25 dark and 25 light); block MAITTY wordmark on the splash — shipped in v0.42.0 — `crates/maitty-terminal-client/src/theme.rs`, `crates/maitty-terminal-client/src/client/screen/splash.rs`

**Extras**
- A tray (Windows) or menu-bar (macOS) item in the desktop app listing the sessions that need you and the ones that finished; choosing one opens it — built 2026-11-09, tried on Windows only — `apps/gui/src-tauri/src/tray.rs`, `apps/gui/src/features/workspace/useTrayItem.ts`
- Alert when an agent needs you: Maitty's two rising notes and an in-app notice; a desktop notification only with no window open. Completions stay quiet — `crates/maitty-core/src/alert.rs`
- Click a notification to jump to its session — shipped — `crates/maitty-core/src/reveal.rs`, `crates/maitty-core/src/alert.rs` (#19)
- Sidebar groups sessions by where they work now (repo, or folder outside git); no projects to open; splits stay with their session — shipped — `crates/maitty-terminal-client/src/client/screen/mod.rs`
- Releases and installers (Windows, macOS, Linux; checksums), `maitty update` and a daily update check — shipped — `.github/workflows/release.yml`, `install.sh`, `install.ps1`, `crates/maitty-terminal-client/src/cli/update.rs`
- `maitty uninstall`: stops the server, disconnects the agents and removes the program; `--purge` deletes settings and saved sessions too — shipped — `crates/maitty-terminal-client/src/cli/uninstall.rs`
- CI: test and clippy on Linux and Windows, a macOS check — shipped — `.github/workflows/ci.yml`
- Config sync across machines (local file never synced) — shipped — `crates/maitty-core/src/sync.rs`
- SSH remotes (`--remote host`) — shipped — `crates/maitty-core/src/ipc.rs` `connect_remote`
- `maitty doctor` — shipped — `crates/maitty-terminal-client/src/cli.rs`
- Dark and light themes are two remembered slots and an appearance switch; the browser has a Dark and a Light tab; Vim, PaperColor Light, Flexoki Light, Selenized Light and Modus Operandi are added — phase 40 — `crates/maitty-settings/src/theme.rs`, `crates/maitty-terminal-client/src/client/themes.rs`
- A Codex pane's working, needs-you and finished come from Codex's own hooks, with the screen guess only as a per-pane fallback; needs-you and finished never ring for the pane you are looking at, and needs-you stays quiet for 2 minutes after it rings — phase 39 — `crates/adapters/maitty-provider-codex/src/status_hooks.rs`, `crates/maitty-core/src/daemon/status.rs`
- GitHub repositories in the desktop app: clone from an address or from a list of your repositories, on this computer or another machine; who is signed in to GitHub on each machine, and signing in, in Settings; changes, pull, push, commit and Create PR for a session on another machine; a GitHub account and commit identity per folder, added, changed and removed in Settings › GitHub with the lines shown before they are written — Plane module "GitHub repos" — [intent](features/github-repos/intent.md)

## Out

- Saved role profiles and the planner/approval/worker/reviewer workflow runner were removed by user decision (2026-10-18). Use the agent's own configuration and external workflow tools; Maitty remains the session manager. The team list for the commander that phase 17 allowed (2026-10-22) was removed with the commander in phase 19. A Maitty-run pipeline, where Maitty starts or orders stages itself, stays cut.

- Features cut 2026-10-09 remain out unless re-approved: open project, go to, next agent that needs you, arrange panes, undo automatic workspace, the map, ideas, races, run a preset as a list (presets stay in the new-agent dialog), past chats, agent tools, extensions, and the tmux stand-in for Claude agent teams. Memory per session later returned as the load line under a busy session. Items explicitly restored since the cut are listed in In scope; races remain cut.
- Desktop notifications while a maitty window is open, the terminal bell on an alert, and choosing among system sounds (or your own sound file) — cut 2026-10-11 (the user's call: three overlapping alerts, two of them the Windows error ding). Bringing one back needs a new decision.
- Other structural terminal layouts (workspaces, tree, dock, alternate sidebar) — declined 2026-10-03. Superseded 2026-11-06: the sidebar becomes one flat list of sessions, with folder and machine as badges on each row, and a window layout is app-local state (see `FLAT-SESSIONS-PLAN.md`).
- Phase 7 proposals canceled 2026-10-17: named hosts and remote panes/agents (#23, reopened 2026-11-03 by ADR 0007); terminal graphics and Kitty keyboard protocol (#27); launcher, templates, recording/replay, screenshots and key display (#29); web terminal and SSH server mode (#30); package-manager distribution (#31). Revisit only through a new proposal. This does not remove shipped SSH remote-client support.
- Claude Code's own fork list showing forks that are not running (seen 2026-10-28): the `main / fork …` block with token counts is drawn by Claude inside the pane; Maitty only shows it. Not fixed, the user's call. A subagent whose stop was missed clears at the agent's next turn end (`crates/maitty-core/src/daemon/status.rs`); one missed while the agent's turn carries on stays listed until then.
- Fleet work declined 2026-10-24 (the fleet was removed in phase 19): the phase 9 live fleet check, the designed team view, pause the fleet, stop all, cost per fleet.
- Maitty writing Codex hook trust hashes itself — declined 2026-11-01: it would bypass Codex's own review of hook commands. You trust Maitty's hooks once in Codex's `/hooks`; a pane without trusted hooks keeps the screen guess.

- Brain dump (split a pasted plan into sessions, v19 section 12) — declined 2026-11-06, the user's call.
- Community mods (a gallery of scripts that change the app, as other multiplexers have) — declined 2026-11-09, the user's call.

## Later

- Non-blocking validation follow-ups: use the four Phase 10 features in real work; address the squash-merged-branch detection gap.
- Phase 18 follow-ups, kept 2026-10-24: `maitty wait` should exit non-zero when a check needs approval or could not run, and approving a check has no default key. Comes back after phase 28, or sooner if a script relies on `wait`.

- Personas are dropped, not deferred: saved role profiles were removed 2026-10-18 (see [Out](#out)). Phase 15 replaces the idea with checkpoints that Maitty checks itself.
- Checkpoint kinds beyond the first (`commits`, `approval`, `review`) and a project-defined checkpoint list — deferred 2026-10-22 until Phase 15's verify check has been used in real work. `approval` and `review` sit close to the workflow runner cut on 2026-10-18; each needs the user's explicit re-approval before any ticket.
- Threads between agents, take over and hand back — deferred 2026-10-11: waiting on a design for it. Comes back when the prototype is in.
- Live GUI-to-native-TUI Codex transfer and platform acceptance — deferred until the standalone Codex/Claude GUI is usable and packaged. Existing TUI support remains required.
- Extensions store and extension panels (v19 section 9) — deferred 2026-11-06: not built yet, on the list to add.
- Project defaults in the project menu (Start in, Run when it opens; v19 section 3) — on hold 2026-11-06: it needs a shared-config key the terminal app also reads, and the user is not sure they want it.
- Using Maitty from a phone (a companion app or web view of your sessions) — deferred 2026-11-09: not before the terminal app and the desktop app are solid in daily use.
- A pop-up launcher from the tray item — deferred 2026-11-09: it needs a design first.
- Per-turn diff and rewind in chat, on top of the existing snapshots — deferred 2026-11-09.
- Close with a six-second Undo (v19 section 5) — open 2026-11-06: it needs the daemon to delay the kill, so it is a daemon change.
