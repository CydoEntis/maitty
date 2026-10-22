# Intent: clear session state and workflow controls

Type: change and feature
Author: Cody · Status: accepted, partly superseded 2026-10-18 (the workflow parts were removed; see [Out](../../ROADMAP.md#out))
Date: 2026-10-15 · Plan: [phase 9](../../ROADMAP.md#9--one-agent-runs-a-fleet-for-you--complete-2026-10-16-at-63b1a72)

## Problem

Moving the mouse over the sidebar hides which session is actually selected.
Users cannot easily tell whether an update is active, why an agent has its
status, or how to activate and follow a saved workflow.

## Proposed outcome

Keep active selection, keyboard browsing and hover distinct. Explain installed,
window and daemon versions, show status evidence and its age, prepare workflow
instructions for review, surface stage progress and plan approvals, and measure
switching, scrolling and resizing under sustained output.

## Constraints

The daemon owns sessions and status. Do not restart agents, answer questions,
approve plans, modify instruction files or change native permissions automatically.
Reuse Settings, the status popover and Inbox. Carry the pending
false-working and bell-removal fixes forward.

## Out of scope

Another layout, a new executor, automatic instruction-file edits or daemon
restarts, telemetry, and unrelated roadmap work.

## Open questions

## Implementation

Direct implementation authorized by the user; no remote tickets created.
See [spec](spec.md) for requirements and verification.
