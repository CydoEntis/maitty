# Handoff: Seshi wake-up transition

## Overview
This is the transition a pane plays when a sleeping session wakes up (`zzZ asleep` → `○ idle`). Matrix-style rain fills the pane, then dissolves from the centre outwards. A "waking up" pill and a progress bar sit over it, and then the scrollback replays line by line.

## About the design file
`Seshi Wake Up.dc.html` is an **HTML reference**: a 160×45 cell grid drawn on a canvas. Open it in a browser with `support.js` beside it. Click the pane, or press Enter, to wake the session; click again to put it to sleep.

Rebuild it in the TUI renderer, cell by cell. Everything is plain characters with fg/bg colours, plus bold on the rain heads. The logic class in the file is the reference implementation: look for `frame()`, `initRain()` and `toggle()`.

## Fidelity
**High fidelity.** The timings, glyphs, colours and copy are final.

## Area
Only the pane's **inner area** animates: inside the border, excluding the footer row. The border, title, sidebar and tabs stay drawn as normal throughout. Only the state glyphs change:
- the pane's title tag: `zzZ asleep` → ` ⣾ waking ` (teal, bold) → `○ idle`
- the session's sidebar row glyph: `z` → `⣾` → `○`
- the tab pill: `1 z` → `1 ○`

## Timeline (ms from wake)
| Phase | Time | What happens |
|---|---|---|
| Rain | 0–900 | Every column rains at full speed. |
| Dissolve | 900–2600 | The rain slows to 0.6× and cells drop out. |
| Restore | 2600 + 55 ms per line | Scrollback lines appear top to bottom. |
| Done | when the last line is shown | State becomes `○ idle`. |

The "✓ awake" pill fades out over the first 900 ms of Restore.

### Rain (from 0 ms)
- **Glyphs:** half-width katakana `ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ`, digits `0-9` and `:.=*+<>¦`.
  - All are single-cell (the half-width forms U+FF66–FF9D).
  - Fallback if the font lacks katakana: digits and symbols only.
- **Columns:** one stream per inner column.
  - Starts at a random height above the top, from −innerHeight to 0.
  - Speed is random, 0.35–1.25 cells per frame at 60 fps (scale to your tick rate).
  - When a stream falls past the bottom by its length (6–23 cells), it restarts 0–7 rows above the top with a new speed.
- **Head:** the cell a stream enters gets a new random glyph and brightness 1.0. It is drawn bold in `#d8fffb`.
- **Trail:** every cell's brightness × 0.955 per frame. Its fg is `mix(paneBg, teal, min(1, a × 1.1))`. Cells below 0.04 are blank.
- **Shimmer:** each frame, each lit cell has a 2% chance to swap to a random glyph.

### Dissolve (900–2600 ms)
- Each inner cell has a fixed death threshold:
  - `die = 0.15 + dist × 0.55 + rand × 0.3`
  - `dist` = the normalised distance from the centre (0 at the centre, capped at 1)
  - So the centre clears first and the edges last, with noise.
- **Progress:** `p = clamp((t − 900) / 1500)`.
  - A cell is gone once `p × 1.15 > die`; it never comes back.
  - Cells within 0.08 of their threshold draw dim, at `mix(teal, paneBg, 0.7)`, so they flicker out rather than pop.
- **Slow agent** (spec only; the reference doesn't do this yet): cap `p` at 0.85 and loop the rain in the surviving cells until the agent reattaches, then let the dissolve finish.

### Overlay (Rain and Dissolve)
- **Pill:** centred, one row above the pane's middle. Background `#333333` with round ends `U+E0B6` / `U+E0B4`.
  - Copy: `⣾ waking up · picking up where it left off`
  - The spinner glyph is amber `#e5a50a` and steps through `⣾⣽⣻⢿⡿⣟⣯⣷` every 80 ms.
  - "waking up" is teal and bold; the rest is in `text`.
- **Bar:** 2 rows below the pill. 24 cells of `━`: filled in teal, empty in `line`.
  - Fill: `clamp(t / 2400)`.
  - Then a step label, dim: `restoring session` (until 900 ms), `reattaching opus 4.5` (until 1800 ms), `replaying scrollback`.

### Restore (from 2600 ms)
- Scrollback lines appear one every 55 ms, from the top of the inner area, with their own colours.
- The `✓ awake · back where it left off` pill fades from full colour to the pane bg over 900 ms.
- **Done:** the cursor shows at the prompt, and every state glyph switches to `○ idle`.

### Asleep (the resting state)
Centred in the pane, two lines with one blank row between:
- `zzZ brave-wren is asleep`: `zzZ` dim, the rest in `text`
- `click or Enter to wake it · its session is saved`: `Enter` teal and bold, the rest dim

## Input
- **Wake:** click anywhere in the pane, or press Enter or Space while it's focused. A wake can also start on its own when an agent is resumed (e.g. from the Inbox).
- **Sleep:** reverting to asleep is instant, with no animation.
- **Reduced motion** (spec only; the reference doesn't do this yet): skip Rain and Dissolve, fade the scrollback in over 300 ms, and switch the state directly. Offer this as a setting: Appearance › Motion › Wake-up animation (on / simple / off).

## Colours (as in the screenshot theme; map to roles)
| Role | Hex | Use |
|---|---|---|
| desk | #1c1c1c | background around the cards |
| pane | #222222 | inside of the pane |
| line | #3a3a3a | borders, empty bar |
| teal (accent) | #22b8b0 | focus border, rain trail, "waking up", bar fill |
| head | #d8fffb | rain heads (accent mixed toward white) |
| pill | #333333 | overlay pill |
| amber | #e5a50a | spinner |
| green | #7fb800 | ✓, branch |
| text / dim / strong | #c8c8c8 / #7a7a7a / #eeeeee | text levels |
| selection | #5b5fc7 | selected sidebar row |

Derive these from the active theme:
- rain colours from `accent`
- the head from `mix(accent, #ffffff, 0.85)`
- the pill from `card2`

Check that the pill text passes the contrast audit in every theme.

## Performance
- Only the inner area is redrawn each frame. That is up to about 6k cells at 240×60.
- Run at the renderer's tick rate (30–60 fps); the timings are in ms, so they don't depend on frame rate.
- Stop the loop as soon as Restore finishes.

## Files
- `Seshi Wake Up.dc.html`: the reference animation (open it in a browser)
- `support.js`: runtime for the HTML viewer only
