# 0023. The mode is always shown, the window is framed in shortcut mode, and dead keys say so

- **Status:** Accepted
- **Date:** 2026-10-24

## Context

The user tried v0.43 on 2026-10-24 and could not tell which mode they were in. They were in
shortcut mode, typed, nothing reached the terminal, and they thought Maitty had frozen. The
only signs were a small `SHORTCUT · Esc` pill at the top right, the focused pane's border in
sky, and a dimmed `SHORTCUT · paused` pill under a popup. Input mode showed nothing (ADR 0016
chose that: a label while typing is a label nearly all the time). A key with no action in
shortcut mode was dropped without a word. ADR 0016's consequences already said the mode must
stay hard to miss. The user asked for the mode to be unmistakable before the next release.

## Decision

- **A frame round the whole window while shortcut mode is on.** With `ui.padding` 1 or 2
  the outermost padding ring is a solid sky line: `▀` along the top, `▄` along the bottom,
  `█` at each side, so all four read about equally thick. With padding 0 there is no ring,
  so the box edges on the window's edge turn sky and the bare ground between them takes a
  thin line; chips, words and Compact's header strip stay as they are. While a popup or
  view holds the mode, the frame stays, faded 60% toward the ground and drawn above the
  dimming, like the paused pill. Input mode has no frame. Every look, every sidebar place.
- **The mode tag is always in the corner of row 0:** ` TYPING ` (quiet, `muted` on `btn`)
  in input mode, ` SHORTCUT · Esc to type ` (sky, bold) in shortcut mode, ` SHORTCUT ·
  paused ` (dimmed) while held. Under 100 columns of room the lit tag reads ` SHORTCUT · Esc `;
  under 76 (narrower than any supported window) input mode shows no tag. **Update** or
  **Restart** sits a cell left of the tag, which keeps the corner. A click on the tag opens
  the key map, as the leader's pause does; it changes nothing risky. Typing under a popup,
  the tag takes no click, so the popup's outside-click still works.
- **The top-right title holds still.** From 140 columns of room it leaves the widest tag's
  room in all three modes; narrower, input mode leaves only `TYPING`'s, so a narrow title
  keeps its tabs, and lit and paused share the wider of their two. A narrow pane's status
  glyph is still the last thing its title drops.
- **A key with no action in shortcut mode shows a toast**, `Shortcut mode — Esc to type`, at
  most once every 6 s and never over a toast still showing; it is not kept in the
  notification history. Keys that act are unchanged.

Amends ADR 0016 (input mode is now shown) and ADR 0021 (the mode's signs).

## Alternatives considered

- **Only a louder pill:** a corner is easy to miss when you are looking at the terminal; the
  user chose a frame you see wherever you look.
- **A sky background in the whole padding ring:** reads strongly, but the sides are a full
  cell wide and the top and bottom a full row tall, so the frame looks heavier at the top;
  the half-row and full-column glyphs read even.
- **Leave input mode unlabelled (ADR 0016):** that is what made the frozen-looking screen
  possible; the user asked for the mode always shown.
- **Clicking TYPING enters shortcut mode:** a mode change from a stray click; opening the key
  map is harmless and shows what the leader does.

## Consequences

Row 0 always gives the corner up to the mode tag, so the top-right title is shorter by up to
26 cells; at 80 columns a second tab can give way. The v7 reference frame tests take the tag
and its room out of row 0 (`v7_fixture::frame`), as they already take out the `+` chip.
Revisit if the frame clashes with a terminal's own border, or if a hint toast ever hides
news.
