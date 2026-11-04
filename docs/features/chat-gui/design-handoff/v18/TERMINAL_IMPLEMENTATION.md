# Maitty Desktop v6: Terminal-only view, implementation notes

**Scope change:** chat is parked. The desktop app is a **terminal-first multiplexer window**: sidebar of projects and sessions, tabs, split native-terminal panes, Review (Changes / Files / Context), notifications and settings. The prototype is `Maitty Desktop v6 Terminal.dc.html`.

> **I couldn't read the repo.** These notes are written against the names you gave: `apps/gui/src/TerminalView.tsx`, `crates/maitty-core/src/daemon/pane_terminal.rs` (`pane_screen`, `pane_replay`, `pane_input`, `pane_resize`), `daemon/mod.rs` (`may_use_terminal`) and ADR 0045. Your message reached me starting at criterion 3, so criteria 1–2 are missing. Check §5 against the full list.

---

## 1. What's wrong with the current TerminalView (and why)

| Current behavior | Problem |
|---|---|
| Polls `pane_screen` every 250 ms | CPU and IPC cost scale with the number of panes. Output lands up to 250 ms late. Polling a *screen snapshot* also throws away everything that scrolled off between polls, so **scrollback is wrong**. |
| Writes the full screen after every keystroke | xterm re-parses and repaints the whole grid, so the screen **flickers**. The cursor and selection jump. If the poll and the keystroke write race, the screen goes **stale or repeated**. |
| Resizes the shared pane on fit | `FitAddon.fit()` runs on every layout change → `pane_resize` → the PTY reflows → output changes size → layout changes → fit runs again. That's a **resize loop**. It also changes the size for the **TUI**, which shares the session and must not be resized by the GUI (ADR 0045). |

## 2. Target data flow

```
attach(sessionId)
  ├─ may_use_terminal(client, pane)?   (daemon decides; GUI never overrides)
  ├─ pane_replay(pane) → { bytes, seq } (history + current screen as a VT byte stream)
  └─ subscribe(pane, fromSeq = seq+1)  → stream of { seq, bytes }

xterm.write(replay.bytes)    once, before any live chunk
xterm.write(chunk.bytes)     for each live chunk, in seq order
xterm.onData(d) → pane_input(pane, d)   (no local echo; the PTY echoes)
```

- **Push, don't poll.** Add (or reuse) a daemon subscription that streams raw PTY output **bytes** with a monotonically increasing `seq`. Over Tauri, forward with `Channel<Vec<u8>>` (Tauri 2), or events carrying base64 chunks, batched at most every ~8–16 ms.
  *Why:* xterm.js is a VT parser. Feed it the same bytes the TUI gets and it handles scrollback, colors and cursor itself.
- **Replay then stream, without gaps or duplicates.** `pane_replay` returns bytes **and** the seq they end at. The GUI subscribes from `seq + 1`. Any live chunk with `seq <= replaySeq` is dropped. A gap in seq means re-attach.
  *Why:* this is what makes "no stale or repeated screen" hold on attach and on reconnect.
- **Keystrokes are input only.** `onData` sends to `pane_input`. Never call `pane_screen` or write a screen snapshot after typing. The echo comes back through the stream.
- **Keep `pane_screen`** only as a fallback, for example when the replay buffer was truncated. In that case write `\x1b[2J\x1b[H` + the snapshot **once**, then switch to streaming.

## 3. Ownership and resize (ADR 0045)

- Ownership checks live in `may_use_terminal`. **Don't weaken them.** The GUI asks once at attach and gets back `{ canInput, canResize }` (or the equivalent). If the daemon refuses, show the pane **view-only** with the reason in the pane header ("Owned by the Maitty TUI · view-only").
- **The GUI never lists, types into, or resizes a session the TUI owns.** Concretely:
  - **Lists:** sessions the GUI may not use are filtered out daemon-side. Don't filter only in the UI, because a client-side filter can drift.
  - **Types:** `pane_input` is only wired when `canInput`. Otherwise `term.options.disableStdin = true`.
  - **Resizes:** `pane_resize` is only called when `canResize`. Otherwise the GUI fits **locally**: keep the PTY's cols and rows, and scale or letterbox the xterm viewport (`term.resize(ptyCols, ptyRows)` and let the container scroll or center). Never write back.
- **Breaking the resize loop** (when the GUI does own the pane):
  1. `ResizeObserver` on the pane container → `fit.proposeDimensions()`.
  2. Debounce 50–100 ms (trailing).
  3. Skip if `{cols, rows}` equals the last value **sent**, not the last value observed.
  4. Send `pane_resize` once. Don't call `fit.fit()` again in response to output.
  5. Ignore size changes caused by the font-size preference until fonts have loaded (`document.fonts.ready`).

## 4. Scrollback

