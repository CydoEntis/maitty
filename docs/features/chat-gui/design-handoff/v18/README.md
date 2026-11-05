# Maitty Desktop v18: Terminal-first GUI, handoff (ship build)

**Prototype:** `Maitty Desktop v18.dc.html`. **Components:** `Maitty Components.dc.html` (v17 additions at the bottom). **Glass/Tiled detail:** `GLASS_AND_TILED.md` and `screens/`. **Precedence:** §D (v18) wins over §C (v17), which wins over §B (v16), which wins over §A (v15), §0 (v14) and everything below. **Where the README and the prototype disagree, the README wins.**

**Stack:** Tauri + React + TypeScript + shadcn/ui primitives, xterm.js for panes. The prototype fakes the shell and the daemon. **Don't port its code**; lift the values and behavior.

**Repo not read.** Reconcile token names with `docs/DESIGN.md`, components with `apps/gui/src`, and daemon calls with `crates/maitty-core/src/daemon/` (`pane_screen`, `pane_replay`, `pane_input`, `pane_resize`, `may_use_terminal`) and ADR 0045. Pane streaming details are in `../design_handoff_maitty_desktop_v6_terminal/TERMINAL_IMPLEMENTATION.md`.

---

## 1. Product model

- **The app is a terminal multiplexer window.** No chat UI. Every session is a real shell PTY owned by `maittyd`. The GUI only attaches.
- **A new session opens a shell in the start folder** (Settings → General, default `~/code`). The user `cd`s wherever they want.
- **The sidebar follows the shell.** Sessions are grouped by the **git repo containing their current cwd**. When a session `cd`s into or out of a repo, it moves to that group with a short teal highlight and a toast ("Session moved to maitty"). Sessions outside any repo sit under **Other folders**.
- **Tabs hold layouts; panes hold sessions.** A tab is a split layout of 1–4 panes. Each pane shows one session. Closing a pane or tab **ends its session(s)**, with 6s Undo (§0.3). Only **closing the window** leaves sessions running in maittyd. Clicking a sidebar session focuses its pane if it's open in a tab; otherwise it opens **in a new tab** (never into another project's layout).
- **Agents are just processes.** `codex` (or any agent) runs inside a shell. Status comes from the daemon (foreground process + agent state), not from a chat protocol. Approvals are answered **in the terminal** (y / a / n); the GUI only surfaces that one is waiting.
- **Status vocabulary:** Working, Needs you, Done, Exited (error), Idle, Unknown (daemon disconnected). Each has a distinct shape (§4.3), never color alone.
- **"Done" is short-lived:** it clears to Idle as soon as the user runs the next command in that session.
- **Sidebar order is stable.** Rows keep creation order inside a group and never reorder on status changes. Groups are sorted by name, with Other folders last.

## D. v18: Glass, Tiled, status effects, spinners, settings (read first)

v18 = v17 plus a redesign of the **Glass** and **Tiled** styles, new status effects, web spinners and a few Settings cleanups. Flat is unchanged. The full token tables, contrast numbers and CSS for Glass and Tiled are in **`GLASS_AND_TILED.md`**; this section summarises them and adds everything else.

> **Not done until measured.** Run `apps/gui/scripts/perf/webkit-harness.py` scenarios **a** and **b**, before and after, and put the numbers in the PR.
> - **Budget:** quiet session under **5%** of one core, streaming under **30%**.
> - **Above budget** means the design isn't done, however it looks.
> - **Also attach:** screenshots in Midnight, Monokai, Paper and Day, each with one Claude Code pane running, one idle shell and Settings open.

### D.1 Glass (summary; full spec in GLASS_AND_TILED.md)
- **Ground:** built from each theme's `--bg` plus two faint accent and magenta tints. It's no longer hardcoded dark, which was what broke light mode.
  - **Background glow switch:** Settings → Appearance, **off by default**. When off, the ground is flat `--bg`.
  - **Software path:** `data-gpu="software"` always uses flat `--bg`.
- **Large cards** (sidebar, panes, tab bar, status bar, drawers, session panel):
  - **Fill:** opaque `--main`, 1px `--pane-line` hairline.
  - **Highlight:** dark themes get a 1px top highlight (`inset 0 1px 0 rgba(255,255,255,.045)`); **light themes get none.**
  - **No resting drop shadow and no backdrop blur.** This removes the heavy shadow that made sessions look smudged in light Glass.
- **Blur** (`--blur-pop`) only on short-lived surfaces: menus, popovers, tooltips, listboxes, find, centred dialogs and their scrims. Drawers that stay open are never blurred.
- **Popover shadow** `--sh-pop`: a soft shadow in light themes, a deeper one in dark. On the software path it's replaced by a 1px hairline ring.
- **Terminal:** opaque on `--term`, and ANSI black equals `--term`. A see-through terminal is opt-in only.
- **Themes:**
  - **Paper** is retuned and **Day** is new; every text token passes WCAG AA on `--term` and `--bg`.
  - **Monokai** `--dim` still needs checking in the real theme file.
  - **Generated light themes** need a CI contrast check; that's the separate light-mode contrast ticket.
- **Focus border:** a single even `--accent-line` border all the way round the focused pane. **No extra 2px line on the header**, which used to make the top look doubled.

### D.2 Tiled (hairline grid)
- **Spacing and depth:**
  - pane gap and outer margin **0**; every region is separated by a 1px hairline;
  - radius 0, no shadows;
  - popovers are hairline-only, with no blur.
- **Chrome type:** tab titles, pane process names, project names and status-bar text use Geist Mono, uppercase, letter-spacing .1em, 11–11.5px. Terminal text and session names are unchanged.
- **Corner ticks:** static 9px "+" ticks at the pane area's corners (`--tick`, transparent outside Tiled).
- **Tab bar:** a 40px flush strip, with tabs and controls separated by hairlines.

### D.3 Status effects (rows and tabs)
- **Per status, the user picks the style:** Settings → Appearance → Status effects has, for **Working**, **Needs you** and **Done**, an on/off switch, a **Waves / Sweep** choice, and a live preview chip that is clipped to the chip.

| Effect | What it is | Defaults |
|---|---|---|
| **Waves** | Two or three soft sine-wave bands (SVG paths, status colour at low alpha) drifting slowly across the row, each at a different speed and opposite directions, like a calm wave screensaver | Working |
| **Sweep** | One soft gradient band that crosses the row once when the status appears (1.4s), then rests | Needs you, Done |

- **Waves scope:** a **Working effect** setting (All / Selected / Off). "All" means every working row and tab animates the same, selected or not.
- **How it's built (both effects):**
  - an absolutely positioned child of the row, `overflow:hidden; contain: paint`;
  - animates **only `transform`** (`translate3d`), with `will-change: transform`;
  - never `box-shadow`, `filter` or `background-position`.
  - Waves: a 200%-wide SVG translating -50% over 18s linear. Sweep: one band translating -110% → 330%.
- **Off** for disconnected sessions, under Reduce motion, and when Performance → "Animations" is switched off.

> **⚠ Conflict to resolve with perf:** Waves **loops while a session is working**. That goes against the "nothing loops" rule in GLASS_AND_TILED.md §4. It's kept because you asked for it, but it's built to be the cheapest possible loop: one composited transform layer per row, nothing repainting.
> - **Measure it** with the harness (scenario b, with several working rows).
> - **If it's over budget,** change the default to **Selected** or **Sweep**. Don't drop the option.
> - **Software GPU:** fall back to Sweep automatically on `data-gpu="software"`.

