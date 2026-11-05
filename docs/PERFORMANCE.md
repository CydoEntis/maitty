# Desktop GUI performance: audit and plan (2026-11-03)

Why the desktop app lags with a single session, what each cause costs, and what to change, in
order. Numbers come from running the real production build inside the same WebKitGTK engine the
Linux app uses (2.52.6), plus Chromium for function-level profiles. The harness is in
`apps/gui/scripts/perf/`; the raw tables are `results-*.txt` beside it.

## Summary

The daemon is nearly idle with one session (under 1% of a core). The web view's main thread is
not: a session that is merely marked *working*, with no output at all, costs **58%** of a core in
WebKitGTK, and a streaming session **94%**. Almost all of it is drawing, not logic:

| What the web view is doing | WebKitGTK main thread |
| --- | --- |
| 1 idle shell | 2% |
| 1 session *working*, silent | **58%** |
| same, with ~20 KB/s of agent-style output | **94%** (frame rate halves) |
| 4 sessions listed, 1 streaming | **97%** |

From the 58% baseline, switching one thing off at a time:

| Change | Main thread |
| --- | --- |
| all CSS animations off | 25% |
| only the *working* sweep off | 26% |
| only the 80 ms spinner clock off | 46% |
| all timers off | 43% |
| animations off + timers off | **1.7%** |
| stop the 1.5 s session poll | 64% (no change) |
| stop the 1 s clock | 62% (no change) |
| box-shadows off | 40% |
| gradients off | 48% |

So two things explain a quiet working session: always-on CSS animations, and a spinner that is
driven through React 12.5 times a second. Output rendering is the third, separate problem.

## Root causes, ranked by measured cost

### 1. Infinite CSS animations on working and waiting sessions (about 32 points per working session; stacks)

- `apps/gui/src/features/workspace/workspace.css:502-505`: `.mx-fx-working::after`, `.mx-fx-needs::after`, `.mx-fx-done::after` move a 250%-wide gradient layer forever (`v14sweep`, `v14breathe`).
- `workspace.css:244` `.mx-badge.is-needs` and `:754` `.mx-inbox-count.is-needs` animate `box-shadow` forever (`v7pulse`); `:1005` `.mx-upd-dot` too; `:527` `.mx-pane-status.is-idle .mx-status-dot` breathes forever.
- `workspace.css:483` the Settings preview chip loops the sweep.

The sweep was rewritten earlier to animate `transform`/`opacity`, which is cheap in Chromium (1-2
points) but **not** here: this WebKitGTK paints in software, and every frame repaints a glass card
with shadows and a gradient. The animation runs on every working row and tab, so it grows with the
number of sessions.

**Change:** no infinite animation for a session that is just working or waiting. Show a one-shot
sweep when status *changes* (that is the signal that matters) and let it settle. Never animate
`box-shadow`; pulse with a `transform: scale` on a pseudo-element if a pulse is wanted, and only for
a few seconds. Keep the Settings › Motion switches, but their default on Glass should be "on
change", not "always". Expected: 58% to about 25%.

### 2. The spinner clock drives React 12.5 times a second (about 12-15 points)

- `apps/gui/src/lib/spinner-clock.ts:34`: `setInterval(tick, 80)`.
- `features/workspace/components/glyphs.tsx:11` and `components/StatusMark.tsx:17` read it through
  `useSyncExternalStore`, so every mounted spinner forces a synchronous React commit each tick.
  One working session mounts at least three (sidebar row, tab, pane badge; rail and tray too).
- Each tick swaps a braille glyph in `.mx-spin` inside a flex row, which re-lays-out the row and
  repaints the card.

**Change:** keep the frames but do not go through React. One shared clock updates a text node per
spinner by ref (`element.textContent = frame`), at 150-200 ms instead of 80 ms, and stops while the
window is not focused or the document is hidden. Give `.mx-spin` a fixed `inline-size` (`1ch` or
`2ch`) and `contain: strict` so a frame change cannot re-layout the row. Expected: a further 12-15
points.

Items 1 and 2 together take a quiet working session from 58% to low single digits in the
measurement (animations off + timers off = 1.7%).

