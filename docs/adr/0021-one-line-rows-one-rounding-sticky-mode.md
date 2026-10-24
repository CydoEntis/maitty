# 0021. One-line session rows, one rounding setting, and shortcut mode that stays

- **Status:** Accepted (amended by [ADR 0023](0023-the-mode-is-always-shown.md))
- **Date:** 2026-10-24

## Context

The user asked for four changes on 2026-10-24:

- The sidebar's second line repeated what the mark already said (`working`, `needs you`,
  `done · not seen`), and a two-line selected or hovered row could not get round ends: a
  terminal has no glyph for a rounded corner on a block taller than one line.
- Two settings rounded things (`ui.corners` for card corners, `ui.rounded` for pill ends),
  so turning rounding on took two steps and half-rounded screens were easy to end up with.
- Shortcut mode (ADR 0016) ended by itself after an action that opened something to type
  into, and after any popup closed, so repeated actions still needed the leader.
- The screen ran into the window's edges.

## Decision

- **A session row is one line.** The mark and the name carry the state: a name in a state
  worth a look (working, checking, done and not seen, needs you) is drawn in its state's
  colour, shimmering or pulsing with motion on. At the right: the agent's mark (`✻` Claude,
  `◆` Codex, `✦` Gemini, `◇` OpenCode, else its first letter; letters with the ASCII glyph
  set), the checkout's uncommitted lines (`+N −N`, counted in the background every 5 s) and
  the age. A second line shows only when it says more: a question, a check result, helpers
  asked for, a plan-limit wait, a new agent's first step, asleep, a quiet spell, helpers at
  work, or a message just sent.
- **One rounding setting**, `ui.rounded`: `full` (round corners and half-circle ends on
  every chip, button, row and hover; needs a Nerd Font), `corners` (rounded squares: round
  card corners, square ends) or `off`. Older files still load: `rounded = true` is `full`,
  `false` is `off`, and `corners = "rounded"` alone is `corners`. The pointer's hover tint
  takes the same round ends.
- **Shortcut mode stays until you leave it**: only Esc, the leader again, Esc in the key map
  or a click end it. An action that opens a popup hands it the keys, and closing the popup
  returns to shortcut mode. Amends ADR 0016.
- **Padding round the screen**, `ui.padding` `0`, `1` (the default) or `2` cells, in every
  layout. The handoff's reference frames have none, so their tests pin it to 0.

## Alternatives considered

- **Two-line rows with the meta below:** room for long branch names, but only stepped
  corners on the highlight. The user chose one line.
- **Keep two rounding switches:** the user asked for one control.
- **Shortcut mode stays after popups only:** a new pane would still drop you into typing.
  The user chose Esc, the leader or a click as the only ways out.

## Consequences

A row's state is read from its mark and its name's colour, not a word. Typing into a new
pane needs Esc first while shortcut mode is on; the `SHORTCUT` pill and the sky border say
so. The sidebar runs `git diff --numstat` and `git ls-files --others` per checkout with a
session every 5 s, off the UI thread.
