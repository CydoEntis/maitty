# Flat sessions: audit and plan (draft, 2026-11-06)

Status: **decided in principle, nothing built.** The decisions in section 3 were taken on the owner's say-so; ADR 0004, ADR 0007 and `FEATURES.md` are edited to match (uncommitted). It compares the v26 spec
(`Maitty Desktop specification (3).zip`: `README.md` for the desktop app, `TUI_SPEC.md` for the
terminal app) with the repo at v0.66.0 (`dev` 5687632f). Facts come from three read-only audits
of the daemon, the desktop app and the terminal app; guesses are marked.

## 1. What the spec changes

- **One flat list.** The sidebar lists sessions in creation order. Folder and machine are badges on
  a row. No projects, folders, spaces, machine headers, tab bar, activity bar or Inspector.
- **A session is one shell PTY.** A window layout (up to 4 panes) is local client state; the spec
  says "keep this as local layout state; sessions come from maittyd".
- **Close means end, with a 6-second Undo.** Only closing the window leaves sessions running.
- **One New session modal** (sentence header: project, machine, agent, view) replaces every other
  way to start a session.
- **Both apps show the same sessions** (TUI_SPEC §1), with the same glyphs, words and keys.

## 2. Where the repo stands

| Area | Today | Gap |
| --- | --- | --- |
| Daemon model | Workspace > Tab (layout tree) > Pane (`TermInfo`), groups, saved to `session-<socket>.json` | Containers and the layout tree live in the daemon and the protocol |
| Close | `ClosePane` kills the process tree at once | Undo needs a delayed kill or a parked PTY (Windows: ConPTY) |
| Status | None, Idle, Working, Blocked, Done | No `Exited` state; needs a daemon writer (ADR 0003) |
| Row data | cwd, root, branch, process, label, name, pinned exist | Per-pane +N/-N is missing (only on request), no creation-order index, `machine` is only an ssh hint |
| Shared config | `config.toml` read by core; GUI may not link settings (ADR 0004) | No protocol way to read or write config; key names differ (`ui.sidebar_position`, `ui.motion`) |
| Service states | Client-side only; remote reconnect tries 3, spec says 5 | No "try N of 5" in the local path, no service-down card, no explicit start-service command |
| Desktop app | Tab bar, activity bar, Inspector (about 929 lines), grouped sidebar, no filter, no New session modal, no model picker, no Undo | Most of sections 2 to 5 of the spec |
| Terminal app | Project > Worktree > Session tree, floating cards, mode bar, leader `ctrl+space`, about 100k lines with heavy frame snapshots | Almost all of TUI_SPEC; it has no chat view at all |
| Already done | Settings pages, updates card, Find, palette, notifications, spinners, themes, brain dump/presets/Overview/side panel already gone | Small fixes only |

## 3. Decisions (taken 2026-11-06; the owner delegated them: "do whatever you think is best")

Revisit any of these before its phase starts; reversing one later is expensive.

1. **Layout belongs to each app.** The daemon keeps a flat list of sessions in creation order; the
   workspace/tab/group tree leaves the daemon. The TUI and the desktop app stop sharing a window layout.
2. **Both apps see every session.** ADR 0004 now says any app lists, attaches and types; the creating
   app stays the size owner so one app never reflows a pane the other draws.
3. **`FEATURES.md` "Out" no longer says flat adds no session model;** it points here. ADR 0007's
   "new session goes to the focused machine" became the dialog's default.
4. **No chat view in the TUI yet.** It stays desktop-only (AGENTS non-negotiable 4 is unchanged);
   the TUI opens a chat session as a terminal, as today.
5. **Saved layouts are reset** with a one-time notice: the desktop's `maitty.gui.layout.v2` and the daemon's saved workspaces and tabs.
6. **The key changes are accepted** (TUI prefix `ctrl+a`; `ctrl+t`, `ctrl+k`, `ctrl+j`). Old bindings keep
   working for one release with a note, where they do not collide.