### 3. Terminal output is rendered with xterm's DOM renderer (saturates WebKit while streaming)

- `apps/gui/package.json` has `@xterm/xterm`, `addon-fit` and `addon-search` only; `TerminalView.tsx:175-176` loads exactly those. xterm 6 has no canvas addon, so every changed row is rebuilt as `<span>`s.
- Hot functions (Chromium, streaming): `replaceChildren` 282 ms/20 s, GC 152, xterm `createRow` 138, `createElement` 40, `WidthCache.get` 32. 80 style recalcs and 33 layouts per second. All app code (React, zod, react-query) together: under 60 ms per 20 s.
- xterm's `scrollableElement.ts:583` resets a 500 ms scrollbar-hide timer **217 times a second** while output scrolls (once per line).
- The terminal paints over a translucent gradient with big shadows: `.mx-pane` (`workspace.css:341-349`) and `allowTransparency: true` with a transparent xterm background (`TerminalView.tsx:181`, `theme/themes.ts:209`), so each row repaint blends through the card.

**Change:** add `@xterm/addon-webgl`, load it after `open()`, fall back to the DOM renderer on
`onContextLoss` or when `getContext('webgl2')` is null. Whether WebGL works on the Linux
WebKitGTK build has to be tried on a real machine; if it does not, coalesce writes to at most 30
flushes a second (the rAF batch in `TerminalView.tsx:148-169` flushes at 60) and make the xterm
background opaque (`--term`), `allowTransparency: false`, with the card's shadow on a wrapper that
never repaints. Turn off the scrollbar-hide timer (xterm's `scrollbar` option) or hide the
scrollbar while output streams.

### 4. Cursor blink keeps an idle pane awake (idle 2.4% instead of 0.2%)

- xterm's blink animates colour at 60 Hz and causes 60 style recalcs a second even when nothing happens. It already blinks only on the focused pane (`TerminalView.tsx:182,391`).

**Change:** blink only while the window has focus (`document.hasFocus()`), and stop when the
document is hidden.

### 5. Not the cause of the lag, but real waste (do after 1-4)

Measured as negligible on the web view (all together under 3 ms per second): the 1.5 s
`list_sessions` poll and its zod parse, the 1 s clock, the React re-render of the whole tree on
every poll, IPC parsing. They still deserve cleaning up, mostly for battery and for the daemon:

- **Daemon writes and fsyncs the session file every 2 s with identical bytes.** `crates/maitty-core/src/daemon/restore.rs` `schedule_session_save` serialises and spawns the write without comparing to `last_saved`; only the shutdown path (`restore.rs:151`) compares. Fix: compare before dispatching.
- **Daemon builds its full snapshot twice a second even when nothing changed.** `daemon/mod.rs:1448-1455`: the process scan sets `dirty` whenever the second ticked; `mod.rs:1014-1023` then builds `snapshot_for` for both surfaces whether or not a client of that surface is attached. Fix: only mark dirty when something other than the clock changed; build only for attached surfaces.
- **The GUI polls the same data again every 1.5 s on a brand-new socket connection** (`crates/maitty-desktop-client/src/lib.rs:1189-1195`, `ipc.rs:146-164`), while the open stream already receives `ServerMsg::State` and discards it (`apps/gui/src-tauri/src/terminal_stream.rs:112-117`). Fix: forward `State` from the stream as a `sessions-changed` event and drop the poll, or keep one connection.
- **One IPC event per PTY read, no batching, about seven copies per chunk.** `daemon/term.rs:733-751` → `mod.rs:1423-1426` → `terminal_stream.rs:125,151` base64 + `app.emit` per read; every pane's listener (`apps/gui/src/api/terminalStream.ts:23-25`) zod-parses and `atob`s each one before checking `term`. Fix: buffer per pane in the Tauri host and flush every 8-16 ms or at 64 KiB; check `term` before parsing.
- **Replays travel as JSON number arrays.** `apps/gui/src-tauri/src/lib.rs:133-176` return `Vec<u8>` (about 3.5 bytes of JSON per byte; a 2 MiB replay is about 7 MB to parse), and `terminal_history` base64-encodes up to 8 MiB while holding the mutex that live output needs. Fix: base64 or raw `tauri::ipc::Response`, trim to the last N KiB plus the formatted screen, encode outside the lock, replay only the pane being viewed.
- **React tree re-renders twice per poll.** `DesktopApp.tsx` reads `sessionsQuery.dataUpdatedAt` in render (changes every fetch) and `setTracks(new Map)` unconditionally; nothing under `DesktopApp` is memoised. Fix: key the reconcile on `data` identity in an effect, set tracks only when a track changed, `React.memo` Sidebar/TabBar/PaneCard/StatusBar with stable callbacks.
- `matchMedia` is created and re-subscribed on every render (`DesktopApp.tsx:141,156`); the palette's 30+ items are rebuilt while closed; a full `git diff` runs on every window focus (`main.tsx:15` `refetchOnWindowFocus: true` plus the diff-stat query); `HoverTips` walks the terminal DOM on every pointer move; the browser preview app is bundled into the desktop startup (`App.tsx:214`).

## Plan

Done on 2026-11-03 on Windows (gate: build, tests, clippy, cross-checks). The WebKit harness
needs a Linux machine with WebKitGTK, so the before/after numbers below are still owed: run
scenario `b` (silent working session) and `b` with the stream, and put them in the next commit
that touches this area.

Tier 1 (small changes, measured gain):

1. [x] Status effects play once when a status appears and then rest: the working sweep is one pass, the needs-you glow breathes three times, the needs-you badge, Inbox count and update dot pulse three times with a `transform` ring on a pseudo-element, the idle dot is still. Nothing animates `box-shadow` or loops. `workspace.css`, `App.css`, Settings › Motion copy.
2. [x] Spinner outside React: `lib/spinner-clock.ts` writes each frame into the elements (`attachSpinner`) at 160 ms, and stops while the window is hidden or not focused (`lib/window-activity.ts`). `.mx-spin` has a fixed `inline-size` and `contain: layout`.
3. [x] Cursor blink only while the pane is focused and the window is in front (`TerminalView.tsx`, `useWindowActive`).

Tier 2 (needs a real machine to verify):

4. [x] `@xterm/addon-webgl` is loaded after `open()` when the web view has WebGL 2, with the DOM renderer as the fallback (no WebGL 2, the addon failing to start, or a lost context). With the DOM renderer the terminal background is the theme's opaque `term` colour and writes are flushed at most every 32 ms; with WebGL the canvas stays see-through over the card. Not done: xterm 6 has no option for the scrollbar's hide timer (`vertical: Auto` is hard-coded in its scrollable element), so that 500 ms timer still resets per scrolled line while the DOM renderer draws. On Linux, WebKitGTK 2.52 gives the AppImage WebGL 2 through the NVIDIA EGL libraries on an RTX 4060 under Wayland (seen 2026-11-04); the see-through canvas then answers a program's colour query with the theme's colours itself (see *Measuring in the app*).

Tier 3 (waste; not felt as lag with one session):

5. [x] Daemon: the periodic save compares the serialised session with `last_saved` and skips the write (a barrier save still goes out); the process scan marks the state dirty for the clock only when `why_process_at` enters a new minute, which is the resolution the TUI's why popover shows it at; a surface's snapshot is built only when a client of that surface is attached.
6. [x] Tauri host: output is gathered per pane for up to 12 ms or 64 KiB and sent as one raw frame over a `tauri::ipc::Channel` the webview opens once (`watch_terminal_output`), so nothing is base64-encoded, JSON-parsed or decoded per chunk, and each view sees only its pane's frames; replays and `terminal_history` are raw bytes (`tauri::ipc::Response`; the history reply is the sequence number as 8 big-endian bytes, then the output; empty means dropped); `ServerMsg::State` goes out as `sessions-changed` (the `Snapshot`, or `null` when the stream dropped) and the GUI sets the query data from it. The 1.5 s poll is now a 15 s safety net. Replays are trimmed and asked for per pane since item 11.
7. [x] React: the reconcile is keyed on `data` identity, tracks are set only when one changed (`tracksEqual`), `TabBar` and `StatusBar` are memoised with stable callbacks, `matchMedia` is created once, the palette's items are built only while it is open, the diff stat does not refetch on window focus, and the browser preview (`App.tsx`) is a lazy chunk the desktop never loads (`Root.tsx`). Not memoised: `Sidebar` and `PaneCard`, whose props are rebuilt by the parent on every real change anyway; `HoverTips` was measured as negligible and is unchanged.

Follow-ups done on the same day, after a second look:

8. [x] The one-second clock is a shared store only the text showing a time subscribes to (`lib/clock.ts`; `components/times.tsx`: `Ago`, `ElapsedSince`), and a move highlight ends through `useRecent`, so a working session ticks a few small spans, not the window; the Settings spinner preview steps outside React; terminal output goes over one raw `tauri::ipc::Channel` instead of base64 events.
9. [x] Glass on a web view without WebGL 2 (`data-gpu="software"`, set by `lib/gpu.ts`) drops its resting shadows, card gradient and popup blur, the three things the audit measured at 18, 10 and more points on software WebKitGTK; colours, radii and layout stay.
10. [x] State deltas: when only panes changed, the daemon sends `ServerMsg::TermsChanged` (compared as encoded bytes against what that surface's clients last got) and nothing at all when nothing visible changed; `PROTOCOL_VERSION` 69. The TUI and the desktop host merge deltas into the snapshot they hold.
11. [x] Replays only for viewed panes: the desktop stream attaches with `Hello.replays: false` and asks `WantReplay` per pane when a view opens; a desktop replay is the last 1 MiB of the ring plus the screen; the host keeps no history for a pane until its replay arrived.
12. [x] The process scan runs every 5 s instead of every second while no client is attached.

Verification for every step: run the WebKit harness scenario `b` (silent working session) and
`b` with the stream before and after, and put the two numbers in the commit message. The target is
a quiet working session under 5% and a streaming one under 40% on software-rendered WebKitGTK.

## Measured again on Linux (2026-11-04)

The same harness on the user's Linux machine (WebKitGTK 2.52.6, NVIDIA RTX 4060, Wayland), 15 s
samples, web process CPU as a share of one core. "Software" is the offscreen run the audit used;
"GPU" is a visible window with `WEBKIT_DISABLE_DMABUF_RENDERER=1`, which gets WebGL 2 and keeps
the Glass finish, as the AppImage does on that machine. The harness mock now feeds output over the
`watch_terminal_output` channel like the host (it had kept sending the old `terminal-chunk` events,
so the streaming scenario measured nothing until it was fixed).

| Scenario | Software | GPU | Nov 3 |
| --- | --- | --- | --- |
| 1 idle shell | 1.2% | 1.3% | 1.8% |
| 1 working session, silent | 1.4% | 11.5% → **1.8%** | 66% |
| same with ~20 KB/s output | 24.4% | 25.7% | 91% |

- The silent working session's 11.5% on the GPU path was the spinner: swapping its glyph every
  160 ms repainted the whole sidebar card, whose gradient (`ns_nogradient`: 1.5%) and shadow
  (`ns_noshadow`: 7.5%) are repainted at full cost under GPU compositing; with the Flat finish
  (1.6%), without the sidebar (1.6%) or with the 160 ms timer stopped (1.9%) it went away.
  Fix: `.mx-spin` has `contain: layout style paint` and `will-change: transform`, so it paints
  in its own layer and a frame change touches nothing else. 1.8% after.
- Streaming at 25% is the terminal path itself: hiding the chrome (32.9%, the pane grows),
  stopping animations (24.9%) or the cursor blink (25.1%) changes nothing, and the mock's own
  generator costs under 1% (`genonly`). A Chromium profile shows almost no script time: the cost
  is compositing the redrawn terminal, once per write. The mock sends 20 writes a second; the
  host sends one every 12 ms under a steady stream, and with `--tick 12` the pane drew 37 times
  a second and busy time doubled. Fix: writes now land at most every 32 ms on both renderers
  (`FLUSH_GAP_MS`, waited out with a timer rather than polled frames), which held the draw rate
  at 30 a second and took the 12 ms stream from 91% to 63% busy in Chromium's software-WebGL
  profile. A key's echo waits at most 32 ms, and only while output is already streaming.
  Still open: xterm 6's scrollbar-hide timer is reset per scrolled line (about 230 `setTimeout`
  calls a second under the stream), and the WebGL addon asks for its own animation frames.
- The AppImage with one idle pane was seen at about 13% of a core on the same machine before the
  spinner fix, against 1.3% for the harness's idle shell; the difference is the real daemon's
  traffic and whatever the user's panes were doing, which Settings › Performance now shows.

## Measuring in the app

Settings › Performance (`apps/gui/src/features/settings/components/PerformanceSection.tsx`) shows,
once a second while it is open, what the web view and the processes are doing, and **Copy report**
puts the same figures as plain text on the clipboard for a bug report or an agent. Its **Overlay**
switch (`perfOverlay`) keeps a one-line readout in the window's corner (`PerfOverlay.tsx`) so a
lag spike can be matched with what the app was doing. Nothing is sampled while neither is shown.

What the numbers are and where they come from (`apps/gui/src/lib/perf-meter.ts`, the host's
`performance.rs`):

- **Frames**: animation frames per second, how many took over 50 ms (a hitch you feel), and the
  worst one. The web view's main thread is busy when this drops while the window is idle.
- **Drawing**: whether the web view has WebGL 2 (`data-gpu`) and how many open panes draw with
  WebGL against xterm's DOM renderer. A DOM pane is cause 3 above.
- **Animations running**: `document.getAnimations().length`. Each one repaints its card every
  frame (cause 1), so an idle window should show 0. Also the element count of the DOM.
- **Terminal output**: bytes and frames per second received over the host's channel, xterm
  writes per second, and how long a write waited until xterm had parsed it.
- **Processes**: CPU (percent of one core) and resident memory of the host, of the web view's
  processes (the host's children: WebKit's web and network processes on Linux, WebView2 on
  Windows; on macOS the content process is not a child, so the row says so) and of every daemon
  of this product on the machine, found by its fixed `daemon` argument. Several daemons usually
  means stale ones from other checkouts are still running. The first sample of a process reads 0;
  the figure is the work since the previous sample, from `sysinfo`.

Two things seen on the user's Linux machine that look like app bugs and are not:

- **Another window shows faintly through Maitty.** Omarchy gives every window 98.5% opacity when
  focused and 96% otherwise (`/usr/share/omarchy/default/hypr/windows.lua`, tag
  `default-opacity`). Bright terminal text behind a dark window reads as a ghost. The fix is a
  window rule for `maitty-gui` in the user's Hyprland config, not a change here.
- **Black bars under Claude Code's prompt and status line.** With WebGL the terminal's theme
  background is `rgba(0,0,0,0)`, and xterm answers a program's OSC 11 background query from that
  colour with the alpha dropped: pure black. Claude Code asks (`ESC ] 11 ; ? ST`) and paints a few
  cells in the colour it was told. `TerminalView.tsx` now answers OSC 10 and 11 queries with the
  theme's own foreground and background and leaves everything else to xterm. The boxes that
  remained were ANSI black backgrounds (SGR 40), which Claude Code paints under its prompt and
  status line: the palette's black is now the terminal ground and the pane is opaque unless
  Settings › Appearance › See-through terminal is on, where any painted background is a box.

## What the measurement did not cover

- WebKitGTK ran offscreen without GL (`WEBKIT_DISABLE_COMPOSITING_MODE=1`). A real window may have accelerated compositing, which would make cause 1 cheaper than measured; the app's observed behaviour on the user's machine (web process at 70-85% while idle, no GPU use) suggests it does not.
- The daemon and Tauri's real IPC were mocked (IPC cost emulated with `JSON.stringify`/`parse`). Keyboard focus, real resize and DPI scaling were not exercised in WebKit.
- Output rate (20 KB/s) is a guess at what Claude Code produces; cost scales roughly linearly with it.
- Single 15-20 s samples; WebKit numbers are ±4 points, Chromium ±3.

## Linux measurement, 2026-11-04 (after the plan, on v0.64.0 + plumbing)

WebKitGTK 2.52.6, software compositing (`WEBKIT_DISABLE_COMPOSITING_MODE=1`), the
harness's scenarios `a` and `b`, 20 s measured after 5 s warm:

| Scenario | 2026-11-03 | 2026-11-04 |
| --- | --- | --- |
| `a` one idle shell | 1.5% | 1.1% |
| `b` one working session, silent | 58% | 1.6% |

The sweep plays once and finishes (`v14sweep:finished`); no 80 ms timer fires;
the only intervals are the 15 s and 30 s ones. The silent-working target (under 5%)
is met.

**The streaming number** is in *Measured again on Linux* above: the mock now feeds the
`watch_terminal_output` channel with the host's frame header (machine, pane, sequence, replay).
Still owed: the hub's state messages, so that scenario `e` lists its four panes instead of one.

## Status effects and adaptive animations (2026-11-04)

Waves (Settings › Appearance › Motion) are the one deliberate loop: a 200%-wide SVG layer per band
translating -50% with `transform` only, in a clipped child of the row or tab. Auto starts at Waves
with WebGL 2 and at Sweep without it, samples frame times (3 s, 2 s after launch and again when Waves
begin to run) and steps down one level on slow frames; Full, Light and Off fix the level
([DESIGN](DESIGN.md), Status effects).

Harness, cage headless with pixman and `WEBKIT_DISABLE_COMPOSITING_MODE=1` (WebKitGTK 2.52.6; the page
still reports `data-gpu="webgl"` there), web process CPU as a share of one core, 15 s after 5 s warm,
base `a9b0828` against this branch, runs alternated. The machine was shared and the same build
swung between two modes, about 18-24% and about 46% on `b` and `w`, whatever the configuration, so
only same-mode runs compare:

| Scenario | Base | This branch (Waves) | Sweep (Light) | Off |
| --- | --- | --- | --- | --- |
| `a` quiet | 1.3-1.5% | 1.4% (3.7-3.9% in a loaded run) | | |
| `b` streaming | 16.5-33% / 46% | 16.6-18% / 46-48% | | |
| `w` six working rows, three tabs, Waves on 7 layers | 18-20% / 46% | 18.5-19% / 47% | 47% (loaded) | 32-44% |

Waves added nothing measurable over the base in either mode, so there is no Waves-only overrun to
hide. The streaming runs in the 46% mode are over the 30% budget for the base build too, so that is
the harness environment and the terminal path (cause 3 above), not the effects.

### Paper, scenario `b`: the occasional 37% (unexplained)

Seventeen finished Paper runs of `b` (settings `themeMode: light`, `lightTheme: Paper`, 15 s after 5 s
warm, cage headless with pixman): sixteen read 14.1-18.9% of one core on the main thread (17.8-21.0% for
the whole web process, where recorded), one read 37.7% (45.1% whole process), and an eighth run of the last
batch never finished. In that batch a fixed Python loop run before each run took 0.33-0.40 s in the fast
CPU mode and 0.80-0.96 s in the slow one.

Ruled out, by reading the page's own state at the end of every run:

- **Machine mode, partly.** The one slow-mode run that finished read 16.5%, so the slow mode alone does
  not make 37%. The 37.7% run had no control loop beside it, so its mode is not known.
- **Effects and animations.** Every run that printed it (the six of the second batch, the 37.7% one among them), had the same Animation list (four paused
  `mx-fx-drift` layers and the finished focus ring) and the same two Waves layers, so no animation
  started or stopped.
- **Compositing mode and renderer.** `data-gpu` was `webgl` and the page drew 61-62 frames a second in
  the bad run as in the good ones.
- **Hidden page.** `document.hidden` was false in all of them (`hasFocus` was false in all of them too).

What the bad run does differ in is the other threads: 7 points beyond the main thread against 2 in
the good runs, so extra work sat outside the page's script. It was the first run of a batch, which
might point at the harness (a cold web process, cage's Xwayland starting lazily) rather than the app,
but one sample cannot show that. If it appears in the app, compare `webProcessCpuPct` against
`webProcessMainThreadCpuPct` first.
