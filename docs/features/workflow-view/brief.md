# Design brief: showing workflow runs

Type: feature · Status: brief for design · Date: 2026-11-13 · Tickets: MAITTY-364 (this brief),
MAITTY-365 (terminal app), MAITTY-366 (desktop app)

## What it is

A workflow such as Kitchen runs inside an agent session and takes tickets through fixed steps,
each done by a helper agent. It tells Maitty where each run is with `maitty workflow report`. Maitty
shows those runs; it never starts, orders or stops a step
([ADR 0010](../../adr/0010-workflows-and-specialist-agents-stay-outside-maitty.md)).

The person wants to see, at a glance: which tickets are being worked, which step each is on, which
one needs them, and who is doing the work right now.

## The data the design draws

Exactly what a report holds (`crates/maitty-shared/src/workflow.rs`); nothing else is known.

- **Run:** the workflow's name (`kitchen`), a run id, an optional ticket (key such as `MAITTY-42`, a
  title, a link), the time of its last report, and its steps in order.
- **Step:** a name (`intake`, `spec`, `build`, `checks`, `review`, `approve`), a state, an optional
  one-line note ("2 findings", "checks failed"), and optionally the label of the helper doing it.
- **States:** waiting, working, passed, failed, sent back, needs you, skipped. A state added later
  reads as unknown and is drawn as quiet text.
- **Where:** each run belongs to the pane (session) that reported it, so to its machine, project and
  branch too. A pane keeps its latest 20 runs.
- **Helper link:** a step's helper label matches one of that session's subagents. Where it matches,
  the step opens that subagent's own chat (desktop) or view (terminal); where it does not, nothing.

Workflow state is display only. A session's status mark still comes from the agent: when a run needs
the person, the agent has stopped to ask, so the session already reads needs you. The run view says
why.

## Rules from the design system

- Status first, and amber (`blocked`) means needs you and only that. A step that needs you is the
  loudest thing in the view, as the session's own mark is.
- Shape and word carry the meaning; colour backs it. Every step state reads in the Unicode glyph
  set and without colour. Suggested marks: waiting `·`, working the spinner, passed `✓`, failed `✕`,
  sent back `↺`, needs you `●`, skipped `–`; the designer may choose others from the glyph set.
- Theme roles only: `working` for working, `blocked` for needs you, `done` for passed, `err` for
  failed, `muted` for waiting, skipped and unknown. No new roles.
- Plain words, sentence case. A run is named by its ticket key and title, else the workflow and run id.

## Where it appears

**Terminal app** (34-column sidebar by default, 24 to 60; see DESIGN › *Sidebar*):

1. Under an agent session in the sidebar, after its subagents: one line per **active** run, the
   current step's mark, the ticket key and the current step's name, cut with `…` to fit
   (`├ ● MAITTY-43 spec`). A run is active until every step has passed, failed or been skipped.
   Finished runs fold into one quiet line (`└ 3 runs done`).
2. A detail view of one run, opened from that line: the ticket key and title, then each step on its
   own row with its mark, name, note and helper. Works in 80×24.

**Desktop app** (see DESIGN › *Desktop app*):

1. Under an agent session in the sidebar, the same active runs as compact rows, beside the subagent
   tiles, with the same fold for finished ones.
2. A run view in the session's chat area, like a subagent's chat: a bar ("Run · MAITTY-42 · Add
   export", Back to main session), then the steps as a vertical track: mark, name, note, helper. A
   step with a matching helper is a link that opens that subagent's chat.
3. Grouping by project already comes from the sidebar's folder view; no separate board for now.

## Cases the design must cover

1. A smooth run: intake ✓ spec ✓ build (working, helper "MAITTY-42 builder") review · approve ·.
2. A sent-back step: checks ✕ "2 tests failed", build ↺ working again.
3. A run that needs the person: spec ● "approve the spec".
4. Three runs at once under one session, one of them needing the person.
5. Finished runs folded away, and opened again.
6. A run with no ticket (named "kitchen 007"), and one whose step names are long.
7. A session on another machine: its runs show; a helper link opens the reported task and result only.

## Out of scope

Starting, pausing or reordering a workflow from Maitty; editing a run; a board across all projects;
anything a report does not hold (estimates, durations per step, cost).
