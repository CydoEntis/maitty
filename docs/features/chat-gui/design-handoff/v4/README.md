# Handoff: Maitty Desktop — chat GUI

## Overview
**Scope decision:** the Maitty TUI stays as it is today (minimal, native Codex panes). This handoff is only for the **desktop app**, which offers two views of the same daemon-owned session: a **chat view** (Maitty renders Codex's structured events) and a **terminal view** (the live native Codex PTY). The chat view needs Codex's structured event stream (`codex app-server` JSON-RPC); the terminal view embeds the PTY that `maittyd` already owns. See "Chat ⇄ terminal architecture" below.

Maitty is a terminal multiplexer for running several coding agents at once. This handoff covers its optional desktop GUI (Windows, macOS, Linux), a client of the existing Rust daemon `maittyd`. The GUI lets a developer see which agents need attention, open the right thread, inspect what an agent did (tool steps, diffs, files), answer approvals, and drop into the agent's native terminal. Codex is the first provider; Claude is shown as "not supported yet".

**Product rules that the UI is built around (do not break):**
- `maittyd` owns sessions and history. Closing the GUI never stops an agent. Archiving never stops an agent.
- The TUI stays supported and shares the same sessions. Chat ⇄ Terminal is the *same* session, never a copy.
- Status is shown plainly: Working, Needs you (approval/question), Done, Idle, Disconnected/Unknown, Recovery needed. If the daemon connection is lost, every status becomes "unknown/last known" — never imply progress.
- Provider permission/sandbox choices are shown, never widened. Approval cards show the exact command, cwd, why it's blocked, the agent's reason, and **only the choices the provider offers**.
- Never silently retry a prompt that might have been delivered. "Not delivered" (safe retry) vs "Delivery unknown" (confirm before resend) are distinct states.
- Drafts and selection survive reconnects and restarts.

## About the Design Files
The files in this bundle are **design references created in HTML** — an interactive prototype showing intended look and behavior, not production code. Recreate them in the target codebase's environment (e.g. Tauri/Electron + React with shadcn/ui primitives, as the brief suggests) using its established patterns. The prototype's simulated agents, files, diffs and terminal are fake data; wire the real thing to `maittyd`'s socket protocol.

## Fidelity
**High-fidelity.** Colors, type, spacing, radii, motion and interactions are final. Recreate pixel-accurately. Use shadcn/ui as raw building blocks (Command, Tabs, Collapsible, Resizable, Textarea, ScrollArea, Dialog, DropdownMenu, Switch, Tooltip) but restyle to these tokens — avoid the stock shadcn dashboard look.

## Files
- `Maitty Prototype v4.dc.html` — **the source of truth.** Fully interactive; open in a browser. A "Simulate" button (sidebar footer, prototype-only) triggers disconnect, send failure, unknown delivery, crash, provider unavailable, background notifications, and first launch.
- `reference/Maitty Prototype v2.dc.html` — the previous iteration (separate Pinned / Blocked sidebar sections, no tabs). Kept for comparison only.
- `reference/Maitty Desktop (v1 spec).dc.html` + `MaittySidebar.dc.html` — early spec canvas: screen map, state matrix, a11y notes, tradeoffs. Visuals superseded; behavior notes still apply.
- `reference/Maitty TUI v2 (chat-in-TUI exploration).dc.html` — exploration of a styled TUI; **not** in scope (the TUI is staying as-is).

The prototype is a single "Design Component" file: markup + a logic class (`class Component`) in a `<script data-dc-script>` tag. All data (seed threads, projects, file contents, themes, spinners) lives at the top of that script.

---

## Layout (desktop shell)
- Window: full viewport, min-width 720px. Background `--bg`.
- **Sidebar** 264px, flush on `--bg` (no border). Collapsible (⌘\).
- **Main card**: inset 8px from top/right/bottom (and left when sidebar hidden), background `--card`, 1px `--line` border, radius 12px, overflow hidden. Contains the conversation column and, optionally, the right panel.
- **Right panel** (inside card, left border `--sel`, bg `--panel`): widths — Changes/Files 580px (`min(580px,44vw)`), Files with an open file `min(880px,58vw)`, References 420px, Context 320px. Below 1280px wide the panel overlays the conversation (absolute, right:0, shadow `-20px 0 60px rgba(0,0,0,.55)`), and with a file open it fills the card.
- **Breakpoints**: ≥1280 docked panel · 900–1279 overlay panel · <900 sidebar becomes a 52px **status rail** (icons for every visible thread, in sidebar order) and the full sidebar opens as an overlay drawer.

### Sidebar (top → bottom)
1. 48px row: traffic lights (12px dots, gap 8), spacer, then three 28×28 icon buttons (radius 7, `--muted` → `--fg` on hover): **Search ⌘K**, **New session ⌘N**, **Toggle sidebar ⌘\\**.
2. **PROJECTS** caps label (11px/500, .06em, `--dim`) with a 22×22 **+ Open project (⌘O)** button on the right.
3. Project rows (30px, 13.5px/500, folder icon). The whole row hovers `--raised`; on the right: running spinner / badge (mono 11.5px — "◆ n" amber when something needs you, "n running" accent when collapsed, count otherwise) and a 24×24 **+ New session in <project>** button.
4. Threads indented 17px with a 1px `--sel` guide line. Order inside a project: **blocked first** (needs you / recovery), then pinned, then by last activity (desc). Then a collapsible **"Settled n"** row (done/idle, read, not selected, inactive > 1h).
5. **ARCHIVED n** collapsible at the bottom.
6. Footer: daemon dot + "maittyd · N sessions" (dashed-ring dot + "Reconnecting to maittyd…" when offline), settings gear, Simulate (prototype-only; remove).

There are deliberately **no separate Pinned / Blocked sections** any more — every thread appears exactly once, in its project. Blocked threads are found via their amber row, the project badge, the tab strip, notifications, and ⌘⇧A.

**Thread row**: min-height 30px (26 compact), radius 8, padding 4px 4px 4px 8px, gap 9. Status icon 14px slot → title 13.5px (500 if unread/needs, else 400; `--fg` if selected/loud else `--fg3`-ish `#9AA4AF`) with optional 11.5px live-activity line → unread dot 6px `--accent` → pin glyph → meta 11.5px tabular (elapsed for working, relative time otherwise, "exited" in `--danger`) → ⋯ button (20×20, visible on selected/hover). Selected bg `--sel`; hover `--raised`; press scale .985. Right-click or ⋯ opens the thread menu: Pin/Unpin, Rename (F2, inline input), Mark read/unread, Open in terminal, Archive/Unarchive (blocked while working).

**Status icons (shape-distinct, never color-only):** Working = animated spinner (see Spinners), Needs you = 7px amber diamond (rotated square, radius 1.5, soft pulse ring 2.4s), Done = check (`--ok`), Idle = 7px hollow circle (`--line3`), Recovery = refresh arrow (`--danger`), Unknown/offline = 8px dashed circle.

**Running shimmer**: the **selected** row (and its active tab) gets a slow sheen while its agent is working — `linear-gradient(100deg, transparent 30%, accent 9% 45%, accent 16% 50%, accent 9% 55%, transparent 70%)` at `background-size: 250% 100%`, animated `background-position 150% → −50%` over 2.6s `cubic-bezier(.4,0,.2,1)` infinite. Off when not working, when disconnected, and under Reduce motion. Other running rows keep only their spinner.

**Live activity line** (Settings → Sidebar activity: All / Selected only / Off): shows what a working agent is doing ("Running cargo test -p maitty-pty resize", "Writing a reply…"). Throttled to update at most every 2.5s except the selected thread. Blocked threads always show their reason in amber/red.

### Tab strip (top of the main card, 48px, bottom border `--sel`)
Replaces the old breadcrumb header. Each **open session is a tab**:
- Tab: 30px tall, radius 8, `flex: 0 1 210px`, min-width 110px, padding `0 4px 0 10px`, gap 8, 13px. Contents: 14px status slot (same shape-distinct icons as the sidebar — spinner / amber diamond / check / hollow circle / red "!"), title (ellipsis), close × (20×20, radius 5; visible on the active tab at 80%, on hover for others).
- Active tab: bg `--chip`, inset 1px `--line2` ring, text `--fg`. Inactive: transparent, `--muted`; hover `--raised`. Press scale .97. New tabs pop in (`scale .97→1, .22s`).
- After the tabs: a 28×28 **+ New tab (⌘T)** button (opens the New session dialog).
- Right side (unchanged): `+a −d` changes button (mono 11.5px), Chat/Terminal segmented icon toggle (30×26 buttons in a `--sunken` 9px-radius track), right-panel toggle.
- Tab strip scrolls horizontally when full (scrollbar hidden).

**Tab behavior**
- Clicking a thread in the sidebar opens it as a tab right after the current one (or focuses it if already open).
- Close: × / middle-click / ⌘W. **Closing a tab never stops the agent** — it just leaves the sidebar. The last tab can't be closed.
- ⌃Tab / ⌃⇧Tab cycle; ⌘1–⌘9 jump to tab n.
- Open tab list is persisted per user (prototype: `localStorage['maitty-tabs-v4']`). On first run it opens the selected session plus up to 2 working/blocked ones.

### Conversation column
- Max content width 720px, centered, padding 28px.
- **User message**: right-aligned, max 540px, bg `--chip`, radius 14, padding 10/14, 14px/1.55. Attached images render as 96×72 thumbnails above the bubble.
- **Agent**: small "Codex" label (bot icon, 12.5px `--muted`) at the start of each turn; text 14px/1.65 `--fg-body`, indented 21px. Streaming caret: 2×16px `--accent`, 1s steps blink.
- **Tool group card** (consecutive tool calls fold together): margin-left 21, radius 10, bg `--surface`, 1px `--line` (`--accent-line` while running). 38px header: icon (spinner / stacked-layers / check-circle / x-circle) + label ("Running …" or "N actions completed" or the single tool label) + right meta ("2 edits · 1 command" or timer) + chevron. While running and collapsed, shows the last 3 output lines (mono 11.5px). Expanded: per-step rows (search/edit/terminal icons, +/−, duration) with command output in a `--sunken` mono block.
- **Files changed card** at end of a finished turn: "N files changed +a −d" + "Open diff" button + up to 4 file rows.
- **Approval / question in chat** is a one-line dashed marker ("Waiting for approval · cmd · Answer below ↓"), resolved to "✓ Allowed once" / "✕ Denied". The actionable card lives above the composer (below).
- **Recovery card**: `--danger-soft` bg, `--danger-line` border, radius 12: "Codex exited mid-turn (signal 9)", what's saved, what may not have finished, "nothing will be replayed automatically", [Resume session] [Open terminal].
- **Notices**: centered 12px `--dim` with hairlines either side (e.g. "Stopped by you after 1:42").
- Auto-scroll sticks to bottom only while the user is within 40px of the bottom.

### Approval dock (above the composer)
Card tucked **behind** the composer: wrapper margin `0 14px -30px`, overflow hidden; card bg `--warn-soft`, 1px `--warn-line` (no bottom border), radius 14 14 0 0, padding-bottom 22px, shadow `0 -10px 36px rgba(0,0,0,.28)`. Enters by translating up from 100% (`.42s cubic-bezier(.16,1,.3,1)`), exits by sinking (`.3s`).
- Expanded: title ("Codex wants to run a command outside the sandbox"), wait timer, `$ command` mono block on `--bg`, "in <cwd> · blocked · <why> · Codex says <reason>", buttons right-aligned: Deny and stop ⇧N (ghost), Deny N, Allow for session A (chip), **Allow once Y** (amber fill `--warn`, text `--on-warn`). Footnote "Only the choices Codex offers · also answers it in the TUI".
- Collapsed (toggle chevron; auto when window height < 760px): one row — diamond, "Approval needed", truncated command, Deny / Allow once, timer.
- Typing in the composer while an approval is pending turns Send into "Deny + send" (deny with instructions). Questions use the same dock. When Codex offers choices, they render as numbered option buttons (grid, min 200px columns, radius 10, bg `--bg`, 1px `--warn-line`, hover `--warn` border; 20px mono number badge + title 13px/500 + 11.5px sub); clicking or pressing 1–9 (when not typing) sends that choice. A free-text reply in the composer always works too.
- Disabled (45% opacity) while disconnected.

### Composer
Floating card: bg `--raised`, 1px `--line2` (`#2A3440` focused), radius 16, shadow `0 12px 40px rgba(0,0,0,.3)` (+ `0 0 0 4px` accent 7% ring on focus). Textarea 14px/1.55, auto-grows 1→10 rows. Bottom row: + attach, **model pill** ("● Codex · gpt-5-codex") and **sandbox pill** ("workspace-write · on-request") — both open a read-only popover explaining the value comes from `~/.codex/config.toml` with an "Open config" action — then Stop (32px white circle with square) while running, else Send (32px round, `--accent`, arrow-up; becomes pill with label for "Deny + send"/"Reply"/"Start").
- One-line status strip above the card: "Working for 1:42 · ⌘. to stop · draft saved", "Disconnected. Your draft is saved. Nothing sends until you press send after reconnect.", "Codex CLI not found on PATH", "Resume the session to continue" (composer disabled), etc.
- Footer line: project, branch (mono), hint "↵ send · ⇧↵ newline · / commands".
- **Attachment chips** above the textarea (24px thumb + name + ×). Pasting an image while the composer is focused attaches it and saves it to References.
- **Slash commands** (typing `/` at line start): `/review /files /terminal /stop /new /refs /status /rename`. Popover above composer, ↑↓ to choose, ↵/Tab to run, Esc to dismiss.

### Right panel tabs: Changes · Files · References · Context
- **Changes**: file list (icon, dir dimmed + name, +/−, A/M tag; spinner while Codex is editing), then a file header with prev/next change buttons (and `[` `]` keys) + "n of m", Quote, Open. Diff: mono 12px/20px; added lines `color-mix(accent 8%)` bg with 2px inset accent edge and "+"; removed lines danger 9% bg, 2px danger edge, "−"; hunk headers `--surface`. Syntax-highlighted. Read-only by design (no stage/revert).
- **Files**: tree (folders first, 14px indent per level, changed files marked M/A, editing spinner, unsaved ●) with a filter that searches **names and contents** (shows "L12 matching line · +n more"). Opening a file keeps the tree visible on the left (`min(230px,34%)`, hideable) and opens it in a **tab** (max 8, close ×, middle-click closes, unsaved dot). Viewer modes: Markdown → Preview/Edit; code → View/Edit. Edit = textarea overlaid on a syntax-highlighted `<pre>` with a sticky line-number gutter; Tab indents 2 spaces; ⌘S saves; Revert; warning banner if Codex is currently editing that file. Footer: language · lines · "changed by Codex (+a −d)" · unsaved/saved.
- **References** (per project, shared across its threads): drop zone (drag/drop, ⌘V anywhere, click to browse; accent-tinted while dragging), grid of cards (4:3 thumb, name, WxH · size, Copy-to-clipboard, Insert→attaches to composer, Remove). Click thumbnail → lightbox (contain, pill toolbar: Copy, Insert in message, Close).
- **Context**: read-only Project, Worktree path, Provider + binary path, Model + reasoning, Sandbox + network, Approvals + config path, Session "Owned by maittyd · keeps running if you close this window". Note card: "Sandbox and approvals come from your Codex config. Maitty shows them and never widens them."

### Native terminal view (Chat/Terminal toggle, ⌘`)
Replaces the conversation area with the live Codex PTY, shown as a **pane card**:
- Area padding `10px 12px 12px`, panes in a CSS grid (1 column, or 2 equal `minmax(0,1fr)` columns when split), gap 10.
- Pane: bg `--term`, 1px `--line` border (focused pane in a split: `--accent-line` + `0 0 0 3px --accent-soft` ring), radius 12.
- Pane header 36px, bottom border `--line`: 16px `>_` provider tile, "Codex" (500), worktree path (`--dim`, ellipsis), status at right ("working · 42s" with spinner, "◆ needs you", "! exited", "✓ done", "○ idle"), **Split** button (single pane, window ≥ 1000px) or × close (secondary pane).
- Body: Geist Mono 12.5px/19px, padding 12/14. This is the **real PTY** in production (xterm.js or equivalent bound to the daemon's PTY). The prototype fakes Codex's output in its own style: `• Ran <cmd>` with command tokens colored (first token accent, flags muted, strings ok-green), `└` output lines dimmed, "+ N lines (ctrl+t to expand)", `• Edited path (+a −d)`, approval as Codex's own numbered prompt (1/2/3), "Working (12s • esc to interrupt)", and Codex's footer (model · path · context left). **Do not restyle real Codex output** — Maitty only owns the pane chrome.
- **Split**: opens a second session beside the focused one (prefers a working one). Clicking the secondary pane swaps focus (it becomes the selected tab). × closes the pane; the session keeps running.
- Only ⌘` and ⌘K are intercepted while the terminal has focus; everything else goes to Codex.

### Overlays
- **New session** (⌘N, ⌘T, sidebar +, tab-strip +, project-row +): 540px dialog, top 10vh, radius 16, bg `--pop`, padding 20, gap 18. Sections (caps labels 11px/500 .06em `--dim`):
  - PROJECT: chips (30px, radius 8, folder icon) for each open project + "Open folder…".
  - RUN: 3 equal option cards (radius 9, title 13px/500 + 11.5px `--dim` sub; selected = `--accent-soft` bg + inset `--accent-line` ring): **Codex** "Chat + native terminal" · **Claude** "Not supported yet" (disabled, 45% opacity) · **Shell** "Plain terminal, no agent".
  - WHERE (agents only): **New worktree** "Isolated branch, safe to run in parallel" (disabled for non-git folders) · **Project folder**. Worktree shows an editable mono branch field (prefilled `codex/xxxx`) and the resulting path `~/.maitty/wt/<project>-<branch>`.
  - PERMISSIONS: Read only · Workspace write (default) · Full access "Never asks · use with care". Maps to Codex sandbox flags; never widened silently.
  - FIRST MESSAGE · OPTIONAL: 3-row textarea.
  - Footer: resolved path · codex · sandbox, Cancel, primary (accent) "Start session" / "Start & send" / "Open shell" ⌘↵. Esc cancels.
  - Creating opens the session as a new tab (Shell opens straight into the terminal view) and, if a first message was typed, sends it.
- **Open project** (⌘O, PROJECTS +, "Open folder…", first-launch CTA): 520px command-palette style dialog. Top: mono input "Folder path, or search recent projects". Typing a path (starts with `~`, `/`, `.`) shows an "Open <path>" row; otherwise filters RECENT (name, mono path, "n sessions" or "not open"). Footer: "Any folder works. Git repos also get worktrees per session." + **Browse…** (native folder picker). Opening a project adds it to the sidebar and goes straight into New session for it.
- **Quick switcher ⌘K**: 600px, top 14vh, radius 14, bg `--pop`; threads first (blocked sorted up, status word on the right), then commands. ↑↓, ↵ open, ⌘↵ open in terminal, Esc.
- **Settings ⌘,**: 800×580 dialog, left nav (Appearance, Notifications, Daemon & providers, Keyboard). Appearance: Working spinner, Themes (built-in + "From Maitty TUI"), Accent, Sidebar activity, Reduce motion, Compact sidebar. Notifications: toast position (4 corners with mini previews, "Send a test"), switches for blocked / finished / sound. Daemon & providers: connection + reconnect, Codex status/path, Claude "not supported yet", config note. Keyboard: full shortcut list.
- **Notifications (toasts)**: default bottom-right (configurable), `position: fixed` 16px from the window edges, 368px wide (max `100vw − 32px`). Card: radius 14, bg `--pop`, 1px border tinted by kind (needs `--warn-line`, recovery `--danger-line`, finished `--line2`), shadow `0 18px 50px rgba(0,0,0,.5)`, padding 12. Layout: 30px icon tile (tinted soft bg + inset ring; amber diamond / ok check / red "!") → header row (kind word in its color 11.5px/500 · project · spacer · age · 20px × dismiss) → title 13.5px/500 → body → action row (26px buttons, radius 7, gap 6).
  - **Needs you · approval**: body is the exact command in a mono `$ cmd` well (bg `--bg`, radius 7). Actions: **Allow once** (amber fill) · Review (chip) · Deny (ghost). Answers the approval without leaving the current thread.
  - **Needs you · question with choices**: question text (2-line clamp) + one button per choice (last/recommended filled amber). Picking one sends it as the reply.
  - **Needs recovery**: message + Resume (chip) · Open (ghost).
  - **Finished**: "N files changed +a −d" (mono, accent/danger) + Open (chip) · View diff (ghost, opens Changes). 2px `--ok` progress bar along the bottom shrinking over 9s, then auto-dismiss.
  - Clicking the card body opens the thread. Approvals, questions and recovery persist until acted on or dismissed. Only for threads you aren't viewing.
  - **Stacking**: newest card in front; older ones collapse behind it to 14px slivers, scaled `1 − 0.05·i`, content hidden (max 3 visible). Hovering the stack fans them out (gap 0 → 8px, `.3s cubic-bezier(.2,.8,.2,1)`) and **pauses** the finished-toast timer. "+n more · Clear all" pill beyond 4.
  - Slide in 16px from the docked side `.34s cubic-bezier(.16,1,.3,1)`, slide out `.24s`.
- **Thread menu** (right-click / ⋯): 196px, radius 11, pop animation.
- **Offline pill**: floating at top of the conversation: "Reconnecting to maittyd · attempt n · 4s" + Retry now. Backoff 1→2→4→8s.

### First launch
Centered 480px column: logo tile, "Run your coding agents side by side.", reassurance copy, checklist (maittyd running ✓ version, Codex CLI found ✓ version, Claude: not supported yet ○), primary "Open a project folder ⌘O". "New thread" is unavailable until a project exists.

---

## Interactions & Behavior
- **Press feedback**: every button/row scales to .96 (small) or .985 (wide rows) on :active, `transform .12s ease`. All hover color changes ease `.14s`.
- **Motion** (all disabled by Reduce motion or `prefers-reduced-motion`):
  - Sidebar drawer in/out: margin-left −264→0, `.3s cubic-bezier(.2,.8,.2,1)` / `.26s`.
  - Right panel: docked → grow max-width 0→960 `.32s`; overlay → slide 16px `.28s`; reverse on close.
  - Dialogs (settings, ⌘K, lightbox, menu): fade backdrop + scale .97→1 `.2–.26s cubic-bezier(.2,.8,.2,1)`; exit reverse `.15–.18s`.
  - Approval dock rise/sink as above. Toasts slide 16px from their side `.34s cubic-bezier(.16,1,.3,1)`.
  - Content swaps (thread, chat/terminal, panel tab, settings section) fade + 4px rise `.26s`.
  - Expanding groups/rows: fade + 4px drop `.22s`.
  - Theme switch: View Transitions API cross-fade `.38s` (fallback: temporary global color transitions).
  - Needs-you diamond: 2.4s soft ring pulse. Spinners: frame-based (below).
- **Keyboard** (⌘ = Ctrl on Win/Linux): ⌘K switcher · ⌘N / ⌘T new session · ⌘O open project · ⌘W close tab · ⌃Tab / ⌃⇧Tab next/prev tab · ⌘1–9 jump to tab · ⌘⇧A next blocked thread · ⌥↑/⌥↓ and J/K previous/next thread (in sidebar order) · ⌘` chat⇄terminal · ⌘⇧D changes · ⌘I context · ⌘\ sidebar · ⌘. stop turn · Y/A/N/⇧N answer approval (when not typing) · ⌘, settings · F2 rename · ⌘S save file · [ ] prev/next diff hunk · Esc closes the top overlay, then returns focus to the composer.
- **Sending**: ↵ sends, ⇧↵ newline. Send disabled while a turn runs (draft kept), while disconnected, in recovery, or when the provider is unavailable.
- **Failure states**: Not delivered (danger border, "Retry / Edit / Discard"); Delivery unknown (amber dashed border, "Open terminal / It arrived / Send anyway…" → inline confirm "If it arrived, Codex runs it twice.").
- **Stop**: ends the turn, marks streaming text with " …", running tools "Stopped: …", adds a notice.
- **Recovery**: "Resume session" reattaches via daemon; notice "Session resumed · the interrupted turn was not replayed".

## State Management (what the real app needs)
- From daemon (subscribe): sessions list (id, project, title, status, reason, last-activity ts, unread, branch/worktree), per-session event stream (messages, tool calls with live output, approval/question requests with offered choices, file-change events), connection state.
- Local persisted (per user): open tabs (ordered) + selected session, terminal split partner, per-session composer drafts, attachments-in-progress, pinned/archived flags (or daemon-side if shared with TUI), settled-group open state, theme/accent/spinner/motion/compact/sidebar-activity/toast-position/notification prefs, panel tab + width, open file tabs per project.
- Delivery tracking: each sent prompt needs a client id + daemon ack; no ack before disconnect → "unknown".
- Performance note: the prototype re-renders everything every 60ms to animate spinners and timers. In production, isolate spinners/timers into small self-ticking components (or CSS animations) and drive the rest from daemon events.

## Chat ⇄ terminal architecture (important)
The two views must be the **same session**. Codex can't take input from an interactive TTY and an app-server connection for the same live turn, so the daemon owns one source of truth:
1. **Recommended — chat-first, terminal on demand.** `maittyd` runs Codex via `codex app-server` and turns its events into the chat view (messages, tool calls + output, approval requests with the offered choices, file changes). Switching to the terminal view hands the session to `codex resume <session>` in a PTY **between turns**; switching back re-attaches app-server. History carries over because Codex persists it. While a turn is running, the toggle shows the live terminal read-only or is disabled with a tooltip ("Switch after this turn").
2. Alternative — terminal-first: Codex runs in its PTY as today and the chat view is a read-only render of Codex's session files; approvals and typing happen in the terminal. Simpler, but loses one-key approvals.
Verify before committing: app-server event coverage (live command output, approval choices), protocol stability across Codex versions, and clean `resume` of app-server sessions.
Shell sessions (no agent) only have the terminal view.

## Design Tokens (Midnight — default theme)
| Token | Hex | Use |
|---|---|---|
| --bg | #080B0F | window |
| --card | #0D1117 | main card |
| --panel | #0B0F14 | right panel, settings nav |
| --term | #06080B | terminal |
| --sunken | #0A0E13 | inputs, code wells, tracks |
| --surface | #0F141A | tool cards |
| --surface-hover | #121820 | |
| --raised | #131820 | composer, row hover |
| --pop | #11161D | popovers, toasts |
| --sel | #151B22 | selected row, hairlines |
| --chip | #1A2029 | user bubble, chips, secondary buttons |
| --chip-hover | #222A34 | |
| --line / --line2 / --line3 | #171D25 / #1E252E / #3A4450 | borders |
| --fg / --fg-body / --fg2 / --fg2b / --fg3 | #E6EAEE / #D9DEE3 / #C4CBD2 / #AEB6BF / #9AA4AF | text ramp |
| --muted / --dim / --faint | #8A949F / #77828E / #66707C | secondary text |
| --accent (Sea glass) | #5FD4BF | working, primary, additions |
| --accent-soft / --accent-line | accent 13% / 32% mixed into --card | |
| --on-accent | #071011 | |
| --warn / --warn-hi / --on-warn | #F2B65A / #F6C77A / #14100A | **needs you only** |
| --warn-soft / --warn-line | #14120D / #4A3A1E | |
| --danger / --danger-soft / --danger-line | #F07F78 / #160F11 / #4A2629 | **failure/recovery only** |
| --ok / --ok-soft | #7FA98D / #121A16 | done |
| Syntax | kw #C3A6FF · str #9ED6A8 · num/attr #F2A7C3 · fn #82C8F0 · type #E6C99A · comment = --dim italic · punct = --fg3 | |

**Themes** (full var sets in `THEMES` in the prototype): Midnight (default), Abyss (deep Maitty navy #08121C), Graphite (neutral), Paper (light; accent #0E8F7A, warn #9A5B00, danger #C0392F), plus TUI-derived Tokyo Night, Catppuccin Mocha, Gruvbox Dark, Nord (each with its own accent and syntax palette). TUI themes are derived from 5 inputs (bg, card, fg, dim, accent) via `color-mix` — the same derivation can import all ~170 Maitty TUI themes.
**Accents**: Sea glass #5FD4BF (light #0E8F7A), Azure #6AA8FF (#2563EB), Violet #A99BFF (#6D4AE0), Lime #B5E36B (#4D7C0F). Amber and red never change.

**Typography**: Geist (UI) and Geist Mono (code, paths, shortcuts, numbers in diffs). Scale: 26/22 display (600, −.01em) · 15/14 titles (500–600) · 14 body (1.55–1.65) · 13.5 rows · 12.5 meta/buttons · 11.5 captions · 11 section caps (500, .06em) · mono 12–13.
**Spacing**: 4px base — 4, 6, 8, 10, 12, 14, 16, 20, 24, 28.
**Radii**: 4–6 small controls · 7–8 buttons/rows · 9–10 cards/inputs · 12 main card, dialogs-inner · 14 toasts/quick switcher/dock · 16 composer/settings · 999 round buttons & pills.
**Shadows**: popover `0 18px 50px rgba(0,0,0,.45)` · dialog `0 30–40px 80–100px rgba(0,0,0,.5–.6)` · composer `0 12px 40px rgba(0,0,0,.3)`.
**Focus**: `2px solid --accent`, offset 2 (inset on rows).

## Spinners (match the Maitty TUI)
Frame strings, ms per frame: Braille `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` 80 (default) · Snake `⠏⠛⠹⢸⣰⣤⣆⡇` 90 · Bars `⡀⣀⣄⣤⣦⣶⣷⣿⣷⣶⣦⣤⣄⣀` 85 · Dots `⣾⣽⣻⢿⡿⣟⣯⣷` 80 · Line `-\|/` 110 · Arc `◜◠◝◞◡◟` 100 · Pulse `·•●•` 180 · Ring (CSS border spinner, .8s). Rendered in Geist Mono 13px, `--accent`. **Snake and Bars frames are approximations — replace with the TUI's exact strings.**

## Accessibility
- Every status = icon shape + word in accessible names ("Fix PTY resize race, Working, maitty").
- Dialogs: `role=dialog aria-modal`, Tab focus trap, initial focus inside, focus restored on close.
- Tablists (view toggle, panel tabs, file tabs, settings nav) use roving arrow keys; file tree supports ↑↓ Home End ←→ (collapse/expand/parent).
- A polite live region announces "Finished / Needs you / Needs recovery: <thread>" and command/attachment actions; streamed tokens are not announced.
- Text contrast ≥ 4.5:1 on its surface in every theme (dim/faint were tuned for this).
- Reduce motion stops spinners' frame animation, pulses, slides and theme cross-fades.

## Release scope suggestion (first usable Codex release)
Must: tabs, New session + Open project dialogs, sidebar (projects with blocked-first ordering, settled, pin/rename/archive), conversation with tool groups + streaming, composer with drafts/stop/status, approval dock, all failure/recovery/offline states, chat⇄native terminal, Changes list + diff, Context, ⌘K, notifications, keyboard map, Midnight + Paper themes.
Can follow: terminal split panes, Files editing + tabs + content search, References, TUI theme import, spinner options, slash commands, narrow status rail, toast position.

## Assets
No raster assets. Icons are inline 24-viewBox stroke icons (1.75–2.25 stroke, round caps) in the Lucide style — use `lucide-react` equivalents (Plus, Search, Folder, FolderOpen, Columns2 (split), PanelLeft, PanelRight, MessageCircle, Terminal, GitBranch, GitCompare, Layers, CheckCircle2, XCircle, RefreshCw, Pencil, File, Image, Shield, Bot, Settings, FlaskConical, X, ChevronUp/Down/Left/Right, MoreHorizontal, Pin). Fonts: Geist / Geist Mono (Google Fonts or `geist` npm).
