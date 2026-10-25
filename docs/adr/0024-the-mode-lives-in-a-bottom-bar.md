# 0024. The mode lives in a bar along the bottom, with key suggestions

- **Status:** Accepted
- **Date:** 2026-10-24

## Context

ADR 0023 showed the mode three ways: a tag in the corner of row 0, a sky frame round the
window in shortcut mode, and a toast on a dead key. The user tried v0.43.3 on 2026-10-24 and
chose a different design: one bar along the bottom that always says the mode and, in shortcut
mode, what the keys do. The corner tag also cost the top-right title up to 26 cells, and the
frame drew over the padding in a way the user did not want kept. ADR 0014 had ruled out an
application-wide footer bar; the user's decision on 2026-10-24 reverses that for this one bar.

## Decision

- **One full-width row at the bottom of the room inside `ui.padding`** is the mode bar
  (`screen/mode_bar.rs`). The layout (sidebar, panes, sheet, toast) is one row shorter; popups
  and their dimming sit above the bar, so it is never covered or dimmed.
- **Typing:** quiet, on `bg`: ` TYPING ` (`muted` on `btn`) and `· Ctrl+Space for shortcuts`
  in `muted`, naming the leader key from the keymap. A click on either starts shortcut mode, as
  the leader does.
- **Shortcut mode lit:** the whole row is `sky()`. A bold ` SHORTCUT ` chip (`sky` on `bg`;
  a click opens the key map) and then key suggestions read from the keymap: `n shell`,
  `s session`, `c changes`, `f files`, `i inbox`, `, settings`, then `? all keys` and
  `Esc type`. Keys are bold, labels plain, in `ink_on(sky)`; a rebound key shows its new key, an
  unbound action is left out. A narrow bar drops suggestions from the right, then Update
  shortens and goes, then `? all keys`, then `Esc type`. A click on a suggestion presses its key
  in shortcut mode, so it does exactly what the key does and the mode stays.
- **Paused** (a popup or view an action opened has the keys): ` SHORTCUT · paused ` (`muted` on
  `btn`) and no suggestions, since the popup shows its own keys. Under anything that has the
  keys (a popup, modal or view opened from either mode, or the key map and its search) the bar
  only says the mode: a click on it does nothing, so it never closes the popup, runs a
  suggestion or drops what was typed.
- **Update and Restart** move from the top-right corner to the bar's right end, clickable
  except while paused. The top-right title gets its corner back.
- **No frame round the window and no corner tag** in any mode. The focused pane's sky border
  in shortcut mode and the dead-key toast (ADR 0023) stay.

Amends ADR 0023 (the frame and the corner tag go; the toast and the always-shown mode stay) and
ADR 0014's no-footer rule (this bar is the one exception).

## Alternatives considered

- **Keep the corner tag and frame (ADR 0023):** the user found the bar clearer and preferred
  the corner for the title.
- **Update kept in the top-right corner:** it would keep the corner-room logic for one pill
  that is rarely there; on the bar it has a fixed place and costs the titles nothing.
- **Keys as raised keycaps:** two cells each; bold keys fit more suggestions at 80 columns.

## Consequences

Every look loses one row to the bar. The v7 reference frame tests draw the world one row
taller and leave out that row, the bar's (`v7_fixture::cells`), so the handoff's frames hold
cell for cell with no corner normalisation. Revisit if the bar should carry more (a session
count, a clock) or if a theme's `sky` makes the lit bar hard to read.
