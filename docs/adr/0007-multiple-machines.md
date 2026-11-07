# 0007. Multiple machines: one connection per machine, held by the client

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

People run agents on more than one computer and want one window over all of them.

## Decision

- **One supervised connection per machine, held by the client.** There is no daemon-to-daemon hub;
  each machine's daemon stays the only owner of its sessions (0001). The local daemon is mandatory.
- A new session goes to the machine chosen in the New session dialog; the dialog starts on the
  focused session's machine, and on this computer when nothing is focused.
- A dropped machine's agents stay in the sidebar, greyed, with the reason. Reconnecting only
  attaches; it never starts a daemon the user stopped.
- All reach goes through the shared `crates/maitty-machines` crate: the machine list, one hardened
  ssh line (no prompts, no agent forwarding, the host after `--`, fixed remote scripts, quoted
  arguments), failure classes, and the connection supervisor. It never depends on `maitty-core`,
  `maitty-settings` or `maitty-terminal-client`.
- Images and files for another machine use that same ssh mechanism, not the daemon. The file
  name is picked locally and checked twice; images are capped at 25 MB, files at 100 MB; the far
  side lands them in a private cache folder (mode 700, files 600).
- **A remote daemon is untrusted.** Its clipboard writes, notices and sounds are off unless that
  machine opts in. Every string it supplies is cleaned before display. Only the line after a
  marker, and only an absolute path, is taken from a script's output.
- The machine list is local-only and never synced. A daemon id in the snapshot lets the client
  refuse an alias that now answers as a different machine.
- A Windows client may reach Linux and macOS remotes. A Windows remote is out.

## Alternatives considered

- **A hub daemon**: rejected, a second owner of sessions breaks 0001.

## Consequences

- Features that read the local disk are off for a remote pane and say which machine.
