# Glass and Tiled, v18 redesign

This is the design side of the "Glass on light themes" ticket. The prototype is `Maitty Desktop v18.dc.html`.

Screenshots are in `screens/`: Glass in Midnight, Monokai, Paper and Day (Day with Settings open), and Tiled in Midnight and Paper.

> **Not done until measured.** I can't run `apps/gui/scripts/perf/webkit-harness.py` from here. The PR that ports this must include the before/after numbers for scenarios **a** and **b**: quiet under 5% of one core, streaming under 30%. Numbers above budget mean the design is not done.

## 1. What changed and why

| Before (v17 Glass) | After (v18 Glass) | Rule it satisfies |
|---|---|---|
| Ambient ground hardcoded dark (`#1B201F` greens) in every theme, so Paper got a dark smudge | Ground built from each theme's `--bg` plus two faint accent and magenta tints | Readable in light |
| Cards: 140px top gradient + `0 18px 50px rgba(0,0,0,.5)` + `backdrop-filter: blur(18px)` on the sidebar, panes, tab bar and drawers | Cards opaque `--main`, 1px `--pane-line` hairline, 1px top highlight (`inset 0 1px 0`, no blur radius). No drop shadow, no blur | Cheap to draw |
| `--pane` translucent (.86 / .92) | `--pane` = `--main` (opaque) in Glass | Opaque terminal |
| Blur on every overlay, including drawers that stay open | Blur only on menus, popovers, tooltips, listboxes, find, centred dialogs and their scrims (`--blur-pop`). Drawers (`aside`) stay unblurred | Blur on short-lived surfaces only |
| Paper `--dim` #646E78 on #FAFBFA = 4.6:1 (borderline); accent 4.2:1 on ground | Retuned Paper, plus a new **Day** theme. All text tokens pass AA on `--term` and on `--bg` (§3) | WCAG AA |
| Needs-you badge: `box-shadow` pulse, 2.4s, infinite | `transform` + `opacity` pop, .6s, **once** | No loops; transform/opacity only |
| Update dot: `box-shadow` breathe, infinite | `transform` + `opacity`, .7s, once | Same |
| Terminal caret blink (CSS, infinite) | Steady caret. Blink is xterm's own option, off by default | No loops |
| Text spinners repaint their parent card | Each spinner is `display:inline-block; contain: paint; will-change: transform`. Rotating spinners add `will-change: transform` | Isolate what changes on its own |

Spinners are the one thing that still ticks, because Working needs a moving shape. They're isolated so a tick repaints only the glyph.

## 2. Glass tokens (per theme)

```css
/* workspace.css: [data-style="glass"] on the app root */
[data-style="glass"] {
  --card: var(--main);            /* opaque */
  --pane: var(--main);
  --sh: inset 0 1px 0 rgba(255,255,255,.045);  /* dark: top highlight only */
  --blur: none;                   /* persistent surfaces */
  --blur-pop: blur(14px) saturate(1.2);
  --sh-pop: 0 16px 40px rgba(0,0,0,.45);
  --glow:
    radial-gradient(1000px 600px at 15% 0%, color-mix(in oklab, var(--accent) 10%, transparent), transparent 70%),
    radial-gradient(900px 560px at 100% 100%, color-mix(in oklab, var(--ansi-magenta) 7%, transparent), transparent 70%),
    var(--bg);
}
[data-style="glass"][data-theme-kind="light"] {
  --sh: inset 0 1px 0 #fff;
  --sh-pop: 0 12px 32px rgba(17,24,32,.14), 0 0 0 1px rgba(17,22,27,.08);
  /* glow: same recipe, accent 9% and magenta 6% */
}
/* Software path: same rules, cheaper values */
[data-gpu="software"] {
  --blur-pop: none;
  --sh-pop: 0 0 0 1px var(--line2);   /* light: rgba(17,22,27,.14) */
  --glow: var(--bg);
}
```

**Applying the tokens:**
- Large cards (sidebar, panes, tab bar, status bar, drawers, session panel) use `background: var(--card); border: 1px solid var(--pane-line); box-shadow: var(--sh)`.
- Popovers and dialogs use `var(--sh-pop)` and `backdrop-filter: var(--blur-pop)`.
- The ground is the app root's background. It's painted once and never repainted by spinners, because every card above it is opaque.

### Theme values

