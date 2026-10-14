# Spec: saved profile workflows

Intent: [intent.md](intent.md) · Status: accepted · Date: 2026-10-14

## Concerns

- Only the active stage's verified pane may report completion. Only a human
  connection may approve a plan. Existing grants and helper limits still apply.
- Native role instructions are behavioral, not an independent permission sandbox.
- A stage needs a nonempty report and a finished turn before advancing. A quiet
  pane, permission prompt, missing report, or launch failure must not advance it.
- Save workflow progress with sessions. A restart must never replay a task;
  interrupted execution requires an explicit human retry.
- Reuse the existing right sheet and motion settings; support keyboard and mouse
  and the 80×24 minimum. No new dependencies or network services.

## Requirements

- **R1.** Named personal/project workflows select exact planner, worker, and
  reviewer profiles; project names override personal names. Validate before launch.
- **R2.** Freeze effective profiles and create one task worktree. Handoffs include
  the original goal, approved plan, worker report, and latest review findings.
- **R3.** Stop after planning for human approval. Run at most three build/review
  rounds; pass completes the run and remaining findings exhaust the run.
- **R4.** Authenticate stage reports, require actual turn completion, and retain
  reports and progress across client close and daemon restart without replay.
- **R5.** CLI and MCP support discovering, inspecting, starting, and monitoring
  workflows. Provide a repository instruction snippet and prevent stage recursion.
- **R6.** Profiles, workflow editing, task entry, run progress, reports, approval,
  cancellation, and explicit retry are reachable in the right-hand sheet.

## Design

- Data: `[[workflows]]` in existing settings and `.seshi.toml`; each has `name`,
  `planner`, `worker`, and `reviewer`. Freeze definitions in a run, alongside its
  stage, round, reports, checkout, and pane references. Old saved sessions default
  to no runs. Bump the wire protocol for new commands and snapshot fields.
- Execution: daemon background jobs prepare the worktree and handoff files;
  daemon-owned panes launch the chosen native agents. An explicit report is
  accepted only from the current stage pane and consumed after its turn finishes.
  Missing reports and prompts stay visible. A retry preserves existing files.
- UI: reuse `client/hydra/sheet.rs` and motion. Profiles, Workflows, and Runs tabs;
  empty/loading/error states; keyboard shortcuts and clickable controls. Review
  reports before approval. A cancelled run starts no further stages and keeps files.
- Interfaces: `seshi workflow list|show|init|run|status|finish|approve|retry|cancel`;
  matching discovery/start/status MCP tools. Human controls remain human-only.

## Out of scope

See the intent. Do not edit the user's agent instruction files automatically.

## Verification

Domain tests cover validation, handoffs, approval and bounded review transitions.
Daemon tests cover authenticated reports, blocked/missing completion, failure,
cancellation, helper limits and restart mapping. UI tests cover sheet placement,
editing, launch and controls at supported sizes. Run the full AGENTS.md gate in
order, inspect rendered screens and check the built UI on a named nested server.

## Coverage

- R1 → workflow configuration and validation.
- R2, R3, R4 → daemon runner and persistence.
- R5 → CLI, MCP and usage documentation.
- R6 → right sheet, editors and run controls.
