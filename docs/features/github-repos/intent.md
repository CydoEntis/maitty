# Intent: bring GitHub repositories into Maitty

Type: feature
Author: Cody · Status: accepted
Date: 2026-11-12

## Problem

"There's no way for Maitty to clone down repos. I'd have to leave Maitty to bring new projects
down and inside of it. That's probably bad UX." Starting on a project that is not on the disk yet
means cloning it by hand in a terminal, and on another machine it means an ssh session first.
The desktop app's Pull, Push, Commit and Create PR work only for a checkout on this computer;
for a session on another machine the commit bar is not there at all. And a person with more than
one GitHub account has nothing that says which account a folder uses.

## Proposed outcome

In the desktop app a person can, without leaving it:

1. clone a repository from a pasted address into a folder they pick, on this computer or on
   another machine;
2. sign in to GitHub and pick one of their repositories to clone;
3. see the changes of a session on another machine and pull, push, commit and open a pull
   request for it;
4. say which GitHub account and which commit name and email a folder uses, see the exact change
   first, and have plain git follow the same rule outside Maitty.

## Affected users and systems

People who work on several projects, on one or more machines, some with more than one GitHub
account. The daemon (it runs `git` and `gh` on the machine that holds the folder), the wire
protocol, the desktop host and the desktop window (the new-chat page, the side panel's Changes
tab, Settings).

## Constraints

- GitHub sign-in is the GitHub command line's own (`gh`). Maitty stores no password and no token
  ([ADR 0006](../../adr/0006-secrets-never-enter-chat.md); `docs/SECURITY.md`, the threat model).
  A machine without `gh` is told so; Maitty does not install it.
- The folder is on the machine the daemon runs on, so the daemon does the git work there. The
  desktop host does not link `maitty-core`
  ([ADR 0004](../../adr/0004-client-split-and-desktop-host.md)).
- A remote is untrusted: every string it sends is cleaned and bounded before it is shown
  ([ADR 0007](../../adr/0007-multiple-machines.md)).
- A new message gets a protocol version and its own `introduced_in` arm
  ([ADR 0002](../../adr/0002-terminal-substrate-and-wire-protocol.md)).
- Only a person's client may ask for any of this; an agent's connection is refused.
- Nothing is cloned over an existing folder, and no pull merges: a branch that cannot be
  fast-forwarded says so, as the Changes tab does today.
- Accounts per folder are ordinary git configuration (`includeIf`), written only after the person
  has seen the exact lines.

## Out of scope

- The terminal app: it has a shell for `git clone`.
- SSH keys per folder.
- GitLab, Bitbucket and other hosts in the repository picker (a pasted address from any host
  still clones).
- Installing `git` or `gh`.
- Files, Find and Branches for a session on another machine (Plane MAITTY-139).
- Merging, rebasing and resolving conflicts.

## Open questions

## Tickets

Plane module "GitHub repos", in order: clone from an address MAITTY-323, 324, 325; sign in and pick a
repository MAITTY-326 to 330; changes, pull, push, commit and pull requests on other machines
MAITTY-331 to 336; accounts per folder MAITTY-337 to 340.
