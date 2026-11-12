# 0007. Multiple machines: one connection per machine, held by the client

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

People run agents on more than one computer and want one window over all of them.

## Decision

- **One supervised connection per machine, held by the client.** There is no daemon-to-daemon hub;
  each machine's daemon stays the only owner of its sessions (0001). The local daemon is mandatory.
- A new session goes to the machine chosen on the new chat page; the page starts on the
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
- **A machine's files can be pulled to this computer**, up to 1 GB, over the same ssh line and a
  fixed script. The bytes come from an untrusted machine, so the local name is made here from the
  path (never taken from the machine), nothing is overwritten, the file appears only when exactly
  the announced bytes arrived, and a cancel, a failure or the cap leaves nothing behind
  ([spec](../features/moving-files/spec.md)). A folder travels as a tar archive made with the system
  `tar` (no library). One pulled here is unpacked only after its listing is checked: plain files and
  folders only, nothing outside its one top folder, no name this computer cannot store safely. One
  sent from here is unpacked in the machine's private cache folder.
- **A remote daemon is untrusted.** Its clipboard writes, notices and sounds are off unless that
  machine opts in. Every string it supplies is cleaned before display. Only the line after a
  marker, and only an absolute path, is taken from a script's output.
- **Updating a machine replaces its daemon.** `maitty machine update <name>` installs the new file
  and restarts the far daemon so the two ends match. Without `--restart` it restarts only an idle
  daemon; with it (the window's Update button always passes it, after asking) it restarts a daemon
  whose sessions are saved. They come back on the new daemon with their agents resumed; a turn in
  progress is lost. A daemon whose sessions are not saved is left running and the update says so.
- The machine list is local-only and never synced. A daemon id in the snapshot lets the client
  refuse an alias that now answers as a different machine.
- **Remote chat.** A pane on another machine has a chat view and the new chat page, like one on
  this computer. The window asks that machine's daemon for its conversation with the messages it
  already speaks, on request connections of its own beside the machine's long one (same ssh line,
  attach only). Nothing changes on the remote side and `PROTOCOL_VERSION` does not move. What the
  remote supplies is data: cleaned before display, links confirmed, nothing fetched on its own, and
  the remote daemon alone decides its status (0003). Anything that reads a disk (Changes, Files,
  Find, opening a path) stays local-only. See [the spec](../features/remote-chat/spec.md).
- A Windows client may reach Linux and macOS remotes. A Windows remote is out.

## Alternatives considered

- **A hub daemon**: rejected, a second owner of sessions breaks 0001.
- **Remote chat on the long connection**: rejected, conversation replies carry no tag, so it would
  need a message change and a daemon update on every machine.
- **Reading a remote transcript over ssh**: rejected, the daemon is the only verified reader (0003).

## Consequences

- Features that read the local disk are off for a remote pane and say which machine. Chat is not
  one of them.
- Each machine can hold a few idle request connections besides its long one.
