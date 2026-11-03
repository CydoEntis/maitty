# Maitty Desktop v12: Terminal-first GUI, handoff

**Prototype:** `Maitty Desktop v12.dc.html` (use **Style: Flat / Tiled / Glass** in the bar above the window to compare). Choose **First run** in the State switch to see onboarding. Click **New features 22** in the bar above the window for a guided list with a **Show** button per feature; the **After restart** state demos session restore. Open it in a browser with `support.js` next to it. The bar above the window is prototype-only: it switches the window size (1440×900, 1024×700, 800×600) and the state (Sessions running, No sessions, Daemon disconnected). The terminals are interactive: try `cd maitty`, `ls`, `cd ..`, `codex`, `cargo test`, `exit`.

**Stack:** Tauri + React + TypeScript + shadcn/ui primitives, xterm.js for panes. The prototype fakes the shell and the daemon. **Don't port its code**; lift the values and behavior.

**Repo not read.** Reconcile token names with `docs/DESIGN.md`, components with `apps/gui/src`, and daemon calls with `crates/maitty-core/src/daemon/` (`pane_screen`, `pane_replay`, `pane_input`, `pane_resize`, `may_use_terminal`) and ADR 0045. Pane streaming details are in `../design_handoff_maitty_desktop_v6_terminal/TERMINAL_IMPLEMENTATION.md`.

---

## 1. Product model

- **The app is a terminal multiplexer window.** No chat UI. Every session is a real shell PTY owned by `maittyd`. The GUI only attaches.
- **A new session opens a shell in the start folder** (Settings → General, default `~/code`). The user `cd`s wherever they want.
- **The sidebar follows the shell.** Sessions are grouped by the **git repo containing their current cwd**. When a session `cd`s into or out of a repo, it moves to that group with a short teal highlight and a toast ("Session moved to maitty"). Sessions outside any repo sit under **Other folders**.
- **Tabs hold layouts; panes hold sessions.** A tab is a split layout of 1–4 panes. Each pane shows one session. Closing a pane or tab **detaches**; the session keeps running and stays in the sidebar. Clicking a sidebar session focuses its pane if it's open in a tab; otherwise it opens **in a new tab** (never into another project's layout).
- **Agents are just processes.** `codex` (or any agent) runs inside a shell. Status comes from the daemon (foreground process + agent state), not from a chat protocol. Approvals are answered **in the terminal** (y / a / n); the GUI only surfaces that one is waiting.
- **Status vocabulary:** Working, Needs you, Done, Exited (error), Idle, Unknown (daemon disconnected). Each has a distinct shape (§4.3), never color alone.
- **"Done" is short-lived:** it clears to Idle as soon as the user runs the next command in that session.
- **Sidebar order is stable.** Rows keep creation order inside a group and never reorder on status changes. Groups are sorted by name, with Other folders last.

### What changed in v12 (read this first)
v11 still applies, with one rename: **Minimal → Flat, Expressive → Glass**, plus a new **Tiled** style. v12 adds the items below; each has a full spec in §12.

| # | Change | Where |
|---|---|---|
| 1 | **Window style**: Flat (default) / **Tiled** (new) / Glass | Settings → Appearance → Window style |
| 2 | Settings: fixed header with Close + esc | Every settings page |
| 3 | **Theme**: Appearance System / Dark / Light, plus separate **Dark theme** and **Light theme** dropdowns with search and live preview | Settings → Appearance → Theme |
| 4 | High Contrast theme | Dark theme list |
| 5 | **Project presets**: save a tab layout, reopen it per project | Project right-click menu, Settings → Presets, ⌘K |
| 6 | **First run** with TUI config import | First launch; Settings → General → Show the welcome again |
| 7 | **Shared config** with the Maitty TUI | Settings → General |
| 8 | **Status bar** | Bottom of the window |
| 9 | **Rename, pin, archive** sessions | Right-click a session |
| 10 | **Shortcut overlay** | ⌘/ or the status bar |
| 11 | **Collapsed sidebar redesign**: tinted project groups, folder tiles with 2-letter labels | Rail |
| 12 | **Rail popovers** for projects and sessions | Hover or focus in the rail |

### What changed in v11
v10 still applies. v11 adds the items below; each has a full spec in §11.

| # | Change | Where |
|---|---|---|
| 1 | **Style setting: Minimal (default) / Expressive** | Settings → Appearance → Style |
| 2 | Minimal drops nested borders (tray, tab tray, selected rings) | Sidebar, tab bar |
| 3 | Inspect becomes an **overlay drawer** with one fixed, resizable width | ⌘I |
| 4 | New drawer header and tabs (text tabs, underline) | Inspect |
| 5 | Drawer slide in/out | Inspect |
| 6 | Directional slide when switching tabs, Inspect sections, and Settings pages | Everywhere |
| 7 | **Vertical pane resize** (stacked splits) | Between stacked panes |
| 8 | Tab badge on the sidebar main row; agents sort before shells in the tray | Sidebar |

### What changed in v10
Everything in v9 still applies. v10 adds the features below; each has a full spec in §10.

| # | Feature | Entry points |
|---|---|---|
| 1 | Command palette | ⌘K |
| 2 | Attention inbox | ⌘J, **Inbox** button in the tab bar (count badge, amber when something needs you) |
| 3 | Native OS notifications | Fired by maittyd events for sessions you aren't viewing |
| 4 | Session restore and crash recovery | On launch, and after maittyd restarts |
| 5 | Find in terminal (pane or all sessions) | ⌘F |
| 6 | Broadcast input | ⌘⇧B, pane-header broadcast button |
| 7 | Turn timeline | Strip on each pane's right edge |
| 8 | Checkpoint compare | Inspect → History → **Compare** |
| 9 | Worktree sessions (new and open existing) | ⌘⇧N, palette, right-click a project |
| 10 | Usage meter | Pane-header pill on agent panes |
| 11 | Pane resize | Drag the gap between columns |
| 12 | Drag a session into a pane | Drag any sidebar row or tray item |
| 13 | Quick-look file links | ⌘-click a path in output |
| 14 | Focus mode | ⌘. |
| 15 | Project colors | Settings → Appearance → Sidebar |
| 16 | Resizable sidebar | Drag the sidebar's right edge |
| 17 | Project context menu | Right-click a project header (full sidebar or rail) |

### What changed in v9
1. **Tab bar is scoped to the active project.** The tab bar only shows tabs whose focused pane is in the same group (git repo, or Other folders) as the current tab (§3.3a).
2. **Sidebar: one main row per project plus a tray of other sessions** (§3.1a). The tray comes in three styles (With names / Icons / Stacked). A **+N** opens a side menu listing all of the project's sessions.
3. **Lead and priority order:** Settings decides whether the main row is the *focused* session or the one that *needs attention most*. Everything is sorted needs you → error → done → working → idle.
4. **Tab overflow:** edge fade, ‹ › arrows, wheel scrolling, auto-scroll to the active tab, and an **All tabs** menu (§3.3b).
5. **Status motion differs per state** (Working sweep / Needs-you breathe / Done one-shot), with per-state switches (§4.5, §5).
6. **Inspect panel** (⌘I): Files tree + preview, Changes, Branches, History + checkpoints, References (§5b).
7. **Hotkeys render as keycaps** (§5).

## 2. Layout

```
┌───────────── window (ambient background) ─────────────────────────────────┐
│ ┌ Sidebar card (264 / 60 rail) ┐  ┌ Tab bar ───────────────────┐ [+]     │
│ │ ⊟ SESSIONS            + New  │  │ [>_ zsh maitty][✻ codex …] │          │
│ │ ▾ ▣ maitty  main   2         │  └────────────────────────────┘          │
│ │   │ >_ (root)          now   │  ┌ Pane card ────────┐ ┌ Pane card ────┐ │
│ │   │ ✻ crates/…  ⠋     1:42   │  │ ✻ codex ~/code/…  │ │ rs cargo test │ │
│ │ ▸ ▣ maitty-docs   1          │  │   [status][⑂ repo]│ │               │ │
│ │ ▾ ⬚ Other folders 1          │  │ ⊞ ⊟ ⤢ ×           │ ├───────────────┤ │
│ │ ● maittyd · 4    ⚙ Settings  │  │ terminal…         │ │ pane card     │ │
│ └──────────────────────────────┘  └───────────────────┘ └───────────────┘ │
└───────────────────────────────────────────────────────────────────────────┘
```

