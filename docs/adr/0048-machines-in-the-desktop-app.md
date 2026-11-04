# 0048. Machines in the desktop app, through one shared crate

- **Status:** Proposed
- **Date:** 2026-11-03
- **Amends:** [0047](0047-multiple-machines.md) (decision 13: the connection code leaves the
  terminal client now that a second consumer exists)
- **Relates to:** [0042](0042-decouple-desktop-clients-and-providers.md),
  [0045](0045-each-session-belongs-to-one-app.md)

## Context

The terminal app reaches several machines at once (ADR 0047). A machine is a daemon reached
over ssh, and the client holds one connection per machine. The code for that lives in the
terminal client and in core: how the ssh line is built and hardened, how a failed link is
classified and which fix text it gets, the reconnect state machine, the cleaning of every
string a remote sends, the daemon-id check, and the `[[machines]]` list in `config.local.toml`.

The desktop app has no machines (0047, known limits). It talks to the local daemon only. Its
Rust host links `maitty-desktop-client`, `maitty-shared` and `maitty-ipc-transport` and no other
crate of ours; ADR 0042 forbids it from linking the runtime crate. The web view has no network
or process access and reaches the daemon only through the host's fixed commands.

Two facts shape the design. ADR 0045 makes every session belong to one app, so a desktop
connection to a machine sees only the sessions the desktop app started there. The ssh
hardening and the failure classes are security rules; a second copy would drift, and a
mistake in a copy is a security bug.

The owner has answered the open questions of the planning document (Q1 to Q3). This record
holds those answers.

## Decision

1. **One shared crate, `crates/maitty-machines`.** It holds the machine list and its checks,
   the ssh line and its hardening, the failure classes and their fix texts, the connection
   state machine and supervisor, the cleaning of remote strings, the remote opt-ins and the
   daemon-id store. Both apps use it. It depends on `maitty-shared`, `maitty-ipc-transport` and
   small utility crates only. It never depends on `maitty-core`, `maitty-settings` or
   `maitty-terminal-client`; those three point at it.
2. **The desktop host's dependency rule.** The Tauri host may link `maitty-desktop-client`,
   `maitty-machines`, `maitty-shared` and `maitty-ipc-transport`. It must not link
   `maitty-core`, `maitty-settings` or `maitty-terminal-client`, so ADR 0042 stays true. A
   `cargo tree` check in CI fails when one of the three appears in the host's tree.
3. **Connections live in the Rust host.** The host holds one supervised connection per machine,
   cleans and vets everything that arrives, and sends each command on the right machine's
   line. The web view renders and never connects. It knows machines by name only.
4. **Only desktop-started sessions are shown (Q1: no, for now).** On a machine the desktop app
   lists, types into and resizes only the sessions it started there, as ADR 0045 requires.
   Sessions the terminal app started on that machine are not shown. A read-only "observer"
   view of them is a separate, later decision; it needs a protocol change, and every machine
   would have to update.
5. **The machine list is shared (Q2: yes).** The desktop app reads the same `[[machines]]`
   entries from the local-only `config.local.toml` that the terminal app writes. There is no
   second list. The 0047 rule holds: the machine list is never synced.
6. **The first release is read-mostly (Q3: yes).** It shows machines with their status, opens
   sessions, types into them, starts sessions on a machine, and lists their agents in the
   Inbox. Files, Changes, Find, Branches, References, image paste and worktrees are disabled
   on a remote pane, and each says so, naming the machine. The desktop app does not install,
   update or restart maitty on a machine; its machine chip shows the exact
   `maitty machine update <name> --install` command to run instead.
7. **Output stays on one channel.** The host keeps one raw channel to the window and the
   existing coalescing. The frame header gains a machine id. That header is the host-to-web-view
   format only, so `PROTOCOL_VERSION` does not change.
8. **A remote daemon stays untrusted**, exactly as 0047 decision 12: every string it supplies is
   cleaned before display, and clipboard writes, notices, raises and sounds are off unless
   that machine opts in.

Out of scope: a remote file API (MAITTY-139), image paste into a remote pane (MAITTY-140), a
Windows machine as the remote (MAITTY-141), a hub (MAITTY-143), installing or updating a
remote from the desktop app, observing terminal-app sessions, and any change to the wire
protocol.

## Alternatives considered

- **Option B: re-implement inside `maitty-desktop-client`.** Copy the ssh line, the classifier
  and the state machine into the desktop client. Faster to a first screen, but two copies of the
  hardening (`BatchMode`, `ForwardAgent` off, the `--` guard) and of the failure classes, which
  would drift. A fix in the terminal app would have to be remembered twice. Rejected.
- **Option C: the local daemon as a hub** that links the other daemons and hands the desktop
  app one merged list. Rejected in ADR 0047 decision 1: a second owner of sessions, and a new
  place for a wrong status to appear (project non-negotiable 5). The spike MAITTY-143 exists
  and is not part of this decision.
- **Shell out to the bundled CLI for the live view.** Each call is a new ssh login with no
  streaming. It may serve one-shot jobs later; it cannot carry a live view.
- **Connections in the web view.** Impossible by rule: the web view has no network or process
  access, and ssh is a process.

## Consequences

- The extraction comes first: about a week of pure refactor, with the terminal app unchanged
  and green throughout, before the desktop app shows anything new. Reviewers diff old against
  new, because the moved code is security-sensitive.
- One copy of the ssh safety rules and one state machine serve both apps. A fix lands once.
- The desktop host gains a small crate and a CI check, not the runtime. ADR 0042 holds.
- The host's per-pane state (history, last snapshot, sequence numbers) becomes keyed by
  machine. The web view's session ids gain a machine part, and the saved layout migrates
  once. These are internal formats, not the wire protocol.
- A user whose agents on a machine were started from the terminal app does not see them in
  the desktop app. They start new ones from the desktop app, or use the terminal app.
- The GUI, the local daemon and every machine must speak the same protocol version. A GUI
  update that bumps `PROTOCOL_VERSION` makes every remote read "needs update" until the CLI
  update is run, because the desktop app does not update remotes.
- We now maintain the shared crate's public surface for two consumers, and a dependency rule
  that CI enforces.
- Revisit when a remote file API exists (Files and Changes could turn on), when observing
  terminal-app sessions is wanted (a protocol change), or if a hub is ever adopted.
