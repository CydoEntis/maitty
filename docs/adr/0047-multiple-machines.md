# 0047. One terminal app, several machines

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

Maitty can already attach to one remote daemon: `maitty --remote host` runs
`ssh host maitty proxy` and the same msgpack frames travel over ssh's stdio
(ADR 0003). That is one remote per window, chosen at start, held in a process-wide
variable that about 25 call sites read. A developer who runs agents on a laptop, a build
box and a GPU machine cannot see them together, and a dropped link ends the window.

"Named hosts and remote panes/agents" (#23) was canceled on 2026-10-17 as too large for the
design then in place. It is reopened because the cost of the one-remote design now shows
(the global, the blocking error handling, no reconnect), and the groundwork is the same work
each of those fixes needs. The daemon already owns every session (ADR 0001), so a remote
daemon needs no change to be a source of sessions; only the client has to hold more than one.

## Decision

1. **One connection per machine, held by the client.** There is no daemon-to-daemon hub.
   Each machine's own daemon stays the only owner of its sessions.
2. **A new session targets the focused session's machine**, local when nothing is focused.
3. **The local daemon stays mandatory.** The window always has a local machine.
4. **A dropped machine's agents stay in the sidebar**, greyed, with "disconnected since" and
   the reason. They come back when the link does. Reconnecting only attaches; it never starts
   a remote daemon the user did not start.
5. **`[[machines]]` is the primary setting.** `--remote host` remains, as an ad-hoc machine
   named after the host.
6. **A Windows client reaching Linux or macOS remotes is in v1.** A Windows machine as the
   remote is out.
7. **Files, Changes, Find, Branches, References and image paste are off for a remote pane
   in v1.** They read the local disk; each says so, naming the machine.
8. **Installing on the remote streams a verified binary over ssh's stdin.** Same OS and CPU:
   the local binary. Otherwise the matching release is fetched on the local machine and
   streamed, so the remote needs no GitHub access. It is never done silently.
9. **The machine list is local-only and never synced.** A `machines` entry in the synced
   config is refused.
10. **A daemon id (additive, random, persisted) in the snapshot** lets the client refuse an
    alias that now answers as a different machine, so a swapped host never reuses a machine's
    saved layout.
11. **The CLI gets `--on <machine>` first**; MCP follows.
12. **A remote daemon is untrusted.** Its clipboard writes, notices, raises and sounds are off
    unless that machine opts in, and every string it supplies is escaped before display.
13. **The terminal app goes first.** The desktop and chat apps follow later; the connection
    registry stays in the terminal client until a second consumer needs it.

Out of v1: a remote file API, remote image paste, a Windows machine as the remote, the
desktop app, and a hub.

The wire protocol does not change for the client work. The client gains a `MachineId` (local
is 0) and keys its per-pane state by `(MachineId, TermId)`.

## Alternatives considered

- **A hub daemon that links the others:** a second owner of sessions, and a new place for the
  wrong status to appear (non-negotiable 5). The client already sees every daemon.
- **Keep one remote per window:** the reported problem stays.
- **A custom network protocol instead of ssh:** a listener to secure, and keys to manage,
  where ssh already carries them.
- **Make the remote the only machine (no local daemon):** the window would die with the link.

## Consequences

A machine id appears in client state, so about 200 client call sites change in one reviewable
step. Failures are classed per connection, so a dead machine shows a state instead of ending
the window. The remote keeps running throughout, since the daemon owns the sessions. Windows
clients need OpenSSH and keys or an agent, since password prompts are answered in a popup only
the first time. Revisit if a second client (the desktop app) wants machines, or if a remote
file API is wanted.

## Update 2026-11-03: what shipped and what we learned

The decisions above stand. This section records what was built and where building changed or
sharpened them. This work adds nothing to the wire protocol (version 69): `Snapshot.daemon_id` is an optional
field with a default.

### What shipped

The Machines dialog (leader `w`) adds, retries and removes machines; `[[machines]]` in
`config.local.toml` is the setting. `--on <machine>` and `machine/id` pane addresses reach a
machine from the one-shot commands, and `ls --all` asks every machine at once. `maitty machine
update <name>` installs or updates the app on a machine. `maitty doctor` has a section per
machine. The MCP tools take a `machine` argument. A machine with `via` is reached through a
program of the user's own instead of ssh. The docs that carry the rules are
[API_PATTERNS](../API_PATTERNS.md) (connections and routing), [SECURITY](../SECURITY.md) (the
remote daemon, ssh lines, installing, MCP), [ENVIRONMENT](../ENVIRONMENT.md) (the setting and the
flags), [UI](../UI.md) and [guide](../guide.md).

### Decisions refined while building

- **A configured machine is attached to, never started (refines 4).** Its connection runs `proxy
  --attach-only`; with no daemon there the proxy exits 74 before writing anything and the
  machine shows `unreachable` with the fix. This holds for the first connection and every redial.
  An ad-hoc `--remote host` still starts the daemon, as it always did.
- **One bounded reader channel per machine (512 messages), not the shared channel.** A machine
  that floods or stalls cannot starve another's, and a lost line is marked lost without
  ending the window.
- **Failures are classified from structured signals.** The error's type, ssh's exit status and
  whether the far side ever sent its stream marker or a frame decide the class. ssh's words are
  read for exactly three tokens (`Permission denied`, `Host key verification failed`, `Too many
  authentication failures`), only on exit 255 with nothing received. Each class has one fix
  text, a constant shared by the screen, the tests and the docs.
- **The stream marker sits outside the msgpack frames (no protocol bump).** `proxy` writes it
  once, after it reaches the daemon; the client discards login noise up to it (64 KiB, 30 s).
  A remote from before the marker is still read when its first four bytes are a plausible
  frame length; an old remote whose login also prints text gets the "update it" error.
- **Two watchdogs.** ssh's `ServerAlive*` ends a silent line after 45 s. The client's own
  stall detector drops a line whose frame began and stopped growing for two 15 s windows. Known
  gap: a line that stops right after a frame's four length bytes has nothing buffered, so only
  ssh's check ends it.
- **The daemon id is stricter than first written (refines 10).** A daemon with no id that
  answers at a name with a recorded id is refused too, so an old daemon cannot stand in for a
  known one. A daemon with no id at a name with no record is let in.
- **Reply bookkeeping is a 30 s allowance.** Without a wire change the client cannot tell which
  request a `Reply` answers, so it keeps a timestamp per request sent to a machine (at most
  64) and applies a reply or error only while one is waiting. Unsolicited ones are ignored.
- **A machine that is down is left out of counts and the Inbox.** Its rows stay, grey, with
  "disconnected since" and the fix; its needs-you and failed marks do not count until it is back.
- **MCP needs `mcp = true` on the machine and the caller's local grants (refines 11).** The
  machine never learns which pane called, so this computer decides before it dials. It does so
  by probing the local daemon with a read and a write to a pane that does not exist, which is a
  shortcut: it works only while the daemon's grant check answers the same for a missing pane
  (a daemon test pins it). Answering, interrupting and starting are refused for a remote.
- **Worktrees are refused for any machine but this one**, while an ad-hoc `--remote` window keeps
  its own. Every local-only feature goes through one gate, `guard_local`.
- **`restart-if-idle` means idle now.** After `machine update`, the daemon is restarted only if
  no session is open at that moment; otherwise it is left running on the old file.
- **`MachineId`s are assigned by lowercased name in name order and never persisted**, so
  adding a machine renumbers the ones after it harmlessly; saved layout is keyed by name.
- **`via` is refused on Windows and in the synced config**, because it names a program this
  computer runs. `ssh_args` beside `via` is refused.

### Known limits (v1)

- A Windows client needs OpenSSH, and keys or an agent: a password is asked in a popup and
  not remembered.
- A Windows machine as the remote is unsupported.
- Files, Changes, Find, Branches, References and image paste are off for a remote pane.
- Nothing has been run against a live terminal and a real ssh host; every test goes through
  the fake-ssh shim.
- The tests that need a unix host (custom transports, the login-PATH shell probe, the install
  mode bits) are compiled but were not run on the Windows development machine.
- The desktop app has no machines.

### Follow-ups

Left out of v1 on purpose:

- MAITTY-139: a remote file API.
- MAITTY-140: image paste into a remote pane.
- MAITTY-141: a Windows machine as the remote.
- MAITTY-142: machines in the desktop app.
- MAITTY-143: spike: a hub.

Found while building:

- MAITTY-144: a read-only query for a pane's own grants, to replace the MCP grant probe.
- MAITTY-145: core conversation tests fail about 1 run in 4 when run in parallel.
- MAITTY-146: run the unix-only machine tests on Linux and macOS CI.
- MAITTY-147: a live check with a real ssh host and the Machines dialog.
- MAITTY-148: restart a remote daemon when it goes idle after an update, and show the protocol
  in `--version`.
- MAITTY-149: flag machines that are down in the sidebar title, and retry from a machine header.
- MAITTY-150: worktrees on configured machines.
- MAITTY-151: re-read `[[machines]]` on config reload for running supervisors.
- MAITTY-152: conversation ids and reply matching across machines.
- MAITTY-153: `wait` reconnects every 400 ms; `teach` prints the local config path.
- MAITTY-154: bound what a hostile remote can make the client allocate.