- **Window background:** two soft radial glows over `--bg`, `radial-gradient(1100px 620px at 8% -10%, var(--accent-soft), transparent 60%)` and `radial-gradient(900px 520px at 100% 110%, rgba(195,166,242,.07), transparent 60%)`. The sidebar, tab bar and panes float on top of it.
- **Floating cards** (sidebar, panes, Settings):
  - Background: `linear-gradient(180deg, rgba(255,255,255,.035), transparent 120–160px), var(--pane)`.
  - Border: `1px solid var(--pane-line)`, radius 14 (Settings 16).
  - Shadow: `0 14px 34px rgba(0,0,0,.3)`, plus `backdrop-filter: blur(18px)`.
- **Spacing:** 8px outer margin; 12px gaps between panes and between the tab bar and panes.
- **Responsive:**
  - **≥1100px:** the sidebar is docked and collapsible to the rail.
  - **<1100px:** the sidebar becomes a drawer (slides from the left over a scrim), opened by a "Sessions" button at the left of the tab bar. That button shows an amber mark when something needs you.
  - **<900px:** panes stack in one column and tabs shrink to a 96px minimum.

## 3. Components

### 3.1 Sidebar (expanded, 264px)
- **Header** (48px):
  - collapse button (panel icon, ⌘B);
  - a "SESSIONS" label (11/600, letter-spacing .08em, `--dim`);
  - a **+ New** button (opens a shell in the start folder).
- **Folder header** (button, 34px, radius 9):
  - **Chevron:** rotates 90° when open, `.2s cubic-bezier(.2,.8,.2,1)`.
  - **Folder tile:** 22×22, radius 6. Teal gradient `#6FE0CB → #2E9C88` with a dark folder glyph when it contains the focused session; otherwise a slate gradient `#3A4656 → #1A212B`. Other folders uses a dashed outline tile.
  - **Text:** name (13.5/600) and branch (mono 11, `--dim`).
  - **Right side:** spinner when collapsed with something running, amber "N needs you", and a count pill.
  - **Click** toggles collapse. A collapsed group auto-opens briefly when a session moves into it.
- **Session rows** (indented 20px, with a 1px `--pane-line` guide at x=15):
  - **Icon:** 24×24 app icon (§4.4) with a 14px status badge at the bottom-right corner, ringed 2px in the card color.
  - **Line 1:** the folder within the repo (`(root)`, `crates/maitty-core`) or a `~`-path. Unread dot, spinner when working, then meta: `m:ss` while working, otherwise relative time.
  - **Line 2:** process in mono, then a status phrase: "· Allow git fetch origin main?" (amber), "· finished", "· exited with error" (red), the last output line while working, "· idle".
  - **Selected row:** `--raise` background, `inset 0 0 0 1px var(--pane-line)` plus `0 4px 12px rgba(0,0,0,.25)`.
  - **Hover** (via JS state, not CSS `:hover`, because inline backgrounds win): `rgba(255,255,255,.055)` background, inset hairline, `translateX(2px)`, text to `--fg`.
- **Footer:** daemon dot and "maittyd · N sessions" (dashed red dot and "Reconnecting to maittyd…" when offline), plus a Settings button.

### 3.1a Sidebar v9: main row + session tray
- **One stack per project group.** The **main row** is the full session row from §3.1 (24px icon with status badge, folder title, process and status line, meta).
  - **Who leads:** with **Focused** (default), it's the focused session if it's in this group, else the last one picked here, else the highest priority. With **Needs attention**, it's always the highest priority.
  - **Switching the lead:** animate with `v8swap` (opacity .35 → 1, translateY 5px → 0, scale .985 → 1, .32s).
- **Tray** (only when the group has more than one session):
  - **Shape:** hangs off the bottom of the main row, `margin:-6px 5px 4px; padding:10px 5px 5px`, radius `0 0 10px 10px`, `rgba(255,255,255,.028)` with an inset `--pane-line` hairline. The main row sits above it (`position:relative; z-index:1`), so the tray tucks under it.
  - **Order:** needs you, error, done, working, idle; ties broken by most recent activity.
  - **Item:** 24px tall, radius 7, `rgba(255,255,255,.03)` with a hairline. An 18px app icon carries the status badge **anchored to the icon** (spinner / amber pause / green check / red "!" at −5px, unread dot top-right).
  - **Hover:** `rgba(255,255,255,.08)` background, a stronger hairline, `translateY(-1px) scale(1.06)`.
  - **Click:** focuses that session (it becomes the main row).
  - **Styles** (Settings → Appearance → Sidebar → "Other sessions in a project"):

| Style | Layout | Visible before +N |
|---|---|---|
| **With names** (default) | equal-width grid, `repeat(n,1fr)`; icon + process name (mono 11) | 1–2 full width; above 2 → 2 + `30px` "+N" column |
| **Icons** | 6-column grid of icon squares, no label | 6; above 6 → 5 + "+N" |
| **Stacked** | one per line, full width; icon + process + folder (dim, ellipsis) + status word on the right (teal timer, amber "needs you", green "done", red "exited") | 3; above 3 → 2 + full-width "+N more" |

- **+N:** mono 10.5. It turns amber if any hidden session needs you, and its aria-label includes the count. Clicking it opens the **side menu**:
  - **Placement:** 300px wide, max 420px tall, placed 8px to the right of the sidebar and level with the button. It enters with a slide (`v8insp`).
  - **Header:** project name and "N sessions", plus ×.
  - **List:** `overflow-x:hidden`. 44px rows (icon, folder, process, status word with spinner, time), sorted by priority. Each row slides in with `v8row`. The focused session is raised.
  - **Closing:** clicking a row opens that session and closes the menu. Esc or clicking outside also closes it. ARIA: `role=menu/menuitem`, and the button has `aria-haspopup`/`aria-expanded`.
- **Rail mode** (§3.2) is unchanged and shows every session.

### 3.2 Sidebar rail (collapsed, 60px)
- **Width animation:** 264 ↔ 60, `.32s cubic-bezier(.2,.8,.2,1)`.
- **Top:** expand button, then New (36×32, hairline pill), then a 28px divider.
- **Projects:** each is a short label button: initials of hyphenated parts (`maitty-docs` → `MD`), else the first two letters (`MA`); Other folders shows `···`. An amber dot means needs you. Clicking toggles that project's sessions.
- **Sessions:** 40×40 hit area with a 28×28 app icon.
  - **Badges:** spinner (16px dark circle), needs-you pause (amber, pulsing), done check, error "!", and a top-right unread dot.
  - **States:** selected rows are raised and outlined; hover is `scale(1.06)`.
  - **Tooltip:** `folder · process · status`.
- **Bottom:** daemon dot, then Settings.

### 3.3 Tab bar
- **Container:** pill tray, radius 14, `rgba(255,255,255,.025)` with an inset `--pane-line` hairline. It is sized to its tabs (`flex: 0 1 auto`) and scrolls only when tabs overflow.
- **Tab** (36px, radius 10, `flex: 0 0 auto`, max 250px):
  - 22×22 app icon of the focused pane's process;
  - **process** (bold) + folder (`--muted`);
  - a needs-you mark or spinner for the worst status inside the tab;
  - pane count when there are 2 or more;
  - ×.
- **Active tab:** `--raise` with `0 1px 0 rgba(255,255,255,.05) inset, 0 0 0 1px var(--pane-line), 0 6px 16px rgba(0,0,0,.28)`. If it's working, it also gets the running shimmer (§5).
- **Hover** (JS state): `rgba(255,255,255,.075)` background, `inset 0 0 0 1px rgba(255,255,255,.09), 0 4px 12px rgba(0,0,0,.22)`, `translateY(-1px)`, icon `scale(1.08)`, × fully visible. The × on inactive tabs rests at 30% opacity.
- **New tab:** a separate 36×36 rounded **+** button right after the tray, not inside it.
- **Entry animation:** `v8tabin`, opacity 0 → 1 and `translateY(-3px) scale(.97)` → none, `.28s`.

### 3.3a Project-scoped tabs
- `groupOf(tab)` = git root of the tab's **focused** pane's cwd, or `_other`. The tab bar shows only `tabs.filter(t => groupOf(t) === groupOf(activeTab))`.
- **Switching projects:** selecting a session in another group shows that group's tabs. If a session `cd`s into another repo, its tab moves with it.
- **+ after the tabs:** opens a new shell in **the current project's root** (or the start folder in Other folders). Its label says "New session in maitty". The sidebar's **+ New** always uses the start folder.
- **⌃Tab / ⌃⇧Tab:** cycle only through the visible tabs.
- **Closing a tab:** jumps to the next tab in the same group, or to any tab if none are left.
- **Tab status marks:** spinner (working), amber pause (needs you), green check (done), red "!" (exited), shown for the worst status inside the tab.

