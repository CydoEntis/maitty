# Intent: the daily loop

Type: feature
Author: Cody · Status: accepted
Date: 2026-11-12 · Plan: Plane modules *Daily loop 1* to *Daily loop 4*

## Problem

"It just feels like we are missing features that should work… I want Maitty to be a
tool I can reliably use for all my day-to-day work." Sessions sometimes vanish from
the desktop sidebar. A status does not say where it came from. Reviewing what an agent
did in one turn, steering it, and starting several agents each take more steps than
they should. A comparison with T3 Code on 2026-11-12 listed the gaps; each was checked
against `dev` at `8200739cf`, and only what is still missing is planned.

## Proposed outcome

A session never vanishes or misreports, and every status says why. Reviewing a turn,
rewinding it, steering a running turn, and starting agents take fewer steps, in the
desktop app and, where the feature exists there, the TUI.

## Affected users and systems

People who run several agents across projects every day. The daemon's session list and
conversation history, the desktop chat, sidebar and New chat page, the TUI sidebar and
worktree list, the Codex adapter, and the user guide.

## Constraints

- The terminal stays the default and the source of truth. An agent runs as its own CLI
  in a daemon-owned terminal ([ADR 0001](../../adr/0001-the-daemon-owns-every-session.md),
  [ADR 0003](../../adr/0003-status-and-conversation-trust.md)).
- The TUI and the desktop app both stay supported
  ([ADR 0004](../../adr/0004-client-split-and-desktop-host.md)).
- Usage is never estimated ([ADR 0009](../../adr/0009-usage-is-provider-reported-never-estimated.md)).
- Only a worktree that is clean and merged is removed automatically. An inactive one is
  listed for the person to remove.
- Codex gets Approve and Decline through verified keys typed into its terminal, not by
  moving Codex chats to `codex app-server`.
- Rewind restores files and tells the agent. It does not rewind the agent's own
  conversation.

## Out of scope

- A phone app, a cloud relay, scheduled tasks and webhooks.
- Voice, HTML replies, browser automation, review guides and PR stacks.
- Searching chat history from the palette; it searches session output only.
- A model-written branch name.
- The TUI's status "why" popover, removed earlier on purpose.
- Custom lint rules and trial-child daemon updates.

## Open questions

None. How side questions about a session should work is split off to MAITTY-307.

## Tickets

- Daily loop 1, Trust: MAITTY-271 to MAITTY-286, and MAITTY-344 to MAITTY-347
- Daily loop 2, Review loop: MAITTY-287 to MAITTY-300, and MAITTY-348 to MAITTY-353
- Daily loop 3, Throughput: MAITTY-301 to MAITTY-307
- Daily loop 4, Hygiene and docs: MAITTY-308 to MAITTY-322, and MAITTY-354
