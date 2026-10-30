# xterm.js

> **Use for:** Terminal emulation inside Maitty's desktop GUI.
> **Requires:** React, TypeScript
> **Fills:** UI Â§ terminal interaction

## Rules

- Use `@xterm/xterm` for terminal rendering and input encoding. Use the fit addon
  when the containing view changes size.
- Treat the terminal as a client of the daemon-owned pane. It never launches or
  owns a process.
- Render only daemon-provided terminal state and send terminal input through the
  conversation-bound desktop command. Do not expose a shell command API.
- Resize only through a command bound to the conversation ID and generation;
  validate dimensions at the daemon boundary.
- Keep xterm imports and terminal lifecycle in the terminal feature component.
  Dispose observers, subscriptions and the emulator when that view unmounts.

## Project-specific: xterm

Use the upstream `@xterm/xterm` package and `@xterm/addon-fit`. The daemon
returns raw replay bytes when the terminal opens, then formatted ANSI screen
snapshots. The emulator interprets control sequences and encodes keyboard, paste
and terminal responses for the daemon.