### 3.3b Tab overflow
- **Scrolling:** the tablist is `position:relative; overflow-x:auto; scroll-behavior:smooth`. Vertical wheel scrolling maps to `scrollLeft`.
- **Edge fade:** when overflowing, apply `mask-image: linear-gradient(90deg, transparent 0, #000 28px, …, #000 calc(100% - 28px), transparent 100%)`, using only the side(s) that can scroll.
- **‹ › buttons:** 26×36, shown only when that direction can scroll. Each click scrolls by 70% of the width.
- **Active tab into view:** on tab change, scroll it into view with a 30px margin. Compute from `getBoundingClientRect` relative to the tablist (not `offsetLeft`). **Don't use `scrollIntoView`.**
- **All tabs** (shown only when overflowing): a count button with a chevron after **+**, and an amber diamond if a tab here needs you.
  - The dropdown is anchored `right:0` (opens leftward), 280px wide, max 360px tall.
  - Rows: icon, **process**, folder, spinner / "Needs you", pane count. The current tab is raised.
  - Closing: Esc or clicking outside.

### 3.4 Pane
- **Card:** as §2, entering with `v8pane` (opacity plus `scale(.985)` → 1, `.3s`). Hovering brightens the border to `rgba(255,255,255,.14)`.
- **Focus** (when there's more than one pane): border `--accent-line`, ring `0 0 0 3px var(--accent-soft)` plus a deeper shadow.
- **Header** (44px):
  - 20×20 app icon, **process** (14/600), cwd (13, `--muted`);
  - **status pill** (22px, radius 999): working `--accent-soft`/accent with the spinner and `m:ss`; Needs you `--warn-soft`/warn; Done green 12%; Exited danger-soft;
  - **repo pill** (git-branch icon in `--ansi-mag`, repo name);
  - controls (28×28, radius 8): **Split right** (⌘D), **Split down** (⌘⇧D), **Maximize / Restore** (also double-click the header), **Close** (disabled at 30% with a single pane; closing detaches, never kills).
- **Header behavior:** the header is the drag handle. Dropping one pane onto another **swaps their sessions**, and the target shows a 2px dashed accent outline.
- **Body:** Geist Mono at the user's size (12–16, default 13), line-height 1.6, padding `4px 18px 16px`, auto-scrolls to the bottom on new output and on (re)mount.
- **Layout:** columns of stacked panes, max 4. Split right inserts a column after the current one; split down adds to the same column. Maximize shows a single pane until restored.

### 3.5 Terminal colors (ANSI palette on top of the theme)
| Token | Midnight | Use |
|---|---|---|
| `--accent` | `#5FD4BF` | prompt path (600), paths in commands, `•` step bullets |
| `--ansi-mag` | `#C3A6F2` | git branch `(main)`, repo pill icon |
| `--ansi-blue` | `#82AAFF` | flags (`-p`), directories in `ls` (600) |
| `--ansi-green` | `#9FD39A` | file names in agent steps, quoted strings, `+N` |
| `--ok` | `#8DB89B` | `ok`, `N passed` |
| `--danger` | `#F07F78` | errors, `−N` |
| `--warn` | `#F2B65A` | agent approval prompts only |
| `--dim` | `#75808B` | `❯`, banners, timestamps |

In production these come from the PTY's real ANSI output rendered by xterm.js. Map the theme's 16 ANSI colors from these tokens. The prototype's colorizer is a stand-in for that.

### 3.6 Empty and offline states
- **No sessions:** a centered 440px column.
  - 40px accent tile with a terminal glyph;
  - "No sessions running" (22/600);
  - explanation (opens a shell in `~/code`; cd into a project and it shows up; start an agent like `codex`);
  - **New session ⌘T** (primary) and **Change start folder**;
  - note: sessions live in maittyd.
- **Daemon disconnected:**
  - a danger-soft banner under the tabs ("Disconnected from maittyd. Sessions keep running… last known screen. Retrying in Ns.") with **Retry now**;
  - every status becomes Unknown (dashed square), spinners and shimmer stop, inputs are disabled, and the sidebar footer shows reconnecting.

## 4. Tokens

### 4.1 Theme tokens (per theme, set on the app root)
`--bg --main --raise --hover --sel --line --line2 --well --term --fg --fg2 --muted --dim --accent --accent-soft --accent-line --on-accent --warn --warn-soft --warn-line --on-warn --danger --danger-soft --danger-line --ok --ansi-blue --ansi-mag --ansi-green --pane --pane-line`

**Midnight (default):**
- Surfaces: bg `#090C10`, main `#0D1116`, raise `#141A21`, sel `#1A212A`, line `#1B222B`, line2 `#26303B`, well `#080A0D`, term `#06080B`, pane `rgba(14,18,24,.86)`, pane-line `rgba(255,255,255,.07)`.
- Text: fg `#E8ECEF`, fg2 `#B3BCC5`, muted `#8C96A1`, dim `#75808B`.
- Accent: `#5FD4BF` (soft 11%, line 50%).
- Fixed status colours: warn `#F2B65A`, danger `#F07F78`, ok `#8DB89B`.

**Generated themes** (26 in the prototype): derive every token from six inputs (`bg, fg, accent, mag, blue, green`, plus `light`) with `color-mix(in oklab, …)`:
- `main = bg`
- `raise = mix(bg, fg 7%)` (5% light)
- `sel = mix(bg, fg 10%)`
- `line = mix(bg, fg 9%)`, `line2 = 15%`
- `bg = mix(bg, black 18%)`, or `mix(bg, fg 4%)` when light
- `fg2 / muted / dim = mix(fg 80% / 62% / 52%, bg)`
- `pane = bg at 90%`
- `pane-line = fg at 10%`

**Warn and danger never change meaning** and use fixed light/dark values. This is how Maitty's ~170 TUI themes import: map each TUI theme's 6 inputs and generate the rest.

### 4.2 Type
- **Fonts:** Geist (UI) and Geist Mono (terminal, paths, counts, shortcuts, spinners). Ship them with the app (`npm i geist`); don't load from a CDN.
- **UI sizes:** 20/600 settings page title · 15/600 dialog titles · 14/600 pane process · 13.5 sidebar and tabs · 13 body · 12.5 secondary · 12 captions · 11/600 caps labels (.08em).

### 4.3 Status marks
| State | Mark |
|---|---|
| Working | TUI spinner frame in `--accent` (Braille `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` 80ms · Snake `⠏⠛⠹⢸⣰⣤⣆⡇` 90ms · Bars `⡀⣀⣄⣤⣦⣶⣷⣿⣷⣶⣦⣤⣄⣀` 85ms; use the TUI's exact strings) |
| Needs you | amber tile with two dark pause bars, pulsing `box-shadow 0→4px` over 2.4s |
| Done | dark green tile with a green check |
| Exited | dark red tile with a red "!" |
| Idle | no badge (sidebar) / no pill (pane) |
| Unknown | dashed outlined square, everything dimmed |

### 4.4 App icons (process tiles)
Rounded squares (radius ≈ 28% of size) with an inset 1px white 12% ring and a `0 1px 2px` shadow:
- **zsh / shell:** `linear-gradient(140deg,#3A4656,#151B23)`, glyph `>_` in teal.
- **codex:** `linear-gradient(140deg,#FFFFFF,#C7CFD8)`, generic `✻` glyph in near-black. Don't use OpenAI's mark.
- **cargo / rust tools:** `linear-gradient(140deg,#F4B07A,#C9622F)`, `rs` in dark brown.
- **Everything else:** the shell tile. Add more tiles as needed (node, python, git…), keeping each one generic.

### 4.5 Motion (all off under Reduce motion, setting or OS)
| What | Spec |
|---|---|
| Hover color/bg | `.14–.18s ease` |
| Press | `scale(.88–.98)`, `.12s` |
| Hover lift | tabs `translateY(-1px)`, rows `translateX(2px)`, rail and theme cards `scale(1.06)` / `translateY(-1px)`, `.2s cubic-bezier(.2,.8,.2,1)` |
| New tab / pane / row | `v8tabin` / `v8pane` / `v8row` (4px slide + fade), `.26–.3s` |
| Folder open | chevron rotate + content fade-rise `.2s` |
| Sidebar collapse | width `.32s cubic-bezier(.2,.8,.2,1)` |
| Session moved to another repo | row background flashes `--accent-soft` → transparent, 1.6s |
| Status motion (sidebar rows, tray items, tabs; stronger on selected/active) | **Working:** continuous teal sweep — 100° gradient band, `background-size 250%`, position 150% → −50%, 2.6s infinite. **Needs you:** no sweep; an amber radial glow anchored behind the icon breathes (`background-size 90%→170%`, 2.4s ease-in-out infinite) — attention without travel. **Done:** one green sweep, 1.4s, plays once then settles clean. All off when disconnected or Reduce motion. |
| Theme switch | cards transition `background-color .35s` |
| Settings / drawer | scrim fade `.2s`, card `v8pane`, drawer slide from the left `.28s` |

### 4.6 Scrollbars
WebKit (Tauri on macOS/Linux, WebView2 on Windows):
```css
*::-webkit-scrollbar{width:8px;height:8px;background:transparent}
*::-webkit-scrollbar-button{display:none}
*::-webkit-scrollbar-track{background:transparent}
*::-webkit-scrollbar-thumb{background-color:rgba(160,175,190,.16);border-radius:999px;border:2px solid transparent;background-clip:padding-box;min-height:32px}
*::-webkit-scrollbar-thumb:hover{background-color:rgba(95,212,191,.45)}
*::-webkit-scrollbar-thumb:active{background-color:rgba(95,212,191,.7)}
```
**Don't** also set `scrollbar-width`/`scrollbar-color` globally: Chrome 121+ then ignores the WebKit pseudo-elements and draws the default bar with arrows. Use them only as an `@supports not selector(::-webkit-scrollbar)` fallback. In xterm.js, style `.xterm-viewport` the same way.

## 5. Settings (⌘,)
- **Dialog:** 880×620 max, radius 16, blurred `--pane` card over a blurred 50% scrim.
- **Left nav** (212px, 176 under 900px):
  - groups ESSENTIALS / TERMINAL / WORKSPACE / ADVANCED, as caps labels at 10.5/600;
  - items: a 22px tinted icon tile with a line icon, the label, and a mono shortcut letter. While Settings is open and focus isn't in a text field, the letter jumps straight to that section;
  - selected item: `--raise` with an inset hairline.
- **Content:** page title (20/600) and subtitle, then **cards**: a 46px header (18px tinted icon, 13.5/600 title, right-side meta or control), rows split by `--pane-line`, and an optional dim footer note. Esc or × closes.

| Section (key) | Content |
|---|---|
| General (G) | Start folder: a `cd` + mono input, validated live ("Saved" / red "Folder not found"). Sessions: Reopen tabs on launch (switch); closing the window always keeps sessions running. |
| Text & cursor (T) | Size 12–16 (segmented). Cursor: Block / Bar / Underline (applies to real panes). A live mini-terminal preview. Scrollback 5k / 10k / 50k (the GUI drops old lines; maittyd keeps everything). |
| Appearance (A) | **Sidebar:** "Other sessions in a project": With names / Icons / Stacked; "Main row shows": Focused / Needs attention. |
| Appearance (A) | **Theme browser:** search, All/Dark/Light filter, a scrollable 2-column grid of compact cards (46×32 swatch: bg with accent/fg/mag/blue/green bars, name, "Family · dark"), current theme pinned first, "N installed", "Showing X of N", **Import from Maitty TUI**. **Working spinner:** 3 live cards, each animating its real frames with a sample `cargo test` line. **Motion:** Reduce motion switch (master), then **Status effects**: separate switches for Working sweep, Needs-you glow and Done sweep, each with a live preview chip. Off = static icon + word only. Disabled (dimmed) while Reduce motion is on. |
| Notifications (N) | Background session needs you / finishes / exits with an error (switches). |
| Hotkeys (?) | Full list (§6), rendered as **keycaps**: each key is a `<kbd>` with min-width 24px and height 24px (auto width for "Tab"), padding 0 7px, radius 6. Background `linear-gradient(180deg, raise + 6% white, raise)`, border 1px `--line2` with a **2.5px bottom border**, shadow `0 1px 0 rgba(0,0,0,.45), inset 0 1px 0 rgba(255,255,255,.07)`. Modifier symbols ⌘⇧⌃⌥ in Geist at 13px, other keys in Geist Mono 11.5px/500, 4px gap. Alternatives are separated by a dim "or". On Windows/Linux, render Ctrl/Shift/Alt words instead of symbols. |
| Daemon (D) | Connected/Disconnected, version, socket path, session count. |

Controls:
- **Segmented control:** `--well` tray, selected `--raise` + 500 weight.
- **Switch:** 30×17, accent track when on, knob slides `.2s`.
- **Selected list items:** `--accent-soft` + `inset 0 0 0 1px var(--accent-line)` + accent check.

## 5b. Inspect panel (⌘I)
A floating card on the right, opened by the **Inspect** button at the right end of the tab bar. The button shows the repo's `+N −N` when closed. The panel is scoped to the **focused pane's repo** (or its cwd if outside a repo). Every action runs a real command **in the focused pane**, so the user always sees what git does. The panel never writes on its own.

- **Placement:** `top:56 right:12 bottom:12`, radius 14, the same card treatment as the panes. It enters with a slide from the right (`translateX(18px)` → 0, `.32s`).
  - **Docked (≥1100px):** the panes container gains right padding equal to the panel width plus 24, and transitions `.32s`.
  - **Compact:** it overlays the panes with a 45% scrim. Esc or × closes it.
- **Width:** 360px. The **Files** tab widens it to `min(640, window − 264 − 460)`.
- **Header:**
  - project tile (teal gradient in a repo, slate outside), name, and a branch pill in `--ansi-mag`;
  - a segmented tab tray: Files · Changes (count) · Branches · History · References (count).
- **Files:**
  - **Two columns** when the panel is ≥520px: a 220px tree with a right hairline, and the preview filling the rest on `--term`. Below 520px the tree stacks above the preview (42% max height).
  - **Tree:**
    - a caps project label at the top;
    - rows 26px, radius 6, with one 12px indent guide per depth level (`inset 1px 0 0 var(--pane-line)`);
    - folders get a chevron that rotates 90° over `.18s` and an open/closed folder icon in `--ansi-blue`;
    - file icons are tinted by extension (rs `#E8946A`, ts/tsx blue, md mag, toml/json green);
    - M (warn) / A (green) git tags, and changed folders show "•";
    - selected row: `--raise` + hairline. ARIA: `role=tree/treeitem`, `aria-expanded`.
  - **Preview:** a 38px header (file icon, dim dir + bold name, "modified" pill, **Open in pane** → `cat <path>`, ×), then line numbers and syntax highlighting from the ANSI tokens. Empty state: "Select a file to preview it". Read-only by design; editing happens in the user's editor or terminal.
- **Changes:** a file list (status tile, dim dir + name, `+a −d`). Selecting one shows its unified diff below:
  - added lines: green 9% background + 2px green left edge;
  - removed lines: red 9% + red edge;
  - hunk headers: blue.
  - Footer: **git diff in pane** and **git status**. Outside a repo: "Not in a git repo…".
- **Branches:** rows of 52px with the branch icon, mono name, last commit subject and ahead/behind (`↑2 ↓1`). The current branch has an accent-soft fill and a "current" pill. The others get **Checkout** → `git checkout <name>` in the pane, and the prompt and branch pills update.
- **History:** a vertical timeline (1.5px rail).
  - **Commits:** 8px violet dots.
  - **Checkpoints** (agent snapshots from maittyd): 10px teal rounded squares plus a "checkpoint" pill.
  - **Rows:** subject, short sha, author, relative time, `+a −d`. Clicking expands to show the files, **git show in pane**, and **Restore…**. Restore needs a second click to confirm (it turns amber, "Click again to restore") and then runs `git restore --source <sha> -- .`.
- **References:** a drop zone (dashed, accent when dragging over). It accepts drop, paste (while this tab is open) and browse. Thumbnails sit in a 2-column grid with **Insert path** (types `.maitty/refs/<name>` into the pane's input) and **Copy**. References are stored per repo under `.maitty/refs/`, which is git-ignored by default.
- **Daemon/data:** the panel needs `repo_status(paneId)`, `repo_tree(path)`, `repo_diff(path)`, `repo_branches`, `repo_log(limit)` (merging checkpoints) and `refs_*`. These are read-only queries; mutations go through the PTY.

## 6. Keyboard

The prototype also binds Esc to close (in order) the All tabs menu, the side menu, Inspect, then Settings.

| Action | Keys |
|---|---|
| New session (new tab, start folder) | ⌘T |
| Split right · Split down | ⌘D · ⌘⇧D |
| Focus previous / next pane | ⌘[ · ⌘] |
| Close pane (or tab if last) | ⌘W |
| Next / previous tab | ⌃Tab · ⌃⇧Tab |
| Collapse sidebar | ⌘B |
| Inspect panel (Esc closes) | ⌘I |
| Settings | ⌘, |
| Interrupt · Clear | ⌃C · ⌃L (these go to the PTY) |

⌘ is Ctrl on Windows and Linux. Inside a focused pane the window intercepts **only** the shortcuts above. Everything else goes to the PTY.

## 7. Accessibility
- Tabs use `role=tablist`/`tab` with `aria-selected`. Panes are `<section aria-label="Terminal N: ~/path, focused">`. Every icon-only button has an `aria-label` and a `title`.
- Status is always shape + word (badge + "Needs you" pill + row phrase). Sidebar rows get `aria-label="folder, process, status"`.
- **Focus:** a visible 2px `--accent` ring on all controls (the prototype relies on browser defaults). Settings traps focus and restores it on close.
- **Text contrast:** at least 4.5:1 on cards in Midnight. Check generated themes in CI by computing the contrast of `--muted`/`--dim` against `--pane` and nudging the mix percentage up if it fails.
- Reduce motion stops the shimmer, pulses, slides and lifts; spinners show a static frame.

## 8. Implementation notes
- **Hover states:** drive tab, row and theme-card hover from React state (`onMouseEnter`/`onMouseLeave`) or put background in classes, not inline. Inline `background` beats `:hover` rules, which made hover invisible in an earlier build.
- **Sidebar grouping:** the daemon should report `cwd` per session (via OSC 7 from the shell, or by polling the foreground process cwd) and the containing git root. The UI groups by git root. Compute "moved" when the root changes and flash once.
- **Status:** working = a non-shell foreground process. Needs you = the agent adapter (Codex app-server events, or a prompt pattern) reports a pending approval. Done = the foreground process exited 0 while the session wasn't visible. Clear Done on the next input.
- **Panes:** see the v6 terminal notes (replay then stream with seq, no polling, never resize a pane the GUI doesn't own, xterm WebGL renderer, rAF batching).
- **Layout state** (`tabs[{id, panes[{id, sessionId, col}], focus, max}]`) persists locally. Sessions are the daemon's.
- **Themes:** store as 6 inputs plus a light flag. Generate tokens at runtime with `color-mix`, and derive xterm's `ITheme` from the same tokens.

## 9. Now vs later

**v12 additions:** Window style (Flat / Tiled / Glass), the settings header, split dark/light themes and High Contrast, presets, first run, shared config, status bar, rename/pin/archive, the shortcut overlay, and the rail redesign with popovers all ship in the first release. Importing all ~170 TUI themes can follow.

**Include in the first usable release:**
- sidebar (expanded and rail), repo grouping that follows `cd`, stable order;
- tabs, splits right/down, focus, maximize, swap by drag;
- status marks and pills, empty and offline states;
- Settings: General, Text & cursor, Appearance with Midnight/Abyss/Paper plus generated themes, spinner, motion; Hotkeys, Daemon;
- custom scrollbars, reduce motion.

- Inspect panel: Files (tree + preview), Changes (diff), Branches (checkout in pane), History (commits + checkpoints, restore with confirm), References
- Status effects switches (Working / Needs you / Done) under Reduce motion
- Project-scoped tab bar, tab overflow (fade, arrows, All tabs menu)
- Sidebar main row + tray (three styles), +N side menu, lead/priority setting
- Keycap hotkey list

**Later:**
- Import from Maitty TUI (all ~170 themes);
- notifications (OS + in-app cards);
- per-process icons beyond shell/codex/cargo;
- drag a session from the sidebar into a pane;
- pane resize handles;
- Claude agent adapter.

## 10. v10 features: specs

General rules for everything below:
- **Overlays** share the floating-card style from §2 (blurred `--pane` card, `--pane-line` border, deep shadow) and enter with `v8pane` or a slide.
- **Esc closes, innermost first:** project menu, All tabs menu, side menu, palette/inbox/worktree/find/tour, focus mode, Inspect, Settings.
- Clicking outside closes popovers.
- All new list views use `overflow-x:hidden`.

### 10.1 Command palette (⌘K)
- **Layout:** 600px wide, centered at 12% from the top, over a 50% scrim with a 3px blur. A 54px input row ("Jump to a session, project, action or theme…", plus an esc keycap), up to 14 results (max 380px tall), and a 36px footer hint (↑ ↓ move, ↵ open).
- **Sources:**
  - **Actions:** new session here, new worktree, split right/down, inbox, find, broadcast, focus mode, Inspect, settings, reset pane widths, start codex, and "Open worktree X" for every worktree.
  - **Sessions:** folder · process, then the project; shows a status spinner or "needs you".
  - **Projects** and **Themes:** themes only appear once the user types.
- **Matching:** substring match scores higher than a subsequence match. Results are sorted by score, and the kind ("Action", "Session", …) is right-aligned.
- **Row:** 40px; a 22px icon tile, label, project-colour dot, shortcut, kind. The selected row is raised; hover moves the selection; Enter runs it and closes.
- **Implementation:** shadcn `Command` (cmdk) is a good fit; keep our row styling.

### 10.2 Attention inbox (⌘J)
- **Panel:** slides in from the right; 440px, inset 12px, over a 45% scrim. Header: "Inbox", "N across all projects", **Clear done**, ×.
- **Contents:** every session whose status is needs you, error or done, across all projects, sorted needs → error → done, then by recency.
- **Card** (radius 12):
  - header: icon, project colour square, **project**, folder, then process · time on a second line, and a status word on the right;
  - a mono message well: the exact approval command, or the last output line;
  - actions:
    - needs you: **Yes, once (y)** in amber, **This session (a)**, **No (n)**. These **send the keystroke to that session's PTY**; the GUI never decides on its own;
    - done or error: **Clear** (sets the session back to idle) and **Open**.
- **Empty state:** a green check, "You're all caught up".
- **Inbox button:** in the tab bar, left of Inspect. The count badge is amber when any item needs you.

### 10.3 Native OS notifications
- **When:** for a session that isn't visible, when it needs you, finishes, or exits with an error. Each kind follows its switch in Settings → Notifications. At most 3 are queued; a newer notification for the same session replaces the older one.
- **Production:** use the Tauri notification plugin with actions.
  - Click opens that session (focus window → tab → pane).
  - The "Allow once" action sends `y` to the PTY.
- **Prototype mock:** shows them at the top-right of the screen.
  - 356px, radius 16, `rgba(40,44,52,.82)` with blur.
  - Maitty app tile, then "MAITTY", a project colour dot and the time; then the title ("Needs you · maitty") and body ("codex in (root) wants to run $ git push origin main").
  - Buttons: Allow once (amber, needs only), Open, Dismiss. Finished and error notifications auto-dismiss after 7s, with a thin progress line.

### 10.4 Session restore and crash recovery
- **On launch:** restore the saved tab and pane layout (stored locally), re-attaching to maittyd sessions by id.
- **After a daemon restart:** sessions whose agent process died mid-turn become **error** with a pane note ("maittyd restarted · codex exited (SIGHUP) mid-turn" / "History up to the last event is kept.").
- **Banner** under the tab bar: accent-tinted, with a restore icon.
  - Text: "Restored N tabs and N sessions after maittyd restarted." plus "1 agent stopped mid-turn: codex in maitty."
  - Buttons: **Resume codex** (runs `codex resume --last` in that pane) and **Dismiss**.
  - **Never auto-resume.**

### 10.5 Find (⌘F)
- **Bar:** floats at the top-right of the pane area (360px): search icon, mono input, "3 of 7" (red "No matches"), ↑ ↓ buttons, an **All sessions** toggle, ×.
- **Focused pane:** matching lines get `warn 9%` and the current match `warn 24%`, with the body scrolled to keep the match visible. Enter goes to the next match, ⇧Enter to the previous.
- **All sessions:** lists up to 8 hits (folder · process, then the matched line). Clicking a hit opens that session and scrolls to the line.
- **Production:** use the xterm.js search addon for the focused pane. Cross-session search goes through maittyd scrollback (`pane_search(query)`).

### 10.6 Broadcast input (⌘⇧B)
- **Scope:** per tab. When on, every pane in the tab gets a 1.5px violet (`--ansi-mag` 60%) outline and a "◉ Broadcast" pill. The header button is tinted violet.
- **Behaviour:** Enter in any pane sends the same line to **every idle pane** in the tab. Panes with a running process are skipped; never type into a busy agent.
- **Toast:** confirms "Broadcast on…".
- **Production:** write to each PTY via `pane_input`. This needs `may_use_terminal` for each pane.

### 10.7 Turn timeline
- **Strip:** an 8px strip on the right edge of each pane body (top 52px, bottom 14px). Each mark sits at `lineIndex / lines`.
- **Mark types:**

| Mark | Colour |
|---|---|
| Shell command | `--accent`, 4px |
| Agent prompt (`› …`) | `--ansi-mag` |
| Approval asked | `--warn`, 6px |
| Finished / tests passed | `--ok` |
| Error line | `--danger` |
| Current find match | `--fg`, 3px |

- **Interaction:** hover scales a mark ×1.6 and shows a tooltip; click scrolls to that line.
- **Production:** derive marks from OSC 133 prompt markers and from agent adapter events, not from regexes.

### 10.8 Checkpoint compare
- **Entry:** Inspect → History gets a **Compare** toggle; it reads **Done** while on, with a hint ("Pick 2 more…" / "Comparing checkpoint "X" → 8f3c2a1").
- **Picking:** in compare mode, clicking a row selects it (numbered teal badge 1/2, accent ring). Picking a third drops the oldest.
- **Result:** with 2 picked, a diff block appears above the timeline (unified diff, same colours as Changes), plus **Restore newer**, which runs `git restore --source <sha> -- .` in the pane.
- **Data:** `repo_diff(a, b)`.

### 10.9 Worktree sessions
- **New worktree** (⌘⇧N, palette, or a project's context menu): a dialog with the repo name, a **New branch** field (mono; validated against `[\w./-]+`), **From** (segmented, the repo's branches), and a path preview `~/code/.maitty-wt/<repo>-<branch>`.
  - **Create and open ↵** runs `git worktree add -b <branch> <path> <base>` in a new session and opens it in a new tab.
- **Grouping:** worktree sessions group under the **parent repo**. The group key is the parent repo of a worktree root, so tabs scope to the parent project. Rows show `⑂ <branch>`.
- **Open existing:** the context menu lists the project's worktrees (branch, plus "N sessions open" in teal or "created 2d ago"). Clicking focuses an existing session there, or opens a new shell in it.
- **Data:** `git worktree list --porcelain` per repo, cached by maittyd.

### 10.10 Usage meter
- **Pill:** in the pane header, for agent sessions with usage data: a bar-chart icon (`--ansi-blue`) and mono "18.2k tok", tooltip "Tokens used this session (reported by the provider)".
- **Only show real provider-reported numbers.** Hide the pill when unknown; never estimate. The prototype's counter is fake.

### 10.11 Pane resize
- **Handle:** a 10px handle between columns (`col-resize`). It's invisible at rest; on hover and while dragging it shows a 2px `--accent` line, `linear-gradient(90deg, transparent 3px, accent 3px 5px, transparent 5px)`.
- **Drag:** sets per-column flex-grow, with a 180px minimum per column. It snaps at 50/50 (±2.5%). Double-click resets to equal widths.
- **Persistence:** widths persist per tab in the layout state.

### 10.12 Drag a session into a pane
- **Sources:** any sidebar row or tray item is draggable.
- **Targets:** panes show a 2px dashed accent outline while a drag is over them.
- **Drop:**
  - if the session is already in this tab, it swaps with the pane;
  - if it's in another tab, it moves here;
  - the pane's old session detaches and keeps running.
- **Toast:** "Session moved into this pane".

### 10.13 Quick-look file links
- **Detection:** file paths in output (`*.rs|tsx|ts|md|toml`, resolved against the session's repo) get a dotted underline and the tooltip "⌘-click to open in Inspect".
- **⌘/Ctrl-click:** opens Inspect → Files with that file selected and its folders expanded. A plain click still focuses the terminal.
- **Production:** use the xterm.js link provider; extend to `path:line`.

### 10.14 Focus mode (⌘.)
- **Hides** the sidebar, the tab bar and the Inspect button.
- **Status strip:** a floating pill at bottom centre: **Focus** (accent) · project · folder · process, then "N need you elsewhere" in amber (or "Nothing else needs you"), then **Exit ⌘.**. Esc also exits.

### 10.15 Project colours
- **Palette:** `#5FD4BF #C3A6F2 #82AAFF #9FD39A #F2A0C4 #E8946A`. **Never amber or red**; those stay reserved for status.
- **Where it shows:** the project tile gradient (accent → 55% black when focused, 45% tint otherwise), plus small dots and squares in the inbox, palette, find results and notifications.
- **Setting:** Settings → Appearance → Sidebar → Project colors, an 18px swatch row per project with a selected ring. Store the colour in the project config.

### 10.16 Resizable sidebar
- **Handle:** an 8px handle **inside** the sidebar card's right edge, z-index above its content. The card has `overflow:hidden`, so a handle outside it is clipped.
- **Look:** a teal line on hover and while dragging, as in §10.11.
- **Range:** 208–440px; snaps to 264px within 8px; double-click resets.
- **Behaviour:** the width transition is off during the drag. Not available in rail or drawer mode.

### 10.17 Project context menu
- **Trigger:** right-click a project header (or its rail initials).
- **Menu:** 280px, radius 12, placed at the cursor and clamped to the window. Contents:
  - a caps project name;
  - **New session here** and **New worktree… ⌘⇧N**;
  - divider, "Open worktree", then the worktree list (§10.9), or "No worktrees yet";
  - divider, **Collapse/Expand**, **Inspect project**, **Copy path**.
- Other folders only gets New session and Collapse.

### 10.18 Hotkeys added
⌘K palette · ⌘J inbox · ⌘F find · ⌘⇧B broadcast · ⌘⇧N new worktree · ⌘. focus mode · ⌘I Inspect. These are shown as keycaps in Settings → Hotkeys (§5).

### 10.19 Daemon / backend additions
`pane_search`, `repo_diff(a,b)`, `worktree_list` / `worktree_add`, a checkpoint list merged into `repo_log`, usage events from the agent adapter, prompt marks (OS 133) per pane, and notification events (needs / done / error) with session id. **Names are suggestions; match maittyd's API.**

### 10.20 Not in this round
- **Priority sidebar layout:** tried and removed. Don't build it.
- **"Stacked" deck chips:** tried and removed. "Stacked" now means one session per line in the tray.

## 11. v11: specs

### 11.1 Style: Minimal / Expressive
- **What it is:** one setting that controls only the finish. It doesn't change layout, features or status colours.
- **Default:** Minimal.
- **Where:** Settings → Appearance → **Style**, two cards with a live mini preview (name, one-line description, check on the selected one).
- **Implementation:** set these CSS variables on the app root. Components read the variables, never the literals.

| Var | Minimal | Expressive |
|---|---|---|
| `--card` (sidebar, panes, overlays) | `var(--main)` flat | `linear-gradient(180deg,rgba(255,255,255,.035),transparent 140px), var(--pane)` |
| `--blur` (backdrop-filter) | `none` | `blur(18px)` |
| `--glow` (window background) | `var(--bg)` | two radial glows (§2) over `--bg` |
| `--sh` (overlay shadow) | `0 8px 24px rgba(0,0,0,.3)` | `0 18px 50px rgba(0,0,0,.5)` |
| `--r14` / `--r16` (card radii) | `8px` / `10px` | `14px` / `16px` |
| `--tray-m` / `--tray-p` | `0 0 4px` / `2px 0 2px` | `-6px 5px 4px` / `10px 5px 5px` |
| `--tray-bg` / `--tray-ring` | `transparent` / `none` | `rgba(255,255,255,.028)` / inset `--pane-line` hairline |
| `--tl-bg` / `--tl-ring` (tab tray) | `transparent` / `none` | `rgba(255,255,255,.025)` / inset hairline |

- **Minimal also changes:**
  - **App and project icons:** neutral `--raise` tiles with the glyph in `--fg2`, no gradients. Status badges keep their colours.
  - **No rings:** the selected sidebar row, tray items and the active tab lose their outline and shadow, keeping only the fill. Accent and warn rings, like the focused pane and needs-you, stay.
  - **Tray:** sits flush under the main row at full width, with no box.
  - **Status sweeps and glow:** default off. The per-state switches still work, and an explicit user choice wins over the default.

### 11.2 Inspect drawer
- **Behaviour:** an **overlay**, no longer docked. It never squeezes the panes.
- **Placement:** `position:absolute; top:12px; right:12px; bottom:12px` inside the main area (over the tab bar and panes). The sidebar stays visible. A scrim at `rgba(3,5,8,.32)` sits behind it; clicking the scrim, ×, Esc or ⌘I closes.
- **One width for every tab:** 680px default, so tabs never jump.
  - Resizable by an 8px handle on the drawer's **left** edge: a teal 2px line on hover and drag, `col-resize`.
  - Range 420px to window − 300px. Snaps to 680 within 10px; double-click resets.
  - Width persists. In compact windows it's full width minus 24px, with no handle.
- **Files** uses two columns whenever the drawer is ≥520px (always, at the default width).
- **Header** (replaces the old tile + pill + segmented tray):
  - **Row 1** (48px, padding `0 10px 0 18px`): project name 14/600, then the branch icon + branch in mono 11.5 `--muted`, then ×.
  - **Row 2** (38px): text tabs **Files · Changes · Branches · History · References**, left-aligned, 12.5px, padding `0 8px`.
    - Active: `--fg`/500. Inactive: `--muted`, hover `--fg`.
    - Counts: 16px round pills, `--accent-soft`/accent on the active tab, `rgba(255,255,255,.06)`/muted otherwise.
  - A 1px `--pane-line` divider sits under row 2.
- **Underline:** a 2px accent bar per tab (`left/right:6px`), `transform:scaleX(0→1)`, `.28s cubic-bezier(.2,.8,.2,1)`. The new tab grows from the side you came from while the old one shrinks toward the new one (`transform-origin` left or right by direction).
- **Open/close:**
  - In: `opacity 0→1, translateX(36px)→0`, `.36s cubic-bezier(.16,1,.3,1)`, with the scrim fading in.
  - Out: the reverse, `.22s cubic-bezier(.4,0,1,1)`. **Unmount after the animation (≈210ms)**, not immediately.
  - Reduce motion: instant.

### 11.3 Switch transitions
- **One keyframe pair per direction:** `from{opacity:0;transform:translateX(±14px)} to{opacity:1;transform:none}`, `.3s cubic-bezier(.2,.8,.2,1)`. Moving to a higher index enters from the right (+14px); lower enters from the left.
- **Applied to:**
  - the **panes area** when the active tab changes (click, ⌃Tab, All tabs menu, or picking a session in another project);
  - the **Inspect section body** on tab change;
  - the **Settings page** (title + content) on nav change.
- **Not on first mount.** Off under Reduce motion.
- **Implementation tip:** in React, re-key the container on change (`key={activeTabId}`) and pick the class by direction. The prototype alternates two identical keyframes to restart the animation.

### 11.4 Pane resize, both directions
- **Columns:** horizontal resize, as specified in §10.11.
- **Stacked panes** (Split down): the 12px gap *is* the handle (`role=separator`, `aria-orientation=horizontal`, `row-resize`).
  - Look: a teal 2px horizontal line on hover and drag.
  - Each pane keeps at least 90px. Snaps at 50/50 (±2.5%); double-click resets that column to equal heights.
  - Stored per tab as flex-grow per pane id (`tab.h[paneId]`), next to `tab.w[colKey]`.
- **Production:** `react-resizable-panels` maps onto this directly: nested PanelGroups, horizontal for columns and vertical inside each column.

### 11.5 Sidebar refinements (the v9 model stays)
- **Keep:** one main row per project plus its tray (§3.1a). We tried grouping by agent and by launched terminal/layout and removed both. **Don't build either.**
- **Tab badge:** the main row's second line ends with a small badge, `Tab 2` or `Tab 1 · 2` when the tab is split. It uses 10.5px `--dim` on `rgba(255,255,255,.05)`, radius 4. Tab numbers are counted among the project's visible tabs.
  - Tray tooltips include the same text, or "not open".
  - Clicking still focuses that exact pane.
- **Tray order:** needs you, then error, done, working, idle. On equal status, **agents** (any session where an agent has run) come before plain shells, then most recent first.
- **Resize handle** (§10.16): place it *inside* the card edge, because the card clips overflow.

### 11.6 Also in v11
- **Resize handles share one look:** sidebar, drawer, columns and rows all use the same teal 2px line on hover and drag.
- **Prototype top bar:** the **New features** button sits next to the title, and the bar wraps on narrow screens. These are prototype-only controls.

## 12. v12: specs

### 12.1 Window style: Flat / Tiled / Glass
- **Names:** "Minimal" and "Expressive" are retired. The setting is now called **Window style**; it changes the finish only. Features, layout and status colours stay the same.
- **Default:** Flat.
- **Where:** Settings → Appearance → Window style. Three cards, each with a live mini preview, name, one-line description and a check on the selected one. The prototype bar has the same switch.
- **Card descriptions:**
  - **Flat:** "Solid panels, soft corners, quiet".
  - **Tiled:** "Edge-to-edge panes, square corners, hairline dividers".
  - **Glass:** "Floating frosted cards, colour icons, status sweeps".
- **Implementation:** set these CSS variables on the app root (in addition to the §11.1 variables). Flat uses the old Minimal values and Glass the old Expressive values from §11.1.

| Var | Flat | Tiled | Glass |
|---|---|---|---|
| `--rk` radius multiplier (all `calc(Npx * var(--rk))` radii) | `1` | `0` | `1` |
| `--r14` / `--r16` | `8px` / `10px` | `0` / `0` | `14px` / `16px` |
| `--pg` pane gap and outer padding | `12px` | `4px` | `12px` |
| `--ptop` panes top padding | `0` | `4px` | `0` |
| `--tbh` tab bar height | `56px` | `40px` | `56px` |
| `--tbp` tab bar padding | `0 10px` | `0` | `0 10px` |
| `--tbg` gap between tab bar items | `6px` | `0` | `6px` |
| `--tbb` tab bar bottom border | `0` | `1px solid var(--pane-line)` | `0` |
| `--tabh` tab and tab-bar button height | `36px` | `40px` | `36px` |
| `--tabbr` divider between tabs | `0` | `1px solid var(--pane-line)` | `0` |
| `--sh` overlay shadow | `0 8px 24px rgba(0,0,0,.3)` | `0 6px 18px rgba(0,0,0,.35)` | `0 18px 50px rgba(0,0,0,.5)` |

- **Tiled detail:** the tab bar is a flush strip. Tabs, **+**, Inbox and Inspect sit edge to edge, divided by hairlines, with one line under the bar. Panes keep a 4px gap on every side, including next to the Inspect drawer.
- **Flat and Tiled share these:**
  - neutral grey icon tiles;
  - no nested outlines (§11.1);
  - status sweeps off by default.
- **Glass keeps:** colour icons, glass cards and sweeps on.
- **Persistence:** persist the style name; store old `minimal` as `flat` and `expressive` as `glass`.

### 12.2 Settings header
- **Layout:** the right column is a fixed **header** plus a scrolling body. The page itself never scrolls.
- **Header:** min-height 68px, padding `14px 14px 14px 28px`, bottom border `--pane-line`, background `--pane`.
  - Left: page title (18/600) and one-line description (12.5px `--muted`, ellipsis).
  - Right: a **Close** button with an esc `<kbd>` keycap, 32px tall, ghost style.
- **Page switching:** the title and description slide with the page (§11.3).
- **Body:** padding `20px 28px 28px`, `overflow-y:auto`, `scroll-behavior:smooth`.

### 12.3 Theme: appearance plus separate dark and light themes
The old grid of every theme is replaced by one card with three rows:
1. **Appearance**: segmented **System / Dark / Light**, each with an icon (monitor, moon, sun). The note underneath reads "Follows your OS. Right now: dark." / "Always uses your dark theme." / "Always uses your light theme."
2. **Dark theme**: a dropdown button showing a swatch, the name and the family, e.g. "Midnight · Maitty · default".
3. **Light theme**: the same, listing light themes only.

- **Dropdown button:** 250×38, `--well` background, inset `--pane-line` ring, which turns `--accent-line` while open; the chevron rotates 180°.
  - The theme in use gets an "in use" pill.
  - The row that isn't in use is dimmed to 62% opacity, except in System mode.
- **Dropdown menu:** 300px wide, right-aligned, 44px below the button.
  - A search field at the top ("Search dark themes").
  - A list (max 280px tall, scrolls): 34×24 swatch, name, family, and a check on the current theme. The current theme is listed first.
  - Footer: "N dark themes".
  - Esc in the search field closes it, as does a click outside.
- **Hover to preview:** hovering a theme in the list for the slot that's **in use** applies it to the whole app; leaving or closing restores the saved theme. Picking a theme commits it.
- **Fitting:** when a menu opens near the bottom of the settings body, the body grows extra bottom padding and scrolls just enough for the menu to fit. It must never be clipped.
- **Data:** settings keys `theme_mode = system|dark|light`, `dark_theme`, `light_theme`.
  - The active theme is `dark_theme` when the mode is dark, or when it's system and the OS is dark; otherwise `light_theme`.
  - Listen to `prefers-color-scheme` changes in System mode.
  - Themes are classified dark or light by their `light` flag (§4.1).
- **Footer:** "Amber and red keep their meaning in every theme…", plus **Import from Maitty TUI**.

### 12.4 High Contrast theme
- Pure black surfaces, white text.
- **Lines:** `#5A5A5A` / `#8A8A8A`, with a pane hairline of white at 30%.
- **Accent** `#3CF2D2`, **warn** `#FFC94D`, **danger** `#FF7A7A`, **ok** `#7CF29A`.
- Listed as "Accessibility · dark".

### 12.5 Project presets
- **What:** a named tab layout tied to a project. Each pane stores `{cmd, col}`, where an empty cmd means a plain shell.
- **Open:** a new tab in the project root; every pane starts a shell and runs its cmd. Toast: "Opened preset · Agent + tests".
- **Save:** "Save current tab as preset…" from the project menu, Settings → Presets or ⌘K.
  - Only allowed when the current tab belongs to a project; otherwise the toast says "Presets belong to a project. cd into one first."
  - The dialog has a name field (placeholder "Agent + tests"), a mini layout preview of the columns with each pane's command, Cancel, and **Save preset ↵**.
  - The command saved per pane is the running agent/process, or blank for an idle shell.
- **Project right-click menu:** a new "Open preset" section after the worktrees. Rows show a layout icon in `--ansi-green`, the name and the commands in mono. Then "Save current tab as preset…".
- **Settings → Presets (P):** one card per project.
  - Rows: a 64×40 layout thumbnail (agent panes in teal), name, commands, **Open**, **Delete**.
  - Footer button: Save current tab as preset.
- **Palette:** "Open preset "Name" (repo)".
- **Storage:** `presets` in the shared config (§12.7), keyed by repo path.

### 12.6 First run and TUI import
- **When:** on first launch (no desktop state yet). It can be reopened from Settings → General → Show the welcome again.
- **Dialog:** a 560px modal over a blurred scrim, with three progress bars at the top. The step content slides on change.
  - **Welcome:** a terminal icon, "Welcome to Maitty", a one-line explanation (same sessions as the TUI; closing never stops agents), and the **New sessions start in** field.
  - **Bring your TUI setup:** shown only when a TUI config is found.
    - The detected path with a green check and its age.
    - A list of switches: Theme, Spinner, Hotkeys, Start folder, each showing the TUI's value.
    - Note: imported settings stay linked.
  - **You're set:** six key shortcuts as keycaps, then **Start a session**.
- **Footer:** Skip (ghost), Back, and the primary button (Continue / Import and continue / Start a session).
- **While it's open:** global shortcuts are ignored.

### 12.7 Shared config with the TUI
- **Settings → General → Shared config** card.
  - A status pill: **In sync with TUI** (green) or **Desktop only**.
  - A switch, "Use the Maitty TUI config".
  - The path (`~/.config/maitty/config.toml`, or the desktop-only file when off), "synced Ns ago", **Open file**.
- **How it works:** theme, spinner, hotkeys, start folder and presets live in one TOML file. Watch it for changes and apply external edits live; write desktop changes back.
  - Toast on an external change: "Theme changed in Maitty TUI · now Nord".
  - Desktop-only settings (window style, sidebar width, layouts) stay in the desktop file.
- **Prototype only:** "Simulate TUI edit".

### 12.8 Status bar
- **Bar:** 28px tall, at the bottom of the main area, with a top hairline. Text is 11.5px `--muted`, on one line, clipped.
- **Left:**
  - daemon dot + "maittyd" (or "reconnecting…");
  - git branch in mono with the branch icon;
  - spinner + "N working" in teal;
  - amber "N needs you", which opens the Inbox.
- **Right:** focused process · path in mono, text size, and a ⌘/ shortcuts button.
- **Visibility:** hidden in Focus mode. Toggled in Settings → General → Window → Status bar (default on).

### 12.9 Rename, pin, archive
- **Right-click** any sidebar session row or tray item to open a 260px menu.
  - A **Name** field with the folder as placeholder. Enter saves; empty clears.
  - Then **Pin to top / Unpin**, **Open**, and **Archive** (labelled "keeps running").
- **Name:** replaces the folder title everywhere: sidebar, tray tooltip, palette, inbox.
- **Pinned:**
  - shows a pin icon on the main row;
  - leads its project when no focused session is in it;
  - sorts first in the tray.
- **Archived:**
  - removed from projects and tabs;
  - listed under a collapsible **Archived N** section at the bottom of the sidebar, with **Restore**.
  - **The session keeps running in maittyd**; archive is a desktop view setting, never a kill.
- **Data:** `name`, `pinned`, `archived` per session id, stored by maittyd so the TUI can show the names too.

### 12.10 Shortcut overlay (⌘/)
- **Dialog:** 820px, centred over a blurred scrim. Title "Keyboard shortcuts", "Close esc".
- **Content:** an auto-fill grid (230px min) of groups: SESSIONS, PANES, FIND & JUMP, WINDOW. Each row is an action plus `<kbd>` keycaps (§5 keycap spec).
- **Shortcuts:** ⌘/ toggles it; Esc closes.
- **Generate it** from the same keymap table that drives Settings → Hotkeys.

### 12.11 Collapsed sidebar (rail) redesign
- **Project group:** a 48px-wide rounded container (radius 14), padding `4px 0`, gap 4.
  - **Background:** `color-mix(accent N%, transparent)`, where N is 9 when open and 6 when collapsed, +4 if the group holds the focused session.
  - **Border:** an inset 1px ring at accent 20% (34% when focused).
  - Other folders uses `#8C96A1` as its colour.
  - The focused project also gets a 3px accent bar at the rail's left edge.
- **Folder tile** (40×36 button):
  - a 30×26 folder outline filled with accent at 14% (26% when focused), stroked with accent at 55%;
  - a **2-letter label** inside: initials of the hyphenated parts (`maitty-docs` → MD), else the first two letters (`maitty` → Ma). Other folders shows "…".
  - 10.5/700, `--fg` when focused, otherwise accent 70% mixed with fg.
- **Badges on the tile:**
  - top-right: an amber square when something needs you;
  - when collapsed: a count pill bottom-right, and a spinner bottom-left if anything is working.
- **Open groups:** a 20px hairline under the tile, then the session icons.
- **Clicks:** clicking the tile toggles collapse; right-click opens the project menu (§10.17).

### 12.12 Rail popovers
- **Only in rail mode.** They open on hover or keyboard focus after **280ms**; once one is open, moving to another item swaps it instantly. They close 160ms after the pointer leaves both the item and the popover, so the popover itself can be used.
- **Placement:** 10px right of the item, vertically centred on it and clamped to the window. A 9px rotated-square arrow points at the item.
- **Look:** `--main` background, `--line2` border, radius 12, shadow `0 16px 44px rgba(0,0,0,.5)`. It enters with a 4px slide + scale .97, 0.16s.
- **Project popover** (260px):
  - header: colour square, name (13.5/600), path in mono, branch pill;
  - every session in priority order: icon, name + process, status word (teal timer while working, "needs you" amber, "done" green, "exited" red); clicking one opens it;
  - footer hint: "Click the folder to collapse · right-click for more".
- **Session popover** (240px):
  - project colour + "project · name";
  - status word;
  - process · path in mono;
  - a mono well with the pending approval command (amber) or the last output line;
  - hint: "In tab N · right-click to rename, pin or archive".
- **ARIA:** `role=tooltip`, and `aria-describedby` from the item in production. They replace the native `title` tooltips in the rail.

### 12.13 Hotkeys added
⌘/ shortcut overlay. (Everything from §10.18 still applies.)

### 12.14 Backend additions
- Session metadata (`name`, `pinned`, `archived`) in maittyd.
- A shared config file watcher.
- Detecting the TUI config path.
- **Names are suggestions; match maittyd's API.**
