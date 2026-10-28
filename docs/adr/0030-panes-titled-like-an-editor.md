# 0030. Panes titled like an editor's, and toasts drawn to suit each look

- **Status:** Accepted
- **Date:** 2026-10-28
- **Amends:** [ADR 0028](0028-tab-rows-and-popups-drawn-like-panes.md) (pane titles, the toast)
  and [ADR 0027](0027-two-looks-and-tabs-in-the-sidebar.md) (a session's tab lines)

## Context

After 0.47 the user found the frame read as tabs twice: Floating's tab chip `1 BUgs ✕` sat right
over its pane's title `1 · BUgs` with `○ idle ✕` at the far right, every split pane had its own
✕, and Hairline's pane rows did the same. They pointed at Zed, where a pane is titled by its
tabs and nothing repeats. The sidebar named a session twice (its row `BUgs`, then a tab line
`1 BUgs` for the tab its own pane is in), the screen sat inside an empty cell of padding, and
the toast, a filled square box, looked wrong in Floating (the user wanted Hairline's outline,
rounded) and boxed in Hairline (they wanted a solid block with a coloured left edge).

## Decision

- **Pane titles:** a split pane's title is its number, its state's glyph and its name
  (`1 ● claude`) and at most a load note; no state word, no ✕. A tab's only pane has no title:
  Floating draws a plain top border, Hairline no title row or rule. The tab names it, shows its
  state (its glyph opens why) and closes it.
- **Closing:** every tab has a ✕, a session's only tab too (a split one closes all its panes);
  a pane closes with Alt+x, the leader's `x` or its menu. Each asks first (ADR 0029).
- **Sidebar tab lines** never list the tab the session's own pane is in: the row is that tab.
- **Padding** defaults to 0: the frame reaches the window's edge.
- **Toast:** Floating draws a `line` outline on the ground with round corners whatever
  `ui.rounded` says, its left side the notice's colour; Hairline a solid `card2` block, no
  outline, its left edge a heavy `┃` in the notice's colour.

## Alternatives considered

- **Tabs in the first pane's border, no tab row:** the old Floating, whose narrow-title rules
  were the hardest code in the frame (ADR 0028).
- **Keep the titles, drop only the ✕ and the word:** still two rows naming one pane.
- **Sidebar tab lines off by default:** hides a busy tab's state, which the lines exist to show.

## Consequences

A pane's ✕ (`ScreenHit::CloseSplit`) is now only a lone tab's. The reference frames' pane
tops are plain borders. Branch and project no longer show on a lone pane in Floating; the
sidebar's project header carries them.
