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

4. [x] `@xterm/addon-webgl` is loaded after `open()` when the web view has WebGL 2, with the DOM renderer as the fallback (no WebGL 2, the addon failing to start, or a lost context). With the DOM renderer the terminal background is the theme's opaque `term` colour and writes are flushed at most every 32 ms; with WebGL the canvas stays see-through over the card. Not done: xterm 6 has no option for the scrollbar's hide timer (`vertical: Auto` is hard-coded in its scrollable element), so that 500 ms timer still resets per scrolled line while the DOM renderer draws. Whether WebGL 2 works on the Linux WebKitGTK build is still to be seen on a real machine.

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

**Still owed: the streaming number.** The harness's `mock-tauri.js` still emits
`terminal-chunk` events and answers `list_sessions`, but the app now takes output
through the `watch_terminal_output` channel and session state through the hub's
messages, so in the harness no bytes reach xterm and scenario `e` lists one pane.
Before the next measurement: teach the mock the channel (raw frames with the
machine in the header, see `apps/gui/src-tauri/src/streams.rs`) and the hub's
state messages, then rerun `b` and `e`. Ticket this with the performance page.
