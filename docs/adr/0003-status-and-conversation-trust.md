# 0003. Status comes only from the owning runtime, and one writer decides it

- **Status:** Accepted. The points under *To confirm* are open.
- **Date:** 2026-11-06

## Context

Status (working, needs you, done, idle) is the one thing Maitty must get right. It had several
writers (hooks, a screen scan, user actions, check results, chat state) and several readers that
derived their own view (the TUI, the GUI poll). Every fix added a time-window guard to one copy,
and the same bugs came back.

## Decision

1. **A status is accepted only from the runtime that owns the pane or conversation.** For a pane,
   a hook report verified by the pane's process tree or `MAITTY_PANE_TOKEN`. For a structured
   conversation, an event on the daemon-owned connection at the current generation. An
   unverified or superseded source cannot change status.
2. **The daemon alone decides status and alerts.** Clients display what it sends. A client never
   derives "done", "needs you" or "ring" from its own diffs.
3. **The screen scan is a labelled fallback.** It may set status only for an agent with no
   trusted hook source, and never overrides a hook-reported status.
4. **History is display, not status.** Reading a provider's transcript never changes status, and
   a file the daemon did not verify belongs to the pane's session is not read.
5. **Nothing is guessed.** No "most recent file", no unverified report.
6. **An agent session is one PTY with a chat view.** The agent runs as the user would run it, so
   its native terminal is always there. The chat view follows the provider's own transcript. The
   composer types into the PTY only while the trusted status says the agent waits for a prompt.

## Alternatives considered

- **Treat the screen as equal to hooks**: the two disagree in time, and each reconciliation guard
  became another place to regress.
- **Drive every session through a provider SDK**: those runtimes are not PTYs, and the native
  terminal must stay. They remain valid for structured conversations the daemon starts itself (0005).

## Consequences

- Code with a second writer or a client-side derivation breaks this ADR and is fixed, not guarded.
- Tests assert one decision per turn: one Done, at most one ring.

## Alerts

The daemon decides that an alert is due, once per finish or request. It tells exactly one
window, the one a person last typed or acted in (else the newest that can hear it), and that
window alerts: the terminal app rings, the desktop app shows its toast or system notice and
plays its tone. With no window attached, the daemon rings itself. Other windows are not told,
so several open windows never each alert for one event. A window never derives an alert for a
terminal session from its own status diffs; the desktop app diffs statuses only for chat
sessions, which the daemon does not announce.

## To confirm

1. What owns "seen": a per-client report to the daemon (a turn counter against a shared
   seen-turn), or a daemon-wide focus.
