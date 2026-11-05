# Tickets to file in Plane

Follow-ups from 2026-11-04 that need tickets in the `MAITTY` project. Plane is only reachable
from the home network, so they are parked here. **Delete this file once they are filed.**

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

On the merged tree (948786e) one `cargo test` run (without `--no-fail-fast`, so it stopped early at 832
tests) exited 101 with 1 test failed; two reruns of the whole suite passed (2293 passed, 0 failed) and
the first run's log did not keep the test name. A daemon helper saw
`structured_local_ipc_create_send_read_reconnect_and_exact_identity` fail once under load, but it
passes 6 of 6 in isolation, so the culprit is not confirmed. Next time it happens, keep the log, name
the test, and fix its timing instead of retrying. The three slow `maitty-terminal-client` shell tests
(`remote_status_tests::a_remote_agents_row_is_what_its_daemon_reported_and_follows_its_next_report`,
`cli_machine_tests::send_and_wait_use_the_remote_pane_s_screen`,
`mcp_machine_tests::an_agent_lists_reads_and_sends_to_another_machine_s_panes`) also time out under load.

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
