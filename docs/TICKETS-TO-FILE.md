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
