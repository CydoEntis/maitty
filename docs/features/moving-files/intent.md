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

- The remote file API for the Files, Changes and Find views (Plane MAITTY-139). Browsing and
  searching inside the Get a file dialog is in scope and does not need it: it reads folders with
  fixed scripts over ssh.
- Syncing, and resuming a cut-off transfer.
- A Windows machine as the remote.
- A key or screen for it in the terminal app (the command does the job).

## Phases

1b. Browse and search in the Get a file dialog (added 2026-11-07 at the user's request: typing a
    whole path is a rough edge). Click through a machine's folders, and search by typing a few
    letters of a name inside the session's folder and everything below it.

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

Phase 1b, in the same module:

7. MAITTY-233 the fixed script and ssh line that list one folder
8. MAITTY-234 the fixed script and ssh line that index files below a folder
9. MAITTY-235 list a folder and index its files, cleaned and capped (blocked by 7 and 8)
10. MAITTY-236 host commands to list a folder and index files (blocked by 9)
11. MAITTY-237 a fuzzy matcher for file paths
12. MAITTY-238 browse and search in the Get a file dialog (blocked by 10 and 11)

Phase 2, folders both ways, in the same module (no new library: the system `tar` is used and its
listing is checked before anything is unpacked, as the release installer already does):

13. MAITTY-239 the fixed script and ssh line that stream a folder as a tar
14. MAITTY-240 receive a folder tar, check its listing, unpack it safely (blocked by 13)
15. MAITTY-241 send a folder to a machine as a tar
16. MAITTY-242 `machine get` pulls a folder, `machine put` sends a file or folder (blocked by 14 and 15)
17. MAITTY-243 drop a folder on a remote session, and Get this folder in the dialog (blocked by 14 and 15)
