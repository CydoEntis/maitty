# Intent: chat view and the new chat page on a remote machine

Type: feature
Author: Cody · Status: accepted · Date: 2026-11-12
Plan: Plane module "Remote chat" (MAITTY-261 to 264)

## Problem

A session on another machine is a terminal only. Its pane cannot switch to the chat view
(`chatAvailability` answers `remote`), and the sidebar's New chat on a machine opens a small
New session dialog that starts a shell and types the agent's name into it, with no model,
reasoning or permission choice. The same button gives two different experiences depending on
where the session goes, and the remote one is the weaker one.

## Proposed outcome

A session on another machine can switch between the terminal and the chat view, and New chat on
a machine opens the same new chat page as on this computer. The remote New session dialog goes
away.

## Affected users and systems

People who run agents on a machine reached over ssh, on the desktop app. The window's host
(`apps/gui/src-tauri`, `crates/maitty-desktop-client`), `crates/maitty-machines`, and the
desktop front end. The remote daemon is unchanged: it already holds the conversation and
answers the messages (see [spec.md](spec.md)).

## Constraints

- [ADR 0007](../../adr/0007-multiple-machines.md): a remote daemon is untrusted; one connection
  per machine held by the client; no daemon-to-daemon hub.
- [ADR 0003](../../adr/0003-status-and-conversation-trust.md): the remote daemon alone decides
  its status; this window displays it.
- [ADR 0004](../../adr/0004-client-split-and-desktop-host.md): the desktop app does not link
  `maitty-core`, `maitty-terminal-client` or `maitty-settings`.
- Windows, macOS and Linux clients (ADR 0008); a Windows remote stays out.

## Out of scope

- Anything that reads the remote machine's disk from this computer: Changes, Files, Find and
  opening a path stay local-only and say which machine. (Plane MAITTY-139 owns the remote file API.)
- Remote model lists. The new chat page offers the agent's default model on a remote machine.
- Starting a structured chat (no terminal) on a remote machine.
- A TUI change: the terminal app already shows remote sessions.
