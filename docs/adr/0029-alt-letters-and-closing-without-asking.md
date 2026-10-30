# 0029. Alt+letter shortcuts, and every close asks first

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
  with Alt (`keys::ALT_LETTER_KEYS`): Alt+n new shell, Alt+a new session, Alt+x close the
  pane, Alt+s split side by side, Alt+d split down. None is a key an agent needs: Claude Code takes Alt+v (paste an
  image, on Windows) and Alt+t (thinking), so the splits are not the leader's v and h, a new
  session is Alt+a (an agent), and a new tab has no Alt key. Alt+d is readline's delete-word; the user chose it anyway.
  `ui.alt_keys` (on by default, Settings › General ›
  Input) turns them off for shells that use those keys. Each is rebindable under `[keys.global]`,
  and Settings › Keys lists them under "Without the leader".
  No Alt+[ or Alt+]: terminals send those as the start of an escape sequence.
- **Every close asks first:** a pane, a tab (its ✕, a middle click, the leader's close) and an
  idle shell alike, in the one confirm (`menu::Confirm`, danger). Closing an idle shell at once
  was tried first and dropped on 2026-10-27 at the user's word: a close you didn't mean, from a
  key now one press away, is worse than the extra Enter.
- The leader and shortcut mode stay as they are, for everything.

## Alternatives considered

- **Drop the leader for bare keys:** every letter would be taken from the agent you type to.
- **Ctrl+Shift shortcuts:** many terminals can't tell Ctrl+Shift+letter from Ctrl+letter, and
  Windows Terminal keeps Ctrl+Shift+D (duplicate tab) for itself.
- **Ctrl+D to split:** it is end-of-input, how a shell exits and Claude Code quits.
- **Never confirm a close:** an agent's work in progress is one keypress from gone.
- **Confirm only when something would be lost** (an idle shell closes at once): shipped in
  0.47.0, then dropped; a pane closed by a stray Alt+x surprised the user.

## Consequences

Six Alt keys are taken from programs while `ui.alt_keys` is on. All keys lists them under "without the leader".