- Set `scrollback` on the xterm instance (e.g. 10 000 lines, or from settings).
- History comes from `pane_replay` bytes, so scrolling up shows what really happened instead of repeated snapshots.
- **No jumps:** don't call `term.reset()` or rewrite the screen while the user is scrolled up. xterm keeps the viewport fixed while new output arrives as long as you only `write()` incremental bytes.
- **No flicker:** use the WebGL renderer (`@xterm/addon-webgl`), with a fallback to canvas or DOM on context loss. Batch chunks per animation frame: `queue.push(bytes); if (!raf) raf = requestAnimationFrame(flush)`.
- **Alternate screen** (Codex's full-screen TUI): xterm handles `\x1b[?1049h/l`. Replay must include those mode switches, so send raw bytes rather than a rendered screen.

## 5. Acceptance checks (from your list; criteria 1–2 to be filled in)

3. Attaching shows the **current** screen, not a stale or repeated one, and typing still works.
   *Test:* attach while output is streaming. Detach and reattach five times quickly. Type immediately after attaching.
4. The terminal app never lists, types into or resizes a session it doesn't own (ADR 0045).
   *Test:* open the TUI alongside and resize the GUI window, split panes and change font size. The TUI's pane size must not change. Typing in a refused pane does nothing, and its header says view-only.
5. Scrolling back works and doesn't jump or flicker.
   *Test:* `seq 1 100000` in a pane. Scroll up mid-stream: the viewport holds. Scroll to the top: the first lines are there once. No visible repaint flashes.

Also verify:
- CPU stays low with 4 idle panes (no polling).
- Reconnecting to maittyd replays without duplicates.
- Changing Settings → Appearance → Terminal text size refits once.

## 6. Sketch

```tsx
export function TerminalView({ sessionId, focused }: { sessionId: string; focused: boolean }) {
  const host = useRef<HTMLDivElement>(null);
  const [access, setAccess] = useState<Access | null>(null);

  useEffect(() => {
    const term = new Terminal({ fontFamily: 'Geist Mono', fontSize: prefs.terminalTextSize, scrollback: 10_000, allowProposedApi: true });
    const fit = new FitAddon(); term.loadAddon(fit);
    term.open(host.current!); tryWebgl(term);

    let lastSeq = -1, lastSent = '', raf = 0; const q: Uint8Array[] = [];
    const flush = () => { raf = 0; for (const b of q.splice(0)) term.write(b); };
    let off = () => {};

    (async () => {
      const a = await invoke<Access>('pane_attach', { sessionId });      // wraps may_use_terminal
      setAccess(a);
      if (!a.canView) return;
      const r = await invoke<{ bytes: number[]; seq: number }>('pane_replay', { sessionId });
      term.write(new Uint8Array(r.bytes)); lastSeq = r.seq;
      off = await subscribePane(sessionId, lastSeq + 1, (seq, bytes) => {
        if (seq <= lastSeq) return;                     // duplicate
        if (seq !== lastSeq + 1) return reattach();     // gap: replay again
        lastSeq = seq; q.push(bytes); if (!raf) raf = requestAnimationFrame(flush);
      });
      if (a.canInput) term.onData((d) => invoke('pane_input', { sessionId, data: d }));
      else term.options.disableStdin = true;
    })();

    const ro = new ResizeObserver(debounce(() => {
      const d = fit.proposeDimensions(); if (!d) return;
      if (access?.canResize) {
        const key = d.cols + 'x' + d.rows; if (key === lastSent) return;
        lastSent = key; term.resize(d.cols, d.rows);
        invoke('pane_resize', { sessionId, cols: d.cols, rows: d.rows });
      } // else: keep PTY size; container scrolls/letterboxes
    }, 80));
    ro.observe(host.current!);

    return () => { ro.disconnect(); off(); cancelAnimationFrame(raf); invoke('pane_detach', { sessionId }); term.dispose(); };
  }, [sessionId]);

  return <div ref={host} className="terminal-host" data-focused={focused} />;
}
```
`pane_detach` must **never** stop the session.

## 7. Design spec for the terminal-only window (prototype v6)

- **Removed:** chat messages, composer, Chat | Terminal switch, and chat-only states (tool activity, send failed, Codex unavailable).
- **Kept from v5:** sidebar with detailed rows, tabs, Review sheet, notifications, Open project, New session, Settings, and all status marks and tokens.
- **Terminal area:**
  - hint line plus **Split ⌘D** (up to 4 panes);
  - focused pane: accent border, 3px `--accent-soft` ring, "Focused" label;
  - drag headers to swap panes;
  - pane header: status mark · session picker · worktree path · "view-only this turn" / "Owned by the Maitty TUI · view-only" · ×. Closing a pane detaches and never stops the session.
- **Needs you:** when the focused pane's agent is waiting on an approval, an amber strip above the panes reads "Codex is asking in the focused pane. Answer in the terminal: `<cmd>`. Maitty never answers for you." The answer is typed in the PTY, so the provider's own prompt stays authoritative.
- **Notifications** for needs-you have **Open terminal** instead of Allow/Deny. The GUI doesn't answer approvals outside the provider's UI in terminal-only mode.
- **Failure:** "Couldn't attach to this session" (replay failed), with Try again and Open in Maitty TUI. Disconnected and recovery states are unchanged from v5.