### D.4 Working spinner (dropdown)
- **Where:** a **dropdown row** in the Theme card, under Dark theme and Light theme. The old big spinner card is gone.
- **Button** (220×40): a live preview tile, the name, and its source ("From the Maitty TUI" / "Web icon"), plus a chevron.
- **List** (260px, max 340px tall, opens upward when there's no room):
  - two groups, **MAITTY TUI** (Braille, Snake, Bars) and **WEB** (Circle, Dots, Equalizer, Spokes, Ping);
  - every option previews live, and the selected one is ticked;
  - Esc or clicking outside closes it.
- **Web spinners** are inline SVG drawn in `currentColor`, sized `1em`, so they take the status colour and the text size anywhere a spinner appears:

| Name | Shape | Animation |
|---|---|---|
| Circle | faint ring with a 90° arc | rotate 360°, .8s linear |
| Dots | three dots | scale .5 → 1 and fade, 1s, staggered .16s |
| Equalizer | three bars | scaleY .35 → 1, .9s, staggered .15s |
| Spokes | eight spokes at rising opacity | rotate in steps(8), .9s |
| Ping | dot plus an expanding ring | ring scale .35 → 1 and fade, 1.3s |

- **Spinner rules:**
  - transform/opacity only;
  - each spinner sits in its own `contain: paint; will-change: transform` span;
  - static under Reduce motion.
- **TUI spinners** still use the TUI's exact frame strings, and plain-text places (hover-card timers) keep the text frames.

### D.5 Settings cleanups
- **Close** is a plain 30×30 **×** icon button (tooltip "Close (Esc)"), like every other panel. No "Close Esc" text.
- **Project colours removed.** There's no user-set colour per project; every project tile uses the neutral tile. Remove the setting, the swatch row, and the colour dots that depended on it.
- **Background glow** switch added (§D.1).

### D.6 Removed in v18 (don't build)
These override any earlier section that describes them (C.2, C.3, C.4 and the v17 status-row spec).
- **Overview screen** (C.3): the sidebar entry, its badge, the rail button, Ctrl/⌘0 and the ⌘K entry are all gone. **Jump to next needing you (Ctrl/⌘J) stays.**
- **New agent on a branch** (C.4): the sidebar button with the branch glyph, the rail button and the ⌘K entry are gone. Sessions still show their branch under the name when they have one.
- **Session side panel** (C.2): the right-hand Checklist / Agents / Changes·Files·Artifacts panel, its tab-bar toggle, Ctrl/⌘\ and its ⌘K entry are gone. Changes, Files, Branches, History and References are still in the activity bar (§A).
- **Step details in sidebar rows:** no current checklist item, no "2/5" progress bar and no "Ran a subagent…" sub-step line. Agent rows use the same one-line status as every other row.
- **Project colours** (§D.5).

### D.7 Usage & cost moved into Settings
- **Location:** Settings → **Workspace → Usage & cost** (key letter U), above Notifications. Ctrl/⌘⇧U and ⌘K open Settings on that page.
- **Contents:** unchanged from C.7 (week total, optional budget ring, Day / Week / Month, the sortable table, the subscription note, the empty state). It's rendered as one Settings card: radius 12, `rgba(255,255,255,.025)` fill, hairline, no shadow or blur.
- **Removed:** the bar-chart button in the sidebar footer (expanded and rail) and the full-screen usage view.

### D.8 Sidebar sessions keep their place
- **Fixed order:** sessions inside a project always render in **creation order** (`seq`). Status, pin and focus never reorder them (the v16 rule that order never changes, now applied inside each project too).
- **Focused session:** shown as the full card **in its own position**. The project's other sessions are compact tree rows above and/or below it.
- **Click a compact row:** it grows into the full card in place (`v18grow`: opacity .55 → 1, scale .97 → 1, .28s), and the previous card shrinks back to a compact row. Nothing moves up or down. Off under Reduce motion.
- **Which session is the card:** the focused session if it's in this project; otherwise the last one picked here; otherwise the first by creation order.
- **Remove the Settings option** "Main row shows: Focused / Needs attention"; it no longer does anything. Pinning still exists (menu + pin glyph) but doesn't reorder.
- **"N more":** the overflow cap applies only to rows **after** the card.

### D.9 Ship checklist additions
- The Glass/Tiled tokens and rules land in `workspace.css` and the theme files, **not** a new stylesheet.
- The harness numbers (a and b, before and after) are in the PR and under budget, **including Waves** (§D.3).
- The four theme screenshots are attached, and docs/DESIGN.md is updated (no history).
- Status effects: per-status style picker, Working scope, previews clipped to their chip, the software fallback to Sweep.
- The spinner dropdown with eight options, and the web SVG spinners.
- Settings: the × close, project colours removed.
- The §D.6 removals are done, and nothing in the build still references Overview, New agent or the session panel.
- Usage & cost lives only in Settings (§D.7).
- Sidebar sessions never reorder, and the card grows in place (§D.8).


## C. v17: chat body, session panel, overview, branch agents, idle update (read first)

v17 builds on v16 exactly as shipped: the same tokens, floating cards, sidebar, status vocabulary and shapes. Nothing in v16 is restyled. Everything below is additive.

**Prototype controls** (bar above the window, prototype-only):
- **Platform:** macOS / Windows / Linux. Changes the title bar and how shortcuts are written.
- **Style:** Flat / Tiled / Glass.
- **Window:** 1440×900, **1280×800**, **1920×1080**, 1024×700, 800×600.
- **State:** includes **After update**.
- **Theme:** the status bar's theme menu switches Dark and Light.

### C.0 Hard rules (apply to everything in §C)
- **Cross-platform.**
  - **Title bar:** macOS draws traffic lights on the left; Windows draws caption buttons on the right (46×32, close hover `#C42B1C`); Linux draws three 20px round buttons on the right. Nothing else is platform-specific.
  - **Shortcut text:** written as **Cmd** symbols on macOS (⌘ ⇧ ⌥) and **Ctrl / Shift / Alt** on Windows and Linux. Build one `formatShortcut(binding, platform)` helper and use it for tooltips, keycaps, menus and the palette.
  - **Prototype only:** the prototype rewrites the glyphs in the DOM. Don't ship that.
- **Tooltips:** every icon control has one (B.4). That includes the new ones: Overview, New agent, the session panel toggle and close, Back to changes, Comment on this line, Remove queued, and Dismiss.
- **Reduce motion:** every animation is off under Reduce motion (OS setting or Settings → Motion). That covers the working shimmer, card enter, the jump ring pulse, drawer slides and the view transition.
- **Status always comes from the daemon** (session state plus agent events). The UI never infers status from output text.
- **Order is stable.** Sidebar order never changes on status changes, and sessions are never hidden automatically. Overview sections group by state but keep **creation order** inside each section.
- **Machines:** anything remote shows the machine chip (B.1): chat header, session panel header, overview cards, and the New agent project picker.

### C.0a Session switching has no loading screen
Clicking a session shows its pane **immediately**: either the last known screen (kept client-side per session), or the replay rendered in one frame. For local sessions, attach plus replay takes well under 100ms, so a loader would only flash.
- **Local sessions:** no spinner, overlay or "Connecting…" text, ever.
- **Remote sessions:** if attach hasn't finished after **300ms**, show a 24px strip under the pane header: the spinner, "Catching up from devbox · 48 ms", and a 2px progress line along its top edge. It never covers the terminal. It goes away as soon as the stream is live.
- **Replay output:** write the replay in one batch (one `term.write` per animation frame, never line by line), so there's no visible scroll-through.
- **Real failures** (daemon gone, machine offline) use the existing banners (§B.1 / disconnected). They're not loading states.

### C.1 Chat body (refines B.3; same switch, same transition)
All items sit in the 760px column. Agent-side items are indented 36px under the agent's 24px tile; the user's items are right-aligned. **One agent message per turn** (lead sentence in `--fg`, then paragraphs in `--fg2`, 14/1.6). Turn dividers as in v16.

| Item | Spec |
|---|---|
| **Edit block** | `--term` card, radius 10, hairline. Header (34px): pencil icon, "Edited", the file (mono 12), `+N` in `--ok`, `−N` in `--danger` (hidden when 0). Body: unified diff, mono 11.5/19, added lines with an ok tint of 10% and removed lines with a danger tint of 10%, and a 22px sign gutter. **Folds past 12 lines** into a "23 more lines" footer button that toggles to "Show less". Several edits in one turn stack as separate cards. **Clicking the header opens the file's diff in the session panel → Changes** (C.2), opening the panel if it's hidden. |
| **Command card** | `--term` card. Header (36px min): a `$` prompt, the exact command (mono 12.5), and the state. **Running:** an accent-soft pill with the spinner and "Running · m:ss" (live). **Finished:** the elapsed time, then an `exit N` pill (green for 0, red otherwise). Output: mono 11.5/18, showing **the last 3 lines** with a "Show all N lines" toggle when longer. |
| **Tool step chip** | Unchanged from v16: a 28px mono chip with a spinner (running), a green check (passed or exit 0) or a terminal glyph (neutral). Used for reads, searches and finished subagents. |
| **Approval card** | Amber card (`--warn-soft` + `--warn-line`): the needs-you tile, "Wants to run a command", then the exact command in a `--term` well. Buttons: **Yes, once (y)** (amber fill), **This session (a)** (secondary), **No (n)** (ghost). **After answering:** "You answered: Yes, once", with no buttons, and it's cleared on every surface (B.6). While pending, the composer is disabled ("Answer the request above first"). |
| **Helper agents** | A card (`rgba(255,255,255,.022)` + hairline) with the header "3 agents, 2 working" (people icon, 11.5/600 muted). Rows (38px min): status (the working spinner, or the done tile), then the title (13) and phase line (mono 11.5 muted, e.g. "Reading EmptyState.swift"), then the elapsed time (mono, right). A done row reads "**Done · 2 steps · 5m 30s**". |
| **Comment bubble** | A user bubble with a header "Comment on `file:line`", the line's code in a mono well, then the comment text. It's created by C.2's line comments. |
| **Working line** | Under the last item: the spinner, then "claude is working · 1:40 · 238k tokens". Tokens show only if the provider reports them. |
| **Queued message** | While a turn is running, the composer placeholder is "**Queue a message for when claude finishes this step**", Send becomes **Queue**, and the hint reads "Enter queues it · Esc interrupts claude". A queued message shows as a **pending bubble** (dashed `--line2` border, `--fg2` text) with "Queued · sends when claude finishes this step · Remove". It's delivered at the next **step boundary** (a tool or command finishes), then becomes a normal bubble. Several can queue; they're delivered in order. |
| **Shell tab** | Unchanged from v16: "Chat view is for agent sessions", the running process, the project's agents, **Start claude here** and **Back to terminal**. |

**Events needed** (codex app-server, claude stream-json):
- `message{turnId, text}`;
- `edit{file, hunks}`;
- `command{id, cmd, started, ended?, exit?, output}`;
- `approval{id, cmd, answered?}`;
- `todo{items[]}`;
- `subagent{id, title, phase, started, done?, steps}`;
- `turn{start|end, duration}`.

The UI renders from these only and never parses terminal output.

### C.2 Session panel (agent sessions; both views)
- **Placement:**
  - **Docked:** a floating card (radius `--r14`, `--card`, hairline, `--sh`, `--blur`) on the **activity-bar side**: right by default, mirrored with `sidebar_side`. It runs the **full height**: its top edge aligns with the tab bar, so there is no empty strip above it, and it reaches the bottom margin. Width **320px**; **520px** while a diff is open (`width .3s`). Panes and chat shrink to make room.
  - **Toggle:** a 36px floating button next to the Terminal/Chat switch, or **⌘\ / Ctrl+\**. The state is remembered per window.
  - **Compact (<1100px, B.7):** it becomes an **overlay** from the activity-bar edge (`top:40px; bottom:8px`) over a 40% scrim, closed by default. The toggle opens it, and the scrim or Esc closes it.
- **Header (46px):** agent tile, session name (13/600) + process (mono 11), the machine chip if remote, and **Hide** (panel icon, tooltip with the shortcut).
- **No agent / no events:** "Nothing to track yet", plus one plain sentence saying why (a shell session, or an agent without structured events).
- **Checklist card:**
  - **Header:** "**Checklist · 2 of 5**", with a 3px progress bar (accent on `--line2`).
  - **Done items:** a green check, then the text struck through in `--dim`.
  - **Current item:** the spinner, then **600 weight** in `--fg`, with the elapsed time (mono, accent) on the right.
  - **Pending items:** a hollow circle, plain `--fg2`.
  - This is the agent's own to-do list (`todo` events). Maitty never invents items.
- **Agents card:** "Agents · 2 working" (or "all done"), with the same rows as C.1's helper block.
- **Segments:** a segmented control, **Changes · Files · Artifacts**, each with a count.
  - **Changes:**
    - **Branch box:** the branch (mono, `--ansi-mag`, branch glyph) and "**vs main · 0 ahead · 0 behind · not pushed**". It turns green with "pushed" once pushed.
    - **Groups:** **Staged · N** and **Unstaged · N** (caps labels). Rows: a dim directory + file name, a comment-count badge if any, then `+a` / `−d`. Clicking a row opens the diff.
    - **Empty state:** "No changes in this worktree."
  - **Diff view** (inside the panel):
    - **Header:** back arrow, the path (truncated from the start), `+a −d`. Helper line: "Hover a line and click + to leave a comment for claude."
    - **Lines:** a 30px line-number gutter, a sign gutter, then the code. Hovering a line reveals a 16px accent **+** button at the left edge.
    - **Comment composer:** clicking **+** opens a composer **anchored directly under that line**. The line gets an accent-soft background, and the composer has an accent ring with "Tell claude what to change here…", **Cancel** and **Send to claude**. Enter or Ctrl/⌘+Enter sends; Esc cancels.
    - **After sending:** the line shows a **count marker** (an accent pill on the right; its tooltip lists the comments), the file row gets a badge, and a **comment bubble** appears in chat (C.1).
    - **Delivery:** to the agent as a structured comment `{file, line, text}`, or queued if a step is running.
  - **Files:** files this session touched, with their kind (**Edited** in amber, **Created** in green, **Read** dim). Edited or created files open their diff; read files open in the Files drawer.
  - **Artifacts:** cards for things the agent produced (test reports, plans, screenshots), each with a tinted glyph tile, title and subtitle. Clicking opens the artifact.
- **Relationship to the activity bar:** the activity bar's Changes drawer is **repo-wide**. The session panel's Changes is **this agent's worktree**. Both can be open.

### C.3 Overview (⌘0 / Ctrl+0)
- **Entry:** in the sidebar's **pinned area** (top of the list, above projects), the row **Overview** with a grid icon and an **amber badge = number of sessions needing you** (hidden when 0). In rail mode it's a 40×36 icon with the same badge. ⌘0 / Ctrl+0 toggles it, and Esc closes it.
- **Screen:**
  - **Layout:** replaces the panes/chat area (the tab bar stays; clicking a tab leaves the overview). One floating card.
  - **Header:** "Overview", "N sessions on M machines", **Jump to next (⌘J)** and close.
  - **Sections in this order:** **Needs you, Working, Done, Idle, Unknown.** Done includes Exited, which keeps its red shape. Each section has a caps label, its count and a hairline. **Needs you always shows**; when it's empty it shows "Nothing needs you. Agents are working or done." in a dashed box. Other empty sections are hidden.
  - **Order:** creation order within a section.
- **Card:**
  - **Size and look:** `minmax(250px,1fr)` grid, gap 10, radius 12, `rgba(255,255,255,.025)`.
  - **Ring:** a hairline, or a `--warn-line` ring for Needs you.
  - **First line:** the status shape and session name (13.5/600), with the app tile on the right.
  - **Second line:** the project, the `⑂ branch` if it's a branch agent, and the **machine chip if remote**.
  - **Context line** (2-line clamp): the current checklist item, otherwise the last agent message's lead sentence, otherwise the pane title (process · cwd).
  - **Status line:** state + time in that state ("Working · 1:40", "Needs you · 12s", "Idle · 52m 0s").
  - **Click:** opens the session.
- **Jump to next** (⌘J / Ctrl+J, works from **any screen**):
  - **Order:** the sessions needing you in creation order, starting after the one currently focused.
  - **On the Overview:** focus moves to the next Needs-you card, with a 2px accent ring + 6px accent-soft halo and a one-shot `scale(.97→1.015→1)` pulse (.5s). The list scrolls to keep it visible.
  - **Anywhere else:** it opens that session, with the toast "Needs you · Rate limiter · 1 of 2".
  - **Nothing needs you:** the toast **"Nothing needs you"**.
- **Inbox moves to ⌘⇧J / Ctrl+Shift+J** (its badge and drawer are unchanged).

### C.4 New agent on a branch
- **Entry:** **New agent** in the sidebar's pinned area (branch-plus icon in accent; rail icon too), and ⌘K "New agent on a branch…". Project right-click can open it with the project prefilled.
- **Dialog** (500px, one screen):
  - **Project:** a chip per project, with the machine chip when remote. It defaults to the focused pane's project.
  - **First prompt:** optional.
  - **Branch:** mono, prefilled `agent/<slug>`. It follows the prompt until the user edits it (slug = the first four meaningful words), and is validated against `[\w./-]+` (red ring when invalid).
  - **Agent:** segmented claude / codex, with app tiles.
  - **Worktree preview:** `<project>/worktrees/<slug>`.
  - **Actions:** **Cancel** and **Start (⌘↵ / Ctrl+Enter)**.
- **Start:**
  - **Backend:** `git worktree add -b <branch> <path> <base>`, then spawn the agent in it with the prompt.
  - **UI:** opens a new tab, and the sidebar row appears under its parent project.
- **Sidebar row for an agent with events:**
  - **Line 1:** session name + spinner (unchanged).
  - **Line 2:** a **branch** line: branch glyph + name (mono 11.5, `--ansi-mag`).
  - **Line 3:** the **current checklist item** (12, `--fg2`) with "2/5" (mono 10.5, dim) on the right.
  - **Line 4:** a **2px progress bar** (accent on `--line2`).
  - **Line 5** (collapsed sub-steps): "› Ran a subagent: Review EmptyState handling", a "+1" for more, and the elapsed time.
  - When the agent isn't working or needing you, the row falls back to the v16 process/status line.
- **Status shimmer (replaces every earlier shimmer spec, v7–v16):**
  - **One compositor-only animation.** Each row, tab and tray item gets a child overlay `<span>` (absolute, inset 0, `overflow:hidden`, `pointer-events:none`). Inside it is a 45%-wide band with `linear-gradient(100deg, transparent, rgba(C, .18) 50%, transparent)` (60% strength when not selected) and `will-change: transform, opacity`.
  - **Keyframes animate only `transform` and `opacity`:** translateX −110% → 330%, opacity 0 → 1 (18%) → 1 (82%) → 0.
  - **Plays once when the status appears,** then rests at opacity 0: `1.4s cubic-bezier(.4,0,.2,1) 1 both`. **It never loops.** Nothing animates `background-position`.
  - **Colours:** working teal `95,212,191`, needs-you amber `242,182,90`, done green `141,184,155`. Idle, exited and unknown get none.
  - **Replay:** each status uses its own keyframe name (`swWorking` / `swNeeds` / `swDone`), so a status change restarts it without remounting. Returning to the same status later replays it.
  - **Off** under Reduce motion, under the Status effects switches, and while Performance → Animations is off.
  - The working **spinner** glyph is the only continuous status motion.

### C.5 Idle self-update
- **Settings → General → Updates:** **Install updates when idle**, **on** by default, with the helper text "**Never while an agent is working.**" It replaces v14's "Restart when nothing is running".
- **"Idle" means:**
  - no session is Working or has a pending approval;
  - no keyboard input for 5 minutes;
  - the window isn't focused, or it's minimized.
- **Behaviour:** the app installs and relaunches itself, maittyd keeps every session, and the tabs reopen.
- **After relaunch:** a quiet pill toast at bottom centre, "● **Updated to 0.65.0** · **What's new**" with ×. It stays 9s and is dismissable.
  - **What's new** opens a 460px dialog with 3–5 one-line items and "Installed while nothing was running. No session was interrupted."
  - The manual flow (§0.4) is unchanged.

### C.6 Brain dump (⌘⇧N / Ctrl+Shift+N; also ⌘K)
- **Shortcut change:** "New worktree session" loses ⌘⇧N and stays in ⌘K and the project menu. The New agent dialog links here ("Several tasks at once? Brain dump").
- **Dialog** (760px, max 86% tall). **Step 1, write:**
  - one large textarea, placeholder **"Everything on your mind"** (280px min, 14/1.65);
  - hint: "One thought per line works best. Mention a project by name and it gets placed there.";
  - **Split (⌘↵)**, disabled while empty.
- **Splitting:** done by the local splitter (in production, an LLM call that returns `[{project?, title, prompt, agent?}]`). The prototype splits on lines, bullets, `; ` and sentence breaks. It matches a project when its name, or a CamelCase part of it, appears in the text.
- **Step 2, review.** Header "N tasks", **Back to text** (keeps the text), **Add task**. Each row is a card:

| Field | Rule |
|---|---|
| Number | 22px tile |
| **Project** | picker, with the machine chip when remote. **Unplaced rows are empty, with an amber ring and the hint "Pick a project".** |
| **Branch** | mono, prefilled `agent/<slug>` from the first four meaningful words, unique within the dump (`-2`, `-3`). Red ring plus a reason if invalid or duplicated. |
| **Agent** | claude / codex segmented. Defaults to claude, or codex if the text says so. |
| **Prompt** | multi-line and editable. Red ring if empty. |
| Delete | trash icon, tooltip "Remove task" |

- **Footer:**
  - **Summary:** "2 need a project" (amber) / "Fix the highlighted fields" / "Ready · 4 agents, one branch each" (green).
  - **Start all (⌘↵)** is **disabled until every row has a project and valid fields**; its tooltip says why.
  - **Nothing starts without the click.**
- **Start all:** each row runs the C.4 flow (worktree + agent + first prompt). The sidebar shows every new row under its project with its branch line, and the toast reads "Started 4 agents · each on its own branch". Tabs open in order, and the last one is focused.
- Esc or × closes, and the dump text survives reopening in the same window.

### C.7 Usage and cost (⌘⇧U / Ctrl+Shift+U)
- **Entry:** a bar-chart icon in the **sidebar footer**, next to Settings (expanded and rail), plus ⌘K. It replaces the pane area like the Overview, and Esc closes it.
- **Header:**
  - "THIS WEEK", then **tokens** (22/600) and **cost** (17/600, `--fg2`), then "reported cost · N sessions without cost";
  - **budget ring:** 56px, 5px stroke; accent below 70%, warn from 70%, danger from 90%. It's drawn **only when a monthly budget is set**. Otherwise the header shows the numbers and a "Set a monthly budget" link (an inline `$` input, Save / No budget).
  - **Period:** a **Day / Week / Month** segmented control on the right.
- **Note row (shown once):** when any session reports no cost (subscription accounts), one line at the top. Rows show **—** in the cost column; the note is never repeated per row. If every session is subscription, the header shows no dollar figure and the total cost is —.
- **Table:**
  - **Columns:** Session (app tile + name), Project, Machine (chip when remote), Agent, Tokens in, Tokens out, Cost, Turns, Time working.
  - **Sorting:** **every header sorts**, with `aria-sort` and an arrow. Clicking again flips the direction; text columns start ascending and numbers descending. Default sort: cost, descending.
  - **Layout:** sticky header, numbers right-aligned in mono with tabular figures, and a totals footer.
  - **Clicking a row** opens the session.
- **Empty (fresh install, no agent sessions):** an icon tile, "No usage yet", one sentence, and **New agent**.
- **Data:** provider-reported only (codex app-server usage events, claude stream-json `usage`). maittyd stores it per session per day so the TUI can show it too. **Never estimate cost.**

### C.8 Performance (Settings → Performance; ⌘⇧P / Ctrl+Shift+P)
- **Live strip, "Last 60 seconds":** four tiles, each with its threshold **in the label**, the current value (20/600, red over the threshold), and a 36px sparkline (60 samples, 1/s, stroke 1.5, red over the threshold).

| Tile | Measures | Red when |
|---|---|---|
| Main thread | % busy (time in render/commit + long tasks per second) | above 50% |
| Frames | fps (rAF count) | below 50 |
| Daemon round-trip | ms (ping on the control socket) | above 50 ms |
| Memory | MB, with web view · host · daemon underneath | above 900 MB |

- **Healthy:** when everything is low (busy <15%, fps ≥55, rtt <15 ms, nothing streaming, nothing animating), a green row reads **"Nothing to show, the app is idle."**
- **"What's costing":** a ranked list (by estimated cost) of what's running **right now**:
  - **Animations:** count, and which rows.
  - **Timers:** by period.
  - **Panes streaming:** one row per pane, with bytes/s.
  - **React commits/s.**
  - **Each row has a switch that turns that thing off for this session only** (it resets on relaunch): animations pause (shimmer + pulse; spinners stay static), the UI tick drops to 1s, a pane stops rendering (maittyd keeps buffering, and it catches up when turned back on), commits batch to 3/s. Rows that are off are dimmed and say what they're doing instead.
- **Renderer line:** "xterm: WebGL" or "xterm: DOM (no WebGL 2)", and "GPU: hardware / software". **Software** is shown in warn with one hint line. Detected from the WebGL2 context and `WEBGL_debug_renderer_info` (SwiftShader, llvmpipe, Microsoft Basic Render).
- **Copy report:** puts on the clipboard, as plain text, the version, user agent, renderer, the last 60 samples of each metric, and the ranked list with on/off state. The button reads "Copied" for 2s.
- **Prototype only:** "Simulate load spike" and "Simulate software GPU". Don't ship them.

### C.12 Machines (B.1) carried over
v17 includes every v16 machine screen unchanged: the machine chip, the machine menu, Add machine (form / connecting / error / done), disconnected and reconnecting banners, and Settings → Machines. Every new v17 surface shows the chip on remote items (C.0).

### C.9 Shortcuts (changes to B.8)
| Shortcut (mac / Win·Linux) | Action |
|---|---|
| ⌘0 / Ctrl+0 | Overview |
| ⌘J / Ctrl+J | Jump to next needing you |
| ⌘⇧J / Ctrl+Shift+J | Inbox (moved from ⌘J) |
| ⌘\ / Ctrl+\ | Session panel |
| ⌘⇧N / Ctrl+Shift+N | Brain dump (was New worktree session) |
| ⌘⇧U / Ctrl+Shift+U | Usage and cost |
| ⌘⇧P / Ctrl+Shift+P | Settings → Performance |
| ⌘↵ / Ctrl+Enter | Start (New agent dialog), Send (comment composer) |

All of these are added to the "caught inside a pane" list. Esc order: comment composer → New agent / What's new dialogs → Overview → drawers.

### C.10 Light, dark and sizes
- **Themes:** every new surface uses existing tokens only, so it works in Midnight, Abyss, Paper and High Contrast. Diff tints use `color-mix(var(--ok|--danger) 10%)`, so they hold in light themes.
- **Sizes:** checked at 1280×800 and 1920×1080. At 1920 the chat column stays 760px, and the session panel and overview grid get the extra width.
- **Compact (<1100px):** activity bar → Panels button (B.7); session panel → overlay; the overview grid reflows to 1–2 columns; the New agent dialog stays at 100% − 40px.

### C.11 Ship checklist additions
- C.1 item renderers from events.
- C.2 panel (docked + overlay), line comments sent to the agent as structured input.
- C.3 Overview + jump, and the Inbox shortcut move.
- C.4 worktree + agent spawn, and the sidebar rows.
- C.5 idle install, toast, What's new.
- C.0 `formatShortcut` and the platform title bars.
- C.4 once-only compositor shimmer.
- C.6 brain dump.
- C.7 usage and cost.
- C.8 performance page.

**Verify:**
- Sidebar order never changes when status changes.
- Queued messages deliver only at a step boundary.
- A comment never sends while an approval is pending; it queues instead.
- The update never installs while an agent is working.
- Start all stays disabled until every row has a project.
- Cost is never estimated.
- The shimmer never loops, and no `background-position` animation exists.
- Performance switches reset on relaunch.


## B. v16: machines, chat view, tooltips and audit fixes (read first)

### B.1 Remote machines
A **machine** is any computer running maittyd that this app is attached to. This Mac is always the first one, and remote ones connect over SSH. Every session belongs to exactly one machine (`session.machineId`).

**Data model**
```ts
type Machine = { id: string; name: string; host: string /* 'local' | user@host */;
  kind: 'laptop' | 'server' | 'gpu'; color: string /* from project palette, never amber/red */;
  status: 'online' | 'connecting' | 'reconnecting' | 'offline';
  latencyMs?: number; maittydVersion: string; home: string; os: string };
```
Saved in the shared config (`[[machines]]`), so the TUI can attach to the same list.

**Where the machine shows**

| Place | What | Rule |
|---|---|---|
| **Status bar, left** | Icon in the machine colour, then the **name** (500), then "local" / "18 ms" / "reconnecting" (amber) | Always the **focused pane's** machine. Click opens the machine menu (upward). |
| **Sidebar header** | Machine switcher: a 22px tile, the name (14/600), and a chevron. An amber dot shows if any machine isn't online. | Replaces the old "Projects" title. Shows **All machines** or the filtered machine. |
| **Project header** (sidebar) | Chip: server glyph + name, `color-mix(machine 14%)` fill | Only on remote projects, and only when the filter is All. |
| **Pane header** | Chip: 22px, `color-mix(machine 14%)` fill, 28% ring | Remote panes only. Local is implied, so the common case stays quiet. |
| **Tab** | 6px dot in the machine colour before the title | Remote tabs only. |
| **Hover card** | Same chip as the pane header | Remote only. |

**Machine menu** (310px; opens from the sidebar header downward or the status bar upward):
- "SHOW SESSIONS ON" (caps label) · **All machines** · one row per machine.
- **Row:** 28px tile, name (13/500), and "host · N sessions" (11 muted). On the right: status dot + text (online green, connecting/reconnecting amber with a dashed dot, offline dim), then **Retry** if not online, **+** to start a session there if online, and a check if selected.
- Divider, then **Add machine…** and **Manage machines…** (opens Settings → Machines).
- Picking a row filters the sidebar to that machine. Esc closes the menu.

**Lost connection:**
- Sessions on that machine show **Unknown** (dashed square) everywhere, and their spinners and shimmer stop.
- Each pane shows a strip **in the flow**, directly under the pane header (never overlaying output): "Lost connection to gpu-01. The session keeps running there. Retrying in 8s." with **Retry now**. Offline reads "Disconnected from devbox." with **Reconnect**.
- The last screen stays visible. Input is disabled.
- Backoff: 2, 4, 8, 16s (capped).

**Add machine** (dialog, 460px):
1. **Form:** SSH host (mono; accepts `user@host` or a `Host` from `~/.ssh/config`) and an optional name. Note: "Uses your SSH config and keys. If maittyd isn't on that machine yet, Maitty installs it for your user only."
2. **Run:** three steps with spinner → check: "Reaching host over ssh", "Checking maittyd (installs it if missing)", "Attaching to its sessions".
3. **Done:** "Connected to X · maittyd 0.9.2 · 31 ms", then **Start a session there** or Done.
4. **Error:** "Couldn't reach that machine", plain-language advice and the raw ssh error in mono, then Edit host / Try again.

**Settings → Machines** (Daemon is merged here):
- **First card:** "This Mac · local maittyd": status, version, socket path, sessions.
- **Then one card per remote machine:** tile, name, host · OS, status chip; then "maittyd x · N sessions · 18 ms", **Disconnect/Reconnect**, and **Remove**.
- **Remove** takes two clicks; the second-click label reads "Remove · N sessions keep running". Removing detaches only; it **never kills remote sessions**.
- **Add machine…** sits at the bottom.

**Backend:** per-machine maittyd connections over an SSH tunnel to the remote socket; reconnect with backoff; machine status events; `machine_add` (bootstrap: check `maittyd --version`, install to `~/.local/bin` if missing, start as a user service).

### B.2 New-session rule (one rule everywhere)
A new session opens **next to what you're looking at**:
1. **If a pane is focused:** the same machine, in the focused pane's **project root** (git root), or its cwd if it isn't in a project.
2. **If nothing is open:** the machine selected in the sidebar (This Mac when it's on "All"), in that machine's start folder.
3. **An explicit choice wins:** the machine menu's **+**, "New session on X" in ⌘K, and "Start a session there" all open in **that machine's start folder**.

This applies to the tab-bar **+**, the sidebar **+**, ⌘T and ⌘K. The **+** tooltip says where: "New session in AeVoxGrowth on This Mac". If the sidebar is filtered to a different machine than the new session's, it switches the filter so the new session is visible.

### B.3 Terminal / Chat view
- **Switch:** icon-only, at the far right of the tab bar. A floating card (36px tall) with two 32×28 buttons (terminal glyph, chat bubble). A raised thumb slides between them (`transform .32s cubic-bezier(.2,.8,.2,1)`), and the selected icon scales to 1.08. ⌘⇧E toggles. Tooltips: "Terminal view ⌘⇧E" and "Chat view ⌘⇧E".
- **Transition:** the incoming view slides 18px from its side with a fade and a 2px blur (`.42s cubic-bezier(.16,1,.3,1)`): chat from the right, terminal from the left. Off under Reduce motion.
- **Chat shows the focused pane's session** (same process). **Switching never restarts the agent.** What you send appears in the terminal too.
- **Chat layout** (one floating card):
  - **Header (46px):** app icon, process, session name (accent), cwd, machine chip (remote), a segmented switcher for the tab's other panes, and a status pill with the timer.
  - **Body:** a 760px centered column, gap 14:
    - **Your messages:** right-aligned bubbles, `--raise` + hairline, radius 16/16/5/16, 14px/1.55.
    - **Agent messages:** a 24px agent tile, a lead sentence (14, `--fg`), then paragraphs (14, `--fg2`). **One message per turn**; paragraphs never split into separate messages.
    - **Tool steps:** a compact mono chip indented 36px: a spinner while running, a green check when it passes (tests passed, exit 0), a terminal glyph otherwise.
    - **Turn dividers:** "Cooked for 7s · done 10:11 AM" as a centered dim label between hairlines.
    - **Approvals:** an amber card with the exact command and **Yes, once (y) / This session (a) / No (n)**. Once answered, it shows "You answered: …" and the buttons are gone everywhere (terminal, Inbox, notification).
    - **Working line:** "claude is working · 1:40 · 238k tokens" with the spinner.
  - **Composer:** a well with radius 14 and an accent ring on focus. Enter sends, Shift+Enter adds a new line, and Esc or **Stop** interrupts. It's disabled while an approval is pending ("Answer the request above first") or the machine is offline. Footer hint: "Same session as the terminal · switching views never restarts claude".
- **Shell tabs:** "Chat view is for agent sessions", the running process name, the project's running agents to jump to, and **Start claude here** / **Back to terminal**.
- **Ending a turn** sets the status to Done but **keeps the process as the agent** (it doesn't revert to zsh).
- **Production: chat MUST come from structured agent events** (codex app-server / claude stream-json): messages, tool calls with status, approvals with ids, and turn boundaries. **Never parse terminal output.** If an agent has no event stream, hide the Chat option for that tab (the switch shows only Terminal).

### B.4 Tooltips (every icon control)
- **One delegated handler** on the app root. Sources: `title` (moved to `data-tip` on first hover to suppress the native tooltip), or `aria-label` on **icon-only** buttons/radios/switches (no text content). Text buttons get none.
- **Timing:** 420ms delay. If a tooltip was visible in the last 400ms, the next one appears at once (40ms). Hidden on mousedown, keydown or wheel.
- **Skipped** while a rich hover card (tab, session, rail) or a menu is open.
- **Content:**
  - **Name:** 12/500.
  - **Shortcut:** a trailing "(⌘K)" is parsed into keycaps (18px, 2px bottom border).
  - **Hint:** text after " · " becomes a muted second line (11.5); max width 260.
- **Placement:** above by default; below if within 56px of the top; to the side, away from the edge, for the activity bar. Centred and clamped 120px from the window edges. It enters with a 4px slide from the anchor side + fade, `.16s`.
- **Style:** `--main` background, `--line2` border, radius 8, shadow `0 10px 28px rgba(0,0,0,.45)`, `pointer-events:none`, `role=tooltip`.

### B.5 Status bar (floating)
The status bar is now a floating card like the panes:
- **Size and look:** 32px tall, radius `--r14`, `--card`, hairline, `--sh`, `--blur`.
- **Placement:** margin `0 var(--pg) var(--pg)`, in the centre column only, aligned with the pane edges. The sidebar and activity bar run the full height.
- **Contents:** machine on the left (B.1); theme, layout style, text size and shortcuts on the right (§0.5).

### B.6 Notifications = Inbox actions
Notification cards and Inbox rows offer **the same actions**:
- **Needs you:** **Yes, once (y)** (amber) · **This session (a)** · **No (n)** · Dismiss.
- **Finished / exited:** **Open** · **Clear** (sets idle) · Dismiss.
- Clicking the card body opens the session.

**One source of truth:** answering anywhere (terminal y/a/n, chat card, Inbox, notification) clears that request everywhere. The notification and Inbox entries are derived from session state, never stored separately. Approvals go through the agent's approval id, not by typing a key into the PTY, once the event stream exists.

### B.7 Activity bar at compact widths
- **Below 1100px:** the activity bar is hidden.
- **Panels button:** a single 36px floating button appears just left of the Terminal/Chat switch, with a grid icon and the Inbox count badge.
- **Its menu** (220px, from the button): every activity item with its icon or extension tile, label and badge. Picking one opens its drawer.
- **Drawers:** they then anchor at `var(--pg)` from the window edge.

### B.8 Shortcuts inside a pane
While focus is in a terminal pane, the app intercepts **only** these. Everything else goes to the PTY (including Ctrl+C/D/Z/L/R, arrows, Tab and Esc):

| Shortcut | Action |
|---|---|
| ⌘K | Command palette |
| ⌘J | Inbox |
| ⌘I | Files drawer |
| ⌘⇧X | Extensions |
| ⌘⇧E | Terminal / Chat |
| ⌘T | New session (B.2) |
| ⌘W | Close pane (with undo) |
| ⌘D / ⌘⇧D | Split right / down |
| ⌘[ / ⌘] | Previous / next pane |
| ⌘1…9 | Go to tab |
| ⌃Tab / ⌃⇧Tab | Next / previous tab |
| ⌘F | Find |
| ⌘, | Settings |
| ⌘/ | Shortcut list |
| ⌘B | Collapse sidebar |
| ⌘Z | Undo close (only during the 6s window) |

- On Windows/Linux, ⌘ = Ctrl **plus Shift** for any letter a shell commonly uses (Ctrl+Shift+T/W/D/F/K). Bare Ctrl+letter always goes to the PTY.
- **Esc** closes the topmost drawer, menu or dialog first, and **only reaches the PTY when nothing is open**. While any overlay is open, Esc never interrupts an agent.

### B.9 Ship checklist additions
- Machines: data model, switcher, status bar, chips, lost-connection strip, Add machine, Settings card.
- New-session rule wired to every entry point.
- Chat view backed by agent events (hidden when unavailable).
- The tooltip system.
- The floating status bar.
- Notification/Inbox action parity.
- The compact Panels button.
- The pane shortcut table, with the Esc rule.

**Verify:**
- Removing a machine never kills its sessions.
- An answered approval disappears from every surface.
- Chat never shows raw markup.
- Below 1100px, the panes get the activity bar's width back.


## A. v15: activity bar and extensions

v15 = v14 plus a second rail, the **activity bar**. It replaces the Inbox and Inspect buttons in the tab bar and adds an extension store.

### A.1 Layout
```
┌ sessions sidebar ┐ ┌ tab bar ─────────────────────────┐ ┌ activity ┐
│ (left by default) │ │ panes…                           │ │  Inbox   │
│                   │ │                                  │ │  ─────   │
│                   │ │                                  │ │  Files   │
│                   │ │                                  │ │  Changes │
│                   │ │                                  │ │  Branch  │
│                   │ │                                  │ │  History │
│                   │ │                                  │ │  Refs    │
│                   │ │                                  │ │   ···    │
│                   │ │                                  │ │ ext ext  │
│                   │ │                                  │ │  ─────   │
│                   │ │                                  │ │  Store   │
└───────────────────┘ └──────────────────────────────────┘ └──────────┘
```
- **Always opposite the sessions sidebar.** Sidebar on the left puts the bar at the far right; sidebar on the right puts it at the far left. This follows the `sidebar_side` setting (§0.6); there's no separate setting.
- **Floating card,** same as the sidebar and panes:
  - 48px wide, radius `--r14`, `--card` background;
  - 1px `--pane-line` border, `--sh` shadow, `--blur` backdrop;
  - margin `var(--pg)` on the top, bottom and outer side, 0 toward the panes.
  - It follows the Flat, Tiled and Glass styles (Tiled squares it off).
- **Order, top to bottom:**
  1. **Inbox** (⌘J). Count badge: amber with dark text when anything needs you, otherwise neutral.
  2. Divider (22×1px `--pane-line`).
  3. **Files** (⌘I), **Changes** (badge = changed-file count), **Branches**, **History**, **References**: everything that was the Inspect panel, now one icon each.
  4. Flexible space.
  5. **Installed extensions**, one tile each (§A.3), in install order.
  6. Divider (only when there are extensions).
  7. **Extensions store** (⌘⇧X), always last.
- **Button:** 36×36, radius 9.
  - Icon: 18px line icon at stroke 1.8; extensions use a 22px tinted tile with a glyph.
  - Rest `--muted`. Hover `--raise` with `--fg`. Press `scale(.9)`.
  - **Active:** `--raise` background, `--fg` text, and a 2px `--accent` bar on the bar's **outer** edge (-6px, inset 9px top and bottom).
  - Badge: top-right, 15px tall, 9.5/700 text, ringed 2px in `--ring-bg`.
  - Every button has `aria-label`, `aria-pressed` and a tooltip with its shortcut. The container is `role=toolbar aria-orientation=vertical`.
- **Removed from the tab bar:** the Inbox and Inspect buttons. Inspect's inner tab tray is hidden, since the activity bar is now the tab control.

### A.2 Drawers
- **One open at a time.** Clicking an icon opens its drawer; clicking it again, clicking another icon, clicking the scrim, or pressing Esc closes or switches.
- **Placement:**
  - next to the activity bar, offset `48px + var(--pg) + 10px` from that window edge;
  - `top: 40px` (below the title bar), `bottom: 8px`, `z-index: 49`, over a 40% scrim;
  - same card styling as the bar, radius `--r16`.
- **Motion:** slides in from the bar's edge with `.36s cubic-bezier(.16,1,.3,1)` and out with `.22s cubic-bezier(.4,0,1,1)`, mirrored by side. Off under Reduce motion.
- **Widths:**

| Drawer | Width |
|---|---|
| Inbox | `min(420px, 100% − 80px)` |
| Files, Changes, Branches, History, References | same widths as before, including Files' wider mode |
| Extensions store | `min(440px, 100% − 72px)` |
| An extension's panel | `min(380px, 100% − 72px)` |

- **Resizing:** the Files drawer is resizable from its **inner** edge (the side facing the panes).
- **Drawers never push panes;** they always overlay. Panes keep their full width.
- **Keyboard:** ⌘J opens Inbox, ⌘I opens Files, ⌘⇧X opens the store, Esc closes. Focus returns to the last focused pane on close.

### A.3 Extensions store (⌘⇧X)
- **Header:**
  - "Extensions" title, ×;
  - search ("Search extensions", 32px well, autofocus on open; matches name, description and author);
  - segmented **Browse (N) / Installed (N)**.
- **Card** (radius 12, `rgba(255,255,255,.022)` + hairline, enters with `v8row`):
  - 36px tinted tile with a glyph (each extension declares a colour and glyph);
  - **name** (13.5/600), plus a teal "Maitty" pill for first-party;
  - meta: "by author · vX · N installs" (first-party omits the author);
  - description (12.5, `--fg2`, `text-wrap: pretty`);
  - **permission chips:** shield icon + text, e.g. "Reads project files", "Types into a pane", "Network access", "Reads running processes", "Reads agent events", "Local storage only".
- **Actions:**
  - **Not installed:** **Install** (accent).
  - **Installing:** a disabled chip with the spinner and "Installing", and a 3px progress bar at the bottom of the card.
  - **Installed:** **Open** (secondary) and **Remove** (ghost). Remove needs a second click ("Click to remove", red text + `--danger-line` ring).
- **On install:** the extension's tile pops into the activity bar (`v10pop .5s`), with the toast "Installed X · added to the activity bar".
- **Empty states:** "Nothing installed yet. Browse to add one." / "No extensions match."
- **Footer note:** "Extensions run sandboxed. They only get the access listed on their card, and anything that types into a pane asks you first."

### A.4 Extension panel
- **Header** (56px):
  - 26px tile, name (14/600), and context (the focused project name, or "All sessions" / "All projects" for global extensions);
  - ×.
- **Body (rows type):** 46px rows, hover `rgba(255,255,255,.04)`, each entering with `v8row`. A row has:
  - a **lead** (mono 11.5/600, min 46px, coloured by meaning);
  - a **title** (13, `--fg`) and **sub** (mono 11, `--muted`);
  - an optional 4px progress bar;
  - an optional action button (26px, secondary).
- **Body (notes type):** a full-height mono textarea in a well.
- **Footer:** a dim summary ("4 listening · refreshes every 2s") and **Manage**, which opens the store's Installed tab.
- **Rule: extensions never run anything silently.** Actions that run a command call `typeInPane(cmd)` on the **focused** pane, so the user sees the real command. If that pane is busy, show the toast "Focused pane is busy · <proc>" instead.

### A.5 Sample extensions (prototype catalogue)
These illustrate the API surface. Names, data and authors are placeholders.

| Extension | Type | Shows | Action |
|---|---|---|---|
| **Ports** (installed) | rows, global | listening ports, owning process and session | Open → browser |
| **Scripts** (installed) | rows, per project | package.json, Cargo and just tasks | Run → types into pane |
| Usage | rows, global | tokens per agent session, with a bar | none (provider-reported only) |
| TODOs | rows, per project | TODO/FIXME with file:line | Open → Files drawer at that file |
| HTTP Requests | rows, per project | requests from `.http` files with the last status | Send |
| Env | rows, per project | .env keys, values masked | Reveal / Hide (values never leave the machine) |
| Containers | rows, per project | containers with status | Logs → types `docker logs …` |
| Scratchpad | notes, per project | free-text notes | none (local only) |

### A.6 Extension API (proposal for the dev)
```ts
interface MaittyExtension {
  id: string; name: string; version: string; author: string;
  icon: { glyph: string; color: string };        // tile in activity bar + store
  scope: 'project' | 'global';                   // drives header context line
  permissions: Permission[];                     // shown as chips; enforced by host
  panel(ctx: { project?: Repo; sessions: Session[] }): PanelModel; // rows | notes
}
type PanelModel =
  | { kind: 'rows'; rows: Row[]; footer?: string }
  | { kind: 'notes'; storageKey: string };
type Row = { lead: string; leadTone?: Tone; title: string; sub?: string;
             bar?: number; action?: { label: string; run: Action } };
type Action =
  | { type: 'typeInPane'; cmd: string }      // needs 'Types into a pane'
  | { type: 'openFile'; path: string; line?: number }
  | { type: 'openUrl'; url: string }
  | { type: 'toast'; text: string };
```
- **Sandbox:** run extensions in a sandboxed webview or worker. The host renders `PanelModel` with **our** components, so extensions never inject UI.
- **Permissions:** checked on every action call. `typeInPane` asks for confirmation the first time per extension.
- **Store:** a signed index (JSON) with versions and checksums. Installs go to `~/.config/maitty/extensions/`, and the installed list lives in the shared config, so the TUI can list them too.

### A.7 Ship checklist additions
- Activity bar on the opposite side, floating, with all buttons and badges.
- Drawers overlay without pushing panes, with mirrored motion.
- Store: search, Browse/Installed, install/remove flow.
- Extension panel host (rows + notes).
- Permission enforcement, and the `typeInPane` confirm.
- Remove the tab-bar Inbox and Inspect buttons.
- **Verify:** Esc and second-click close; focus returns to the pane; the bar flips with `sidebar_side`.


## 0. v14: what's final

v14 is the ship build. It keeps everything from v13 except what this section changes or removes.

### 0.1 Removed. Don't build.
- **Project presets** (§12.5): the Settings page, project-menu entries, tab-menu "Save as preset…", the Project defaults preset picker, and the palette entries are all gone. Project defaults keeps only **Start in** and **Run when it opens**.
- **Detach semantics:** there's no "Close tab · keeps running" and no separate "End session". Close = end (§0.3).
- **Archive**, **priority sidebar layout** and **deck "stacked" chips**: still removed (as in v10–v12).
- **Title-bar update banner/pill:** updates live in the sidebar footer (§0.4).
- **Status-bar gear, version text, "maittyd", branch and "N working":** the tabs, sidebar and pane headers already show these (§0.5).

### 0.2 Focus border (pane)
- **Trigger:** a `mousedown` anywhere inside a pane (capture phase on `document`, matching `[data-pane]`) marks that pane focused. A `mousedown` outside every pane clears it. `focusin` also sets it, but **ignore `focusin` when `activeElement === body`**: agent panes (claude/codex TUIs) often have no focusable input, which is why the border used to never appear.
- **Look when focused** (shown even with a single pane):
  - border `--accent-line`;
  - ring `0 0 0 3px var(--accent-soft)`;
  - a 2px accent line along the top of the header.
- **Motion:** on gaining focus, the ring pulses once (`.45s`). Off under Reduce motion.

### 0.3 Close = end
- **Pane ×** ends that pane's session and removes it from the sidebar. **Tab ×** ends every session in the tab.
- **Sidebar right-click → Close session** ends the session and removes its pane/tab.
  - **Agent running:** the item arms first ("Close and stop claude? Click again", `--danger-soft` fill + `--danger-line` ring) and needs a second click. Closing the menu disarms it.
  - **Plain shell:** closes in one click.
- **Tab menu:** Rename tab (⇧F2) · Reset name (only if named) · Broadcast input (⌘⇧B) · divider · **Close tab** ("ends its session" / "ends N sessions") · **Close other tabs** ("ends N tabs", same project only).
- **Project menu:** **Close project tabs (N)**.
- **Undo:** a toast "Tab closed · session ended · **Undo Ctrl Z**" (or "Pane closed…") shows for 6s with a progress hairline.
  - **Ctrl/⌘Z** restores the session, the layout and focus.
  - **Implementation:** maittyd should **delay the kill until the undo window expires** (or keep the PTY parked), so Undo truly restores the same process. If that isn't possible, Undo respawns a shell in the same cwd, and the toast must say "Undo reopens a new shell".
- **Closing the window** is unchanged: sessions keep running in maittyd.

### 0.4 Updates (sidebar footer)
- **Where:** the expanded sidebar footer's left slot shows **Version 0.60.2** (dim, version in mono). When an update exists, the same slot becomes the **update pill**; Settings stays at the right. In the rail, a 36×28 download button appears above Settings only while an update exists.
- **Pill states:**

| State | Pill | Card (opens upward from the footer, 360px) |
|---|---|---|
| Available | teal "Update to 0.61.0", slow pulse dot | version, "You have 0.60.2 · 18 MB", NEW / FIXED lists, **Download**, Skip this version, Later |
| Downloading | conic progress ring + "42%" | progress bar, "7.6 of 18 MB · about 4s left", Cancel, Hide |
| Ready | solid teal "Restart to update" | safety list (sessions keep running in maittyd, N tabs and splits reopen, agents mid-turn aren't interrupted), **Restart now**, On next launch |
| Failed | red "Update failed" | error box with the real error, **Retry**, Copy details |
| Up to date | no pill, just "Version x" | none |

- **Restart now:** a full-window "Restarting Maitty · Reopening N tabs. Your sessions never stopped." screen (~2s), then the toast "Maitty 0.61.0 · every session kept running".
- **Card placement:** `bottom: 60px` above the footer, aligned to the sidebar's side (`left:20px`, or `right:20px` when the sidebar is on the right); in the rail, offset 84px from the edge.
- **Settings → General → Updates:** one card with the current version, a status chip and **one** button that fits the state (Check now / Download / Cancel / Restart now / Retry), the maittyd version, the Stable/Beta channel, and two switches:
  - **Download updates automatically:** on by default; installing always waits.
  - **Restart when nothing is running:** off by default.
- **Esc** closes the card.
- **Tauri:** the updater plugin (check → download with progress events → install on relaunch). Restarting the GUI must never signal maittyd.

### 0.5 Status bar (final)
- **Left:** empty normally; "Reconnecting to maittyd…" in amber with a dashed dot when the daemon drops.
- **Right:** quick controls, all 22px tall:
  - **Theme dropdown:** Dark / Light / System at the top, the common themes for the current mode underneath, and "All themes in Settings…";
  - **Layout style:** Flat / Tiled / Glass;
  - **Text size:** − / + (clicking the number resets to 14px; Ctrl+= and Ctrl+− also work);
  - **Shortcuts**, shown as **Ctrl** **/** keycaps.
- **Not here:** settings, version, branch, daemon name, working count.

### 0.6 Sidebar position (left or right)
- **Setting:** Settings → Appearance → Sidebar → **Sidebar position: Left / Right**. Also in ⌘K: "Move sidebar to the right/left".
- **Right side mirrors everything:**
  - the sidebar is the last flex child, and its resize edge moves to its left;
  - the drawer (narrow windows) slides in from the right;
  - **Inspect and Inbox** open from the **left** with mirrored slide keyframes, their buttons move to the start of the tab bar, and Inspect's resize edge flips;
  - rail popovers open to the left with the arrow on their right edge;
  - the update card aligns right (§0.4).
- **Store:** `sidebar_side` in the shared config.

### 0.7 Session hover card (final)
- **Width:** 300px.
- **Header:** 22px app icon, a caps project label, the session name (13.5/600), and a status chip on the right (working teal with spinner + m:ss, needs-you amber, done green, exited red, idle neutral), all with tinted fill + ring.
- **Second line:** "claude in …/swift-lynx". Truncate paths **from the start** so the end stays visible (`direction: rtl` + `text-overflow: ellipsis`, or truncate in code), plus "Tab 2 · open" and the relative time.
- **Last activity:** up to two plain-language lines, each with an 18px glyph tile:

| Source | Glyph | Text |
|---|---|---|
| Background task | ✓ green | "Background task finished", with detail "npm run test · exit 0 · task bel0…" |
| Approval | › amber | "Wants to run a command", then the command |
| Prompt / command | ❯ teal | "You asked: …" / "Ran …" |
| Agent step | ● | the step text |
| Error | ! red | the error line |

- **Never show raw markup:** no `<task-notification>`, tool-use ids or output-file paths. Strip tags and collapse whitespace everywhere a terminal line is quoted (hover cards, tab popovers, Inbox, rail popovers). **Production:** take these lines from structured agent events (maittyd), and use the text cleanup only as a fallback.
- **Same tab:** if the tab has other panes, the card also lists "All N panes in this tab".

### 0.8 Small fixes in v14
- The **Inbox** panel starts below the title bar (`top:44px`).
- The project rail labels use initials (`maitty-docs` → MD, `maitty` → Ma).
- The Settings theme dropdown scrolls the settings body so the whole list fits.


### What changed in v13
v13 puts every v10–v12 feature onto **Maitty's current look**. Specs below; everything earlier still applies.

**App chrome (matches the shipping app)**
- **Title bar:** a Windows-style 32px bar. It has a 16px Maitty tile, "Maitty" (12.5px muted) and 46px min / max / close buttons; close hovers `#C42B1C`. On macOS, use native traffic lights and the same height.
- **Defaults:** Glass, with a translucent `--pane` over the user's wallpaper. The prototype's haze gradient stands in for the wallpaper image.
- **Sidebar header:** "Projects" (14px/600) with icon-only **+** and **collapse** buttons, 28×28, on the right.
- **Tab bar:** Inspect is a **branch icon plus a "+a −d" diff** (no label). Inbox is **icon-only** with a count badge. Tabs show the session's name when it has one ("claude Growth Tests").
- **Pane header:**
  - app icon, **process** (14/600), **session name in `--accent`** (13/500), then the path in `--muted`;
  - on the right, a plain mono usage line ("238k tokens · 24% context · $4.97"), the project pill and the pane icons;
  - the status pill moved to a **floating pill at the pane's bottom-right**: 26px, blurred, with a status-coloured hairline ("⠹ Working · 1:09").
- **Status bar:** shows the shortcut overlay as **Ctrl / keycaps** on Windows and Linux.
- **Agent icons:** Claude `linear-gradient(140deg,#F29A6E,#D8613A)` with ✻; pacman green "P"; shell, codex and cargo as before. Generic glyphs only, no vendor marks.
- **Claude panes:** render Claude Code's own TUI unchanged. The prototype mimics it: prompt bars, ● steps, ✻ timers, the input box and its status line. The GUI adds nothing inside the terminal.

**Polish added in v13**
1. **Undo close:** closing a tab or pane shows "Tab closed · session ended · **Undo Ctrl Z** ×" for 6s (see §0.3), with a progress hairline. Ctrl/⌘Z restores the exact layout and focus.
2. **Keyboard reach:**
   - ↑↓ moves through sidebar rows and tree items.
   - In any menu: ↑↓ / Home / End to move, Enter to choose.
   - **Shift+F10 / the Menu key** opens the right-click menu for the focused item and focuses its first entry.
   - **/** focuses the sidebar filter (only when focus isn't in a terminal).
3. **Focus rings:** every control gets `*:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px }`. Text inputs use their own accent ring instead.
4. **Pane lifecycle states:**
   - **Attaching:** an overlay shows "Connecting to session…" (indeterminate bar), then "Replaying N lines…" (determinate bar), driven by the replay progress events.
   - **Ended:** when the shell exits, keep the scrollback and show a bottom card: "Session ended · exit 0" with **Restart shell** (new session in the same cwd) and **Close pane**. Ended sessions leave the sidebar.
5. **Unread clears on view:** after about 1s of the session being focused and visible.
6. **Sidebar filter:** a 30px field under the header. It matches name, process, agent, path, project and worktree branch, shows a match count and ×, and offers "No sessions match". Projects auto-expand while filtering. Esc clears it.
7. **Per-project defaults:** project menu → **Project defaults…** has Start in (subfolder, validated), Run when it opens (free text plus chips: codex, claude, cargo watch, git status). "New session here" uses them; the menu item shows the command as a hint. Store these in project config so the TUI shares them.
8. **Light themes:** `--ring-bg` (the badge cut-out ring) follows the surface (`--main` in light themes). Shadows soften to `0 10px 28px rgba(20,28,36,.12)`.
9. **Menus:** all menus share one behaviour: 32px items, caps heading, dividers, keyboard navigation, and in-place second-click confirmation for destructive items. Build a single `<Menu>` primitive.
10. **Component reference:** `Maitty Components.dc.html` covers:
    - buttons (primary / secondary / ghost / danger-armed / icon / disabled / focused);
    - inputs (rest / focus / error / filter / inline rename), switch, segmented control, keycaps;
    - the status matrix (badge, pill and word for all 6 states);
    - sidebar project + main row (selected-working, hover, default) + tree;
    - tabs (inactive, hover, active, active-working);
    - pane header + floating status pill, menu with focus, undo toast, popover, error card.

### What changed in v12 (read this first)
v11 still applies, with one rename: **Minimal → Flat, Expressive → Glass**, plus a new **Tiled** style. v12 adds the items below; each has a full spec in §12.

| # | Change | Where |
|---|---|---|
| 1 | **Window style**: Flat (default) / **Tiled** (new) / Glass | Settings → Appearance → Window style |
| 2 | Settings: fixed header with Close + esc | Every settings page |
| 3 | **Theme**: Appearance System / Dark / Light, plus separate **Dark theme** and **Light theme** dropdowns with search and live preview | Settings → Appearance → Theme |
| 4 | High Contrast theme | Dark theme list |
| 5 | ~~Project presets~~ **removed in v14** (§0.1) | none |
| 6 | **First run** with TUI config import | First launch; Settings → General → Show the welcome again |
| 7 | **Shared config** with the Maitty TUI | Settings → General |
| 8 | **Status bar** | Bottom of the window |
| 9 | **Rename, pin, close** sessions and tabs | Double-click a name, F2 / ⇧F2, or right-click |
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

**v12 additions:** friendly Inspect errors (§12.15), Window style (Flat / Tiled / Glass), the settings header, split dark/light themes and High Contrast, presets, first run, shared config, status bar, rename/pin/close, the shortcut overlay, and the rail redesign with popovers all ship in the first release. Importing all ~170 TUI themes can follow.

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
  - the pane's old session moves to the sidebar (not ended; nothing was closed).
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

### 12.5 Project presets: removed in v14
Don't build. See §0.1.

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
- **How it works:** theme, spinner, hotkeys, start folder, sidebar side and project defaults live in one TOML file. Watch it for changes and apply external edits live; write desktop changes back.
  - Toast on an external change: "Theme changed in Maitty TUI · now Nord".
  - Desktop-only settings (window style, sidebar width, layouts) stay in the desktop file.
- **Prototype only:** "Simulate TUI edit".

### 12.8 Status bar
Superseded by §0.5.

### 12.9 Rename and pin (close rules in §0.3)
**Archive was removed.** Don't build it.

**Inline rename** (sessions and tabs). The name becomes an edit field **in place**; there's no text field inside a menu.
- **Field:** height 22, radius 6, `--well` background, `inset 0 0 0 1.5px --accent-line` plus `0 0 0 3px --accent-soft`, 13px. The text is fully selected on open. The placeholder is the automatic label.
- **Enter** saves, **Esc** cancels, and blur saves. An empty value clears the custom name. A toast says "Renamed to "X"" or "Name reset".
- **Clicks inside the field never reach the row or tab** (stop mousedown, click and dblclick).
- **Session triggers:** double-click the name in the sidebar, **F2**, or right-click → Rename.
- **Tab triggers:** double-click the tab title, **⇧F2**, or right-click → Rename tab.

**Session menu** (right-click a sidebar row or tray item, 260px): caps heading · **Rename (F2)** · **Pin to top / Unpin** · **Open** · divider · **Close session** (red; two-step when an agent is running, §0.3).

**Pin:** a pinned session leads its project's stack and sorts first in the tray, with an 11px pin glyph.

**Data:** `name` and `pinned` per session id, stored by maittyd so the TUI shows them too. Tab names are local layout state.

### 12.9a Hover popovers (tabs and sessions)
Same card as the rail popovers (§12.12): `--main`, a `--line2` border, radius 12, a 9px rotated-square arrow, and `v10pop` entry. Timing: open after 280ms; once one is open, others swap instantly; close 160ms after leaving both the item and the card. They also open on keyboard focus, and never while dragging or with Settings open. **They replace the native `title` tooltips.**
- **Tab** (below the tab, arrow up, 320px, clamped to the window):
  - **Header:** a 40×28 mini layout map (columns and stacked cells, tinted by status: amber needs, teal working, red exited, green done, grey idle), the project colour and "project · Tab N", plus "N panes · broadcast on · open now".
  - **Summary chips:** "1 needs you", "2 working", "1 exited", "1 done".
  - **One row per pane:** a 22px app icon with its pane-number badge, **process** (mono 600), folder, a "focused" pill, and the status word (teal "⠋ 1:42", amber "needs you", …). The second line is mono 11: the pending approval "Allow $ git push…" in amber, the latest output, or "idle shell · ~/path". Clicking focuses that pane in that tab.
  - **Footer:** "Click a pane to focus it · closing the tab keeps every session running".
- **Session** (sidebar row or tray item, to the right, arrow left, 260px, or 300px when the tab has other panes):
  - project · name, status word, process · path;
  - an activity well (the approval command or last output);
  - "In tab N" / "Not open in a tab · right-click to rename or pin";
  - if its tab has more than one pane, **ALL N PANES IN THIS TAB**, listed in the tab-popover row format with this session highlighted.

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
  - hint: "In tab N · right-click to rename or pin".
- **ARIA:** `role=tooltip`, and `aria-describedby` from the item in production. They replace the native `title` tooltips in the rail.

### 12.13 Hotkeys added
⌘/ shortcut overlay · F2 rename the focused session · ⇧F2 rename the current tab. (Everything from §10.18 still applies.)

### 12.15 Inspect panel errors and empty states
Errors in the Inspect panel (Files, Changes, Branches, History, References) replace the tab's content with **one calm, centred screen**. They are written for users, not developers. The terminals keep working; only the panel is affected.

**Layout** (column, centred, max-width 320, padding `32px 28px`, enters with `v8pane`):
1. **Icon:** a 76px soft radial halo (tone at 14–16%) behind a 52px tile, radius 15. The tile background is the tone mixed 12% into `--raise`, with an inset ring at the tone's 32–35% and a `0 8px 20px rgba(0,0,0,.25)` shadow. A 24px line icon is drawn in the tone colour.
   - **While retrying:** a 2px ring spins around the tile (`border-top-color: --accent`, `.9s linear infinite`).
2. **Title:** 16/600, `text-wrap:balance`, plain language, no codes.
3. **Body:** 13/1.6 `--fg2`, one or two sentences: what happened, and that the user's work is fine.
4. **Reassurance pill:** "✓ Nothing in your project was changed", `--ok` on ok at 11%, 26px tall, fully rounded. Shown on errors, not on empty states.
5. **Primary button:** 34px, min-width 180, `--accent` fill, 600 weight. While busy it reads "Checking…" at 60% opacity and is disabled.
6. **Secondary:** a 30px ghost button directly under it.
7. **Auto-retry line** (12px `--dim`, tabular): "Trying again automatically in Ns". Shown only for errors that usually clear by themselves.
8. **"Show details for support"** link (11.5 `--dim`, chevron). It expands a mono well (`--term`, hairline) with key/value rows (request or command, folder, raw error) and a **Copy details** link. Technical text lives **only** here.

**Tone:** amber = probably temporary; red = needs the user to do something; teal = not an error. Red is never used for something that will fix itself.

| Case | Tone / icon | Title | Body | Primary | Secondary | Auto-retry |
|---|---|---|---|---|---|---|
| Daemon unreachable | amber / plug | Maitty's background service isn't answering | Your terminals and agents keep running. This panel just can't load your {tab} right now. | Try again | Restart the service → `maitty daemon restart` in pane | 6s, backing off ×2, ×3, ×4 |
| Git lock | amber / clock | Git is finishing something | Another git command is working in this project. It usually wraps up in a few seconds. | Try again | See what's running → `ps -ef \| grep git` | 3s |
| Permission denied | red / lock | Maitty needs permission to read "{folder}" | Your Mac is keeping this folder private. Allow Maitty under Privacy & Security → Files and Folders, then come back. | Open System Settings (deep link) | I've allowed it, try again | none |
| Timeout | amber / hourglass | This project is taking a while | Big projects can be slow the first time. Give it another go, or load just the recent {tab}. | Keep trying | Load recent only (smaller limit) | none |
| Not a git repo (empty state) | teal / folder+ | This folder isn't a git project yet | Changes, branches and history show up once a folder uses git. You can still browse its files. | Make it a git project → `git init` in pane | Browse files instead | none; details read "What happens?" |

**Rules:**
- **Actions in the pane:** any action that changes something runs **in the focused pane**, so the user sees the real command. Retries are read-only.
- **Auto-retry:** cancel it while the user is interacting with the details; reset the countdown after a failed manual retry.
- **On success:** swap straight back to the tab content and toast "All good · {tab} loaded".
- **Platforms:** permission copy is OS-specific. On Linux, say "check the folder's permissions" and offer `ls -la`. On Windows, "Windows blocked access…".
- **ARIA:** `role="alert"` for errors and `role="status"` for the empty state. Focus moves to the primary button when the screen appears.
- **Never show:** raw error codes in the title or body, stack traces, or the word "fatal".

**Prototype:** State → **Panel errors** opens Inspect with a **PREVIEW** strip to switch cases. Git busy and Timeout recover on the first retry; the daemon case after two.

### 12.14 Backend additions
- Session metadata (`name`, `pinned`) in maittyd, and an explicit `session_end(id)` (the only call that stops a process).
- A shared config file watcher.
- Typed panel errors from maittyd (`kind: daemon|lock|perm|timeout`, plus raw detail) so the GUI can pick the friendly screen; a `limit` option on `repo_log`/`repo_tree` for "Load recent only".
- Detecting the TUI config path.
- **Names are suggestions; match maittyd's API.**

## 13. Ship checklist (v14)
**Must ship:**
- §0.2 focus border;
- §0.3 close = end, with Undo;
- §0.4 updates;
- §0.5 status bar;
- §0.6 sidebar side;
- §0.7 hover card with clean summaries;
- everything in §12 except presets.

**Backend needed:**
- delayed kill for Undo;
- structured agent events for the hover/inbox summaries;
- the updater plugin wiring;
- `name` / `pinned` / `sidebar_side` in the shared config.

**Verify before release:**
- No raw `<…>` markup reaches any card.
- Closing the window never ends a session.
- A Close with a running agent always takes two clicks.
- Every update state has a working primary action.

### C.13 How to test (prototype walkthrough)
Shortcuts below are Windows/Linux; use ⌘ on macOS. The fastest route is the **New features** button in the prototype bar, which has a **Show** per item.

- **Prototype bar:**
  - **Platform:** switches the title bar and shortcut labels.
  - **Window:** try 1280×800, 1920×1080 and the compact sizes.
  - **Style:** Flat / Tiled / Glass.
  - **State:** No sessions, Daemon disconnected, After restart, First run.
  - **Theme:** the status-bar theme menu switches light and dark.
- **Chat view (C.1):** open the claude **Growth Tests** tab, then the chat icon. Check:
  - edit cards (fold and "N more lines") and command cards (elapsed time, then exit code);
  - tool chips, helper agents and the working line;
  - the approval card: answer y / a / n and it reads "You answered: …";
  - typing during a turn: it queues as a pending bubble;
  - a shell tab: the "Chat view is for agent sessions" state.
- **Session panel (C.2), Ctrl+\:** the checklist (4 of 5), the agents, Changes / Files / Artifacts, and the diff line comment (+, then **Send to claude**: a marker appears on the line and a bubble in the chat). The panel's top aligns with the tab bar.
- **Overview (C.3):**
  - **Ctrl+0:** the sections are in order, and remote cards show their machine chip.
  - **Ctrl+J:** jumps to the next Needs-you card, or shows "Nothing needs you".
- **New agent (C.4):** sidebar **New agent** → Start. The row shows its branch line, current checklist item, 2/5 bar and sub-steps. The shimmer plays once and never loops.
- **Brain dump (C.6), Ctrl+Shift+N:**
  - type tasks, then **Split**; unplaced rows show "Pick a project";
  - **Start all** stays disabled until every row has a project;
  - Back, Add and Delete all work.
- **Usage (C.7), Ctrl+Shift+U:** every column sorts; Day / Week / Month; subscription rows show — with a single note; the budget ring appears only once a budget is set.
- **Performance (C.8), Ctrl+Shift+P:** live tiles, **Simulate load spike** (tiles turn red), the switches under What's costing, **Simulate software GPU** (warn hint), **Copy report**.
- **Updates (C.5):** the footer pill → Download → Restart gives the toast "Updated to 0.65.0 · What's new". Settings → General has **Install updates when idle**.
- **Loading (C.0a):** local sessions show no loader; a remote session shows a slim "Catching up from devbox" strip only when connecting is slow.

**Not yet checked on screen:** the chat-body details, the overview jump, light themes, and window sizes other than 1440×900. Check these first.
