# 0045. Each session belongs to one app: the terminal app or the desktop app

- **Status:** Accepted
- **Date:** 2026-11-02
- **Supersedes:** [0044](0044-chat-and-terminal-are-separate-sessions.md) (its rule that both apps list both kinds)

## Context

[0044](0044-chat-and-terminal-are-separate-sessions.md) lets the terminal app and the desktop app
list the same terminal sessions, and both can type into and resize them. Two clients with
different window sizes share one pane, one PTY and one screen. Whichever client resized last sets
the size, so the other shows the agent's output scrambled, and the agent is signalled to redraw at
each flip. Keeping the sizes in step cannot be done, so the sharing itself is the cause.

## Decision

- Every session belongs to exactly one app, recorded by the daemon when the session starts:
  - **Terminal app sessions** (terminal panes) are listed, typed into and resized only by the
    terminal app.
  - **Desktop app sessions** (chats and any terminal session the desktop app starts) are listed,
    typed into and resized only by the desktop app.
- The daemon still owns every session, so a session keeps running when either app closes. The
  daemon enforces the ownership: a client that does not own a session cannot list, type into, or
  resize it, and gets a clear error.
- Sessions that exist when this lands keep the app that started them, where the daemon can tell.
  Sessions saved before this version are assigned to the terminal app (see Consequences).
- The two apps are two ways into one daemon, not two products. They share the protocol, settings
  files and the daemon, and nothing else.

## Alternatives considered

- **Keep sharing and sync sizes:** impossible with one PTY and one screen, as described above.
- **The desktop app shows terminal sessions read-only:** avoids typing conflicts but still shows
  a scrambled picture to the desktop app at a different size. It keeps the complexity without
  the benefit.
- **Two daemons:** would stop the sessions in one app from surviving the other closing, and
  duplicates the owner of every session. Not needed: ownership inside one daemon fixes the bug.

## Consequences

- A session started in one app is not visible in the other. This is the trade-off: the single
  shared list from 0044 is gone.
- The desktop app keeps chat and terminal sessions, both started there.
- The protocol needs an owner field on sessions, and the daemon checks it on every session
  command. A change to a message shape bumps `PROTOCOL_VERSION`.
- Sessions saved before this version carry no record of which app started them, so they are
  assigned to the terminal app when the daemon restores them. Sessions started in the desktop app
  after this version keep their owner across restarts.

## Decided: the desktop app keeps both kinds

The desktop app keeps chat sessions and terminal sessions, and the kind is still chosen when a
session starts, as in 0044. Its terminal view stays. What changes is ownership: a terminal session
started in the desktop app is never resized or typed into by the terminal app, and the reverse.
Removing the terminal from the desktop app was rejected because the ownership rule already
removes the conflict.
