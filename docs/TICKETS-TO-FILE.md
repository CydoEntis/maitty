# Tickets to file in Plane

Follow-ups from 2026-11-04 that need tickets in the `MAITTY` project. Plane is only reachable
from the home network, so they are parked here. **Delete this file once they are filed.**

## Game plan: the order to work through everything (proposed 2026-11-05)

Read this first. It orders every open fix in this file. The order is a proposal: the owner can
change it, and an agent does not.

**Order changed by the owner (2026-11-05, evening).** Phase 2 goes first, then v0.65.1 without the
desktop app (`apps/gui/RELEASE-HOLD` holds it out), so the terminal fixes in #275 and #276 ship.
Phase 1 follows.

**v0.65.1 is out (2026-11-05)**, without the desktop app: the updater still offers 0.64.0. Phase 1
is next. What is left of phase 2: ticket 26 (draft #281), ten passes in a row on `windows-latest`
for 28 and 29, and ticket 27.

**Resume here (written 2026-11-05, evening).**
- **State.** v0.65.1 is released from `dev`. The desktop app is held out of releases by
  `apps/gui/RELEASE-HOLD`; its last good version is 0.64.0.
- **The design to build from** is the revised v18 handoff the owner supplied this evening:
  `docs/features/chat-gui/design-handoff/v18/` (read `README.md` section D first; D.6 to D.9 are
  new). Its screens are in `.maitty/references/`. Build each screen whole from it, and put a
  screenshot of the build beside the design's before calling the screen done.
- **Next, in order.** Phase 1 (tickets 35, 40, 36, 37, 38), then tickets 48, 49 and 50 from the
  revised handoff, then the rest of phase 4. Ticket 51 (terminal app) is small and can go at any
  point once the owner has answered its question.
- **Ticket 35 starts with step 1** of *The fix, in order* in [PERFORMANCE](PERFORMANCE.md): count
  frames at the host's `channel.send` and at the window's `countOutput`. The owner saw the lag on
  Windows too, so it can be measured there.
- **Where to work.** The owner's remote Linux dev box has no graphical session: build and test
  there, but look at the desktop window on the Windows machine (0.65.0 is installed there) or the
  Linux work machine. On the dev box `cargo` is in `~/.cargo/bin`; `node`, `pnpm` and the Tauri
  Linux packages are in the `devbox` container. A fresh worktree needs `pnpm bundle:daemon:dev` in
  `apps/gui` before `cargo build`.

**How to work**
- One ticket, one branch from `dev`, one pull request into `dev`. Read `AGENTS.md` and the docs the
  ticket touches before planning it.
- Do the tickets in the order below. Do not start the next one because it looks close.
- Run the verification gate before every push and quote the results in the pull request.
- A ticket marked **owner** needs a decision first: stop, ask, and record the answer in the doc
  that owns the topic.
- When a ticket is merged, mark it done here with the pull request number.

**Step 0. Before any code (ten minutes)**
- `git pull` on `dev`. Check that pull request #271 is closed (it is redundant) and that the
  v0.65.0 release page still carries its warning.
- Tickets 1 to 22 and G1 to G25 are older than v0.65.0. Most of them shipped in #268 and are not
  marked. They are **not** part of this plan. Do not start one from here.

**Phase 1. Make the desktop app usable.** Nothing else about the desktop app matters until this is done.

| Order | Ticket | What | Needs |
| --- | --- | --- | --- |
| 1 | 35 | The terminal runs one action behind. Follow *The fix, in order* in [PERFORMANCE](PERFORMANCE.md), *Input latency* | **owner** for the ADR in its step 2 (a local socket for terminal output) |
| 2 | 40 | Opening a pane types old terminal replies into it | nothing |
| 3 | 36 | Agent hooks point into the AppImage's temporary folder | **owner**: where the daemon is kept so its path does not change |
| 4 | 37 | Restart and restore cannot stop the desktop app's own daemon. The fix is on `fix/desktop-upgrade-path`: rebase, gate, pull request | nothing |
| 5 | 38 | Settings › Performance lists no daemons | nothing |
| 6 | release | The first release with the desktop app back. Its pull request deletes `apps/gui/RELEASE-HOLD` and carries the **Key to echo** figure from a real Linux window | **owner** says go |

**Phase 2. Stop a release costing a day.** Small, independent, no owner decision except the last.

| Order | Ticket | What |
| --- | --- | --- |
| 7 | 24 | Pull requests stop waiting on macOS runners. Done in #279 |
| 8 | 25 | Linux package install gets a limit and a retry. Done in #279 |
| 9 | 23 | One command for a release. Done in #279 |
| 10 | 26, 28, 29 | The three intermittent Windows test failures, one pull request each. Name the cause from a Windows run; no retries. 28 fixed in #280 and 29 in #282; 26 is the draft #281 |
| 11 | 27 | A build cache for the release workflow. **owner**: a decision first, and it may not be worth doing |

**Phase 3. The terminal app.**

| Order | Ticket | What | Needs |
| --- | --- | --- | --- |
| 12 | 31 | A needs-you alert rings for a prompt that settles itself. Status has to be right, so this is first here | nothing |
| 13 | 33 | A click on a project row moves the keys into the sidebar | nothing |
| 14 | 30 | What Codex, Gemini and OpenCode send as their last message (spike) | nothing |
| 15 | 32, 34 | Rename a machine; choose the alert sounds | **owner**: these are features, built when the owner asks |
| any | 51 | Updating moves into the Menu | **owner**: a mark on the Menu button when an update is out, or not |

**Phase 4. The desktop app's design.** Only after phase 1: a layout cannot be judged in a window that lags.

| Order | Ticket | What | Needs |
| --- | --- | --- | --- |
| 16 | 39 | Review every screen with the owner: keep, change or cut. Agree the smallest window. Put the reference designs in `.maitty/references/` | **owner**, throughout |
| 17 | 48, 49, 50 | What the revised handoff changed, cuts first: the removals (D.6), usage in Settings (D.7), sidebar sessions keep their place (D.8) | nothing: the handoff says what |
| 17a | from 39 | The other cuts and changes. The list so far is under *Design changes the owner asked for* | **owner** where the handoff is silent |
| 18 | ADR | Chat on any session reverses ADR 0052 | **owner** |

**Phase 5. Borrowed from a read of GG Coder.** Added by the owner on 2026-11-06; it follows phase 4.
Ideas only, written fresh: that project is AGPL-3.0 and Maitty is MIT, so nothing is copied from it.

| Order | Ticket | What | Needs |
| --- | --- | --- | --- |
| 19 | P5-1, P5-2 | A stopped or killed daemon leaves no provider processes behind. Ownership has to be right, so this is first here | nothing |
| 20 | P5-9, P5-10 | The built desktop package is launched on every push to `dev` | nothing |
| 21 | P5-6, P5-7, P5-8 | A working agent that repeats itself says so on its row | nothing |
| 22 | P5-3, P5-4, P5-5 | Chat: tables, no flashing while a reply streams, following the newest message | P5-3 sends screenshots to the **owner** |

**When to stop and ask instead of pressing on**
- The lag in ticket 35 does not show on the machine in use. It was measured on Hyprland, NVIDIA
  and WebKitGTK; a machine without it changes the fix.
- A fix would change what a person sees or does in the app beyond what its ticket says.
- A check in the gate fails for a reason the ticket does not cover.

## 1. Glass on light themes (feature, `client`)

On Paper and Day the Glass finish reads badly: the ambient glow and the cards' resting drop
shadows were tuned on dark grounds and look like smudges on a light one. Flat and Tiled are fine.
Scope: a design pass on Glass for light themes, within the performance rules measured in
[PERFORMANCE](PERFORMANCE.md). The brief below is the prompt to hand to whoever does it, human or
agent. Acceptance: screenshots in Midnight, Monokai, Paper and Day; harness scenarios `a` and `b`
within budget (quiet working session under 5% of a core, streaming under 30%); `docs/DESIGN.md`
updated.

## 2. Terminal text contrast on light themes (bug, `client`)

On Paper the terminal's text uses `--fg2` and dim text `--dim`, both mid greys that fail WCAG AA
on the near-white `--term` ground; the font looks thin and washed out. Darken those tokens for the
light themes (the contrast audit in `themes.test.ts` should cover the terminal foreground and dim
text against `--term`, not only the UI slots). Do with ticket 1 or just before it: a designer
touching the tokens sets these anyway.

## 3. Terminal font settings (feature, `client`)

Settings › Text & cursor gains font family, normal weight and bold weight. xterm supports all
three (`fontFamily`, `fontWeight`, `fontWeightBold`); the family is a text field with the
monospace fonts the system reports, falling back to the bundled Geist Mono. The weights are the
cheap readability win on Linux, where Geist Mono at 400 renders thin in WebKitGTK. Store them with
the other preferences; apply in place like font size.

## Brief for ticket 1

```text
Redesign the "Glass" window style of the Maitty desktop app (apps/gui). Read
docs/DESIGN.md (the current visual language), docs/PERFORMANCE.md (what was
measured and why), and apps/gui/src/features/workspace/workspace.css (where
the styles live) before proposing anything. Keep Flat and Tiled as they are;
this is about Glass only, in both dark (Midnight, Monokai) and light (Paper,
Day) themes.

Goals, in priority order:
1. Readable. On light themes the terminal text and the dim text must pass
   WCAG AA against the terminal ground. Today they do not.
2. Cheap to draw. The app runs in WebKitGTK on Linux, which repaints in
   software or with a weak compositor, and in WebView2 on Windows. The budget
   is a quiet working session under 5% of one core and a streaming session
   under 30%.
3. Still recognisably Glass: floating cards on an ambient ground, a sense of
   depth, status colours that read at a glance.

Hard rules from the measurements (each one cost 10-30% of a core when broken):
- No infinite animations. A status change may play a one-shot effect that
  settles; nothing loops while a session is merely working or waiting.
- Never animate box-shadow, filter or backdrop-filter. Animate transform and
  opacity only.
- backdrop-filter (blur) only on small, short-lived surfaces: menus, popovers,
  dialogs. Never on the sidebar, panes, tab bar or any panel that stays open.
- Resting drop shadows and gradients on large cards are the single most
  expensive thing on software WebKit: a 160 ms spinner repainting a shadowed,
  gradient sidebar card cost 11% of a core. Prefer hairlines, flat fills and a
  single subtle top highlight. If a big card keeps a gradient, every element
  that changes on its own (spinners, counters, clocks) must be isolated in its
  own paint layer (contain: paint; will-change: transform).
- The terminal is opaque by default and sits on the theme's --term ground;
  ANSI black equals that ground. Do not design around a see-through terminal.
  It exists only as an opt-in switch, because programs paint their own
  backgrounds and those show as boxes on anything translucent.
- The web view may have no WebGL (data-gpu="software" on the root). Glass must
  degrade on that path: provide the values for it, not a second set of rules.
- Themes are tokens on :root (--bg, --main, --pane, --term, --fg, --fg2,
  --dim, --accent, the ansi-* colours, the status colours). Design in tokens,
  and keep every status colour (working, needs you, done, idle, error) exactly
  as distinguishable as today in every theme.

Deliverables:
- The token values per theme (dark and light) and the Glass CSS, as a change
  to workspace.css and the theme files, not a separate stylesheet.
- A before/after of the measurement: run
  apps/gui/scripts/perf/webkit-harness.py scenarios a and b (see the README)
  and put the numbers in the PR. Numbers above the budget mean the design is
  not done.
- Screenshots in Midnight, Monokai, Paper and Day with one Claude Code pane
  running, one idle shell and the Settings dialog open.
- Update docs/DESIGN.md to describe the result; do not add history to it.
```

## Added 2026-11-04 (evening): v18 handoff, Machines GUI, send files

The v18 handoff is in `docs/features/chat-gui/design-handoff/v18/` (README §D and
`GLASS_AND_TILED.md`). It is the design for tickets 1 and 2 above: build them from it, not from the
brief. Every v18 ticket carries the same gate: harness scenarios `a` and `b` before and after (quiet
under 5% of a core, streaming under 30%), screenshots in Midnight, Monokai, Paper and Day, and
`docs/DESIGN.md` updated.

## 4. Land the Machines GUI (feature, `client`, `ssh`)

Branch `refactor/maitty-machines-move` (19 commits, not in `dev`): sidebar machine groups and status
chips, the Machines dialog and Settings page, add a machine, start a session on one, per-machine
opt-ins, accessibility pass. Merge with `dev` (conflicts in `lib.rs`, `DesktopApp.tsx`,
`DesktopSettings.tsx` and its test), then the full gate. The macOS cross-check needs a Mac or CI.

## 5. v18 Glass and light themes (feature, `client`; replaces tickets 1 and 2)

Glass tokens, opaque cards with a hairline, blur only on short-lived surfaces, the retuned Paper and
a new Day theme, the Background glow switch, one-shot needs-you and update-dot effects, steady
caret, isolated spinners. All text tokens pass WCAG AA on `--term` and `--bg`; the contrast audit in
`themes.test.ts` covers the terminal foreground and dim text. Still to check: Monokai `--dim`, the
generated light themes.

## 6. v18 Tiled hairline grid (feature, `client`)

Zero gap and radius, 1px hairlines, hairline-only popovers, uppercase Geist Mono chrome type, corner
ticks, 40px flush tab bar. Blocked by 5 (shared tokens).

## 7. v18 status effects: Waves and Sweep (feature, `client`)

Per-status (Working, Needs you, Done) on/off and style, a Working scope (All / Selected / Off),
previews clipped to their chip, automatic Sweep on `data-gpu="software"`. Waves loops while a
session works, which breaks the "nothing loops" rule; the README keeps it by request, so measure
scenario b with several working rows, and if over budget default to Selected or Sweep. Raise this
conflict with the user before building.

## 8. v18 spinner dropdown and web spinners (feature, `client`)

Replace the spinner card with a dropdown in the Theme card: three TUI spinners and five web SVG
spinners (Circle, Dots, Equalizer, Spokes, Ping), live previews, Esc and outside click close it.

## 9. v18 Settings cleanups (chore, `client`)

Close becomes a 30x30 × icon button (tooltip "Close (Esc)"). Remove per-project colours: the
setting, the swatch row and the dots that depend on it.

## 10. Terminal font settings (feature, `client`)

Ticket 3 above, unchanged. Not in v18; GLASS_AND_TILED.md §3 says the weight fix belongs here.

## 11. Send files to a machine (feature, `ssh`, `client`, `daemon`)

Today only a clipboard image goes to a remote pane (ADR 0049: over the machine's own hardened ssh
line, into a private cache folder, the path typed into the pane). Extend that to any file and to
pasted text: drop or paste a file on a remote pane, it lands in that machine's cache folder, its
path is typed into the pane. ADR 0047 listed a remote file API as out of scope, so this needs a new
ADR that widens what a remote machine can receive: size cap, private folder, cleanup, refusal on a
custom transport, and a progress and cancel state for large files. Windows is in scope.
Reuses `crates/maitty-core/src/remote_paste.rs`.

## 12. Check v17 against what is built (spike, `docs`)

v17 describes the chat body, session panel, overview, branch agents, idle update, brain dump, usage
and cost. Nobody has compared it with `apps/gui/src`. Produce a gap list, then file one ticket per
gap.

## v17/v18 gap tickets (G1-G21), from the ticket 12 check, 2026-11-04

Found by reading `apps/gui`; a "partial" may hide a feature under another name. Labels: type / area.
The desktop window is terminal-only today; the chat body (G15-G18) does not exist.

- **G1 `formatShortcut(binding, platform)` and the v18 shortcut map** (chore / client). Route tooltips,
  keycaps, menus, palette and `hotkeyRows` through it; Inbox moves to Ctrl+Shift+J, Ctrl+J jumps, add
  Ctrl+0, Ctrl+\, Ctrl+Shift+N/U/P/E as stubs. Needs question 4.
- **G2 Tooltip parity (B.4)** (bug / client). `HoverTips` also reads icon-only `aria-label`; 420 ms
  delay, 40 ms follow-on, keycap parsing, placement rules. Blocked by G1.
- **G3 Session switch with no loading screen (C.0a)** (feature / client). Remove "Connecting to
  terminal" for local panes, per-session last-screen cache, one `term.write` per frame on replay,
  the 300 ms "Catching up from <machine>" strip for remote.
- **G4 Machine surfaces (B.1)** (feature / client). Status-bar machine item, pane and tab machine chip,
  per-pane "Lost connection ... Retrying in Ns" strip, sidebar machine filter. Needs question 9.
- **G5 Floating status bar (B.5)** (chore / client). 32 px floating card; left holds G4's machine.
- **G6 Idle self-update (C.5)** (feature / client). 5-minute idle timer, never while an agent works,
  default on, relabel, post-relaunch toast and What's new dialog.
- **G7 Platform title bars (C.0)** (chore / client). macOS traffic lights, Windows 46x32 captions,
  Linux round buttons.
- **G8 Overview (C.3)** (feature / client). Ordering model, screen, Ctrl+J / Ctrl+Shift+J / Ctrl+0,
  "Nothing needs you" toast. Blocked by G1.
- **G9 New agent on a branch (C.4)** (feature / daemon, client). Agent choice and first prompt on the
  worktree API, new dialog, pinned button, Ctrl+K entry. Needs question 5. Blocked by G1.
- **G10 Status shimmer to spec (C.4)** (chore / client). Overlay span, amber needs-you, 1.4 s once,
  software-GPU and unfocused gating. Overlaps ticket 7 (v18 status effects): do together.
- **G11 Brain dump (C.6)** (feature / client, daemon). Needs question 7. Blocked by G1, G9.
- **G12 Usage store and API (C.7 daemon)** (feature / daemon). Per-session per-day provider-reported
  usage, never estimated. Needs question 8.
- **G13 Usage and cost page (C.7)** (feature / client). Blocked by G12, G1. Built on `feat/gui-usage-page`
  (page, Ctrl+Shift+U, footer icon, client-side budget); remote machines' usage waits for a daemon report that covers them.
- **G14 Performance page to spec (C.8)** (feature / client). Four tiles with sparklines, daemon ping,
  ranked "What's costing" with session-only switches, Ctrl+Shift+P, harness brought up to date.
- **G15 Event projection (CV-0)** (feature / client). Needs question 1.
- **G16 Chat switch, history, composer (CV-1..3)** (feature / client, daemon). Blocked by G15, G1.
  Client side built (CV-1..3, see the chat view plan). Two daemon gaps found, proposed as new tickets:
  **G23** (daemon part DONE, see ADR 0052; GUI link in the next commit) a Claude/Codex agent running in a terminal
  pane had no daemon conversation, so no pane had a chat: the daemon now follows the agent's own transcript as a
  read-only `Native` conversation whose `native_term` is the pane. Open: a composer for these panes (owner decision);
  **G25** (DONE for Claude and Codex, protocol 72, ADR 0054) chat sessions: the desktop starts a Claude or Codex
  structured conversation (Session: Chat in New agent on a branch, or Start a Claude/Codex chat on a shell), listed as
  a session of its own with the full composer, approvals, user-input form, Stop, queue and attachments. Not built:
  resuming the agent after a daemon restart from the window; another machine's chat sessions; not tried against a live
  Claude or Codex;
  **G24** (DONE, protocol 71) a daemon-held send queue, delivered when the turn ends, and a cheaper-to-follow history
  (every page carries a `resume` cursor that returns only what changed). Not built: delivery in the middle of a
  turn (steer), a queue that survives a daemon restart, and a pushed history (the client still polls, one small read).
- **G17 Chat items (CV-4..7, 9, 10)** (feature / client). Blocked by G16.
- **G18 Session panel (C.2)** (feature / client, daemon). Split: shell, Changes and comments, Files and
  Artifacts. Blocked by G17.
- **G19 Notification and Inbox action parity (B.6)** (chore / client). Dismiss everywhere now; approval
  by id after G17.
- **G20 Pane shortcut contract (B.8)** (bug / client). Non-mac code takes bare Ctrl+T/W/D/F/K, the spec
  says Ctrl+Shift. Needs question 4. Blocked by G1.
- **G21 Activity bar and Panels button (B.7)** (spike / client). Decide if v15 section A is in scope.

### Questions for the owner (they block the tickets named)

1. ANSWERED 2026-11-04: yes, the chat view is wanted for the desktop (the README's "no chat UI" line is outdated). (G15-G18)
2. ANSWERED 2026-11-04: Claude Code and Codex sessions get Chat first; other agents stay terminal-only. (G16)
3. DECIDED (owner delegated, 2026-11-04): yes, keep typing y/a/n into the terminal as a fallback. (G19)
4. DECIDED 2026-11-04: follow the spec. On Windows and Linux, plain Ctrl+letter always goes to the terminal; the app uses Ctrl+Shift (Cmd on Mac). (G1, G20)
5. DECIDED (owner delegated): yes, a small addition to the worktree API. (G9)
6. DECIDED (owner delegated): toast only, per the handoff. (G8)
7. Brain-dump split: which model and account, where does the JSON get validated? (G11)
8. DECIDED 2026-11-04: yes, the daemon may store provider-reported usage per session per day; never estimated. (G12)
9. DECIDED (owner delegated): local only in v1; remote branch agents come later. (G4, G9)
10. Plans link to v17; v18 is newest. Fix the links. (docs)

## 13. Adaptive animations: measure, then pick the effect level (feature, `client`)

Today `data-gpu` is set once from whether WebGL 2 works, which is a guess at "rendering is fast".
Measure frame times for a few seconds after start and again after Waves turns on; if frames run
slow, step down one level (Waves, Sweep, none) and never step back up on its own. Add Settings >
Appearance > Animations: Auto (default), Full, Light, Off. The performance page says what Auto chose
and why ("Software rendering detected, using Sweep"). Reduce motion and the existing hidden/unfocused
rules still win. Acceptance: unit tests with a fake clock for the step-down; harness scenarios `a`
and `b` within budget. Blocked by: 7 (status effects).

## 14. Design system rules and a hard-coded colour lint (chore, `client`, `docs`)

Approved by the owner 2026-11-04. Unify the foundation, not the whole UI:
1. Every colour, radius and shadow comes from a theme token (CSS variable), mapped into Tailwind by
   the `@theme inline` block in `index.css`; no raw hex or `rgb()` in components.
2. Every popup (menu, dropdown, dialog, tooltip, popover, listbox) is a shadcn/Radix primitive from
   `src/components/ui/`; no hand-rolled focus, Esc or arrow-key handling.
3. New components use Tailwind utilities on those tokens; heavy animation and per-style rules
   (Flat, Tiled, Glass, `data-gpu`) stay in `workspace.css`.
4. Old CSS moves to the rules above only when a ticket touches that area; no big-bang rewrite.

Work: put the four rules in `docs/UI.md` (and point `docs/stack/shadcn-tailwind.md` at them), add an
ESLint or script check that fails on a hard-coded colour in `apps/gui/src` outside the theme files
(with a small allowlist for the terminal palette), and move the existing offenders or list them as
the allowlist's baseline. Acceptance: the check runs in `pnpm lint` and passes. Do after wave 1 lands
(it edits `docs/UI.md`); tickets 7 and 8 get rule 2 in their briefs.

## G22. The daemon's conversation history must carry tool input and approval detail (feature, `daemon`)

Found building G15 (the chat event projection, merged). Without this, Claude chat cards are mostly
empty. Today:
1. A Claude tool call is recorded as a bare label ("Edit "): `bounded_content_text(&input)` returns
   an empty string for an object, so there is no file, diff, to-do list or helper-agent title. Add a
   bounded `input` field to `Content::Tool` (a `#[serde(default)]` field needs no protocol bump if a
   test decodes the old shape).
2. A Claude approval's command is only "Edit: " with no path or diff, always `kind: Command`, and its
   `item_id` is the request id, so it cannot be linked to its tool step. Codex links through
   `item_id`.
3. An answered approval vanishes from `read_conversation` and the answer is not recorded, so the
   answer is lost after a GUI restart.
4. No timestamps or exit codes, so "Cooked for 7s" and "exit N" cannot be shown from history.
5. Codex command labels are cut at 256 characters; Codex plan and helper items keep only their kind.

Blocks the real content of G17 (edit blocks, command cards, approvals) and G18. This is chat-view
plan CV-13. Daemon work, so it follows ADR 0036 for status trust. Blocks G17 for Claude.

## 15. Light themes: bright ANSI colours and the pane's bottom edge (bug, `client`, `theme`)

Seen in the Tiled Paper screenshot: some terminal words (ANSI cyan and the bright colours) are a very
pale blue on the white ground, and a heavy black bar runs along the bottom of the pane over the
"Working" chip. Extend the contrast audit in `themes.test.ts` to all 16 ANSI colours on `--term` for
Paper, Day and the generated light themes, retune what fails, and find out whether the black bar is
the mock's cursor line or a real style bug. Also: a Paper streaming run sometimes reads about 37%
instead of about 14% of a core; find what flips (animation or compositing mode) and fix or document it.

## Order of the next batch

Batch A (parallel): G1+G20+G2 shortcuts, G22 daemon history, G12 usage store, ticket 15.
Batch B: G16 chat switch (after G22), G8 Overview and G9 New agent on a branch (after G1), G13 usage
page (after G12 and G1). Postponed by the owner (2026-11-04): G11 brain dump; undecided whether it will be built at all.
Until then Ctrl+Shift+N stays 'new worktree'.

## 16. Dark themes: ANSI colours below 4.5:1 (bug, `theme`)

**Done 2026-11-05** (70ba2d7): every dark theme's ANSI and status colours meet 4.5:1 on both grounds. `terminalMinimumContrast` still returns 1 for dark themes on purpose.

Found by the ticket 15 audit, left alone because ticket 15 was light themes only: Nord magenta 4.41,
Solarized Dark blue 4.08 and magenta 3.30, Rosé Pine blue 3.38, Zenburn red 4.02, green 3.59 and
magenta 4.30. Lighten within the hue family, extend the audit to every dark theme, and decide whether
xterm's `minimumContrastRatio` should also apply on dark themes.

## 17. Design handoff v18 is out of date for generated light themes (docs)

The code now uses warn #8F5400 and danger #B8362C for generated light themes (the handoff says
#9A5B00 and #C0392F) and applies xterm `minimumContrastRatio` 4.5 on light themes. Update
`GLASS_AND_TILED.md` section 3 when the design is next revised.

## 18. Decide how conversation history treats secrets (spike, `daemon`, `docs`) — DONE

Found building G22: the history now stores the full input of a tool call (bounded to 64 KiB, escapes
stripped). The history has never redacted secrets, so a Claude `Write` of a `.env` file, or a command
that echoes a token, is now readable by any machine or client with a read grant, and lands in the
private history cache when it is on. `docs/SECURITY.md` says so plainly. Decide: redact by pattern
(keys, tokens, `.env` content), omit input for sensitive paths, or accept and document. Also confirm
against a live Claude CLI that `can_use_tool` carries `tool_use_id` (the fake CLI does; the code falls
back to the request id). Blocks nothing, but decide before the chat view ships widely.

**Done 2026-11-04** (owner decision in *Decisions on 2026-11-04 (night)*): the daemon masks token- and
key-shaped strings and omits the content of obviously secret files at one choke point
(`daemon/conversation/redact.rs`), with GUI cards that show both honestly. What is masked, what is
omitted and the limits are in [SECURITY](SECURITY.md#conversation-history-and-secrets). No
`PROTOCOL_VERSION` change. **Still open:** confirm against a live Claude CLI that `can_use_tool`
carries `tool_use_id` (the fake CLI does; the code falls back to the request id).

## 19. Paste an image into a remote pane in the desktop app (feature, `ssh`, `client`)

The terminal app sends a pasted clipboard image to a pane on another machine (ADR 0049); the desktop
app still says local-only for it. Reuse the transfer code that now lives in `maitty-machines` (moved
for ticket 11) behind a Tauri command, with the same limits (25 MB for an image, private cache folder
on the far side, path typed into the pane), progress and error states, and tests. Windows is in scope.

## 20. Attachments in the chat: files and images, local and remote (feature, `client`, `ssh`, `daemon`)

Owner idea, 2026-11-04. When the chat view is open (especially on a remote machine) the user must be
able to attach screenshots, images, design zips and other files to a message, as in Claude's own chat:
a paperclip and drag-and-drop on the composer, paste an image, a thumbnail or file chip in the draft
and in the sent message, remove before sending. Remote sessions reuse ticket 11 and 19 (the file lands
in the far side's private cache, the path goes into the message); local sessions reference the local
path. Open questions to settle in an ADR: how each provider receives attachments (Claude Code and
Codex accept image paths or inline images differently), size caps, whether archives are unpacked or
passed as a path, what the history stores (the path or the bytes; see ticket 18 on secrets), and
cleanup. Blocked by the chat composer (G16) and tickets 11 and 19.

Progress (2026-11-04, ADR 0053 proposed): the local case is built. The composer has a paperclip, drop
and image paste, draft chips and thumbnails with remove, and sent messages show their files; the host
copies each file into a private `attachments` cache after checking it. The message names the files by
path in its text, so the daemon and `PROTOCOL_VERSION` are unchanged. Remaining: attachments for a chat
on another machine (needs remote chat; reuse `send_files_to_machine` / `send_pasted_image` and put the
far-side path in the message, behind the `machine` argument that already exists), a structured field
(Codex `localImage`, Claude image blocks) if path-in-text proves weak, and thumbnails for picked or
dropped images (the window never reads those files).

## 21. An intermittent Rust test failure (bug, `tooling`)

**Done 2026-11-05** (d0e00d6, 19b958b): two causes, both test races (a history read right after the ack; typing into a pane before its login shell was ready). Fixed with deterministic waiting.

On the merged tree (948786e) one `cargo test` run (without `--no-fail-fast`, so it stopped early at 832
tests) exited 101 with 1 test failed; two reruns of the whole suite passed (2293 passed, 0 failed) and
the first run's log did not keep the test name. A daemon helper saw
`structured_local_ipc_create_send_read_reconnect_and_exact_identity` fail once under load, but it
passes 6 of 6 in isolation, so the culprit is not confirmed. Next time it happens, keep the log, name
the test, and fix its timing instead of retrying. The three slow `maitty-terminal-client` shell tests
(`remote_status_tests::a_remote_agents_row_is_what_its_daemon_reported_and_follows_its_next_report`,
`cli_machine_tests::send_and_wait_use_the_remote_pane_s_screen`,
`mcp_machine_tests::an_agent_lists_reads_and_sends_to_another_machine_s_panes`) also time out under load.

## 22. Update a machine from the GUI and the TUI (feature, `client`, `ssh`)

Asked for 2026-11-05. Today a machine whose Maitty is older than this computer's shows
"run `maitty machine update <name>`" as text, and the update exists only as that command
(ADR 0047, D8; `crates/maitty-terminal-client/src/cli/machine_update.rs`). The owner wants an
**Update** action on each machine instead of having to type the command.

**Scope**
- GUI: an Update button on a machine in the sidebar group and on its Machines screen, shown when the
  machine is older, missing the app, or refused for a protocol mismatch (the "needs update" state).
- TUI: the same action in the machine list.
- One shared flow, not a copy of the command's logic: look first (the machine's platform and installed
  version), show what was found, install only after a yes, then show progress and a plain error.
- The far-side daemon is never signalled; it keeps its sessions and is offered a restart only when none
  is open (the existing rule). Say so in the confirmation.
- The binary comes from the existing source logic (`machine_update/binary.rs`): this computer's own when
  the platform matches, else the checked release. Say plainly when there is nothing to fetch (for
  example before a release exists).
- A downgrade (`--force`) stays a command-line choice.

**Acceptance**
- A machine that needs an update shows the button; clicking it, confirming and finishing leaves the
  machine connected at the current protocol with its sessions intact.
- Cancel, an ssh failure and a failed checksum each leave the machine as it was and say so.
- Reuses the file-send transport (`maitty-machines` ssh runner); no new ssh code path.
- Tests with the scripted-ssh fixtures. Real ssh is checked by hand.

**Notes**
- Blocked in practice until a release exists, because only a release carries a protocol-72 `maitty`
  to push to a machine whose platform differs from this computer's.
- Needs an ADR amendment to 0047 or 0048 (the desktop app may now change another machine).


## Decisions on 2026-11-04 (night)

- Image paste key: Ctrl+Shift+V on Windows and Linux (plain Ctrl+V stays with the terminal, per the shortcut
  rule), Cmd+V on macOS. Alt+V is NOT used: Claude Code takes Alt+V (image paste on Windows) and Alt+T
  (ADR 0029).
- G25 (new): a composer, approvals, queue and Stop for chats. Owner approved the recommendation: sessions
  started from the GUI as daemon-owned structured Claude/Codex conversations; panes where the user typed
  `claude` or `codex` in a plain shell stay read-only (ADR 0052).
- Ticket 18 (secrets in history): owner delegated. Mask token- and key-shaped strings and omit the content of
  obviously secret files (`.env`, keys, credentials) from stored tool input; keep the file name and kind.
- Push: after the three items above are done.


## Added 2026-11-05: release and CI speed, from the v0.65.0 release

Releasing v0.65.0 took most of a working day, nearly all of it waiting on CI. Where the time went:

| Cause | Lost | State |
| --- | --- | --- |
| `test (windows-latest)` compiled from an empty cache, about 35 min a pull request | ~70 min | fixed in #270: 11 to 12 min |
| The same job hit its 15 minute step limit while still compiling, and was re-run | ~30 min | fixed in #268 and #270 |
| The release was a chain of pull requests (feature, bump #272, release #273), each waiting on CI | ~60 min | ticket 23 |
| GitHub had no macOS runner: jobs cancelled three times on 2026-11-05, once on 2026-11-04 | ~45 min | ticket 24 |
| One intermittent Windows test, re-run | ~30 min | ticket 26 |
| `apt-get` took 30 min on a GitHub runner | 30 min | ticket 25 |

Already on `dev` (#270): only `dev` saves a build cache, failed runs included; pull requests build no
installers (the push to `dev` does); the frontend's tests and lint run once; `scripts/ci-areas.sh`
skips jobs a change cannot reach; a pull request into `main` runs nothing.

**Order.** 23, 24 and 25 are small and independent of each other: one pull request each, or one
`ci` pull request with three commits. 26 needs Windows runs and is its own pull request. 27 is a
decision first and may not be worth doing.

**To do them:** read `AGENTS.md`, then `.github/workflows/ci.yml`, `.github/workflows/release.yml`
and `scripts/ci-areas.sh`. Run the verification gate before each push. A pull request's own CI run
is the test of a workflow change; say in the pull request what could not be run before merging.

## 23. One command for a release (chore, `tooling`)

**Done 2026-11-05** (#279): `scripts/release.sh`. Run against a copy of the repository with a
stand-in remote, then used for v0.65.1: 8 minutes 26 seconds from the tag to the last install
check, with the desktop app held out.

The bump pull request (#272) and the release pull request (#273) each waited on a full CI run to
change three version strings. The release rule in `AGENTS.md` asks only for: set the version,
commit, bring `main` up to `dev`, tag. Add `scripts/release.sh <X.Y.Z> [--dry-run]` that does that
and nothing else.

**What it does, in order**
1. Refuses, with a message and a non-zero exit, unless all of these hold:
   - the argument is `X.Y.Z` and greater than the version in `apps/tui/Cargo.toml`;
   - the tag `vX.Y.Z` exists neither locally nor on `origin`;
   - the newest `ci` run for a push to `dev` is complete and green, and is for `origin/dev`'s head
     (a head whose commits since that run touch only `docs/**` or `*.md` has no run of its own:
     accept the newest run then). `--skip-ci-check` overrides this one check and says so in its
     output: GitHub cancelling a macOS job for lack of a runner marks the whole run failed.
2. Works in a temporary detached worktree of `origin/dev`, so the caller's checkout, index and local
   branches are untouched. `dev` and `main` are often checked out in other worktrees, where
   `git switch` would refuse.
3. Sets the version in `apps/tui/Cargo.toml` and `apps/gui/src-tauri/tauri.conf.json`, refreshes
   `Cargo.lock` with `cargo update --workspace --offline`, and commits the three files as
   `build: bump to vX.Y.Z`.
4. Checks out `origin/main` detached, merges the bump commit with
   `git merge --no-ff -m "Merge dev into main for vX.Y.Z"` (the form used up to v0.61.0), and tags
   the merge commit `vX.Y.Z` (a lightweight tag, like the existing ones).
5. Pushes all three together:
   `git push --atomic origin <bump>:refs/heads/dev <merge>:refs/heads/main refs/tags/vX.Y.Z`.
   If the push is refused, deletes the local tag. Always removes the temporary worktree.
6. Prints the URL of the release run the tag started.

`--dry-run` runs every check in step 1 and prints the commands of steps 3 to 5 without running any
that commit, tag or push.

**Acceptance**
- `scripts/release.sh 0.65.1 --dry-run` on a green `dev` prints the plan and leaves `git status`,
  `git branch -vv`, `git tag -l` and `git worktree list` exactly as they were.
- Each refusal in step 1 is shown once by hand (a bad version, an existing tag, a red run), with its
  message quoted in the pull request.
- The **Releases** bullet in `AGENTS.md` (*Project-specific: workflow and tooling*) names the script
  and drops the hand-run steps. The owner approved changing this rule on 2026-11-05.
- `docs/COMMIT-STANDARDS.md` says a direct commit to a protected branch needs explicit, one-time
  authorization: note in the script's header that running it is that authorization for one release.

**Notes**
- It can only be proven end to end by the next real release. Say so in the pull request.
- `release.yml`'s "Check release versions" step still fails a tag that does not match both version
  fields, so a half-applied bump cannot publish.
- The release notes are every `feat` and `fix` subject since the previous tag (`release.yml`,
  "Release notes"). v0.65.0 has 124 lines, which is what the app's update dialog shows. Out of scope
  here; file separately if it should be capped or grouped.

## 24. Pull requests must not wait on macOS runners (chore, `tooling`)

**Done 2026-11-05** (#279).

GitHub cancelled macOS jobs that never started, with "The job was not acquired by Runner of type
hosted even after multiple attempts": `desktop GUI (macOS)` on #272 twice (run 42060316400),
`desktop GUI (macOS host)` on `dev` (run 42068870969), and `install on macos-15` in the v0.64.0
release (run 41875940232). Each cost a 15 minute wait and marked the run failed with nothing wrong.

**What**
- Remove the `gui-host-macos` job from `.github/workflows/ci.yml` and the comment above it.
  `desktop package (macOS)`, on the push to `dev`, stays as the place the desktop host is compiled on
  macOS. Update the file's header comment to say so.
- Leave `check (macOS)` alone: it cross-checks on a Linux runner.
- Leave `release.yml`'s `verify` job alone: it runs after the release is published and holds nothing up.

**Acceptance**
- A pull request's checks include no job that runs on a macOS runner.
- A push to `dev` still runs `desktop package (macOS)`.

**Notes**
- Trade-off the owner accepted: a break that only shows when compiling the desktop host on macOS is
  found on `dev` after the merge, not on the pull request.

## 25. Linux package install: a limit and a retry (chore, `tooling`)

**Done 2026-11-05** (#279): `scripts/ci-apt-install.sh`.

On `dev` run 42068870969, `test (ubuntu-latest)` spent 30 minutes in "Install Tauri Linux
dependencies" (16:54:14Z to 17:24:24Z). The log shows `azure.archive.ubuntu.com` going quiet for up
to 473 seconds between packages. The step normally takes under a minute, and it has no limit.

**What**
- Add `scripts/ci-apt-install.sh <package>…`: `apt-get update` then `apt-get install -y`, each with
  `-o Acquire::Retries=3 -o Acquire::http::Timeout=30`, each attempt under `timeout 240`, three
  attempts, `sudo dpkg --configure -a` before a retry (a killed `apt-get` can leave `dpkg` half way).
- Call it from the four steps that install the Tauri packages: `ci.yml` jobs `test` and
  `package-linux`, `release.yml` job `build-gui`, and `test-build.yml`. Keep each job's own package
  list. Give each step `timeout-minutes: 15`.
- The Arch job uses `pacman`; leave it.

**Acceptance**
- The four steps call the script, and no workflow has its own `apt-get install` line left.
- The pull request's `test (ubuntu-latest)` passes with the step under two minutes.

## 26. Intermittent Windows failure in the three-machines hub test (bug, `client`, `windows`)

On #270, `test (windows-latest)` failed once
(https://github.com/CydoEntis/maitty/actions/runs/42055392461/job/127360803248):

```text
hub::tests::end_to_end::three_machines_snapshots_typing_and_a_replay_after_one_reconnects
panicked at apps\gui\src-tauri\src\hub\tests\end_to_end.rs:235:13:
assertion `left == right` failed: ECHO_ALPHA_92 reached machine 2
  left: 1
 right: 0
```

The same tree passed on #268 and on the re-run, and the test passes every time on Linux. The
assertion says text typed for one machine showed once in another machine's pane, which would be a
serious bug if the desktop hub did it.

**Guess, not confirmed.** The three sandboxed daemons' shells share one user profile on the Windows
runner (`maitty-test-support`'s `Sandbox::env` sets three variables, none of them the profile), and
PowerShell's PSReadLine keeps one history file for every session and draws an inline suggestion
from it. The token that leaked is the one run most recently, and it showed in the pane typed into
next, which fits a suggestion and not a misrouted frame.

**What**
1. Make the failure say what it saw: put the pane's text in the assertion message at
   `end_to_end.rs:235`.
2. Confirm or rule out the guess on a Windows runner, for example by giving each sandbox its own
   PSReadLine history path, or by turning suggestions off for the test's shells.
3. If it holds, isolate the shells in `Sandbox` so no test's shell reads another's history.
4. If it does not hold, stop and report: a real leak between machines is a routing bug in the
   desktop hub, not a test fix.

**Acceptance**
- The cause is named with evidence from a Windows run.
- The test passes ten runs in a row on `windows-latest`.
- No retry is added (ticket 21's rule: fix the cause, don't retry).

**Cause seen on CI, not fixed (draft #281, 2026-11-05).** Under `pwsh` 7 on `windows-latest` a
shell in one sandbox recalls a line typed in another sandbox's shell: #281's own test failed with
the second sandbox's pane showing `echo TYPED_IN_FIRST`. So the shared history is real there.
Setting `USERPROFILE` for each sandbox moves the history file for Windows PowerShell 5.1 but not
for `pwsh` 7. Next: another lever (the test shells naming their own `HistorySavePath`, or saving
no history), tried on `windows-latest`. #281's first commit, the assertion that prints the pane,
is good by itself.

## 27. A build cache for the release workflow (spike, `tooling`)

`release.yml` has no build cache, and its slowest build took 17 minutes (`build GUI windows-x64`,
v0.64.0). Adding `Swatinem/rust-cache` to it as it stands saves nothing: GitHub lets a run read only
the caches saved by its own ref and by the default branch, a release runs on a tag, and every
release is a new tag.

**Decide**
- Whether to trigger the release from the push to `main` (the default branch, whose caches carry
  over) and have the workflow create the tag, or to leave the release uncached.
- Whether release caches fit: the repository's limit is 10 GB, shared with CI's caches, and eight
  release builds would want several of them.

Do this last, after timing a release made with ticket 23. It may not be worth the change.

## Housekeeping from 2026-11-05 (not tickets)

- Close #271: `dev` already has its two changes (the separate build step from #268, the cache saved
  on failure from #270), and it conflicts with #270.
- `docs/adr/0048-machines-in-the-desktop-app.md` says clipboard writes are off unless granted. Since
  #269 a machine's `clipboard` is on unless its entry says `false` (`crates/maitty-machines/src/def.rs`),
  in both apps. Amend the ADR or record the change in a new one.
- A branch cut before these merges (the terminal fixes for v0.65.1) should merge `dev` before its
  pull request: #268 changed about 500 lines of `crates/maitty-terminal-client`.

## Added 2026-11-05 (afternoon): from the terminal fixes (#275) and the Inbox (#276)

Both were built without access to Plane, so neither has a ticket there. Record them when Plane is
reachable; what is left over from them is below.

**Ticket 22 (update a machine), the TUI half.** #275 added **Update…** to a machine's right-click
menu in the sidebar and `u` in the Machines dialog. It runs `maitty machine update <name>` (with
`--install` when the app is not there at all) in a popup pane and dials the machine again when the
popup closes. That is the command's own look-first, ask-first flow, not the shared flow ticket 22
asks for. Still open from ticket 22: the GUI button, the one shared flow, and the ADR amendment.

**Ticket 24, seen again.** On #275 `desktop GUI (macOS host)` was cancelled after 15 minutes with no
runner and no step run
(https://github.com/CydoEntis/maitty/actions/runs/42086501798/job/127453010943); the re-run passed
in 1m28s.

**Ticket 26, seen again.** On #275's first run
(https://github.com/CydoEntis/maitty/actions/runs/42080268185/job/127431252289) the same test
failed with `ECHO_BETA_93 reached machine 0`. #275 does not touch the desktop app.

## 28. Intermittent Windows failure when a machine is removed (bug, `client`, `windows`)

On #276's first run `test (windows-latest)` failed once
(https://github.com/CydoEntis/maitty/actions/runs/42080810487/job/127433096613):

```text
client::machine_flow::tests::removing_forgets_the_machine_here_and_leaves_its_sessions_and_the_saved_layout_alone
panicked at crates\maitty-terminal-client\src\client\machine_flow\tests.rs:773:9:
assertion failed: h.app.remotes.is_empty() && h.app.refused.is_empty()
```

#276 changes neither the test nor the machine flow, and the re-run of the same commit passed.

**Cause, seen on Windows 11 (1 failure in 421 runs).** With the assertion saying what it saw, the
window still held `(MachineId(1), "", Connecting, 1 pane)`: no name and one pane is an entry made by
the daemon's state, not by the supervisor. The daemon sends `TermsChanged` a few milliseconds after
the new pane appears, as its shell starts, and the test's `pump` takes one more turn of messages
before it lets go of the removed machine's connection. A late report does put the machine back:
nothing in the window stopped it. The app's own loop lets go of the connection in the turn of the
key press, so there a daemon's message cannot arrive late; a supervisor's event queued before it
was stopped can (read from the code, not seen happening).

**Fix (#280, in v0.65.1).** The window remembers the ids it has forgotten and drops what arrives under one, from the
daemon or the supervisor. Two tests hand it the late message on purpose and fail without the fix.

**Acceptance** as ticket 26: the cause named with evidence from a Windows run, ten passes in a row,
no retry added. Still to see: ten passes in a row on `windows-latest`.

## 29. Intermittent Windows failure closing a worktree's last pane (bug, `daemon`, `windows`)

On #275's second run `test (windows-latest)` failed once
(https://github.com/CydoEntis/maitty/actions/runs/42080268185/job/127439731421):

```text
daemon::logic_tests::closing_a_worktrees_last_pane_removes_it_only_when_nothing_lives_only_there
panicked at crates\maitty-core\src\daemon\logic_tests.rs:1018:14:
a pane there
```

The third run of the same commit passed.

**Cause, seen on Windows 11 with the temp folder under an 8.3 name; fixed in #282.** A race in the
test, not in the daemon. The test found a pane by comparing its folder with the fixture path as
text. A pane's shell reports its folder by its real long name at its first prompt, so with a temp
folder spelled like `RUNNER~1` the pane stopped matching once that prompt was handled. The fixture
now names its root canonically. That CI's temp folder is spelled that way is inferred. Still to
see: ten passes in a row on `windows-latest`.

**Acceptance** as ticket 26.

## 30. What Codex, Gemini and OpenCode send as their last message (spike, `daemon`, `agents`)

The Inbox shows what an agent last said (#276). The hook reads only `last_assistant_message` and
`last-assistant-message` (`crates/maitty-terminal-client/src/cli.rs`), which Claude sends. For an
agent that sends neither, the daemon reads the last three lines above the input line off the pane
(`crates/maitty-core/src/daemon/last_lines.rs`), which was tested against made-up screens only.

**What:** on real Codex, Gemini and OpenCode, capture the turn-end payload and add a row per agent
to `docs/agent-status.md`: the field that carries its last message, or none. Where one exists, a
follow-up ticket reads it. Also look at what the screen fallback shows for each and say whether it
is readable.

## 31. A needs-you alert can ring for a prompt that settles itself (bug, `daemon`)

`NEEDS_SETTLES` in `crates/maitty-core/src/alert.rs` is 300 ms: a permission prompt that is
auto-approved more slowly than that still rings. #275 left it alone because nothing measured how
long such prompts last. **What:** measure it on real agents, then pick the delay.

## 32. Rename a machine (feature, `client`, `ssh`)

**Edit…** on a machine (#275) changes its `target` only. Its saved layout and its recorded daemon id
are kept under its name, so a rename has to move both.

## 33. A click on a project row moves the keys into the sidebar (change, `client`)

#275 made a click on a session row open it and leave the keys in its pane, as a click on a tab row
does. A click on a project row still folds it and gives the sidebar the keys
(`ScreenHit::ToggleProj` in `screen/behaviour.rs`). The owner has not said which they want.

## 34. Choose the alert sounds (feature, `client`)

#275 added a switch for each sound and a volume. The two sounds are still generated and fixed; a
config that set `notify.sound_needs` or `notify.sound_done` to a sound's name, from before they
were removed, still loads and is ignored.


## Added 2026-11-05 (afternoon): found testing the v0.65.0 desktop app on Linux

**Start here.** Where things stand. The order to work in is the *Game plan* at the top of this file.

**State when the owner left work (2:55 PM ET, 2026-11-05)**
- v0.65.0 is released. The terminal app in it is fine. The desktop app is not usable (ticket 35).
- The desktop updater no longer offers 0.65.0: `latest.json` on the v0.65.0 release is the v0.64.0
  manifest, and the release page opens with a warning. The 0.65.0 desktop installers are still
  attached to the release. To remove them:
  `for f in $(gh release view v0.65.0 --json assets --jq '.assets[].name' | grep '^Maitty'); do gh release delete-asset v0.65.0 "$f" -y; done`
- `~/.claude/settings.json` on the work machine holds dead Maitty hooks (ticket 36). A script that
  repairs it was written but not run (`fix-maitty-hooks.py`, in that Claude session's scratchpad
  on the work machine): it keeps one Maitty hook an event and points it at `~/.local/bin/maitty`.
  Until then every Claude session there prints hook errors.
- A v0.65.0 desktop window was left open on the work machine's second monitor. Closing it makes
  the hook path it wrote dead as well.
- Pull request #271 is open and redundant (see *Housekeeping* above).

**The bugs, worst first**

| Ticket | Bug | Proposed fix | State |
| --- | --- | --- | --- |
| 35 | The desktop terminal runs one action behind; output does not reach the window until it gets an event | Find which side drops the frame, then carry output on a local socket instead of a Tauri channel | measured, not fixed |
| 40 | Opening a pane types old terminal replies into it as input | Drop what xterm sends while a view writes history or a replay | found, not fixed |
| 36 | Agent hooks point into the AppImage's temporary folder and break every Claude session | Keep the daemon at a fixed path and name that; replace old entries | cause known, not fixed |
| 37 | Restart and restore cannot stop a daemon the desktop app started | Accept the bundled daemon's name | fixed on a branch, not merged |
| 38 | Settings › Performance lists no daemons | Likely the same name test as 37 | not looked at |
| 39 | The desktop UI differs from the design, breaks on small windows, and has elements the owner does not want | Review every screen with the owner: keep, change or cut | not started |

**What to do next, in order**
1. Ticket 35. Open [PERFORMANCE](PERFORMANCE.md), *Input latency*, and follow *The fix, in order*.
   The measurement that shows the bug needs no one at the keyboard and is written out there.
2. Ticket 40, in the same pull request as 35 or right after: it is in the same file.
3. Ticket 37: finish the branch.
4. Ticket 36, then 38.
5. Ticket 39 with the owner, only after 35.
6. Release the desktop app again only with the **Key to echo** figure from a real Linux window in
   the pull request (*Budgets* in PERFORMANCE).

**If the lag does not show on another machine**, say so before changing anything: it was measured
on Hyprland with an NVIDIA RTX 4060 and WebKitGTK, and a machine without it points at that setup.

Found by the owner on the first run of the v0.65.0 AppImage (Arch Linux, Hyprland, NVIDIA RTX 4060,
WebKitGTK). The tickets follow, in number order.

## 35. The desktop terminal runs one action behind (bug, `client`)

Typing in a pane shows late, and a menu in Claude Code shows the choice before the last key: Enter
then acts on a row the screen did not show. Clicking away and back brings the pane up to date.

**Evidence**
- The overlay read 63 fps, 0% CPU and 121 B/s of output arriving while the pane still showed the old
  screen. Settings › Performance at rest: 63 frames/s, 0 slow, WebGL 2 on both panes, 1 animation.
- Starting with `WEBKIT_DISABLE_DMABUF_RENDERER=1` changed nothing, so it is not how frames reach
  the driver.
- `Cargo.lock` and `apps/gui/package.json` did not change between v0.64.0 and v0.65.0 apart from the
  new workspace crates, so no library moved.
- The path a pane's bytes take is new in v0.65.0: `apps/gui/src-tauri/src/hub.rs` and `streams.rs`
  are new, `terminal_stream.rs` and `TerminalView.tsx` were rewritten around them (about 2,700 lines
  across those and `api/daemon.ts`, `maitty-machines/src/connections.rs`).

**Not yet known.** Whether the wait is on the way in (the key reaching the daemon) or on the way
out (the output reaching xterm). Read so far without finding it: the host loop in
`terminal_stream.rs` (`due` and `timeout_at` around `hub.next()`), `Streams::output` and
`flush_into`, `Hub::next`, `Connections::recv_burst`, and `enqueue` / `flushOnFrame` / `apply` in
`TerminalView.tsx`.

**What**
1. Reproduce outside the owner's window: a dev build on a nested socket (`MAITTY_SOCKET=<name>`).
2. Time one key at each hop: xterm `onData`, the `terminal_input` command, the daemon's `Output`,
   `Streams::flush`, the channel message in the window, `emulator.write`. The hop where the time
   goes is the bug.
3. Check first: a frame of 1,024 bytes or more goes to the window through Tauri's channel fetch,
   smaller ones by direct eval, and the window delivers channel messages in order, so one slow
   fetch holds every frame behind it.
4. Fix it there, with a test that fails before the fix.

**Acceptance**
- A key's echo shows without another key or a refocus, in a shell and in Claude Code's menus.
- A test covers the hop that was wrong.

**Update, same afternoon.** Measured: the wait is between the host's `channel.send` and the
window's `onmessage`, and it ends when the window gets an event. The figures, the order of the fix
and how to run the measurement again are in [PERFORMANCE](PERFORMANCE.md), *Input latency*. Start
there, at *The fix, in order*.

**Seen on Windows too (2026-11-05, evening).** The owner ran the 0.65.0 desktop app on Windows 11
(WebView2): a subtle lag, not as bad as on Linux, after a shorter look, so not certain. If it holds,
the cause is not Linux only, and step 1 of the fix can be run on Windows.

## 40. Replayed output makes the pane answer old queries as typed input (bug, `client`)

After the desktop window opened, a shell's input line held `11;rgb:0505/0d0d/1515` and `1;2c`
several times: the replies to an OSC 11 colour query and a device-attributes query. A view writes
a pane's history or replay into xterm, xterm answers every query in it, and the answer leaves
through `onData` as input to whatever is in the pane now. In a menu those are keys nobody pressed.

**What**
- While a view writes history, a replay or a kept screen, drop what xterm sends through `onData`
  and `onBinary`; pass input again from the first live chunk.
- The OSC 10 and 11 answers `TerminalView.tsx` writes itself follow the same rule.

**Acceptance**
- A test: a replay holding `ESC [ c` and `ESC ] 11 ; ? ESC \` sends no input; the same bytes
  arriving live are answered once.
- Opening and closing the window twice leaves a shell's input line as it was.

## 36. Agent hooks point into the AppImage's temporary mount (bug, `agents`, `client`)

The desktop app's daemon installs the Claude Code hooks and status line with its own path, which
for an AppImage is `/tmp/.mount_Maitty<random>/usr/bin/maitty-gui-daemon`. The folder changes on
every launch and is gone when the app exits. `~/.claude/settings.json` is global, so every Claude
session on the machine runs those hooks, in the terminal app too.

**Seen**
- Every hook event listed twice, one path dead: `SessionStart:startup hook error … Transport
  endpoint is not connected`, the same on `UserPromptSubmit`, `Stop` and `SessionEnd`.
- Old entries are never replaced: `runs_our_hook` (`crates/adapters/maitty-provider-claude/src/hooks.rs`)
  takes the program's file stem and looks for `maitty`, and the bundled daemon's is
  `maitty-gui-daemon`. The groups in the file carry no `_maitty` tag either.
- Once the window closes, no hook path is left that works, and no session can report its state.

**What (owner to confirm the first point)**
- Keep a copy of the bundled daemon at a fixed path under the app's data folder
  (`~/.local/share/maitty/`), refreshed when the version changes, and name that path in the hooks
  and the status line.
- Treat the bundled daemon's name as this app's when replacing hook groups, so a launch replaces
  the old groups and does not add to them. Remove groups whose program is under `/tmp/.mount_`.
- The same for every other agent `integrate` writes to.

**Acceptance**
- After two launches and a quit, `~/.claude/settings.json` has one Maitty group per event, its
  program exists, and a Claude session started with the app closed reports no hook error.

## 37. Restart and restore cannot stop a daemon the desktop app started (bug, `client`)

On Linux `is_app_daemon` (`crates/maitty-desktop-client/src/lib.rs`) accepted only a program whose
name ends in `maitty`; the bundled daemon is `maitty-gui-daemon`, so the button answered "couldn't
identify the local Maitty (maitty) daemon process". Windows and macOS skip the check.

**State.** Fixed in one commit on the branch `fix/desktop-upgrade-path` (not on `dev`):
`is_daemon_command_line` accepts the terminal app's program or the bundled one, started with
`daemon`, with a test. That crate's tests and clippy pass; the full gate has not been run. To
finish: rebase the branch on `dev`, run the gate, open the pull request.

## 38. Settings › Performance lists no daemons (bug, `client`)

The Daemons row read `0 · none` with seven daemons running on the machine, the app's own among
them. Likely the same name test as ticket 37. Confirm and fix with it.

## 39. Desktop UI review against the design, screen by screen (chore, `client`, `docs`)

The owner's verdict on the v0.65.0 desktop app: the design is wrong in places, it does not work on
smaller screens, the remote machines section is not the design, and there are elements that should
not be there. Fixing these one complaint at a time is how it drifted, so review it whole first.

**Source of truth (owner, 2026-11-05, evening).** The owner's view is that the app was built from
parts of the v18 handoff and does not match it as a whole. The owner supplied a revised v18 the
same evening and confirmed it is the design to build from. It is in
`docs/features/chat-gui/design-handoff/v18/` (the README and the prototype changed; D.6 to D.9 of
the README are new), with its screens in `.maitty/references/`. That settles step 1 below. A
screen counts as done only when a screenshot of the build sits beside the design's.

**What**
1. Agree the source of truth with the owner before anything else. `.maitty/references/` is empty,
   so the candidates are `docs/DESIGN.md`, the v17 and v18 handoffs under `docs/features/`, and for
   the machines section the terminal app's Local and Remote sidebar (ADR 0050). Ask which; put the
   reference images in `.maitty/references/` so every agent opens the same ones.
2. Walk every screen and panel with the owner and mark each element **keep**, **change** or
   **cut**. Start with the list under *Design changes the owner asked for* below.
3. Decide the smallest window the app supports and what gives way below each width (sidebar to
   rail, pane header buttons into a menu, two panes to one). Record the widths in `docs/UI.md`.
4. Turn the marks into tickets, cuts first: removing what is not wanted is the cheapest way to
   make the rest fit.

**Acceptance**
- One table in `docs/DESIGN.md` or a linked doc: screen, element, verdict, ticket.
- The references are in `.maitty/references/`.
- No UI ticket is started before the owner has marked its screen.

**Notes**
- Do this after ticket 35. A layout cannot be judged in a window that runs a step behind.

## Design changes the owner asked for on 2026-11-05 (to ticket)

- **Small screens.** The app does not work at smaller window sizes.

- **Chat on any session.** Switching a pane to chat should give an empty chat that can be typed
  into, for a `claude` typed into a shell too. ADR 0052 makes those read-only, and the pane showed
  "The daemon has no conversation for this claude session". Needs the ADR changed.
- **Remove Overview** from the sidebar.
- **Remove "New agent on a branch"** from the sidebar header.
- **The remote machines section** of the sidebar is not the original design: compare with the
  reference designs before changing it.
- **The block cursor hides the letter under it.** May be ticket 35 (the letter not drawn yet);
  check again once 35 is fixed.
- **Claude Code is slow to start** in a pane. May be 35 or 36; check again once both are fixed.

Added the same evening, from the 0.65.0 desktop app on Windows:

- **The tab bar is too tall:** too much padding around the tabs.
- **The title bar needs a colour of its own,** apart from the app's ground.
- **Usage moves into Settings.** Its icon in the sidebar footer goes.
- **The terminal and its panel are one surface.** The terminal's ground is a different colour from
  the panel around it and reads as a box inside a card.
- **Chat is the same session drawn another way.** Switching a pane between terminal and chat is
  seamless: the same data, from the same daemon, the same Claude or Codex subscription. Only how
  it is drawn changes. This is *Chat on any session* above, stated in full. Seen: chat on a
  `claude` typed into a shell showed "Reading… Reconnecting to the daemon. What is shown may be out
  of date.", then "Reading the conversation…" and a read-only composer.

Where the revised handoff stands on this list, from its README:
- **In it:** Overview and New agent on a branch are removed (D.6, ticket 48); usage is in Settings
  (D.7, ticket 49).
- **The tab bar:** section 3.3 gives a 36px tab in a pill tray. Compare the build with that
  before changing anything; the build may be off the design.
- **Not found in its new sections:** the title bar's colour, the terminal and its panel as one
  surface, chat on any session, small windows, the remote machines section. Look in the prototype
  (`Maitty Desktop v18.dc.html`) and the older sections; where they are silent, ask the owner.

## Added 2026-11-05 (evening): found while fixing the intermittent Windows tests

Each was seen or read during tickets 26, 28 and 29 and left alone as outside them.

## 41. The desktop hub lets a removed machine come back (bug, `client`)

The shape ticket 28 fixed in the terminal app is also in `apps/gui/src-tauri/src/hub.rs`:
`on_link_event` takes a `Line` or a `State` with no check after `Command::Remove`. Read from the
code, not tested. Dropping a stopped machine's events in `Links::next` (`maitty-machines`) would
cover both apps.

## 42. Other test sandboxes share the real profile's shell history (chore, `tooling`, `windows`)

`apps/tui/tests/fake_ssh.rs`, `remote_status_tests.rs` and the in-process daemon tests set up their
own environment, so their shells can read each other's history and write test lines into the
developer's real PowerShell history. Do with ticket 26, once it has a lever that works.

## 43. `tests/fake_ssh.rs` can leave a daemon running (bug, `tooling`, `windows`)

Seen once: a left-over daemon (`fakessh-<pid>-<n>`) held `target\debug\maitty.exe` open, and 40
desktop tests then failed with `building the maitty binary failed`.

## 44. Intermittent hub test failures under load (bug, `client`, `windows`)

With other test runs on the same machine: `end_to_end.rs:294`, `one dial for the line, one to
redial at once`, left 1, right 2 (2 of 25 runs), and once each
`killing_the_ssh_process_shows_reconnecting_then_connected_and_the_pane_comes_again` (the same
message) and `a_different_daemon_at_a_known_name_is_turned_away_and_says_nothing_that_counts`
(`retry dials a parked machine: Elapsed(())`). Each passed alone. Guess, not confirmed: the shim
writes its `--calls` line after it starts the proxy, so the reconnect can be seen first.

## 45. Intermittent conversation test failure under load (bug, `daemon`, `windows`)

Once in a heavily loaded full run:
`daemon::conversation::tests::user_input_requests_use_exact_ids_and_validated_ephemeral_answers`,
`conversation\tests.rs:880`, `provider process tree survived disposal`. Passed 3 of 3 alone.

## 46. `same_path` compares paths as text (bug, `daemon`, `windows`)

Read from the code while fixing ticket 29, not exercised: a made worktree recorded under an 8.3
spelling would never be removed automatically once its pane reports the long name
(`cleanup_worktrees`), and `runs_in` would miss such a pane.

## 47. The gate's cross-checks cannot build the desktop crate on Windows (chore, `docs`)

On a Windows host `cargo check --target x86_64-unknown-linux-gnu` and
`--target aarch64-apple-darwin` both fail as written, in the desktop crate's system dependencies
(`libdbus-sys`, `objc2-exception-helper`). With `--workspace --exclude maitty-gui` both pass.
[ENVIRONMENT](ENVIRONMENT.md) says so for the macOS target only. **owner**: whether the gate in
`AGENTS.md` names the excluded form.

## Housekeeping from 2026-11-05, evening (not tickets)

- `docs/COMMIT-STANDARDS.md`, *Project-specific: tooling*, still says there is no release tooling.
  `scripts/release.sh` is that tooling now. **owner** to change the line.

## Added 2026-11-05 (evening): from the revised v18 handoff and the owner

Tickets 48 to 50 are the new sections of
`docs/features/chat-gui/design-handoff/v18/README.md`. The handoff is the specification; these
entries only say what to do and how to tell it is done.

## 48. Remove what the revised v18 removes (chore, `client`)

README D.6. Out of the desktop app, with their shortcuts, rail buttons and command palette
entries:
- the Overview screen (Ctrl/Cmd+J, jump to the next session needing you, stays);
- New agent on a branch (sessions still show their branch under the name);
- the session side panel (Checklist, Agents, Changes, Files, Artifacts) and its toggle. Changes,
  Files, Branches, History and References stay in the activity bar;
- step details in sidebar rows: the current checklist item, the progress bar, the sub-step line;
- project colours (also ticket 9).

**Acceptance:** nothing in `apps/gui/src` still refers to Overview, New agent on a branch or the
session panel; `pnpm test`, `pnpm lint` and `pnpm build` pass; `docs/DESIGN.md` and `docs/UI.md`
no longer describe them; screenshots beside the design's. Do this first: it makes the rest fit.

## 49. Usage and cost moves into Settings (feature, `client`)

README D.7. Settings, Workspace, **Usage & cost**, above Notifications, as one Settings card with
the contents of C.7 unchanged. Ctrl/Cmd+Shift+U and the command palette open Settings on that
page. Remove the bar-chart button in the sidebar footer (expanded and rail) and the full-screen
usage view.

## 50. Sidebar sessions keep their place (feature, `client`)

README D.8. Sessions in a project always show in creation order; status, pin and focus never
reorder them. The focused session is the full card in its own position, and clicking a compact
row grows it into the card in place. Remove the Settings option "Main row shows". The "N more"
cap applies only to rows after the card.

## 51. Updating moves into the Menu (change, `client`; terminal app)

Owner idea, 2026-11-05. Today the terminal app offers an update in three places: the button in
the sidebar footer, the banner at the top of Settings, and the Version row under it. Put it in
one: the Menu gets an entry that reads **Check for updates**, or **Update to X.Y.Z** when one is
out. Remove the sidebar button and the Settings banner. Settings keeps the switch for checking
and shows the version as plain information.

**owner:** with the sidebar button gone nothing says an update is out until the Menu is opened.
Either a small mark on the Menu button when one is out, or nothing. Not answered yet.

## 52. No way to start a session on another machine from the terminal app (bug, `client`, `ssh`)

Found by the owner on 2026-11-05, right after updating a machine from the sidebar. A machine with
no sessions is one line in the sidebar, and its right-click menu has Reconnect, Update…, Edit… and
Remove: nothing starts a session there. The guide says adding a machine lets you "create the first
session from this computer", but the only way is the command line:
`maitty new --on <machine> <folder> -- claude`.

**What**
- **New session…** on a machine's right-click menu and in the Machines dialog, asking for the
  folder on that machine and what to run, as New does for this computer.
- Check whether New can aim at a machine once it has sessions, and make the two agree.
- The guide's *Machines* section says how.

**Acceptance:** from a machine with no sessions, a session is started in a chosen folder without
leaving the app, with a test on the scripted-ssh fixtures.

## Added 2026-11-06: phase 5, borrowed from a read of GG Coder

From a comparison with https://github.com/KenKaiii/gg-framework. Ideas only: its code is AGPL-3.0
and is not copied. ADRs are named by title as well as number, because the ADR folder is being
renumbered. A ticket that says `Blocked by N` is not started until ticket N is merged.

## P5-1. Stopping the daemon leaves provider processes running on Linux and macOS (bug, `daemon`)

A structured conversation's provider (Codex app-server, Claude stream-json) runs in its own process
group and is stopped only by `OwnedChild::stop` or its `Drop`
(`crates/maitty-core/src/daemon/conversation/process.rs`). `KillServer` stops the panes and
then calls `std::process::exit(0)` without stopping conversations (the `KillServer` arm in `daemon/commands.rs`),
so no destructor runs. The first line of `daemon/contain.rs` promises "Every process the daemon starts goes when
the daemon goes, however it goes". On Windows the job object keeps that promise; elsewhere only
panes are covered. A provider left behind is a second writer for a conversation that restore marks
recovery-required.

**Reproduction:** a daemon with a conversation on the fake provider
(`conversation/fixtures/fake_provider.rs`, which starts a child that parks with no stdin) is sent
`KillServer`. The two pids in the fixture's `pids` file are still running. Expected: neither is.

**What**
- `KillServer` stops every conversation's process tree before the daemon exits.
- Not the parent-death signal a check uses (`PR_SET_PDEATHSIG` in `daemon/check.rs`): it is tied to
  the thread that started the process, and a provider is started from a `spawn_blocking` thread
  (`OwnedChild::spawn`) that ends after about ten idle seconds, which would kill a healthy
  provider. A provider that outlives a daemon that died is ticket P5-2's.

**Acceptance**
- A new test fails before the fix and passes after it, unedited: a stand-in daemon (the test binary
  re-run, as `a_check_dies_with_the_daemon_that_started_it` does in `daemon/check.rs`) is sent
  `KillServer`, and the fake provider and its child are gone within 5 s.
- `docs/SECURITY.md` says what happens to a provider when the daemon goes, per platform, beside its
  lines for checks ("If the daemon dies, a check dies with it").
- The gate passes.

**Not this ticket:** a daemon that dies without `KillServer` (SIGKILL, a crash) on Linux and macOS:
ticket 54. Windows is already covered by its job object.

## P5-2. A new daemon stops the provider processes an old one left (bug, `daemon`)

Blocked by P5-1

After a SIGKILL or a crash nothing in the dead daemon can run. A provider's children keep its
process group, and on macOS the provider itself survives. Nothing on disk records them: the saved
conversation holds no pid (`Conversation` in `crates/maitty-shared/src/conversation.rs`), and restore only
marks the record recovery-required (`restore_startup` in `daemon/restore.rs`).

**What** (Linux and macOS; Windows does nothing here)
- When a provider starts, the daemon records its process group and the group leader's start time
  in a file in the data folder, named for the socket as the pid file is (`<socket id>.providers`),
  written atomically (`write_atomic`). The entry goes when the provider is stopped and reaped.
- At start, once this daemon holds the socket and before restore, it kills every recorded group
  that still has members, then empties the file. A group is skipped when a process with the
  group's id is alive with a start time other than the recorded one: that pid was reused.
- Each kill is one info line in `daemon.log`: the group and how many processes.

**Acceptance**
- A test on Linux and macOS: a stand-in daemon starts the fake provider and is killed with SIGKILL;
  a second start on the same data folder leaves neither the provider nor its child running, and
  the file empty.
- A test with a recorded group whose id now belongs to an unrelated live process: nothing is
  killed.
- A clean stop leaves no entry. The saved session gains no pid.
- `docs/DATABASE.md` lists the file. The gate passes.

**Watch out:** a single-instance lock for the daemon is coming on branch `lucky-hare` (`ipc.rs`,
`daemon/mod.rs`). The sweep runs only after this daemon is the one daemon for its socket, or it
would kill a live daemon's providers.

## P5-3. A table in a chat message is drawn as a table (feature, `client`)

The chat's markdown reader has no tables (the list of what is supported at the top of `apps/gui/src/features/chat/markdown.ts`), so an
agent's table shows as lines of pipes.

**What**
- `parseMarkdown` gains a `table` block: a header row, a delimiter row (`|---|`, with `:` for
  alignment) and body rows. A pipe line with no delimiter row under it stays a paragraph. Cells
  hold inline markdown through `parseInline`; an escaped pipe is a pipe.
- `components/Markdown.tsx` draws a real table (`<table>`, `<th scope="col">`) as a hairline grid,
  the look the owner chose on 2026-11-06: existing tokens only (`border-line2`, `text-fg`,
  `text-dim`), no shadow. A table wider than the message scrolls sideways inside its own named,
  focusable region; the message itself never scrolls sideways.
- A message that starts with a table: `components/ChatItems.tsx` draws the first paragraph (`lead`) with
  `InlineMarkdown`, which would show the pipes. It is drawn as a table there too.

**Acceptance**
- `markdown.test.ts`: a table parses to header, alignments and rows; a pipe line with no delimiter
  row stays a paragraph; cells with code, bold and a link parse; a `<script>` in a cell is text; a
  cell over `INLINE_PARSE_LIMIT` is plain text.
- A component test: a wide table sits in a named scroll region that Tab reaches.
- Screenshots in a dark and a light theme, at 1440 px and at the 720 px minimum width, go to the
  owner in the pull request.
- `docs/DESIGN.md` describes the chat table. `pnpm test`, `pnpm lint`, `pnpm build` and the gate pass.

**Not this ticket:** sorting, copying a table, resizing columns, tables in the terminal app's
markdown preview.

## P5-4. Half-arrived markdown does not flash while a reply streams (feature, `client`)

Blocked by P5-3

The parser reads every state of a streaming reply as a finished document. An opened `**` shows as
two asterisks until its closer arrives, then the text jumps to bold; with ticket P5-3 a table's
header shows as a line of pipes until its delimiter row arrives.

**What**, only for a message whose status is `running` (the `agent` row in `chatRows.ts`)
- In the paragraph still being written, an unclosed `**`, `*`, `_`, backtick or `[label](` is
  read as if it closed at the end of the text, so the text shows in its final form.
- A table is not drawn until its delimiter row is complete for every header column.
- A last line that is so far only a marker (`-`, `1.`, `#`, `>`) is held back until it has text.
- Nothing is changed inside an open code fence.
- A completed message is parsed as today: a mark that never closed shows as it was written.
- One pure function in `markdown.ts`: the text and whether it is still streaming in, the text to
  parse out.

**Acceptance**
- `markdown.test.ts`: each case above in its streaming and its completed form; for a message whose
  marks all close, no prefix of it shows a raw mark that the whole message shows as formatting;
  text inside an open fence is untouched.
- A chat test: a running row is parsed as streaming and a completed row is not.
- `pnpm test`, `pnpm lint`, `pnpm build` and the gate pass.

**Not this ticket:** fading words in, pacing the text, parsing only the last block, folding long
code. Nothing has measured a need for them.

## P5-5. The chat stops following the newest message as soon as the reader scrolls up (change, `client`)

`VirtualList` follows the newest row while the view is within 48 px of the bottom
(`isAtBottom` in `components/windowing.ts`, `onScroll` in `components/VirtualList.tsx`). While a reply streams, a
reader who scrolls up a little is pulled back down by the next growth (the layout effect on `layout.total`),
and a wheel move can be undone by growth that lands before its scroll event. No decision made it
this way; it is how the list was first written.

**What**
- Any move up by the reader (wheel, keys, dragging the bar, touch) stops following at once, however
  small. A wheel-up stops it before its scroll event arrives.
- Following starts again only when the reader moves down to within `FOLLOW_THRESHOLD_PX` of the
  bottom, or uses **Jump to the latest message**. The button shows whenever the list is not
  following.
- A scroll the list caused itself (its own write to `scrollTop`, a measured row, a resized pane)
  never changes following.
- The rule is a pure function in `windowing.ts`.

**Acceptance**
- `windowing.test.ts`: up by 1 px inside the threshold stops following; down to within the
  threshold resumes it; a scroll the list caused changes nothing; a link to a row (`scrollTarget`)
  still stops following.
- A `VirtualList` test: while not following, a growing last row does not move `scrollTop`, and the
  button is shown.
- `pnpm test`, `pnpm lint`, `pnpm build` and the gate pass.

**Not this ticket:** keeping the view still when rows above it are measured; a count of new
messages on the button.

## P5-6. `maitty hook` reports a fingerprint of each tool call (feature, `daemon`)

Intent: [a working agent that repeats itself says so](features/repeating-agent/intent.md).

The daemon knows that a tool ran, not which: `ClientMsg::Hook` carries the event name only
(`ClientMsg::Hook` in `crates/maitty-shared/src/protocol.rs`), though the agent gives `maitty hook` the tool's
name and input on stdin.

**What**
- `ClientMsg::Hook` gains `#[serde(default)] call: Option<String>`: 16 hex characters, the first
  8 bytes of SHA-256 over the tool's name and its input as `serde_json` writes it (`sha2` is
  already a dependency). `hook_message` (`crates/maitty-terminal-client/src/cli.rs`)
  fills it for a tool-start event read from stdin: `PreToolUse` (Claude, Qwen, Codex) and
  `BeforeTool` (Gemini). It is `None` for every other event and agent.
- The tool's name and input are not sent, kept or logged.
- The daemon checks the value at the boundary (16 hex characters, else dropped) and does nothing
  more with it yet.
- No `PROTOCOL_VERSION` bump: a test decodes a hook from before the field, as the existing
  "a hook from before the field" test in `protocol.rs` does.

**Acceptance**
- Tests: the same name and input give the same fingerprint; another input gives another; an event
  with no tool gives none; a hook without the field decodes; a value that is not 16 hex characters
  is dropped.
- `docs/API_PATTERNS.md` lists the field; `docs/SECURITY.md` says the report carries a fingerprint
  and never the input. The gate passes.

**Not this ticket:** new hook events for any agent (Codex would ask for its hooks to be trusted
again); Kimi, Mastracode, Kilo, OpenCode, Pi and OMP, whose hooks pass no tool input.

## P5-7. The daemon marks a working agent that repeats itself (feature, `daemon`)

Blocked by P5-6

**What**
- Per pane, in memory only, the daemon keeps the fingerprints of the current turn's tool calls. A
  new prompt, leaving working, or the agent exiting clears them.
- An agent is repeating when its newest call has the same fingerprint as the four before it (five
  in a row), or its last eight calls are two fingerprints taking turns. `TermInfo` gains
  `#[serde(default)] repeats: Option<u32>`: how many calls the run has reached, growing while it
  goes on; `None` otherwise. A different call clears it.
- It never changes status, never rings, never enters the Inbox and is not saved with the session.
  `quiet_since` in `daemon/status.rs` is the model.
- Each change is one info line in `daemon.log` with the pane, the agent and the count, and nothing
  about the call.
- From another machine the count is clamped as its other numbers are
  (`clamp_term` in `crates/maitty-machines/src/remote_snapshot.rs`).
- No `PROTOCOL_VERSION` bump: a test decodes a pane from before the field.

**Acceptance**
- `daemon/logic_tests.rs`, beside the quiet tests (`silent_working_agent` to `going_quiet_rings_no_alert`): four the same is nothing; five is
  5 and a sixth is 6; a different call clears it; eight calls of two taking turns is 8; a new
  prompt clears it; status stays working; nothing rings.
- A decode test for the old shape and a clamp test for a remote snapshot. The gate passes.

**Not this ticket:** failed calls in a row (Later); stopping or nudging the agent.

## P5-8. The terminal app, `ls`, `maitty_list` and `wait` say when an agent is repeating itself (feature, `client`)

Blocked by P5-7

**What:** everywhere quiet is shown, except the Inbox.
- The sidebar row's second line reads `repeating 6x` in the muted colour, beside `N agents`
  (`status_words` in `crates/maitty-terminal-client/src/client/screen/side.rs`).
- `ls` reads `claude: working · repeating 6x` (`agent_tag` in `cli.rs`); `ls --json` has
  `repeats` while working (`pane_json`); `maitty_list` shows it on the row
  (where `cli/mcp.rs` adds `quiet_since`); `wait` and `maitty_wait` say `still working, but repeating the same call
  (6x)` when they time out (the `Waited::TimedOut` messages in `cli.rs` and `cli/mcp.rs`).
- Nothing in the Inbox, no popup, no sound, nothing in the desktop app.

**Acceptance**
- A drawn-frame test like `a_quiet_agent_says_how_long_on_its_row` (`side.rs`), at 80x24.
- CLI and MCP tests beside the quiet ones (`ls_says_how_long_a_working_agent_has_been_quiet` in `cli.rs`, and its
  twin in `cli/mcp.rs`); a hostile count from another machine beside the hostile `quiet_since` in
  `machine_group_tests.rs`.
- `docs/agent-status.md`, `docs/UI.md` (card row 2 and the working row of the status table),
  `docs/API_PATTERNS.md` and `docs/guide.md` describe it where they describe quiet. The gate passes.

## P5-9. The built Linux package is launched on every push to `dev` (chore, `tooling`)

CI builds the desktop installers on each push to `dev` and never starts one
(the four `desktop package` jobs in `.github/workflows/ci.yml`). A package that builds but cannot start, or cannot find its
bundled daemon, is only found by hand: the package checks of 2026-10-29 recorded in `docs/ROADMAP.md`
(phase 37) were all manual.

**What**
- `scripts/smoke-desktop-linux.sh <deb>` unpacks the `.deb` with `dpkg-deb -x` (nothing is
  installed), starts the app under `xvfb-run` and `dbus-run-session` with `MAITTY_DATA_DIR`,
  `MAITTY_CONFIG`, `MAITTY_SOCKET` and `XDG_RUNTIME_DIR` set to fresh temporary values, and waits
  15 s. It passes when the app is still running and `<data folder>/<socket id>.pid` names a live
  process started as `maitty-gui-daemon… daemon`. Pass or fail, it stops both and removes the
  temporary folders; on a failure it prints `daemon.log`.
- The `desktop package (Linux)` job runs it after the build, in a step with `timeout-minutes: 3`.

**Acceptance**
- The step is green on a push to `dev`; the pull request links the run.
- Shown red once: with the bundled daemon removed from the unpacked tree the script exits non-zero.
  The pull request quotes that run.
- The script leaves no process behind, and checks that it did not.
- Pull request checks are unchanged: the package jobs run on a push only (the comment at the top
  of `ci.yml`).
- `docs/TESTING.md` says what the smoke proves (the package starts and finds its daemon) and what
  it does not (nobody looks at the window). The gate passes.

**Not this ticket:** the rpm, AppImage, Arch and macOS packages; clicking anything; the
**Key to echo** figure.

## P5-10. The built Windows package is launched on every push to `dev` (chore, `tooling`, `windows`)

Blocked by P5-9

The same rule on Windows. `scripts/smoke-desktop-windows.ps1 <msi>` unpacks the MSI with
`msiexec /a` (an admin image, nothing installed, as was done by hand on 2026-10-29), starts the
exe with the same isolated variables, applies the same pass rule and stops both. The
`desktop package (Windows)` job runs it as one command, so the script's exit code is the step's
(a PowerShell step of several lines reports only its last line).

**Acceptance:** as ticket P5-9, on `windows-latest`: green on a push to `dev`, shown red once with
the bundled daemon removed, no process left behind, `docs/TESTING.md` updated. The gate passes.
