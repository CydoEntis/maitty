# Intent: move files between this computer and a remote machine

Type: feature
Author: Cody · Status: accepted
Date: 2026-11-07

## Problem

Dropping a file or an image on a remote session sends it up
(`docs/guide.md`, "Dropping files on a session of another machine"). Nothing comes back
down. Getting a build output, a log or a screenshot off a remote machine means using `scp`
by hand, and a folder cannot be sent either way. This is important: working across several
machines depends on moving things between them.

## Proposed outcome

From a terminal command or the desktop app, a person can pull a file from a remote machine
to this computer, within a size limit, with progress and Cancel. Later, folders travel both
ways. Moving text needs nothing new and is documented.

## Affected users and systems

People who run agents on a machine reached over ssh. `crates/maitty-machines` (the fixed
scripts and the ssh line), the terminal app's command line, and the desktop app. Not the
daemon: files travel over ssh (ADR 0007).

## Constraints

- [ADR 0007](../../adr/0007-multiple-machines.md): a remote is untrusted, fixed scripts, ssh
  only, every string it supplies is cleaned, a Windows client may reach Linux and macOS
  remotes and a Windows remote is out.
- [ADR 0004](../../adr/0004-client-split-and-desktop-host.md): the desktop app does not link
  `maitty-core`, `maitty-terminal-client` or `maitty-settings`.
- One transfer is at most 1 GB.
- Details and safety rules: [spec.md](spec.md).

## Out of scope

- Browsing a remote's files (Plane MAITTY-139).
- Syncing, and resuming a cut-off transfer.
- A Windows machine as the remote.
- A key or screen for it in the terminal app (the command does the job).

## Phases

1. Pull a file from a remote machine (command line and desktop app).
2. Folders both ways, as a tar stream. Ticketed after phase 1 lands. Needs approval for the
   tar library (docs/SECURITY.md, supply chain).
3. A how-to for moving text between machines (filed with phase 1: it is a docs ticket).

## Open questions

None.

## Tickets

Phase 1, filed in Plane (project MAITTY, module "Moving files"), in this order:

1. MAITTY-227 the fixed script and ssh line that read one file
2. MAITTY-228 stream a remote file into a local file, checked and capped (blocked by 1)
3. MAITTY-229 `maitty machine get` (blocked by 2)
4. MAITTY-230 the desktop host command to get a file (blocked by 2)
5. MAITTY-231 a Get dialog on a remote session (blocked by 4)
6. MAITTY-232 how to move text between machines (docs, no blockers)

Phase 2 (folders both ways) is ticketed after phase 1 lands.
