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

Use the upstream `@xterm/xterm` package, `@xterm/addon-fit`, `@xterm/addon-search` (added
for Find: it searches the emulator's own scrollback, so the window needs no copy of the
output; it is the official addon and its 0.16 line pairs with xterm 6) and
`@xterm/addon-webgl` (0.19 pairs with xterm 6). Load the WebGL addon after `open()` when
`getContext('webgl2')` answers, and keep the DOM renderer as the fallback when it does not, when the
addon throws, or on `onContextLoss`; the
fallback draws on the theme's opaque terminal colour. Writes land at most every 32 ms on either
renderer, since each one redraws the pane (`TerminalView.tsx`, [PERFORMANCE](../PERFORMANCE.md)). The daemon
returns raw replay bytes when the terminal opens, then formatted ANSI screen
snapshots. The emulator interprets control sequences and encodes keyboard, paste
and terminal responses for the daemon. The WebGL canvas is see-through, so the
view answers a program's OSC 10 and 11 colour queries (`?`) with the theme's opaque colours
itself (`lib/osc-color.ts`); xterm would report the transparent black and a program would paint
with it. Setting a colour still goes to xterm.
