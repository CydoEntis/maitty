# Handoff: Maitty splash screen — "All agents on deck."

## Chosen version
Title: **AI on deck** · Letters: **Block**. These are the defaults in the prototype. The other options (Classic, Tall, Shadow, Rope, Tide) are exploration only. Ignore them.

## About the design files
`Maitty Splash.dc.html` is a **design reference built in HTML**: a browser cell grid that imitates a terminal. It is not production code. Build it in Maitty's real TUI renderer. `splash.js` is the source of truth for every cell: `render(s)` draws the screen and `act(s, a)` handles keys and clicks. To view it, serve the folder (`npx serve`) and open the .dc.html.

## Fidelity
High fidelity. Exact layout, copy, colours and timings.

## When it shows
On launch, before the main screen. Keys work from the first frame; nothing waits for the animation.

## Layout (centered, top to bottom)
1. **Wordmark MAITTY**, block letters, 5 rows tall.
   - Bitmaps: `FONT` in `splash.js` (M 5w, A 4w, I 3w, T 5w, T 5w, Y 5w).
   - Each font pixel is `px` cells wide: 2 when the terminal is ≥110 cols, 1 below. Gap between letters is `px`.
   - Draw pixels as **cell background fills**, not `█` glyphs, which leave seams.
   - Colours: **A and I** in sea-glass teal `#3dd6c6`, the rest in foam `#e8f1f8`. The name hides "AI", and the teal brings it out.
2. **1 blank row**, then **the swell**: one row under the name, 8 cells wider than the wordmark (4 each side).
   - Characters cycle `▁▂▃▄▅▄▃▂`, offset by column, advancing one step every 3 frames.
   - Colour `#1f6f8b` (sea), faded toward the background over the 6 cells at each end.
3. **1 blank row**, then the tagline **"All agents on deck."** in italic, text colour, centered.
4. **2 blank rows** (1 below 30 rows), then the **choices**, as cards 56 cols wide (or screen − 8) and centered:
   - **Resume where you left off** `r`: 2 rows, with the last session (`seshi › TUIOS Research`) on row 2 in dim. Selected by default.
   - **New session** `n`: 1 row.
   - **Open a project…** `o`: 1 row.
   - **Selected:** bg `#15253a`, a teal `▌` bar on the left edge of every row, bold strong label, keycap teal `#3dd6c6` with dark ink.
   - **Unselected:** no bg, text label, keycap `#1a2c44` with strong text.
   - Keycap `" k "` sits 4 cols in from the card's right edge, on the label row. Label starts 2 cols in.
   - 1 blank row between cards (0 below 30 rows).
5. **1 blank row**, then the **hints**, centered: `[↑↓] choose   [Enter] open   [?] keys`, keycaps on `#1a2c44` with labels in dim.
6. **Version** `maitty 0.9.0` bottom-right, in the line colour `#1d2d42` (barely visible).

The whole block is vertically centered.

## Animation (50 ms frames)
- **Letters fade up from deep water**, left to right: letter k starts at frame 3k and takes 8 frames with ease-out cubic, colour `#123049` → its final colour. Nothing moves.
- **Swell** fades in over the first 14 frames, then keeps rolling slowly.
- **Tagline** fades in over frames 10–20.
- **Choices and hints** are drawn at full strength from frame 0.
- **Reduce motion / motion off:** draw the final frame; the swell is static.

## Keys
↑↓ or j/k move the selection · `r` `n` `o` jump to that choice · Enter opens the selected one · `?` keys · click a card to select it.

## Sizes
- **≥110 cols:** 2-cell pixels.
- **<110 cols:** 1-cell pixels.
- **<30 rows:** Resume drops its second line, and the gaps between cards and above the choices shrink. Verified at 160×45, 120×34 and 80×24.

## Colours
| Role | Hex |
|---|---|
| bg | #08111d |
| selected card | #15253a |
| keycap | #1a2c44 |
| line / version | #1d2d42 |
| dim | #6f8399 |
| text | #c9d6e3 |
| strong | #eef5fb |
| foam (wordmark) | #e8f1f8 |
| sea-glass accent (A, I, selection) | #3dd6c6 |
| sea (swell) | #1f6f8b |
| deep (fade start) | #123049 |

## Files
- `Maitty Splash.dc.html` — prototype (open this)
- `splash.js` — layout, font, animation, keys
- `seshi-grid.js` — cell-grid renderer
- `support.js` — prototype runtime only
