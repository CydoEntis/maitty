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

Tier 1 (small changes, measured gain; do first, measure after each):

1. [ ] Status effects: one-shot on change, never infinite, no `box-shadow` animation. Files: `workspace.css` (lines above), `components/motion.ts` (`useEffectClass`), Settings › Motion copy.
2. [ ] Spinner outside React, 150-200 ms, fixed width, paused when unfocused. Files: `lib/spinner-clock.ts`, `glyphs.tsx`, `StatusMark.tsx`, `workspace.css` `.mx-spin`.
3. [ ] Cursor blink only with window focus. File: `TerminalView.tsx`.

Tier 2 (needs a real machine to verify):

4. [ ] WebGL renderer with fallback; opaque terminal background; write coalescing; scrollbar timer off. Files: `TerminalView.tsx`, `theme/themes.ts`, `workspace.css` `.mx-pane`, `package.json`.

Tier 3 (waste; not felt as lag with one session):

5. [ ] Daemon: skip unchanged saves; snapshot once, only for attached clients.
6. [ ] Tauri host: batch output events; replays as bytes; push state instead of polling.
7. [ ] React: memo the big components, stop the per-poll double render, `matchMedia` once.

Verification for every step: run the WebKit harness scenario `b` (silent working session) and
`b` with the stream before and after, and put the two numbers in the commit message. The target is
a quiet working session under 5% and a streaming one under 40% on software-rendered WebKitGTK.

## What the measurement did not cover

- WebKitGTK ran offscreen without GL (`WEBKIT_DISABLE_COMPOSITING_MODE=1`). A real window may have accelerated compositing, which would make cause 1 cheaper than measured; the app's observed behaviour on the user's machine (web process at 70-85% while idle, no GPU use) suggests it does not.
- The daemon and Tauri's real IPC were mocked (IPC cost emulated with `JSON.stringify`/`parse`). Keyboard focus, real resize and DPI scaling were not exercised in WebKit.
- Output rate (20 KB/s) is a guess at what Claude Code produces; cost scales roughly linearly with it.
- Single 15-20 s samples; WebKit numbers are ±4 points, Chromium ±3.
