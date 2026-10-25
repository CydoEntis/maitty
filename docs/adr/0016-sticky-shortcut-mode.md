# 0016. The leader opens a shortcut mode that stays until you leave it

- **Status:** Accepted (amended by [ADR 0021](0021-one-line-rows-one-rounding-sticky-mode.md) and [ADR 0023](0023-the-mode-is-always-shown.md))
- **Date:** 2026-10-23

## Context

Every action is the leader (`Ctrl+Space`) then one key, and the leader is spent after that
key (ADR 0009), so three actions are six presses. The user asked for two modes instead: one
where keys type into the pane, one where each key is a shortcut, with the current mode shown.
ADR 0014 bars bare letters from acting in the sidebar because stray letters closed sessions
and opened Changes; that was letters acting with no mode in sight.

## Decision

The leader enters **shortcut mode**, which stays: each key there runs its leader action, and
the mode holds for the next. Esc, the leader again, or a click into a pane leave it for
**input mode**, where keys go to the pane as before. An action that gives you something to
type into (a new shell, session, tab, split, worktree or pane) leaves too. One that opens a
popup or sheet hands the keys to it, and closing it returns to input mode. A key with no
action does nothing and the mode stays. The key map shows on a pause after the leader only
until the first action.

Shortcut mode is shown while it lasts: a `SHORTCUT` pill at the top right of row 0 (in place
of the leader's keycap) and the focused pane's border in sky, in every look. Input mode shows
nothing. Keys outside shortcut mode are unchanged, so ADR 0014 stands: no letter acts in the
sidebar or on a row without the leader.

## Alternatives considered

- **Esc enters shortcut mode (vim):** agents use Esc (Claude interrupts with it).
- **Tap once for one key, twice to lock:** two behaviours behind one key to learn; the user
  chose one.
- **Always show the mode:** a label on screen while typing, which is nearly always.

## Consequences

Repeated actions cost one key each. The risk ADR 0014 names returns inside the mode: a letter
pressed while you think you are typing runs an action, so the mode must stay hard to miss and
closing keeps its confirm. Amends ADR 0009 (the leader is no longer spent after one key).
