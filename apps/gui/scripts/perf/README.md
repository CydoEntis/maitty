# GUI performance harness

Runs the real production build of the desktop GUI in a headless browser with a mocked daemon and
Tauri bridge, and measures what the main thread does. Written for the audit in
`docs/PERFORMANCE.md`; the tables it produced on 2026-11-03 are `results-chromium-2026-11-03.txt`
and `results-webkit-2026-11-03.txt`.

These scripts were written for one machine during the audit and carry its paths. Before running:

- Build the GUI into this folder: `cd apps/gui && pnpm exec vite build --outDir scripts/perf/dist --emptyOutDir --sourcemap` (the harness serves `dist/` next to itself; never point it at `apps/gui/dist`).
- `harness.mjs` imports Playwright from `~/.local/share/mise/installs/npm-playwright/latest/node_modules/playwright/index.mjs` and launches `~/.cache/ms-playwright/chromium-1223/chrome-linux64/chrome`. Edit the two constants at the top (`import` and `CHROME`) for another machine.
- `webkit-harness.py` needs `python-gi` and the system WebKitGTK 4.1 (`webkit2gtk-4.1`). It runs offscreen with `WEBKIT_DISABLE_COMPOSITING_MODE=1` (the only mode that works without GL offscreen) and reads the web process's main-thread CPU from `/proc`.

## Files

- `mock-tauri.js`: injected before the app loads; fakes `isTauri`, `__TAURI_INTERNALS__`, the event plugin, and a daemon answering `list_sessions`, `terminal_history`, `repo_query`, input, resize, window, updater, notification and dialog calls. Streaming sessions get `terminal-chunk` events every 50 ms at about 20 KB/s of agent-style output. Timers are wrapped to count firings.
- `harness.mjs`: Chromium runs. `node harness.mjs --scenario a|b|c|d|e --gpu gpu|cpu [--measure 20] [--warm 5] [--exp <name>] [--dsf 2] [--poll]`. Scenarios: a = 1 idle shell; b = 1 working session with a stream; c = 4 panes, 2 streaming; d = resize during b; e = 4 sessions listed, 1 streaming. Experiments (`--exp`): `noanim`, `nocursorblink`, `hidesidebar`, `hidetabbar`, `hidestatusbar`, `hidepanehead`, `hidetitlebar`, `hidechrome`, `hideterminal`, `noshadow`, `nogradient`, `flatstyle`, `reducemotion`, `stop1s`, `stop80ms`, `stop1500`, `stopalltimers`, `nostream`, `bare`. Writes `results/<label>.json`, a trace, a CPU profile and a screenshot.
- `analyze.mjs`: turns a run's trace and profile into the summary row (busy %, script/style/layout/paint/composite ms per second, frames, timers, long tasks, hot functions mapped through source maps).
- `summarize.mjs`: collects `results/*.json` into `results/summary.txt`.
- `inspect-anims.mjs`: cancels each running CSS animation in turn and reports the change in busy %.
- `webkit-harness.py`: `WEBKIT_DISABLE_COMPOSITING_MODE=1 python3 webkit-harness.py --scenario b [--exp ns_nosweep]`. The `ns_` experiments run the scenario without the stream. Appends to `results/webkit-summary.txt`.
- `run-all.sh`: every Chromium scenario and experiment, three at a time.

Results are large (traces and profiles run to gigabytes); `results/` and `dist/` are not committed.