7. **Smaller calls:** the default window style is Flat and Tiled goes; the pane cap is 4; checks and
   worktree creation keep running but show their command in a visible pane (spec "nothing runs
   silently"); the cost figure stays labelled as the provider's estimate until ADR 0009 is revisited.

Still the owner's: any change to the non-negotiables in `AGENTS.md`. Nothing above needs one.

## 4. Proposed order

**Phase 0, decisions and docs.** Resolve section 3. Edit ADR 0004 (and 0007 for the machine
step), `FEATURES.md` and `AGENTS.md` first, in their own change.

**Phase 1, daemon and protocol.** Additive first (no version bump; each needs a test that decodes the
old shape): diff counts, folder, machine, creation order on `TermInfo`. Then the bump: `Status::Exited`,
`UndoClose` with a delayed kill, a config read/write route, a service-status message, replay
sequence numbers if wanted. Then drop the workspace/tab/group commands once both apps no longer use
them. Rename config keys with a read-old-write-new migration. Every bump restarts hooked panes'
reporting, so batch the bumps.

**Phase 2, desktop app.** Order: a flat session model and local layout store (with the reset) ->
flat sidebar with rows, badges, filter, footer -> New session modal -> pane header, 4-pane cap, focus
rule -> status marks (add `zz`, drop waves and sweep) -> Inbox in the header, key changes -> chat
agent/model/reasoning picker -> states (try N of 5, service down) -> then delete the tab bar,
activity bar, Inspector, worktree dialog and the dead settings. Delete last so `dev` stays usable.

**Phase 3, terminal app.** The largest piece. Order follows TUI_SPEC section 15: flat sidebar ->
glyphs and spinners -> New session dialog -> pane header and splits -> inbox and palette -> footer
updates and states -> settings -> narrow rail. Expect to rewrite or delete most frame snapshots
(`tests.rs` alone is 18.8k lines).

**Phase 4, docs.** Rewrite `UI.md`, `DESIGN.md`, `shortcuts.md`, `ARCHITECTURE.md`, `API_PATTERNS.md`,
`guide.md` and the README tables; ADR 0004 and 0007 are in phase 0.

## 5. Risks

- The TUI rewrite is the biggest and riskiest piece; both apps must keep working at every step
  (AGENTS non-negotiable 4), so phase 3 cannot be one change.
- Dropping the daemon's workspace commands is a breaking protocol change; do it only after both
  clients stop sending them.
- A delayed kill must also work on Windows; test it there before relying on it.
- Verification gate cross-checks (Windows, macOS targets) apply to every step.

## 6. Not read

The 793 KB prototype HTML. The spec says the README wins on behaviour and the prototype on exact
pixels, so read it per screen when building that screen.

## 7. The TUI handoff (`tui.zip`, "Maitty TUI v26") against the rest

It replaces `TUI_SPEC.md` as the terminal app's spec. The desktop README still owns the desktop
app. Where the two disagree, each app follows its own handoff, and the model underneath (the
daemon's flat session list) is the same.

**Closer to today's TUI than `TUI_SPEC.md` was (less rewrite):**
- Keeps the bottom status bar with a mode chip and spelled-out hints (the current mode bar).
- Keeps chips on borders, two separate frames with a one-column gap, and a bottom status bar.
- Settings keep today's sections (General, Appearance, Sessions, Alerts, Agents, Check, Keys), with a
  theme and spinner browser, not the eight pages of `TUI_SPEC.md`.
- Project tools (Files, Changes, Branches, History, References) stay, as a panel from a `Project` menu.
- No chat view in the TUI at all, which settles decision 4.

**New in the TUI:** flat two-line rows (status, name, +N -N, project, machine), a filter row, a
10-column collapsed sidebar (automatic under 110 columns), the footer update states, New session
as a project then machine dialog, Menu and Search commands (`Ctrl+K`), the Inbox panel, four-way
splits (`_ - \ |`), and the service and update states.

**Differences between the two handoffs, and how I resolve them:**
| Topic | Desktop README | TUI v26 | Resolution |
| --- | --- | --- | --- |
| New session | Project, machine, then "start as" (shell, Claude, Codex) and view | Project and machine only; every session is a shell, agents start inside | Same daemon model (a session is a shell). Each app keeps its own dialog; the TUI's Menu has "Session..." for an agent |
| Status marks | A shape for every state, never colour alone | All states are `●`, told apart by colour; a "Status glyphs: unicode" setting for colour-blind users | Each app follows its own; the TUI setting must exist |
| Project tools | Inspector and project actions removed | Kept as a panel | Keep the Inspector until the owner says otherwise (not in phase 2's delete list) |
| Undo window | 6 seconds | 5 seconds | One daemon value; use 6 seconds and update the TUI copy |
| Sidebar width | 264px, 208 to 440px | 46 columns, 24 to 60 | Each its own |
| Settings pages | Nine, including Usage and Daemon | Seven, today's | Each its own |
| Machine marks | Laptop and server icons | `local`, `⇄ name`, `┆ name` | Each its own |

Daemon work is unchanged by this: it needs the same flat list, `Exited`, Undo and config route.

## 8. Round two: desktop v31 and TUI v27 ("Fleets and Ships")

The owner replaced both specs after the first wave landed (`dev` 8f52406e). The sources are
`Maitty Desktop specification (4).zip` (desktop README v31 plus `TUI_SPEC_v31.md`) and
`Sidebar design for remote and local machines (2).zip` (TUI v27). **They supersede sections 1 and 7.**

**The model changes back to named groups, with different words:**
- A **fleet** is a named group of work on one machine, with a colour and initials. A **ship** is one
  session in a fleet (an agent or a shell). Machines are headings in the sidebar, and a fleet belongs to
  exactly one machine. The flat single list is gone; so are the first wave's "no containers" rule and its
  folder-badge rows.
- The **tab bar returns**: one tab per session of the selected fleet; a session's panes (up to 4) live
  in its tab.
- Desktop only: a **Crewmates** mode (Captain, crewmates with chats, crews with invites). The spec itself
  says multi-user crews need a sync service that is not designed. **Decision (revised 2026-11-06):**
  local crews are built first, as one shared chat of people and crewmates that is not a fleet
  ([ADR 0010](adr/0010-crews-are-daemon-owned-and-local-first.md), `features/crews/plan.md`). Inviting
  teammates (the sync service) is not built and waits for its own design. Until the local mode lands,
  the mode switch shows Fleets only.

**What the daemon needs:** a fleet is what the daemon already calls a workspace (name, colour, tabs), and
there is one daemon per machine (ADR 0007), so a workspace on a machine's daemon is a fleet on that
machine. The commands the first wave planned to remove (`NewWorkspace`, `NewTab`, `SetGroup`,
`SetWorkspaceColor`, `MovePane`, `PaneToWorkspace`) are therefore kept. Missing pieces, if any, are
additive: create, rename and recolour a fleet, and move a ship between fleets. Decision 1 of section 3
(layout belongs to each app) is reversed for the fleet and tab structure: it lives in the daemon again.

**What survives from the first wave:** all daemon work (diff counts, `exit`, close with Undo, the shared
settings route, protocol 74); the desktop New session modal, chat model picker, status marks,
service states, shared settings, Flat default, filter, ship row pieces, rail; the TUI New session
dialog (kept reachable from Menu), Menu, Search commands, Inbox, keys, service and update states,
status glyph module, workspace chips. **What is rebuilt:** both sidebars (fleets above, ships below,
machine headings, fleet rows with stacked app icons, Agents and Shells subgroups, Fleets or Priority
view), the tab bar, and the local layout store (its sessions now group into fleets).

**Conflicts between the new documents, and the call taken:**
| Topic | Desktop v31 and `TUI_SPEC_v31.md` | TUI v27 | Call |
| --- | --- | --- | --- |
| Ctrl+T in the TUI | Opens the New session dialog; Ctrl+Shift+T makes a shell | Makes a shell at once, no dialog | v27 is the TUI's own handoff: Ctrl+T makes a shell; the dialog stays in the Menu as "Session..." |
| Sidebar mode switch in the TUI | `[Fleets or Crewmates]` | Not present | Built after the local crews work (see above) |
| Diff counts on rows | Not shown | Not shown | The data stays in the protocol; rows do not show it |
| Status marks | A shape for every state | One glyph, colour only | Each app follows its own handoff |
