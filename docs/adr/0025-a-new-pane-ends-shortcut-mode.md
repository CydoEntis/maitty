# 0025. Starting a pane ends shortcut mode

- **Status:** Accepted
- **Date:** 2026-10-25

## Context

ADR 0021 made shortcut mode end only on Esc, the leader again, Esc in the key map or a
click, so a new shell (`n`) or session (`s`) left you in shortcut mode. The user tried
v0.43.4 on 2026-10-25: after `n` or `s` you are in a new session and mean to type there, but
the first letters ran actions instead. ADR 0016 had called these "typing actions" that leave
the mode; ADR 0021 took that out.

## Decision

An action that starts a pane for you to type into ends shortcut mode: the mode goes back to
input, nothing holds it to return, and the new pane, focused by the daemon, gets the next
key. These are every path that starts a pane (`screen_new_session`, `screen_new_worktree`):

- `n` a shell where you are, `s` a session, `t` a new tab, `v` / `h` (and the other splits
  and `spawn-*` bindings) a pane beside this one;
- `o` once its dialog starts the session (Esc in the dialog still returns to shortcut mode);
- the + New chooser, a recipe, opening a project, a file opened in a terminal editor, and
  the shell in your home folder that replaces the last session you close.

Actions that open a popup or sheet still hand it the keys and return to shortcut mode when
it closes. `Enter` (message the agent) is one of those: the message popover has the keys,
and sending it leaves you where you were, not in a new pane. Everything else keeps the mode.

Amends ADR 0021 (its list of ways out) and restores ADR 0016's typing actions for new panes.

## Alternatives considered

- **Only `n` and `s`:** a split or a tab is just as much a new shell to type into; one rule
  for every new pane is easier to learn than a list.
- **Keep the mode, show it louder:** the bar already says SHORTCUT; the user still typed
  into it, because the new pane is where attention goes.

## Consequences

Several new panes in a row cost the leader once each again. A key pressed by mistake after
`n` goes to the new shell, not to an action, which is the safer of the two.
