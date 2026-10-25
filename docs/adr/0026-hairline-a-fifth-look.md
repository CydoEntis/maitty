# 0026. Hairline, a fifth look with no boxes

- **Status:** Accepted
- **Date:** 2026-10-25

## Context

The user supplied a design handoff (option 1c, "split, one needs you") for a workspace with
less frame and more work: no box round the sidebar or the panes, one title row and one rule
per pane whose colour says its state, one row per session in the sidebar, and a status strip
that keeps every session's state in view. Amber means needs you and nothing else. The user
asked for it as a new layout beside the four of ADR 0015, leaving Floating, Tiled, Compact
and Grid exactly as they are.

## Decision

`ui.layout = "hairline"` is a fifth look over the same sessions, tabs, splits and sheets.
Panes split as in Tiled. The drawing differs:

- **Sidebar:** 28 columns and a `│` in `line`; `sessions` and the count; projects as dim
  headers with their branch; one row per session (`▎` marker, glyph, name, age or
  `needs you`), no second lines but a request to start helpers; the new and menu hints pinned
  to the foot with their real keys.
- **Panes:** a title row (its glyph, its name in `strong`, `project · ⎇ branch`, the state
  word and the ✕; no number, since no key goes to a pane by one) over a full-width `─` rule:
  teal focused, amber needs you, else `line`. `│` / `─` only between panes, never round
  the outside; two columns of padding inside.
- **Bottom:** a status strip, one `━` segment per session in its state's colour, over the
  mode bar, which adds the look and pane count after TYPING and `● N needs you` or
  `all clear` at its right end (before Update or Restart, which keep the end). Shortcut mode
  keeps its sky row.
- **Always square:** `ui.rounded` is ignored while Hairline is chosen (`Ui::rounding`), and
  Settings shows the Rounded row as not in Hairline. The look is lines meeting at right
  angles; a rounded end or corner would be the only curve on screen, and a one-row title has
  no box for a corner to round.

Drawing that has a Hairline variant asks `Look::hairline` once and draws from
`client/screen/hairline/` with its colours from `hairline::Ink`, the handoff's tokens mapped
onto the theme's roles, so every theme draws it.

## Alternatives considered

- **Restyle Tiled or Compact:** the user wanted the four looks unchanged.
- **Honour `ui.rounded` in Hairline:** there are no boxes or chips to round, and rounding the
  few spots that could take it would break the one-weight line language.
- **The handoff's fixed colours:** clash with every other theme and skip the contrast audit.

## Consequences

A fifth look to test: `hairline/tests.rs` draws it at three sizes with one, two and four
panes. Hairline costs two bottom rows instead of one. Menus, popups and side panels still
draw as cards (square) in Hairline until they get a Hairline variant of their own.
