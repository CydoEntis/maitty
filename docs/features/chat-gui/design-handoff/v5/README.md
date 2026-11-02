# Maitty Desktop v5: Design & Implementation Handoff

**For:** the developer building the Maitty desktop GUI (Tauri + React + TypeScript + shadcn/ui).
**Source of truth for visuals and behavior:** `Maitty Desktop v5.dc.html`, an interactive prototype. Open it in a browser with `support.js` next to it.
**Status:** design final for the first usable Codex release. Implementation not started.

---

## 0. How to use this document

1. Open the prototype. The top bar (prototype only, don't build it) switches:
   - the **window size**: 1440×900, 1024×700, 800×600;
   - the **state**: active chat, tool activity, approval, send failed, daemon disconnected, loading, loading failure, recovery, Codex unavailable, first launch.
2. Read §1–§3 for the why and the overall structure.
3. Build in the order in §15 (Build plan). Each part has its own section with **What**, **Why** and **How**.
4. The prototype is a single HTML "Design Component": markup plus one `class Component` in a `<script data-dc-script>`. **Don't port its code.** It fakes the daemon with timers and re-renders everything every 100 ms. Lift the values (colors, sizes, copy, behavior) and build them properly in React.

### What I could not read
The repo wasn't connected when this was designed, so I haven't read:
- `AGENTS.md`
- `docs/UI.md`, `docs/DESIGN.md`
- `docs/features/chat-gui/design-brief.md`
- `apps/gui/src/`

Before you start, do these three things:
- [ ] Reconcile the token names in §4 with `docs/DESIGN.md` (keep the values, adopt the repo's names).
- [ ] Map the components in §5 onto the shadcn components already generated in `apps/gui/src/components/ui`.
- [ ] Map the event and request names in §3 onto the real `maittyd` protocol types.

Thread titles, file paths and diffs in the prototype are illustrative, not real data.

---

## 1. Product rules (never break these)

| Rule | What it means in the UI |
|---|---|
| `maittyd` owns sessions and history | The GUI is a client. Closing the window, a tab, or a terminal pane **never** stops an agent. Say so in tooltips and toasts. |
| The TUI stays supported and shares sessions | Chat ⇄ Terminal is the **same session**, never a copy. Answering an approval in the GUI also answers it in the TUI. |
| Status is plain and honest | Working · Needs you · Done · Idle · Recovery needed · Unknown. When the daemon is disconnected, every status becomes **Unknown (last seen …)** and nothing animates as if it were progressing. |
| Provider permissions are shown, never widened | The approval panel shows the exact command, cwd, why it's blocked, the agent's reason, and **only the choices the provider offered**. |
| Never silently retry | **Not delivered** (safe to retry) and **Delivery unknown** (confirm before resending) are different states. Nothing is ever resent automatically. |
| Drafts and selection survive | Per-session composer drafts, open tabs and selection persist across reconnects and restarts. |
| Codex first | Claude appears as "Not supported yet" everywhere a provider is listed. |

---

## 2. Design rationale

The v4 prototype had every feature but asked users to read too much at once: activity lines everywhere, icon-only controls, a permanent right panel and dense headers. v5 keeps the behavior and makes it **calm and scannable**, like the TUI, without looking like a terminal.

1. **Two planes only.** The window (`--bg`, holding the sidebar) and one main card (`--main`). Separation comes from spacing and a few hairlines, not nested cards. Tool steps are rows, not boxes.
   *Why:* fewer borders means faster scanning, and it stays calm in a window left open all day.
2. **Status in three short places:** a mark in the sidebar row, a mark in the tab, and a mark plus a word in the conversation header. Each mark is a **different shape**, so color is never the only signal.
3. **Words over icons.** "Chat | Terminal", "Review", "Stop", "Send", "Settings", "Split" are written out. Icon-only buttons (open project, new session, tab ×, project +) all have tooltips and accessible names.
   *Why:* nobody should need to memorize glyphs or shortcuts.
4. **Nothing permanent on the right.** Changes, Files and Context sit behind one **Review** button. The chat keeps the full width until you ask.
5. **Color has one job each.** Teal = active work, primary actions, additions. Amber = needs you, only. Red = failure, recovery, offline, only. Done is a muted green so finished work doesn't compete with working.
6. **Provider identity is always visible:** the "›_ Codex · gpt-5-codex" pill in the header, the provider label in the composer, "Codex" over each agent turn, and the terminal pane header. If the provider is unavailable, the composer label turns red and says why.

---

## 3. Architecture

### 3.1 Process model
```
┌──────────── Tauri app ────────────┐        ┌──────── maittyd ────────┐
│ React UI (webview)                │        │ sessions, history, PTYs │
│   └─ store (zustand or similar)   │ events │ codex app-server bridge │
│        ▲            │ requests    │◀──────▶│ worktrees               │
│ Rust side (tauri commands/events) │ socket │                         │
└───────────────────────────────────┘        └─────────────────────────┘
```
- The **Rust side** of Tauri owns the socket to `maittyd` (`~/.maitty/maittyd.sock`, named pipe on Windows). It forwards daemon events to the webview with `app.emit(...)` and exposes requests as `#[tauri::command]`s.
  *Why:* keeps socket handling, reconnection and backoff out of the webview, and survives webview reloads.
- The **webview** holds a client store fed only by daemon events plus local UI state. The UI never writes session state directly.

### 3.2 Chat ⇄ terminal (same session)
Codex can't take input from an interactive TTY and an app-server connection in the same live turn. Recommended model (v4 §"Chat ⇄ terminal architecture"):
- `maittyd` runs Codex through `codex app-server` and turns its JSON-RPC events into chat events: messages, tool calls with live output, approval requests with offered choices, file changes.
- **Terminal view** attaches a PTY running `codex resume <session>` **between turns**. Mid-turn, the terminal is shown **view-only** with the line "Codex is mid-turn… view-only until the turn ends".
- Switching back re-attaches app-server. History carries over because Codex persists it.
- Shell sessions (no agent) only have the terminal view.

**Check before committing:**
- app-server event coverage: live command output, and the approval choice list;
- protocol stability across Codex versions;
- a clean `resume` of app-server sessions.

### 3.3 Daemon contract the UI needs
Rename to the real protocol, but keep these semantics:

```ts
// events (daemon → UI)
type DaemonEvent =
  | { type: 'connection'; state: 'connected' | 'disconnected'; attempt?: number; retryAt?: number }
  | { type: 'provider'; id: 'codex'; available: boolean; version?: string; path?: string; error?: string }
  | { type: 'sessions'; sessions: SessionSummary[] }            // full list on connect
  | { type: 'session.updated'; session: SessionSummary }        // status, title, lastActivity, unread
  | { type: 'item.added'; sessionId: string; item: ChatItem }
  | { type: 'item.delta'; sessionId: string; itemId: string; text?: string; outputLine?: string }
  | { type: 'item.done'; sessionId: string; itemId: string; result?: 'ok' | 'error' | 'stopped'; durationMs?: number }
  | { type: 'approval.requested'; sessionId: string; approval: Approval }
  | { type: 'approval.resolved'; sessionId: string; approvalId: string; choice: ApprovalChoice }
  | { type: 'prompt.ack'; clientId: string; seq: number }
  | { type: 'prompt.rejected'; clientId: string; reason: string }
  | { type: 'files.changed'; sessionId: string; files: ChangedFile[] }
  | { type: 'history.error'; sessionId: string; message: string };

type SessionStatus = 'working' | 'needs' | 'done' | 'idle' | 'recovery';
interface SessionSummary {
  id: string; projectId: string; title: string; status: SessionStatus;
  provider: 'codex' | 'shell'; model?: string; branch: string; worktree: string;
  sandbox: string; approvals: string; startedAt?: number; lastActivity: number; unread: boolean;
}
interface Approval {
  id: string; cmd: string; cwd: string; why: string; reason: string;
  choices: ApprovalChoice[];            // ONLY what the provider offered
  since: number;
}
type ApprovalChoice = 'once' | 'session' | 'deny';
```
Requests (UI → daemon):
- `sendPrompt(sessionId, clientId, text)`
- `answerApproval(sessionId, approvalId, choice)`
- `stopTurn(sessionId)`
- `resumeSession(sessionId)`
- `createSession({projectId, provider, where, branch, permissions})`
- `openProject(path)`
- `attachPty(sessionId)` / `detachPty`
- `reconnect()`

---

## 4. Design tokens

### 4.1 CSS variables (Midnight, default)
Put these on `:root` (or `[data-theme="midnight"]`) in `apps/gui/src/index.css`, and map shadcn's variables to them so stock components pick them up.

```css
:root, [data-theme="midnight"] {
  --bg:#090C10; --main:#0D1116; --raise:#141A21; --hover:#12171E; --sel:#1A212A;
  --line:#1B222B; --line2:#26303B; --well:#080A0D; --term:#06080B;
  --fg:#E8ECEF; --body:#D3D9DF; --fg2:#B3BCC5; --muted:#8C96A1; --dim:#75808B;
  --accent:#5FD4BF; --accent-soft:rgba(95,212,191,.11); --on-accent:#06110F;
  --warn:#F2B65A; --warn-soft:rgba(242,182,90,.07); --warn-line:rgba(242,182,90,.28); --on-warn:#151007;
  --danger:#F07F78; --danger-soft:rgba(240,127,120,.07); --danger-line:rgba(240,127,120,.3);
  --ok:#8DB89B;

  /* shadcn mapping */
  --background:var(--bg); --foreground:var(--fg);
  --card:var(--main); --card-foreground:var(--fg);
  --popover:var(--raise); --popover-foreground:var(--fg);
  --primary:var(--accent); --primary-foreground:var(--on-accent);
  --secondary:var(--raise); --secondary-foreground:var(--fg);
  --muted-foreground:var(--muted); --accent-foreground:var(--fg);
  --destructive:var(--danger); --border:var(--line); --input:var(--line2); --ring:var(--accent);
  --radius:0.5rem;
}
```
**Abyss:** same as Midnight except `--bg:#07111B --main:#0B1724 --raise:#112131 --hover:#0F1C29 --sel:#162839 --line:#14243A --line2:#1E3248 --well:#06101A --term:#050D15`.

**Paper (light):**
- Surfaces: `--bg:#EEF0EE --main:#FFF --raise:#F5F6F4 --hover:#F0F2F0 --sel:#E7EAE8 --line:#E3E6E4 --line2:#D2D7D4 --well:#F2F4F2 --term:#FAFBFA`
- Text: `--fg:#12171C --body:#262D33 --fg2:#3B444C --muted:#58626C --dim:#646E78`
- Accent: `--accent:#0E8F7A --on-accent:#fff`
- Needs you: `--warn:#9A5B00 --on-warn:#fff`
- Failure: `--danger:#C0392F`
- Done: `--ok:#2F7A4B`
- The soft and line variants are the same alpha mixes as Midnight, applied to these colors.

**Rule:** amber and red mean the same thing in every theme. Imported TUI themes may change surfaces and accent, never warn or danger.

### 4.2 Status mark colors (fixed)
| Mark | Spec |
|---|---|
| Working | TUI spinner glyph, Geist Mono 13px, `--accent` |
| Needs you | 14×14 tile, radius 4, fill `--warn`, two 2×6px bars in `--on-warn` (gap 2). Pulse: `box-shadow 0 0 0 0 → 0 0 0 4px` of warn 45% → 0, 2.4s |
| Done | 14×14 tile, radius 4, fill `rgba(141,184,155,.18)`, 9px check, stroke `--ok` 3.5 |
| Recovery / failure | 14×14 tile, radius 4, fill `rgba(240,127,120,.18)`, 10px "!" (line + dot), stroke `--danger` 3.5 |
| Idle | 11×11 outlined square, radius 3, 1.5px `#56616C` |
| Unknown (offline) | 12×12 dashed outlined square, radius 3.5, 1.5px `--dim` |

### 4.3 Type, spacing, radii, shadows, motion
- **Fonts:** Geist (UI) and Geist Mono (paths, commands, diffs, shortcuts, spinners, counts). Install with `npm i geist`, or self-host. Don't rely on Google Fonts in a desktop app.
- **Type scale (px):**
  - 24/600 first-launch title; 18/600 error titles; 17/600 empty-session title; 15/600 dialog titles; 14.5/500 conversation title;
  - 14/1.68 agent text; 14/1.55 user text and composer;
  - 13.5 sidebar titles; 13 tool rows and tabs; 12.5 buttons and meta; 12 sub-lines; 11.5 captions; 11/500/.06em section caps.
- **Spacing:** 4px base: 4, 6, 8, 10, 12, 14, 16, 20, 24.
- **Radii:** 6–8 controls and rows · 9–10 inputs, panes, theme cards · 12 main card, alert cards · 14 composer, dialogs · 999 pills and switches.
- **Shadows:**
  - composer `0 12px 40px rgba(0,0,0,.28)`;
  - dialogs `0 30px 80px rgba(0,0,0,.55)`;
  - drawer `20px 0 60px rgba(0,0,0,.5)`;
  - review overlay `-24px 0 60px rgba(0,0,0,.5)`.
- **Motion** (all off under `prefers-reduced-motion` **or** Settings → Reduce motion):

| Element | Motion |
|---|---|
| Content swaps (session, view, review tab) | opacity 0→1 + translateY 4px→0, .26s ease-out |
| Sidebar drawer | translateX(-100%)→0, .28s `cubic-bezier(.2,.8,.2,1)`; scrim fades .2s |
| Review overlay | translateX 16px→0 + fade, .28s same curve |
| Dialogs | fade + 4px rise, .2s |
| Press | `scale(.96)` small controls, `.985` rows, `.12s` |
| Hover color | `.14s ease` |
| Needs-you pulse | 2.4s ease-out infinite |
| Running shimmer | see §6.4 |

---

## 5. Component map (shadcn → Maitty)

| Maitty part | shadcn / primitive | Notes |
|---|---|---|
| App shell, drawer | `Sheet` (side="left") for the compact drawer | Docked sidebar is a plain `<nav>` |
| Tabs strip | Radix `Tabs` **or** a custom roving-tabindex `role=tablist` | Needs close buttons and middle-click, so custom is simpler |
| Chat / Terminal switch | `ToggleGroup` type="single" | Text labels |
| Review sheet | `Sheet` (side="right") under 1280px; plain `<aside>` when docked | Inner `Tabs` for Changes / Files / Context |
| Tool row expand | `Collapsible` | |
| Composer | `Textarea` (auto-grow) + `Button` | |
| Approval panel | custom (`role=alertdialog`) | Attached to the composer, not a modal |
| Open project | `Command` (cmdk) inside `Dialog` | |
| Settings | `Dialog` + vertical `Tabs` + `ToggleGroup` + `Switch` | |
| Terminal panes | `xterm.js` + `@xterm/addon-fit` per pane | Drag with native HTML5 DnD or `@dnd-kit/core` |
| Tooltips | `Tooltip` | Every icon-only button |
| Toasts | `sonner` (restyled) | Per v4 toast spec |
| Scroll areas | `ScrollArea` | Sidebar, chat, review, panes |
| Icons | `lucide-react` | FolderPlus, PenLine (new session), Search, Settings, X, Columns2, ChevronRight, Check |

Restyle everything to §4 tokens. Avoid the stock shadcn dashboard look: no default card borders and shadows on content, no gray-on-gray badges.

---

## 6. Sidebar

**What.** 256px column on `--bg`, three regions:
- **Header** (one 28px row): logo tile, "Maitty", then two 28×28 icon buttons: **Open project** (folder-plus, ⌘O) and **New session** (pen, ⌘N). Below it, a 30px **Search sessions** field (`--well`, radius 8).
- **List:** projects, each with its sessions.
- **Footer:** daemon status (dot + "maittyd · N sessions", or dashed red dot + "Reconnecting to maittyd…") and a **Settings** button (gear + label).

**Why.** Creating things is rare compared with scanning, so those actions get small icon buttons and the space goes to sessions. The footer gives a permanent, honest daemon indicator.

### 6.1 Project group
- Header row 28px: name (12px/500 `--muted`), amber "N needs you" when any session needs attention, and a 22px "+" (tooltip "New session in <project>").
- Empty project: a row reading "No sessions yet · Start one" (Start one in `--accent`) that creates a session.
- Older sessions (done/idle, untouched for over 24h) fold under "Show N older". Search ignores folding.

### 6.2 Session row (Detailed, default)
Layout: `display:flex; align-items:flex-start; gap:10px; padding:6px 8px; radius:8; min-height:32px`.
- **Line 1** (20px): status mark slot (14×20), title (13.5px; 500 weight and `--fg` when unread, needs or recovery; else 400 `--fg2`), unread dot (6px `--accent`), meta (11.5px tabular: `m:ss` elapsed when working; "Needs you" amber; "Recovery" red; "—" offline; otherwise relative time `now / 12m / 3h / 2d`).
- **Line 2** (12px/17px, ellipsis): what the session is doing.

| Status | Line 2 | Color |
|---|---|---|
| working | current tool label ("Running cargo test -p maittyd delivery"), else "Writing a reply…", else "Thinking…" | `--muted` |
| needs | "Approve: <cmd>" (or "Waiting for your reply" for questions) | `--warn` |
| recovery | "Codex exited mid-turn" | `--danger` |
| done with changes | "N files changed" + mono `+a −d` | `--dim` |
| idle/done | last agent sentence, whitespace collapsed | `--dim` |
| new | "No messages yet · <branch>" | `--dim` |
| offline | "Last seen working / needing you / …" | `--dim` |

- **Ordering inside a project:** needs you and recovery first, then working, then by `lastActivity` (newest first).
- **Selected row:** background `--sel`. Hover `--hover`. Press `scale(.985)`.
- **Compact mode** (Settings → Sidebar sessions → Compact) hides line 2.

**How:**
```tsx
function SessionRow({ s, selected }: { s: SessionView; selected: boolean }) {
  const shimmer = selected && s.status === 'working' && !reducedMotion;
  return (
    <button
      aria-label={`${s.title}, ${statusWord(s)}, ${s.projectName}`}
      aria-current={selected ? 'page' : undefined}
      className={cn('session-row', selected && 'is-selected', shimmer && 'is-shimmering')}
      onClick={() => openSession(s.id)}>
      <StatusMark status={s.effectiveStatus} />
      <span className="row-body">
        <span className="row-line1"><span className="row-title">{s.title}</span>{s.unread && !selected && <i className="unread" />}<span className="row-meta">{s.meta}</span></span>
        {detailed && <span className="row-line2" data-tone={s.subTone}>{s.sub}{s.diff && <Diffstat {...s.diff} />}</span>}
      </span>
    </button>
  );
}
```
`effectiveStatus = connection === 'disconnected' ? 'unknown' : status`. Compute this in **one selector** and use it everywhere: rows, tabs, header, panes.

### 6.3 Activity line throttling
Update a working row's line 2 at most every 2.5s unless the session is selected.
*Why:* a dozen agents updating line by line turns the sidebar into noise.

### 6.4 Running shimmer
The selected row **and** its tab get a moving sheen while that session is working.
```css
.is-shimmering{
  background-image:linear-gradient(100deg,transparent 30%,rgb(95 212 191/.07) 44%,rgb(95 212 191/.15) 50%,rgb(95 212 191/.07) 56%,transparent 70%);
  background-size:250% 100%; background-repeat:no-repeat;
  animation:shimmer 2.6s cubic-bezier(.4,0,.2,1) infinite;
}
@keyframes shimmer{from{background-position:150% 0}to{background-position:-50% 0}}
@media (prefers-reduced-motion:reduce){.is-shimmering{animation:none;background-image:none}}
```
Also off when disconnected. *Why:* it tells you at a glance that the thing you're looking at is alive, without a second spinner.

---

## 7. Tabs strip

**What.** A 46px bar at the top of the main card. Each open session is a tab: 32px tall, `flex:0 1 220px`, min-width 120 (96 below 900px), radius 8, with a status mark, title and a × (22px, shown on the active tab at 80% and on hover).
- Active tab: `--sel` background, `--fg` text. Inactive: transparent, `--muted`.
- Overflow scrolls sideways (scrollbar hidden).
- On the right: the **Chat | Terminal** toggle and **Review** (with `+a −d` when the session has changes).
- Under 1100px a **Threads** button sits at the far left and opens the drawer. It shows an amber needs-you mark and count when relevant.

**Why.** Tabs let people hold the 2–5 sessions they're actively steering, while the sidebar holds everything. Closing a tab is cheap and safe because the session lives in maittyd.

**How.**
- **Opening:** clicking a session in the sidebar opens it as a tab after the current one, or focuses it if it's already open. Store `openTabs: string[]` and `selectedId` in persisted local state (Tauri `store` plugin or `localStorage`).
- **Closing:** × / middle-click / ⌘W closes the tab and toasts "Tab closed · the session keeps running in maittyd". The last tab can't be closed. **Never** call any daemon stop method from close.
- **Keyboard:** ⌃Tab / ⌃⇧Tab cycle, ⌘1–9 jump to a tab.
- **Accessibility:** `role=tablist`, each tab `role=tab` with `aria-selected`, roving tabindex with ←/→, Enter and Space to activate.

---

## 8. Conversation

### 8.1 Header (≥44px, no border)
Title (14.5/500, ellipsis), provider pill ("›_ Codex · gpt-5-codex", `--raise`, radius 999), branch (mono 11.5 `--dim`), sandbox ("Workspace write · asks first"), spacer, then **status mark + word**:
- "Working · 1:42" (teal)
- "Needs you" (amber)
- "Done" (green)
- "Idle"
- "Recovery needed" (red)
- "Unknown · last seen working"

Below 900px, branch and sandbox are hidden; they stay in Review → Context.
*Why:* this is the one place status is always spelled out in words, and where the provider identity always sits.

### 8.2 Messages
- Column max 720px, centered, padding 8px 20px 20px, gap 14.
- **User:** right-aligned bubble, max `min(560px,86%)`, `--raise`, radius 14, padding 10/14. Failed sends get a `--danger-line` border (§11.2).
- **Agent:** plain text, 14/1.68 `--body`, no bubble. Small "Codex" label (12/500 `--muted`) at the start of each turn. While streaming, a 2×16px teal caret blinks (1s steps).
- **Auto-scroll:** follow only while the user is within ~120px of the bottom.

### 8.3 Tool activity
One 30px row per tool call: mark, label, `+a −d`, duration or live timer, chevron. Hover `--hover`.
- **Mark:** spinner while running, dim check when done, red × when failed or stopped.
- **Label:** "Running cargo test …" while running, "Ran cargo test …" when finished.
- **Expanding** (`Collapsible`) shows the command and output in a `--well` mono block (12px/1.6), indented to the label.

*Why:* users need to see what the agent is doing without reading raw logs, and rows instead of cards keep long turns scannable.

### 8.4 Files-changed summary
At the end of a turn that edited files: a 32px button reading "N files changed +a −d Review →" that opens Review → Changes.

### 8.5 Empty conversation
A new session with no messages shows "New Codex session in <project>", then "Worktree <branch> · <sandbox>. Send a message to start. It keeps running in maittyd if you close this window."

---

## 9. Composer & approvals

### 9.1 Composer
A card max 760px, `--raise`, 1px `--line2` (`--warn-line` while an approval is pending), radius 14, composer shadow.
- **Textarea:** 14/1.55, auto-grows up to 200px, starts at 2 rows (1 row when the window is shorter than 720px).
- **Bottom row:** provider label (dot + "Codex · gpt-5-codex"; red "Codex unavailable" when missing), sandbox label, spacer, then:
  - **Send** (teal, "Send ↵") when idle;
  - **Stop** (square icon + "Stop ⌘.") while working;
  - **"Deny & send"** when an approval is pending and you've typed.
- **Status line above the card** (12.5px), shown only when something blocks or explains sending:
  - Working: "Working · 1:42 · your next message waits until this turn ends"
  - Disconnected: "Your draft is saved. Nothing is sent until you press Send after reconnecting."
  - Provider missing (red): "Codex CLI not found on PATH. Install it or set its path in Settings → Providers. Your draft is kept."
  - Recovery (red): "Resume the session to send messages. Your draft is kept."
  - Approval pending: "Typing a message denies the request and sends your instructions instead."
- **Drafts:** `drafts[sessionId]`, persisted on every change (debounced) and never cleared by reconnects.

### 9.2 Approval panel
**What.** Attached to the **top of the composer card** (same container): `--warn-soft` background and a `--warn-line` bottom hairline.
- **Full view:**
  - title "Codex wants to run a command outside the sandbox", wait timer, "Less";
  - `$ <cmd>` in a `--well` mono block;
  - "In <cwd>. <why>" and "Codex says: “<reason>”";
  - footnote "Only the choices Codex offers. Answering here also answers it in the TUI.";
  - buttons for **only `approval.choices`**: Deny (N), Allow for this session (A), **Allow once (Y)** (amber fill, `--on-warn` text).
- **Compact view:** automatic when the window is under 720px tall, toggled with "Details". One row: mark, "Approval needed", truncated command, wait time, choices.
- **In the chat:** a marker line reads "Waiting for your approval below" and resolves to "Allowed once: git fetch origin main" or "Denied: …".

**Why.** The approval is the next action, so it sits where your hands already are, without covering the conversation or opening a modal. Showing only the offered choices keeps the provider's sandbox intact.

**How.**
- Render from `pendingApproval(session)`.
- Y / A / N work when focus isn't in a text field.
- Disable everything (50% opacity) while disconnected.
- Use `role=alertdialog` with `aria-label="Approval needed: <cmd>"`. Announce it via the live region when it arrives for the selected session.

---

## 10. Review sheet (Changes · Files · Context)

**What.** Hidden until **Review** or ⌘⇧D is pressed. Header: tabs **Changes N**, **Files**, **Context**, and "Close esc".
- **≥1280px wide:** docks inside the main card as a 440px column with a left hairline, and the chat narrows.
- **<1280px:** overlays from the right at 480px (full width minus 40px under 900px), over a scrim. It closes on scrim click, Esc or Close.

**Why.** Review is occasional. It shouldn't take chat space until asked for, and on small windows it should cover rather than squeeze.

**Changes**
- File list rows: dim dir + name, `+a −d`.
- Diff below: mono 12/20; added lines `rgba(accent,.08)` background with a 2px inset accent edge and "+"; removed lines danger 8% with a 2px danger edge and "−"; hunk headers on `--raise`.
- Footer: "Read-only. Ask Codex for edits, or open the file in your editor."
- Build with `diff2html` parsing, or render the daemon's unified diff yourself. Syntax highlighting with Shiki is optional (later).

**Files**
- Project tree: dirs muted with indent 14px per level; changed files tagged **M**.
- Selecting a file shows a read-only preview with line numbers.
- Editing is **later**.

**Context** (read-only): Provider (binary + version), Model + reasoning, Sandbox + network + config path, Approvals, Project + path, Worktree + path, Session ("Owned by maittyd. Keeps running if you close this window or this tab."). Closing note: "Sandbox and approval settings come from your Codex config. Maitty shows them and never widens them."

---

## 11. Failure & recovery states

Each one is a first-class screen, and all of them are in the prototype's State menu.

### 11.1 Daemon disconnected
- **Banner** at the top of the main card (`--danger-soft`, `--danger-line` bottom), `role=status`: "**Disconnected from maittyd.** Agents keep running in the daemon. Statuses below are the last known values. Retrying in 4s (attempt 2)." plus **Retry now**.
- **Statuses:** every status mark becomes the dashed square, and the header reads "Unknown · last seen working". Spinners, timers and the shimmer stop. Running tool rows say "(last known)".
- **Actions:** the approval panel is disabled and Send is disabled. The draft stays editable and saved.
- **Backoff:** 1, 2, 4, 8, 8… seconds; manual Retry resets it.
- **On reconnect:** fetch the full session list, then re-subscribe. Toast "Reconnected to maittyd · statuses refreshed".

### 11.2 Send failed
- **Not delivered** (`prompt.rejected`, or the socket failed before the daemon accepted it): red-bordered bubble with "Not delivered. maittyd refused it before Codex saw it, so retrying is safe." plus **Retry** and **Edit** (Edit moves the text back into the composer).
- **Delivery unknown** (sent, socket lost before `prompt.ack`): amber dashed border with "Delivery unknown" and three actions: **Open terminal**, **It arrived**, **Send anyway…**. Send anyway opens an inline confirmation: "If it arrived, Codex runs it twice."
- **How:** generate `clientId` (UUID) per send and keep `pending[clientId] = {sessionId, text, sentAt}`. On ack, mark delivered. On disconnect, every pending entry becomes `unknown`. **Never auto-resend.**

### 11.3 Loading and loading failure
- **Loading:** skeleton bubble and lines (pulse 1.4s) with "Loading history from maittyd…".
- **Failure** (`history.error`): "Couldn't load this conversation", an explanation that the session is unaffected, the actual error in a mono block, then **Try again** and **Open native terminal**.

### 11.4 Recovery needed
- **Card:** `--danger-soft`, `--danger-line`, radius 12, `role=alert`. Titled "Recovery needed: Codex stopped mid-turn". It says what exited (signal, time), that history is saved up to the last event, which command may not have finished, and that **nothing will be replayed automatically**.
- **Actions:** **Resume session** (primary) and **Open native terminal**.
- **Composer:** disabled with its reason.
- **After resume:** a notice reads "Session resumed · the interrupted turn was not replayed".

### 11.5 Codex unavailable
The provider label in the composer turns red, the status line explains PATH and Settings, and Send is disabled. The draft is kept.

### 11.6 First launch and no sessions
- **No projects:** a centered 440px column with "Run your coding agents side by side.", a short explanation (sessions live in maittyd), and a checklist: maittyd running ✓ version, Codex CLI found ✓ version, Claude ○ "Not supported yet". Then a primary **Open a project folder ⌘O** and the note "Any folder works. Git repos also get a worktree per session." New session is disabled.
- **Projects but no open session:** "No open sessions" with a **New session** button.

---

## 12. Terminal view

**What.** **Chat | Terminal** (⌘`) replaces the conversation and composer with native Codex terminal panes.
- **Hint line** above the panes, plus a **Split ⌘D** button (max 4 panes).
- **Layout:** CSS grid with 10px gap. 1 pane fills the area; 2 sit side by side; with 3, the first spans both rows; 4 make a 2×2 grid; under 900px wide, panes stack in one column.
- **Pane:** radius 10, `--term` background, 1px `--line`.
- **Pane header** (36px):
  - a 6-dot drag grip;
  - the status mark;
  - a session picker (select; picking a session already shown in another pane swaps the two);
  - the worktree path (mono);
  - "view-only this turn" while Codex is mid-turn;
  - "Focused" label;
  - × to close the pane (the session keeps running).
- **Focus:** the focused pane (only when there's more than one) gets an accent border (`rgba(95,212,191,.5)`), a `0 0 0 3px var(--accent-soft)` ring and a faint accent header tint. Clicking a pane focuses it, selects its session and syncs the tab. ⌘[ / ⌘] move focus.
- **Drag to rearrange:** drag a header onto another pane and the two swap. The target shows a 2px dashed accent outline.
- **Sidebar link:** in terminal view, clicking a session in the sidebar focuses its pane if it's shown, otherwise loads it into the focused pane.

**Why.** Watching two or three agents side by side is the multiplexer's core use. The GUI should offer it without fighting the native terminal, and should never restyle Codex's output.

**How.**
```tsx
function TerminalPane({ pane, focused }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const term = new Terminal({ fontFamily: 'Geist Mono', fontSize: 12.5, lineHeight: 1.5, theme: xtermTheme() });
    const fit = new FitAddon(); term.loadAddon(fit); term.open(ref.current!); fit.fit();
    const unsub = attachPty(pane.sessionId, (bytes) => term.write(bytes));   // tauri event stream
    term.onData((d) => writePty(pane.sessionId, d));
    const ro = new ResizeObserver(() => { fit.fit(); resizePty(pane.sessionId, term.cols, term.rows); });
    ro.observe(ref.current!);
    return () => { ro.disconnect(); unsub(); detachPty(pane.sessionId); term.dispose(); };  // detach, never kill
  }, [pane.sessionId]);
  useEffect(() => { if (focused) termRef.current?.focus(); }, [focused]);
  ...
}
```
- **Panes state:** `panes: {id, sessionId}[]`, `focusedPaneId`, persisted per window.
- **Keys:** while a pane has focus, intercept **only** ⌘`, ⌘K, ⌘D and ⌘[ ]. Everything else goes to the PTY.
- **Mid-turn:** don't attach input (`term.options.disableStdin = true`) until the turn ends (see §3.2).
- **Resize:** debounce PTY resize ~50ms (SIGWINCH storms are a known Linux race).
- **Drag:** native HTML5 DnD on the header is enough. `@dnd-kit` if you want animated reordering.

---

## 13. Open project, New session, Settings

### 13.1 Open project (⌘O, folder-plus button)
**What.** `Dialog` with `Command` inside: 560px wide, 12vh from the top, radius 14.
- **Input:** "Folder path, or search recent projects".
- **Path typed** (starts with `~`, `/` or `.`): an "Open <name>" row with the full path.
- **Otherwise:** RECENT rows (44px): folder tile, name, mono path, and "N sessions" or "Not open".
- **Keys:** ↑↓ / ↵ / Esc.
- **Footer:** "Any folder works. Git repos also get a worktree per session." plus **Browse…**, which calls the Tauri dialog plugin `open({ directory: true })`.
- **After opening:** the project appears in the sidebar with "No sessions yet · Start one".

**Why.** Keyboard-first and forgiving: paste a path or type part of a name. Recent projects come from the daemon (or local persisted history).

### 13.2 New session (⌘N, pen button, project "+", "Start one")
The prototype creates an idle session in the current project with a fresh `codex/xxxx` branch and focuses the composer. The first message becomes the title (trimmed to 40 chars).

The production dialog (provider, worktree vs folder, permissions, first message) is unchanged from the v4 spec. Restyle it to v5 tokens: a `Dialog` with three option cards for provider (Codex / Claude disabled "Not supported yet" / Shell), a Where choice (New worktree / Project folder), Permissions (Read only / Workspace write / Full access), an optional first message, and **Start session ⌘↵**.

### 13.3 Settings (⌘, or the footer Settings button)
**What.** `Dialog`, 780×560 max, left vertical tabs (180px; 150px under 900px wide). Content rows: label + 12px muted explanation on the left, control on the right, 1px `--line` dividers.

| Section | Controls |
|---|---|
| General | On launch: Restore tabs / Start empty · Default permissions: Read only / Workspace write / Full access · Closing this window: "Always" (explanatory) |
| Appearance | Theme cards (mini previews) Midnight / Abyss / Paper · Working spinner (Braille / Snake / Bars, live glyph) · Sidebar sessions: Detailed / Compact · Reduce motion (switch) |
| Notifications | A session needs you (switch) · A session finished (switch) · Recovery needed: "Always on" |
| Daemon & providers | maittyd status + version + socket + Reconnect · Codex status + version + path + config + Change path… · Claude "Not supported yet" |
| Keyboard | Full shortcut list (§14) |

**How.**
- Persist settings via the Tauri store plugin.
- Theme switching sets `document.documentElement.dataset.theme`. Optionally cross-fade with the View Transitions API (`document.startViewTransition`).
- Spinner frames (match the TUI; **replace Snake and Bars with the TUI's exact strings**):
  - Braille `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` @80ms
  - Snake `⠏⠛⠹⢸⣰⣤⣆⡇` @90ms
  - Bars `⡀⣀⣄⣤⣦⣶⣷⣿⣷⣶⣦⣤⣄⣀` @85ms
- Render spinners with one shared `requestAnimationFrame` clock (or a tiny self-ticking component). **Don't re-render the app per frame.**

---

## 14. Keyboard & accessibility

### 14.1 Shortcuts
⌘ is Ctrl on Windows and Linux.

| Action | Keys |
|---|---|
| New session · Open project · Settings | ⌘N · ⌘O · ⌘, |
| Next / previous tab · Jump to tab | ⌃Tab / ⌃⇧Tab · ⌘1–9 |
| Close tab (session keeps running) | ⌘W |
| Chat ⇄ Terminal | ⌘` |
| Split terminal · Focus prev/next pane | ⌘D · ⌘[ / ⌘] |
| Review: changes · context | ⌘⇧D · ⌘I |
| Sidebar / drawer | ⌘\ |
| Stop the turn | ⌘. |
| Answer approval (not typing) | Y · A · N |
| Send · New line | ↵ · ⇧↵ |
| Close the top layer | Esc (dialog → review → drawer) |
| Shortcut list | ? |
| Carried over from v4 | ⌘K quick switcher · ⌘⇧A next session that needs you |

### 14.2 Accessibility checklist
- [ ] **Status names:** every status mark has a text equivalent: row `aria-label="Title, Needs you, maitty"`, tab tooltip, header word.
- [ ] **Roles:** tablists (tabs, Chat | Terminal, review, settings nav) use roving tabindex and arrow keys. The approval panel is `role=alertdialog`. Failures use `role=alert`; the offline banner and composer status line use `role=status`.
- [ ] **Dialogs and drawer:** trap focus, put initial focus on the primary input, restore focus on close (shadcn Dialog and Sheet do this).
- [ ] **Focus ring:** 2px `--ring` with 2px offset (inset on rows) on every interactive element. The prototype shows browser defaults; implement the ring.
- [ ] **Live region** (polite): "Finished / Needs you / Recovery needed: <session>". Streamed tokens are **not** announced.
- [ ] **Contrast:** text ≥ 4.5:1 on its surface in every theme (`--muted` and `--dim` were picked for this on `--main`). Color is never the only signal.
- [ ] **Hit targets:** ≥28px in dense rows, ≥32px for primary actions.
- [ ] **Reduce motion** (system or setting): stops shimmer, pulses, slide-ins and skeleton pulse. Spinners show a static frame.

---

## 15. Build plan (recommended order)

1. **Tokens and fonts.** §4 CSS, shadcn variable mapping, Geist via npm, the `StatusMark` component (6 states).
2. **Daemon bridge.** Rust socket client with reconnect/backoff, Tauri events/commands, the TS types from §3.3, the store with an `effectiveStatus` selector.
3. **Shell layout.** Sidebar (docked + drawer under 1100px), main card, tab strip with persistence.
4. **Conversation.** Header, messages, streaming, tool rows, files-changed summary, empty conversation.
5. **Composer.** Drafts, Send/Stop, status line, delivery tracking (clientId/ack).
6. **Approval panel.** Full/compact, Y/A/N, Deny & send.
7. **States.** Offline banner, send failure (both kinds), loading, loading failure, recovery, provider unavailable, first launch, no sessions.
8. **Review sheet.** Changes + Context (Files preview if time allows).
9. **Terminal view.** Single pane, view-only mid-turn, then split/focus/drag.
10. **Open project + New session dialogs.**
11. **Settings.** Themes, spinner, rows, motion, notifications, providers, keyboard.
12. **Toasts** for background sessions (v4 spec, simplified to Open / Allow once / Dismiss).
13. **Accessibility pass + reduced motion.** Test at 1440×900, 1024×700, 800×600.

**Performance note:** drive all UI from daemon events. Isolate spinners and timers in small self-updating components (or CSS), and memoize rows by session. The prototype's global 100ms re-render is a prototyping shortcut only.

---

## 16. Now vs later

**Include in the first usable Codex release**
- Sidebar with Detailed rows, ordering, search, Show older, Settings entry and daemon status.
- Open project dialog. New session (v4 dialog, v5 styling).
- Tabs that never stop sessions, with persistence and keyboard.
- Conversation: streaming, tool rows, files-changed summary, empty state, stop.
- Composer with drafts, status line, provider label, delivery tracking.
- Approval panel with only the provider's choices.
- Chat ⇄ Terminal on the same session (chat-first, terminal between turns), single pane.
- Review: Changes (read-only) + Context.
- All the states in §11.
- Settings: General, Appearance (Midnight + Paper at minimum), Notifications, Daemon & providers, Keyboard.
- Background toasts, keyboard map, accessibility checklist, reduced motion.

**Later**
- Terminal split, focus and drag (move to "now" if xterm/PTY embedding lands early).
- Review → Files editing and file tabs; References and image attachments; syntax-highlighted diffs.
- Pin, rename and archive sessions; quick switcher extras; slash commands.
- Importing all Maitty TUI themes (derive from 5 inputs: bg, surface, fg, dim, accent); accent choices.
- Claude provider. Keep "Not supported yet" until it ships.

---

## 17. Files in this package
- `Maitty Desktop v5.dc.html`: the interactive prototype. Open in a browser.
- `support.js`: the prototype runtime (must sit next to the HTML).
- `README.md`: this document.

The previous handoff (`design_handoff_maitty_desktop/`, v4) remains the reference for detailed behavior this document points to: the full New session dialog, toast stacking, Files editing and References.
