# Intent: saved profile workflows

Type: feature
Author: Cody · Status: accepted
Date: 2026-10-14 · Plan: [phase 9](../../ROADMAP.md#9--one-agent-runs-a-fleet-for-you--active)

## Problem

Saved profiles do not tell a session when to use the planner, worker, or reviewer,
or how to use a person's own roles. Users carry the plan and findings by hand.

## Proposed outcome

A saved workflow selects a planner, worker, and reviewer. Give a session an
ordinary implementation request; repository instructions can direct it to start
the workflow. Approve the plan, then let Seshi run up to three build/review rounds.
Manage profiles, workflows, and runs in a sheet sliding in from the right.

## Affected users and systems

Profile users, repository instructions, configuration, CLI/MCP, the client sheet,
daemon-owned sessions, protocol, and session persistence.

## Constraints

- The daemon owns execution, authenticated stage reports, and saved progress.
- The person approves the plan. Agents cannot approve their own plan.
- All stages share one task worktree and frozen profile definitions.
- Permission prompts never count as completion or approval.
- Repository instructions select a workflow; assigned stages must not nest it.

## Out of scope

Arbitrary stage graphs, enforced native read-only sandboxes, automatic commits,
merges, pushes, and redesigning the fleet team view.

## Open questions

## Tickets

Implementation is authorized in the current worktree. No remote tickets filed.

## Verification

Implemented for v0.29.0; full gate passed (263 passed, three pre-existing ignored).
Named-server integration exercised handoffs, approval, restart and three review
rounds with controlled stub agents. Live UI navigation was checked at 80×24.
Real-agent daily use remains a follow-up after release.
