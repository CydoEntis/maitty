# Intent: radar and sending failures

Type: change + feature · Status: planned (phase 27) · Date: 2026-10-24

## Problem

You can't see an agent running out of context until it compacts or stalls, and when a check
fails, getting its output to the agent means copying it by hand.

## Goal

- **Radar**: a session row shows its context as `72%` once it passes half full, in red when it is
  nearly full. The data already reaches the window (`TermInfo.usage.context`).
- **Send failure**: from a failed check (its view, or its Inbox row), one key pre-fills the
  message box with the check's command, its exit code and its last 40 lines of output. Nothing
  is sent until you press Enter.

## Scope

Anyone running several agents. The client only: the sidebar row, the check view, the Inbox,
the message box.

## Decisions

- Radar reverses Phase 14's decision (2026-10-19) that model, context and cost are left to the
  agent's own status line; the user chose it on 2026-10-24, shown only past half full.
- Amber stays for "needs you" (UI.md): context is `muted`, then `err` when nearly full.
- Agents that report no context show nothing.
- "Nearly full" is 85% (decided 2026-10-24).
- The key is `m` on a failed check (its Inbox row or its check view), which opens the message
  box with the failure in it, as `m` already messages the agent on question and finished rows.
  It was free there: on a failed Inbox row it only started a search (decided 2026-10-24).

## Out of scope

- Cost on the row (the user is on a flat plan; the limits meter covers it).

## Links

- Plan: [phase 27](../../ROADMAP.md#27--radar-and-sending-failures--planned-2026-10-24)
- Tickets: none yet; filled in when the phase is ticketed.
