# Intent: a commander that runs a fleet of agents

Type: feature
Author: Cody · Status: accepted
Date: 2026-10-11 · Plan: [phase 9](../../ROADMAP.md#9--one-agent-runs-a-fleet-for-you--active)

## Problem

"Today you run six agents by hand: you brief each one, you remember which is doing
what, and you carry messages between them."
([design brief v5](../../design-brief-v5.md))

## Proposed outcome

You give one agent, the **commander**, a goal. It breaks the work up, starts helpers in
their own worktrees, briefs them, checks on them and reports back. Together they are a
**fleet**. In the sidebar the helpers sit indented under their commander, each with its
own state.

## Affected users and systems

Anyone running several agents in one project. The actions list (starting a fleet), the
agent row menu (making an existing agent the commander), the sidebar, the daemon
(recording who started whom), the saved session, and the protocol version.

## Constraints

- A commander is an ordinary agent session with instructions, not a new kind of session.
- It only uses the seshi commands and tools agents already have, within its pane's
  grants.
- A helper that needs you still goes to the top of its group and shows in the Inbox.
- Its instructions say how to work a repo's plan and tickets in order when there are
  any; Seshi does not depend on the roadmap skill.
- The link between a commander and its helpers survives a daemon restart, a helper or
  commander waking from sleep, and a move into a worktree.
- "Started by" is only set by the daemon, from a pane whose secret checked out
  (`AGENTS.md`, project non-negotiable 5).

## Out of scope

- Personas: deferred 2026-10-11 (the author wants to think about them more).
- The designed team view, threads between agents, take over and hand back, pause the
  fleet, stop all, and cost per fleet: these wait for the design from brief v5.
- A "Work the plan" start option.
- New screens beyond the one box that asks for the goal.

## Open questions

## Tickets

#32, #33, #34, #35, #36, #37, #38
