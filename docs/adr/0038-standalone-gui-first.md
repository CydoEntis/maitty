# 0038. Make the standalone GUI usable before native terminal transfer

- **Status:** Accepted
- **Date:** 2026-10-29
- **Amends:** [0036](0036-chat-gui-shared-conversations.md)

## Context

The GUI needs to work as a complete desktop chat client before users need to
switch a conversation into the TUI. Native Codex handoff is implemented, but
its live acceptance is still open and it has added complexity to GUI validation.
The user wants the GUI usable for daily work and also requires Linux support,
including Omarchy.

## Decision

Prioritize the standalone Codex chat flow: create a conversation in a chosen
folder, send and receive messages, handle tool activity and approvals, cancel or
recover turns, inspect changes, reconnect, and close or reopen the GUI while the
daemon continues its work. Keep the native TUI and terminal-only install fully
supported.

Move live GUI-to-native-TUI transfer validation to follow-up work. It does not
block the initial GUI release. Before that release, verify Codex and Claude GUI
flows and desktop packaging on Windows, macOS and Linux. Build and document Linux
for Arch Linux and Omarchy as well as Ubuntu.

## Consequences

GUI work is measured against normal standalone chat use first. Existing handoff
code can remain, but users must not be told that live cross-interface continuity
is verified until its follow-up acceptance passes. The release still requires
both supported providers and all desktop targets.
