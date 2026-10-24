# Intent: radar and sending failures

Type: change + feature
Author: Cody · Status: accepted
Date: 2026-10-24 · Plan: [phase 27](../../ROADMAP.md#27--radar-and-sending-failures--planned-2026-10-24)

## Problem

You can't see an agent running out of context until it compacts or stalls, and when a check
fails, getting its output to the agent means copying it by hand.

## Proposed outcome

- **Radar**: a session row shows its context as `72%` once it passes half full, in red when it is
  nearly full. The data already reaches the window (`TermInfo.usage.context`).
- **Send failure**: from a failed check (its view, or its Inbox row), one key pre-fills the
  message box with the check's command, its exit code and its last 40 lines of output. Nothing
  is sent until you press Enter.

## Affected users and systems

Anyone running several agents. The client only: the sidebar row, the check view, the Inbox,
the message box.

## Constraints

- Radar reverses Phase 14's decision (2026-10-19) that model, context and cost are left to the
  agent's own status line; the user chose it on 2026-10-24, shown only past half full.
- Amber stays for "needs you" (UI.md): context is `muted`, then `err` when nearly full.
- Agents that report no context show nothing.

## Out

- Cost on the row (the user is on a flat plan; the limits meter covers it).

## Open questions

Decided 2026-10-24: "nearly full" is 85%. Open until phase 27 is ticketed: the key for send failure
(recommended: `m` on a failed check, which already opens the message box, pre-fills the failure).

## Tickets

(filled in when the phase is ticketed)