| Token | Midnight | Monokai | Paper (retuned) | Day (new) |
|---|---|---|---|---|
| --bg | #090C10 | bg mixed with 18% black | #ECEEEA | #E9EEF3 |
| --main / --card | #0D1116 | #272822 | #FFFFFF | #FFFFFF |
| --term | #06080B | #272822 | #FBFBF9 | #FFFFFF |
| --fg | #E8ECEF | #F8F8F2 | #11161B | #0E141A |
| --fg2 | #B3BCC5 | mix 80% | #353E46 | #30404D |
| --muted | #8C96A1 | mix 62% | #4E5862 | #4A5A68 |
| --dim | #75808B | mix 52% | #5B6570 | #566573 |
| --accent | #5FD4BF | #66D9EF | #0A7564 | #0A6F86 |
| --warn | #F2B65A | #F2B65A | #8A5000 | #8C4F00 |
| --danger | #F07F78 | #F07F78 | #B3342A | #B1302A |
| --ok | #8DB89B | #A6E22E | #28713F | #23703E |
| --pane-line | rgba(255,255,255,.07) | fg 10% | rgba(17,22,27,.10) | rgba(14,20,26,.10) |

The status colours keep their hue family in every theme, so Working, Needs you, Done, Exited and Idle stay as distinguishable as today, and each still has its own shape.

## 3. Contrast (WCAG 2.1, against --term / against --bg)

| Token | Paper | Day | Midnight |
|---|---|---|---|
| fg | 17.6 / 15.6 | 18.5 / 15.9 | 16.9 |
| fg2 | 10.5 / 9.3 | 10.7 / 9.2 | 10.4 |
| muted | 7.0 / 6.2 | 7.1 / 6.1 | 6.7 |
| dim | 5.7 / 5.1 | 6.0 / 5.1 | 5.0 |
| accent | ≥4.8 / ≥4.5 | 5.8 / 5.0 | 11.1 |
| warn | 6.3 / 5.6 | 6.5 / 5.6 | 11.1 |
| danger | 5.9 / 5.2 | 6.3 / 5.4 | 7.7 |
| ok | 5.8 / 5.1 | 6.1 / 5.2 | 9.0 |
| ansi blue / magenta / green | 6.6 / 6.7 / 6.0 | 6.6 / 7.0 / 6.4 | n/a |

**Still to check:**
- **Monokai:** `--dim` is derived (fg at 52%). Check it in the real theme file and raise it to 55% if it falls under 4.5.
- **Generated light themes** (Solarized Light, GitHub Light, Catppuccin Latte…): run the same check in CI. This overlaps the separate light-mode contrast ticket, so do it there.
- **Font weight:** also a separate ticket. Light text at weight 400 in Geist Mono reads thin on white; the fix belongs to that ticket, not to the tokens.

## 4. Animation rules (all styles)
- **Nothing loops while a session is working or waiting.** Status changes may play one effect that settles: the needs-you pop, the update dot, and the one-pass row shimmer from v17 (C.4).
- **Animate only `transform` and `opacity`.** Never `box-shadow`, `filter` or `backdrop-filter`.
- **Reduce motion** turns off all of the above. Spinners show a static frame.

## 5. Tiled, v18 (the hairline grid)
Tiled moved toward a hairline-grid look: flush, technical, square.

- **Spacing and depth:**
  - pane gap and outer margin **0**; every region is separated by a 1px `--line` / `--pane-line`;
  - radius 0, no drop shadows;
  - popovers are hairline-only (`0 0 0 1px var(--line2)`), with no blur.
- **Chrome type:**
  - tab titles, pane process names, project names and the status bar use **Geist Mono, uppercase, letter-spacing .1em, 11–11.5px**;
  - terminal text and session names stay as they are.
- **Title bar:** `--main` with a 1px bottom hairline.
- **Corner ticks:** small 9px "+" ticks (1px `--line2`) at the four corners of the pane area mark the grid, as on the reference site. They're static, so they cost nothing.
- **Unchanged:** Flat.

**CSS hooks:**
- `--ch-ff`, `--ch-tt`, `--ch-ls`, `--ch-fs-sm`, `--ch-fs-md` on the root. Tiled sets them; Flat and Glass leave them at `inherit` / `none` / `normal`.
- `--tick`: transparent except in Tiled.

## 6. Done when
- [ ] Token values landed in the theme files, and Glass/Tiled rules in workspace.css (not a new stylesheet).
- [ ] Harness scenarios a and b, before and after, are in the PR and under budget.
- [ ] Screenshots in Midnight, Monokai, Paper and Day, each with one Claude Code pane running, one idle shell and Settings open.
- [ ] docs/DESIGN.md describes the result (no history).
