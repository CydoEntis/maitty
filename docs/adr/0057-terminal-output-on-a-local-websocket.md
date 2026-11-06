# 0057. Terminal output reaches the window on a local WebSocket

- **Status:** Proposed (the owner asked for this fix on 2026-11-05; they have not reviewed the design)
- **Date:** 2026-11-05
- **Relates to:** AGENTS.md non-negotiables 1 and 6; [PERFORMANCE](../PERFORMANCE.md) (*Input latency*);
  [SECURITY](../SECURITY.md) (*Project-specific: terminal output socket*)

## Context

The desktop host sent each pane's output to the window through a Tauri channel. In the real window
on Linux (WebKitGTK) a frame the host had sent did not run in the page until the window next got an
event, so every pane showed what was typed before the last key (ticket 35). The cause was measured
as lying between the host's `channel.send` and the page's `onmessage`, and not in the daemon, the
hub or the host's batching. Pressing a key is an event, which is why the pane ran exactly one step
behind.

## Decision

1. The host listens on `127.0.0.1` on a port the system picks and serves terminal output as binary
   WebSocket messages. The frames and their header are exactly the channel's. A WebSocket is read by
   the web view's own network stack, which waits for neither the host's main loop nor an event.
2. The window asks the host for a **one-time token** with a command, connects, sends the token as its
   first message, and trusts the socket only when the host answers `ready`. Until then the channel
   carries the frames, so a socket that cannot start costs speed and nothing else.
3. Only the app's own origin may connect (`tauri://localhost` and the `tauri.localhost` schemes, and
   the dev server in a debug build). A page in the user's browser has another origin and is refused
   even if it guesses the port.
4. The window de-duplicates by sequence number, resynchronises every open pane from the host's
   history when the socket becomes ready or closes (frames sent on the channel just before the switch
   can still be in flight), and does the same when it sees a gap in a pane's numbers.
5. The CSP's `connect-src` gains `ws://127.0.0.1:*`.

## Alternatives considered

- **Keep the channel and find why the web view holds its messages.** Not found by reading the host's
  event loop; the fix would depend on a WebKitGTK detail we do not control.
- **Move to a custom protocol (`fetch`) with long polling.** More round trips and more host work for
  the same answer.
- **Hand-written WebSocket framing.** Rejected: the handshake needs SHA-1 and a correct frame
  parser; a mistake there is a security bug. `tungstenite` is the standard crate.

## Consequences

- Two new dependencies: `tungstenite` (WebSocket protocol, no TLS, no async runtime) and `getrandom`
  (already in the tree through other crates) for the token.
- A new listener on the loopback address. It is reachable by any process of the same user and by no
  one else; the token and the origin check are what a local process or a web page must get past.
- A pane's output can be duplicated across the two paths for a moment; the sequence numbers make that
  harmless.
- **Not verified in a real window yet.** The host is tested for the handshake, the token, the origin
  and a window that stops reading; the window's side is tested against a stand-in socket. Whether
  WebKitGTK and WebView2 deliver the socket's messages promptly is what the next real run shows
  (PERFORMANCE: *How to run measurement 2 again*).
