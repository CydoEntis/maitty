# 0029. Alt+letter shortcuts, and closing an idle shell without asking

- **Status:** Accepted
- **Date:** 2026-10-27
- **Amends:** [ADR 0017](0017-keys-other-apps-use.md) (the leader is the way to every action)

## Context

The user found everyday actions, closing panes and sessions above all, took too many steps:
the leader, the key, then a confirm. They liked shortcut mode itself (a mode, as in Vim, so a
plain key always reaches the agent) and asked for fewer steps, not a new model. A bare key
can't act in a terminal multiplexer without taking it from the program in the pane, so a
direct shortcut needs a modifier.

## Decision

- **Alt+letter:** the everyday actions run without the leader, on the leader's own letters held
  with Alt (`keys::ALT_LETTER_KEYS`): Alt+n new shell, Alt+s new session, Alt+t new tab, Alt+x
  close the pane, Alt+v and Alt+h split. `ui.alt_keys` (on by default, Settings › General ›
  Input) turns them off for shells that use those keys. Each is rebindable under `[keys.global]`.
  No Alt+[ or Alt+]: terminals send those as the start of an escape sequence.
- **Closing asks only when something would be lost:** a pane with no agent whose running
  program is its shell (`config::is_shell`) closes at once; a pane with an agent, or a shell
  running a program, asks as before. A tab's ✕ already closed shells at once and an agent's on a
  second click.
- The leader and shortcut mode stay as they are, for everything.

## Alternatives considered

- **Drop the leader for bare keys:** every letter would be taken from the agent you type to.
- **Ctrl+Shift shortcuts:** many terminals can't tell Ctrl+Shift+letter from Ctrl+letter.
- **Never confirm a close:** an agent's work in progress is one keypress from gone.

## Consequences

Six Alt+letters are taken from programs while `ui.alt_keys` is on; readline's Alt+t
(transpose words) is among them. All keys lists them under "without the leader".
